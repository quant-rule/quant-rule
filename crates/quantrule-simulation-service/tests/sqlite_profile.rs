//! The simulation service on the embedded SQLite profile.
//!
//! The test builds the same state the desktop branch builds: one SQLite
//! database with the simulation baseline applied to the shared ledger, the
//! SQLite repository, and a `sqlite` health probe. It then drives the real
//! router: the research loop, profile isolation, a restart over the same file,
//! and the CSV mirror. The in-memory object store stands in for the bundle
//! objects, because this change owns the database repository, not the object
//! store wiring.

mod common;

use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{Fixture, MemoryObjects, json as body_json, request, store_bundle};
use quantrule_api::{HealthReport, ServiceState};
use quantrule_runtime::adapters::Database;
use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};
use quantrule_runtime::{Config, HealthRegistry};
use quantrule_simulation_service::sqlite::SqliteSimulationStore;
use quantrule_simulation_service::{SimulationServices, SimulationState, router};
use tower::ServiceExt;

async fn migrated() -> SqliteDatabase {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    let applied = apply_migrations_in(
        &database,
        "simulation",
        &quantrule_simulation_service::migrations::sqlite(),
    )
    .await
    .expect("the simulation baseline");
    assert_eq!(applied, 1);
    database
}

fn state(database: SqliteDatabase, objects: Arc<MemoryObjects>) -> Arc<SimulationState> {
    let mut health = HealthRegistry::new();
    let ping = database.clone();
    health.register("sqlite", move || {
        let ping = ping.clone();
        async move { ping.ping().await.map_err(|error| error.to_string()) }
    });
    Arc::new(SimulationState::with_services(
        &Config::default(),
        health,
        SimulationServices {
            store: Some(Arc::new(SqliteSimulationStore::new(database))),
            objects: Some(objects),
        },
    ))
}

async fn create(state: Arc<SimulationState>, profile: &str, fixture: &Fixture) -> String {
    let (status, body) = request(
        state,
        "POST",
        "/simulation/runs",
        profile,
        Some(serde_json::json!({
            "premise": "a fib level needs a close back on the entry side",
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
    body_json(&body)["sim_id"]
        .as_str()
        .expect("sim_id")
        .to_owned()
}

async fn select_windows(state: Arc<SimulationState>, profile: &str, id: &str) {
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

async fn put_mark(state: Arc<SimulationState>, profile: &str, id: &str, note: &str) {
    let (status, body) = request(
        state,
        "POST",
        &format!("/simulation/runs/{id}/marks"),
        profile,
        Some(serde_json::json!({
            "chart": 0, "bar": 80, "action": "entry", "quality": "good",
            "side": "long", "note": note
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
}

#[tokio::test]
async fn health_names_sqlite() {
    let objects = Arc::new(MemoryObjects::default());
    let response = router(state(migrated().await, objects))
        .oneshot(
            Request::get("/health")
                .body(Body::empty())
                .expect("the request"),
        )
        .await
        .expect("the health response");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 65_536)
        .await
        .expect("the health body");
    let report: HealthReport = serde_json::from_slice(&bytes).expect("the health report");
    assert_eq!(report.services.len(), 1);
    assert_eq!(report.services[0].name, "sqlite");
    assert_eq!(report.state, ServiceState::Healthy);
}

#[tokio::test]
async fn the_research_loop_runs_on_sqlite() {
    let objects = Arc::new(MemoryObjects::default());
    let fixture = store_bundle(&objects, 40).await;
    let service = state(migrated().await, Arc::clone(&objects));
    let id = create(Arc::clone(&service), "default", &fixture).await;

    select_windows(Arc::clone(&service), "default", &id).await;
    let (status, body) = request(
        Arc::clone(&service),
        "GET",
        &format!("/simulation/runs/{id}"),
        "default",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let detail = body_json(&body);
    assert_eq!(detail["status"], "marking");
    assert!(
        detail["counts"]["windows"]
            .as_u64()
            .expect("the window count")
            > 0
    );

    put_mark(
        Arc::clone(&service),
        "default",
        &id,
        "higher low into the level",
    )
    .await;
    let (status, body) = request(
        Arc::clone(&service),
        "GET",
        &format!("/simulation/runs/{id}/marks"),
        "default",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let marks = body_json(&body);
    assert_eq!(marks.as_array().expect("marks").len(), 1);
    assert_eq!(marks[0]["origin"], "human");
    assert!(
        marks[0]["marked_time_utc"]
            .as_str()
            .expect("the mark time")
            .ends_with('Z')
    );

    // The oracle replaces its own rows and is idempotent.
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
    assert_eq!(body_json(&first), body_json(&second));
    assert!(
        body_json(&first)
            .as_array()
            .expect("the oracle")
            .iter()
            .all(|row| row["origin"] == "oracle")
    );

    // The check is idempotent for the same inputs.
    let (first_status, first) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/check"),
        "default",
        None,
    )
    .await;
    let (second_status, second) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/check"),
        "default",
        None,
    )
    .await;
    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(body_json(&first), body_json(&second));
    assert!(body_json(&first)["events"].as_u64().expect("events") >= 30);

    // The decision lands once and closes marking.
    let (status, body) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/decision"),
        "default",
        Some(serde_json::json!({"decision": "refine", "note": "one condition only"})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let decided = body_json(&body);
    let (status, repeated) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/decision"),
        "default",
        Some(serde_json::json!({"decision": "refine", "note": "one condition only"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body_json(&repeated), decided);

    let (status, body) = request(
        Arc::clone(&service),
        "POST",
        &format!("/simulation/runs/{id}/marks"),
        "default",
        Some(serde_json::json!({
            "chart": 0, "bar": 80, "action": "entry", "quality": "bad",
            "side": "short", "note": "late"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(String::from_utf8_lossy(&body).contains("simulation.locked"));

    let (status, body) = request(
        service,
        "DELETE",
        &format!("/simulation/runs/{id}/marks/unknown"),
        "default",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(String::from_utf8_lossy(&body).contains("simulation.locked"));
}

#[tokio::test]
async fn a_profile_sees_only_its_own_runs_on_sqlite() {
    let objects = Arc::new(MemoryObjects::default());
    let fixture = store_bundle(&objects, 40).await;
    let service = state(migrated().await, Arc::clone(&objects));
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
    let (status, body) = request(
        Arc::clone(&service),
        "GET",
        "/simulation/runs",
        "alpha",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body_json(&body).as_array().expect("rows").is_empty());
    let (status, _) = request(
        Arc::clone(&service),
        "GET",
        &format!("/simulation/runs/{id}/marks"),
        "alpha",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_mark_survives_a_restart_on_sqlite() {
    let objects = Arc::new(MemoryObjects::default());
    let fixture = store_bundle(&objects, 40).await;
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("simulation-sqlite-restart");
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("the old restart directory");
    }
    let path = directory.join("quantrule.sqlite");

    let id = {
        let database = SqliteDatabase::open(&path).expect("the database opens");
        let applied = apply_migrations_in(
            &database,
            "simulation",
            &quantrule_simulation_service::migrations::sqlite(),
        )
        .await
        .expect("the simulation baseline");
        assert_eq!(applied, 1);
        let service = state(database, Arc::clone(&objects));
        let id = create(Arc::clone(&service), "default", &fixture).await;
        select_windows(Arc::clone(&service), "default", &id).await;
        put_mark(Arc::clone(&service), "default", &id, "survives a restart").await;
        id
    };

    // A new process opens the same file; the baseline is already applied.
    let database = SqliteDatabase::open(&path).expect("the reopened database");
    let applied = apply_migrations_in(
        &database,
        "simulation",
        &quantrule_simulation_service::migrations::sqlite(),
    )
    .await
    .expect("the repeated baseline");
    assert_eq!(applied, 0, "the second open re-applied a migration");
    let service = state(database, objects);
    let (status, body) = request(
        service,
        "GET",
        &format!("/simulation/runs/{id}/marks"),
        "default",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let marks = body_json(&body);
    assert_eq!(marks.as_array().expect("marks").len(), 1);
    assert_eq!(marks[0]["note"], "survives a restart");
}

#[tokio::test]
async fn the_mark_csv_mirror_is_written_on_sqlite() {
    let objects = Arc::new(MemoryObjects::default());
    let fixture = store_bundle(&objects, 40).await;
    let service = state(migrated().await, Arc::clone(&objects));
    let id = create(Arc::clone(&service), "default", &fixture).await;
    select_windows(Arc::clone(&service), "default", &id).await;
    put_mark(Arc::clone(&service), "default", &id, "a, quoted \"note\"").await;

    let key = format!("simulation/runs/{id}/marks-human.csv");
    let text = String::from_utf8(objects.read(&key).expect("the mirror")).expect("UTF-8");
    assert!(text.starts_with("chart,bar,marked_time_utc,action,quality,side,note\r\n"));
    assert!(text.contains("w001,80,"));
    assert!(text.contains("\"a, quoted \"\"note\"\"\""));
}
