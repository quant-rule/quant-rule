//! The simulation service contract: eleven endpoints, marks, isolation, and
//! the research-plane guard.
//!
//! The bundle and repository are in memory, so the tests bind no socket and
//! need no database. A restart is modelled by building a second router over the
//! same repository and object store.

mod common;

use std::sync::Arc;

use axum::http::StatusCode;
use common::{Fixture, MemoryObjects, MemoryStore, json, request, state, store_bundle};
use serde_json::json;

async fn create(
    state: Arc<quantrule_simulation_service::SimulationState>,
    profile: &str,
    fixture: &Fixture,
) -> String {
    let (status, body) = request(
        state,
        "POST",
        "/simulation/runs",
        profile,
        Some(json!({
            "premise": "A fib level test needs a close back on the entry side",
            "bundle": fixture.digest.to_string(),
            "inputs": "EMA stack and RSI zone",
            "seed": 0,
            "criteria": "min_events=30; mean net > 0; hit rate > 0.5"
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    json(&body)["sim_id"].as_str().expect("sim_id").to_owned()
}

async fn windows(
    state: Arc<quantrule_simulation_service::SimulationState>,
    profile: &str,
    id: &str,
) {
    let (status, body) = request(
        state,
        "POST",
        &format!("/simulation/runs/{id}/windows"),
        profile,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
}

#[tokio::test]
async fn the_eleven_endpoints_answer_in_order() {
    let store = Arc::new(MemoryStore::default());
    let objects = Arc::new(MemoryObjects::default());
    let fixture = store_bundle(&objects, 40).await;
    let service = state(Arc::clone(&store), Arc::clone(&objects));

    let id = create(Arc::clone(&service), "default", &fixture).await;

    // GET /simulation/runs
    let (status, body) = request(
        Arc::clone(&service),
        "GET",
        "/simulation/runs",
        "default",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rows = json(&body);
    assert_eq!(rows.as_array().expect("rows").len(), 1);
    assert_eq!(rows[0]["sim_id"], id);
    assert_eq!(rows[0]["status"], "draft");

    // GET /simulation/runs/{id}
    let (status, body) = request(
        Arc::clone(&service),
        "GET",
        &format!("/simulation/runs/{id}"),
        "default",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let detail = json(&body);
    assert_eq!(
        detail["premise"],
        "A fib level test needs a close back on the entry side"
    );
    assert_eq!(detail["counts"]["windows"], 0);

    // POST /simulation/runs/{id}/windows
    windows(Arc::clone(&service), "default", &id).await;
    let (status, body) = request(
        Arc::clone(&service),
        "GET",
        &format!("/simulation/runs/{id}"),
        "default",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json(&body)["status"], "marking");

    // GET /simulation/runs/{id}/windows/{n}/series
    let (status, body) = request(
        Arc::clone(&service),
        "GET",
        &format!("/simulation/runs/{id}/windows/0/series"),
        "default",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let series = json(&body);
    assert_eq!(series["chart"], 0);
    assert_eq!(series["candles"].as_array().expect("candles").len(), 400);
    assert_eq!(series["panel"]["indicator_code"], 2);
    assert!(series["ema_fast"].as_array().expect("ema").len() == 400);
    assert!(!series["events"].as_array().expect("events").is_empty());
    assert!(!series["levels"].as_array().expect("levels").is_empty());

    // POST /simulation/runs/{id}/marks
    let (status, body) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/marks"),
        "default",
        Some(json!({
            "chart": 0,
            "bar": 80,
            "action": "entry",
            "quality": "good",
            "side": "long",
            "note": "higher low into the level"
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let mark_id = json(&body)["mark_id"].as_str().expect("mark id").to_owned();

    // GET /simulation/runs/{id}/marks
    let (status, body) = request(
        Arc::clone(&service),
        "GET",
        &format!("/simulation/runs/{id}/marks"),
        "default",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let marks = json(&body);
    assert_eq!(marks.as_array().expect("marks").len(), 1);
    assert_eq!(marks[0]["bar"], 80);
    assert_eq!(marks[0]["origin"], "human");
    let stamp = marks[0]["marked_time_utc"].as_str().expect("stamp");
    assert!(stamp.ends_with('Z'), "{stamp}");

    // POST /simulation/runs/{id}/oracle
    let (status, body) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/oracle"),
        "default",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let oracle = json(&body);
    assert!(
        oracle
            .as_array()
            .expect("oracle")
            .iter()
            .all(|row| row["origin"] == "oracle")
    );
    assert!(!oracle.as_array().expect("oracle").is_empty());

    // POST /simulation/runs/{id}/check
    let (status, body) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/check"),
        "default",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let check = json(&body);
    assert!(check["events"].as_u64().expect("events") >= 30);
    assert!(check["passed"].is_boolean());
    assert!(check["failures"].is_array());

    // POST /simulation/runs/{id}/decision
    let (status, body) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/decision"),
        "default",
        Some(json!({"decision": "refine", "note": "one condition only"})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert_eq!(json(&body)["decision"], "refine");

    // DELETE /simulation/runs/{id}/marks/{mark_id} is locked now.
    let (status, body) = request(
        Arc::clone(&service),
        "DELETE",
        &format!("/simulation/runs/{id}/marks/{mark_id}"),
        "default",
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert!(String::from_utf8_lossy(&body).contains("simulation.locked"));
}

#[tokio::test]
async fn a_mark_survives_a_restart() {
    let store = Arc::new(MemoryStore::default());
    let objects = Arc::new(MemoryObjects::default());
    let fixture = store_bundle(&objects, 40).await;
    let first = state(Arc::clone(&store), Arc::clone(&objects));
    let id = create(Arc::clone(&first), "default", &fixture).await;
    windows(Arc::clone(&first), "default", &id).await;
    let (status, _) = request(
        Arc::clone(&first),
        "POST",
        &format!("/simulation/runs/{id}/marks"),
        "default",
        Some(json!({
            "chart": 0, "bar": 80, "action": "entry", "quality": "good",
            "side": "long", "note": "restart probe"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // A new process builds a new router over the durable repository.
    let second = state(Arc::clone(&store), Arc::clone(&objects));
    let (status, body) = request(
        second,
        "GET",
        &format!("/simulation/runs/{id}/marks"),
        "default",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json(&body)[0]["note"], "restart probe");
}

#[tokio::test]
async fn an_invalid_bar_is_rejected() {
    let store = Arc::new(MemoryStore::default());
    let objects = Arc::new(MemoryObjects::default());
    let fixture = store_bundle(&objects, 40).await;
    let service = state(store, objects);
    let id = create(Arc::clone(&service), "default", &fixture).await;
    windows(Arc::clone(&service), "default", &id).await;

    for bar in [9999_u64, 0] {
        let (status, body) = request(
            Arc::clone(&service),
            "POST",
            &format!("/simulation/runs/{id}/marks"),
            "default",
            Some(json!({
                "chart": 0, "bar": bar, "action": "entry", "quality": "good",
                "side": "long", "note": ""
            })),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "bar {bar}: {}",
            String::from_utf8_lossy(&body)
        );
        assert!(String::from_utf8_lossy(&body).contains("simulation.mark_invalid"));
    }
}

#[tokio::test]
async fn the_oracle_is_idempotent() {
    let store = Arc::new(MemoryStore::default());
    let objects = Arc::new(MemoryObjects::default());
    let fixture = store_bundle(&objects, 40).await;
    let service = state(Arc::clone(&store), Arc::clone(&objects));
    let id = create(Arc::clone(&service), "default", &fixture).await;
    windows(Arc::clone(&service), "default", &id).await;

    let (first_status, first) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/oracle"),
        "default",
        None,
    )
    .await;
    let (second_status, second) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/oracle"),
        "default",
        None,
    )
    .await;
    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(json(&first), json(&second));
    let (_, marks) = request(
        Arc::clone(&service),
        "GET",
        &format!("/simulation/runs/{id}/marks?origin=oracle"),
        "default",
        None,
    )
    .await;
    assert_eq!(
        json(&marks).as_array().expect("marks").len(),
        json(&first).as_array().expect("oracle").len()
    );

    // The check is idempotent for the same inputs digest.
    let (first_status, first) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/check"),
        "default",
        None,
    )
    .await;
    let (second_status, second) = request(
        service,
        "POST",
        &format!("/simulation/runs/{id}/check"),
        "default",
        None,
    )
    .await;
    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(json(&first), json(&second));
}

#[tokio::test]
async fn the_check_needs_thirty_events() {
    let store = Arc::new(MemoryStore::default());
    let objects = Arc::new(MemoryObjects::default());
    let fixture = store_bundle(&objects, 10).await;
    let service = state(store, objects);
    let id = create(Arc::clone(&service), "default", &fixture).await;
    windows(Arc::clone(&service), "default", &id).await;
    let (status, body) = request(
        service,
        "POST",
        &format!("/simulation/runs/{id}/check"),
        "default",
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert!(String::from_utf8_lossy(&body).contains("simulation.check_incomplete"));
}

#[tokio::test]
async fn a_profile_sees_only_its_own_runs() {
    let store = Arc::new(MemoryStore::default());
    let objects = Arc::new(MemoryObjects::default());
    let fixture = store_bundle(&objects, 40).await;
    let service = state(store, objects);
    let id = create(Arc::clone(&service), "default", &fixture).await;

    let (status, _) = request(
        Arc::clone(&service),
        "GET",
        &format!("/simulation/runs/{id}"),
        "alpha",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = request(
        Arc::clone(&service),
        "GET",
        &format!("/simulation/runs/{id}/marks"),
        "alpha",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, body) = request(
        Arc::clone(&service),
        "GET",
        "/simulation/runs",
        "alpha",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(json(&body).as_array().expect("rows").is_empty());

    // A missing profile header is refused rather than defaulted.
    let (status, body) = request(Arc::clone(&service), "GET", "/simulation/runs", "", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(String::from_utf8_lossy(&body).contains("profile.not_found"));
}

#[tokio::test]
async fn a_decision_locks_marking() {
    let store = Arc::new(MemoryStore::default());
    let objects = Arc::new(MemoryObjects::default());
    let fixture = store_bundle(&objects, 40).await;
    let service = state(store, objects);
    let id = create(Arc::clone(&service), "default", &fixture).await;
    windows(Arc::clone(&service), "default", &id).await;
    let (status, _) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/decision"),
        "default",
        Some(json!({"decision": "stop", "note": "no edge"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/marks"),
        "default",
        Some(json!({
            "chart": 0, "bar": 80, "action": "entry", "quality": "bad",
            "side": "short", "note": "late"
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert!(String::from_utf8_lossy(&body).contains("simulation.locked"));

    let (status, body) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/oracle"),
        "default",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(String::from_utf8_lossy(&body).contains("simulation.locked"));
}

#[tokio::test]
async fn the_mark_csv_mirror_is_written() {
    let store = Arc::new(MemoryStore::default());
    let objects = Arc::new(MemoryObjects::default());
    let fixture = store_bundle(&objects, 40).await;
    let service = state(Arc::clone(&store), Arc::clone(&objects));
    let id = create(Arc::clone(&service), "default", &fixture).await;
    windows(Arc::clone(&service), "default", &id).await;
    let (status, _) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/marks"),
        "default",
        Some(json!({
            "chart": 0, "bar": 80, "action": "entry", "quality": "good",
            "side": "long", "note": "a, quoted \"note\""
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let key = format!("simulation/runs/{id}/marks-human.csv");
    let text = String::from_utf8(objects.read(&key).expect("mirror")).expect("utf8");
    assert!(text.starts_with("chart,bar,marked_time_utc,action,quality,side,note\r\n"));
    assert!(text.contains("w001,80,"));
    assert!(text.contains("\"a, quoted \"\"note\"\"\""));
}

#[tokio::test]
async fn the_internal_surface_is_not_served() {
    let store = Arc::new(MemoryStore::default());
    let objects = Arc::new(MemoryObjects::default());
    let service = state(store, objects);
    let (status, _) = request(
        service,
        "GET",
        "/internal/v1/simulation/runs",
        "default",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_shell_create_and_string_chart_forms_are_accepted() {
    let store = Arc::new(MemoryStore::default());
    let objects = Arc::new(MemoryObjects::default());
    let fixture = store_bundle(&objects, 40).await;
    let service = state(store, objects);

    // The shell's create form predates the inputs field; the browser sends it.
    let (status, body) = request(
        Arc::clone(&service),
        "POST",
        "/simulation/runs",
        "default",
        Some(json!({
            "premise": "shell premise",
            "bundle": fixture.digest.to_string(),
            "seed": 0,
            "criteria": "min_events=30"
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let id = json(&body)["sim_id"].as_str().expect("sim_id").to_owned();
    windows(Arc::clone(&service), "default", &id).await;

    // The shell sends the chart as text; the browser sends a number.
    let (status, body) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/marks"),
        "default",
        Some(json!({
            "chart": "0", "bar": 80, "action": "entry", "quality": "good",
            "side": "long", "note": "shell chart"
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert!(json(&body)["mark_id"].is_string());
}

#[tokio::test]
async fn a_mark_can_be_deleted_before_the_decision() {
    let store = Arc::new(MemoryStore::default());
    let objects = Arc::new(MemoryObjects::default());
    let fixture = store_bundle(&objects, 40).await;
    let service = state(Arc::clone(&store), Arc::clone(&objects));
    let id = create(Arc::clone(&service), "default", &fixture).await;
    windows(Arc::clone(&service), "default", &id).await;
    let (status, body) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/marks"),
        "default",
        Some(json!({
            "chart": 0, "bar": 80, "action": "entry", "quality": "good",
            "side": "long", "note": "to delete"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let mark_id = json(&body)["mark_id"].as_str().expect("mark id").to_owned();

    let (status, _) = request(
        Arc::clone(&service),
        "DELETE",
        &format!("/simulation/runs/{id}/marks/{mark_id}"),
        "default",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, body) = request(
        Arc::clone(&service),
        "GET",
        &format!("/simulation/runs/{id}/marks"),
        "default",
        None,
    )
    .await;
    assert!(json(&body).as_array().expect("marks").is_empty());
    let text = String::from_utf8(
        objects
            .read(&format!("simulation/runs/{id}/marks-human.csv"))
            .expect("mirror"),
    )
    .expect("utf8");
    assert_eq!(
        text,
        "chart,bar,marked_time_utc,action,quality,side,note\r\n"
    );

    // A second delete is a not-found, not a silent success.
    let (status, body) = request(
        Arc::clone(&service),
        "DELETE",
        &format!("/simulation/runs/{id}/marks/{mark_id}"),
        "default",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(String::from_utf8_lossy(&body).contains("simulation.not_found"));
}

#[tokio::test]
async fn distinct_premises_get_distinct_runs() {
    let store = Arc::new(MemoryStore::default());
    let objects = Arc::new(MemoryObjects::default());
    let fixture = store_bundle(&objects, 40).await;
    let service = state(store, objects);
    let mut ids = Vec::new();
    for premise in ["first premise", "second premise"] {
        let (status, body) = request(
            Arc::clone(&service),
            "POST",
            "/simulation/runs",
            "default",
            Some(json!({
                "premise": premise,
                "bundle": fixture.digest.to_string(),
                "inputs": "EMA stack",
                "seed": 0,
                "criteria": "min_events=30"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        ids.push(json(&body)["sim_id"].as_str().expect("sim_id").to_owned());
    }
    assert_ne!(ids[0], ids[1]);
    let (_, body) = request(service, "GET", "/simulation/runs", "default", None).await;
    assert_eq!(json(&body).as_array().expect("rows").len(), 2);
}

#[test]
fn the_service_isolation_guard_holds() {
    use std::path::{Path, PathBuf};

    fn sources(root: &Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(root).expect("read dir") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                sources(&path, found);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo = std::fs::read_to_string(manifest.join("Cargo.toml")).expect("manifest");
    for forbidden in [
        "quantrule-app",
        "quantrule-model",
        "quantrule-backtest",
        "quantrule-data",
        "quantrule-service-api",
        "quantrule-research",
    ] {
        assert!(
            !cargo.contains(&format!("{forbidden} =")),
            "the simulation service depends on {forbidden}"
        );
    }
    assert!(
        !cargo.contains("rabbitmq"),
        "the simulation service depends on a queue client"
    );

    let mut files = Vec::new();
    sources(&manifest.join("src"), &mut files);
    assert!(!files.is_empty(), "no sources found");
    for file in files {
        let relative = file
            .strip_prefix(&manifest)
            .expect("relative")
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(&file).expect("source");
        for forbidden in [
            "quantrule_model::",
            "quantrule_backtest::",
            "quantrule_app::",
            "compute_model",
            "model_run",
            "RunManifest",
            "blob_key",
            "RunStore",
            "RabbitQueue",
            "ROUTING_",
            "INSERT INTO runs.",
            "UPDATE runs.",
            "DELETE FROM runs.",
            "FROM runs.",
            "strategy.json",
            "\"artifacts/",
        ] {
            assert!(!text.contains(forbidden), "{relative} names {forbidden}");
        }
    }

    let migration = std::fs::read_to_string(manifest.join("migrations/0014_simulation.sql"))
        .expect("migration");
    for forbidden in [
        "catalog.",
        "research.",
        "gateway.",
        "strategy_",
        "strategy.",
        "INSERT INTO runs.",
        "artifacts",
    ] {
        assert!(
            !migration.contains(forbidden),
            "the migration names {forbidden}"
        );
    }

    let postgres = std::fs::read_to_string(manifest.join("src/postgres.rs")).expect("postgres");
    for forbidden in [
        "FROM runs.",
        "INTO runs.",
        "UPDATE runs.",
        "DELETE FROM runs.",
        "catalog.",
        "research.",
        "gateway.",
    ] {
        assert!(
            !postgres.contains(forbidden),
            "the repository names {forbidden}"
        );
    }
}
