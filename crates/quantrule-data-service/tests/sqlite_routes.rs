//! The catalog routes served from the SQLite stores.
//!
//! The test builds the same state the desktop branch builds: one SQLite
//! database, the catalog baseline applied to the shared ledger, the dataset and
//! config stores, and a `sqlite` health probe. It then exercises the real
//! router for a dataset upload, a duplicate lookup, profile scoping, a config
//! upload, and the internal reference routes. The binary is a thin branch over
//! these pieces; the manual verification in the AI log runs that branch.

mod common;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{CSV, MemoryObjects, body_bytes};
use quantrule_api::{
    ConfigSummary, DatasetRecord, DatasetSummary, HealthReport, PROFILE_HEADER, ServiceState,
};
use quantrule_data_service::migrations;
use quantrule_data_service::sqlite::{SqliteConfigStore, SqliteDatasetCatalog};
use quantrule_data_service::{DataServices, DataState, router};
use quantrule_runtime::adapters::Database;
use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};
use quantrule_runtime::{Config, HealthRegistry};
use quantrule_service_api::{ConfigBundle, DatasetObject};
use serde_json::Value;
use tower::ServiceExt;

async fn state() -> Arc<DataState> {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "catalog", &migrations::sqlite())
        .await
        .expect("the catalog baseline");

    let mut health = HealthRegistry::new();
    let ping = database.clone();
    health.register("sqlite", move || {
        let ping = ping.clone();
        async move { ping.ping().await.map_err(|error| error.to_string()) }
    });

    let config = Config {
        config_root: Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config"),
        ..Config::default()
    };
    Arc::new(DataState::with_services(
        &config,
        health,
        DataServices {
            catalog: Some(Arc::new(SqliteDatasetCatalog::new(database.clone()))),
            configs: Some(Arc::new(SqliteConfigStore::new(database))),
            objects: Some(Arc::new(MemoryObjects::default())),
        },
    ))
}

async fn request(state: Arc<DataState>, request: Request<Body>) -> axum::response::Response {
    router(state).oneshot(request).await.expect("the response")
}

fn upload(profile: &str) -> Request<Body> {
    Request::post("/catalog/datasets?name=candles.csv")
        .header(PROFILE_HEADER, profile)
        .body(Body::from(CSV))
        .expect("the dataset upload request")
}

async fn list_datasets(state: Arc<DataState>, profile: &str) -> Vec<DatasetSummary> {
    let response = request(
        state,
        Request::get("/catalog/datasets")
            .header(PROFILE_HEADER, profile)
            .body(Body::empty())
            .expect("the request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&body_bytes(response).await).expect("the dataset list")
}

fn config_bundle() -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config/valid");
    let mut run: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("run.json")).expect("run"))
            .expect("run json");
    run["name"] = serde_json::json!("sqlite-uploaded-run");
    let mut files = serde_json::Map::new();
    files.insert(
        "run.json".to_owned(),
        serde_json::json!(serde_json::to_string(&run).expect("run text")),
    );
    for name in [
        "data.json",
        "model.json",
        "execution.json",
        "portfolio.json",
    ] {
        files.insert(
            name.to_owned(),
            serde_json::json!(std::fs::read_to_string(root.join(name)).expect("fixture")),
        );
    }
    serde_json::json!({ "name": "sqlite-uploaded", "files": files })
}

#[tokio::test]
async fn health_names_sqlite_and_the_dataset_routes_use_the_sqlite_catalog() {
    let state = state().await;

    let response = request(
        Arc::clone(&state),
        Request::get("/health")
            .body(Body::empty())
            .expect("the request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let report: HealthReport =
        serde_json::from_slice(&body_bytes(response).await).expect("the health report");
    let sqlite = report
        .services
        .iter()
        .find(|service| service.name == "sqlite")
        .expect("the sqlite probe");
    assert_eq!(sqlite.state, ServiceState::Healthy);

    let response = request(Arc::clone(&state), upload("default")).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let record: DatasetRecord =
        serde_json::from_slice(&body_bytes(response).await).expect("the dataset record");
    assert_eq!(record.summary.name, "candles.csv");
    assert_eq!(record.summary.candles, 2);

    // A second upload of the same bytes is a lookup of the stored row.
    let response = request(Arc::clone(&state), upload("default")).await;
    assert_eq!(response.status(), StatusCode::OK);
    let duplicate: DatasetRecord =
        serde_json::from_slice(&body_bytes(response).await).expect("the duplicate record");
    assert_eq!(duplicate, record);

    assert_eq!(list_datasets(Arc::clone(&state), "default").await.len(), 1);
    assert!(
        list_datasets(Arc::clone(&state), "alpha").await.is_empty(),
        "the dataset row leaked into another profile"
    );

    let wire = record.summary.data_version.to_string();
    let response = request(
        Arc::clone(&state),
        Request::get(format!("/internal/v1/datasets/{wire}"))
            .header(PROFILE_HEADER, "default")
            .body(Body::empty())
            .expect("the request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let object: DatasetObject =
        serde_json::from_slice(&body_bytes(response).await).expect("the dataset object");
    assert_eq!(object.data_version, record.summary.data_version);
    assert_eq!(object.candles, record.summary.candles);

    let response = request(
        Arc::clone(&state),
        Request::get("/internal/v1/profile/datasets/count")
            .header(PROFILE_HEADER, "default")
            .body(Body::empty())
            .expect("the request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let count: u64 = serde_json::from_slice(&body_bytes(response).await).expect("the count");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn the_config_routes_use_the_sqlite_store_and_one_shared_bundle() {
    let state = state().await;

    let response = request(
        Arc::clone(&state),
        Request::post("/catalog/configs")
            .header(PROFILE_HEADER, "default")
            .header("content-type", "application/json")
            .body(Body::from(config_bundle().to_string()))
            .expect("the request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let uploaded: ConfigSummary =
        serde_json::from_slice(&body_bytes(response).await).expect("the config summary");
    assert!(uploaded.valid);
    assert!(uploaded.config_hash.is_some());

    let response = request(
        Arc::clone(&state),
        Request::get("/catalog/configs")
            .header(PROFILE_HEADER, "default")
            .body(Body::empty())
            .expect("the request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let listed: Vec<ConfigSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("the config list");
    assert_eq!(listed.len(), 4, "three fixture configs plus the upload");
    assert!(
        listed
            .iter()
            .any(|config| config.config_id == uploaded.config_id)
    );

    let response = request(
        Arc::clone(&state),
        Request::get(format!("/internal/v1/configs/{}", uploaded.config_id))
            .header(PROFILE_HEADER, "default")
            .body(Body::empty())
            .expect("the request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let bundle: ConfigBundle =
        serde_json::from_slice(&body_bytes(response).await).expect("the config bundle");
    assert_eq!(bundle.config_id, uploaded.config_id);
    assert!(bundle.valid);
    assert_eq!(bundle.files.len(), 5);
    let files: BTreeMap<String, String> = bundle.files;
    assert!(files.contains_key("run.json"));

    // The fixture scan writes the default profile; alpha gets its own rows and
    // does not see the upload.
    let response = request(
        Arc::clone(&state),
        Request::get("/catalog/configs")
            .header(PROFILE_HEADER, "alpha")
            .body(Body::empty())
            .expect("the request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let alpha: Vec<ConfigSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("the alpha list");
    assert_eq!(alpha.len(), 3);
    assert!(
        !alpha
            .iter()
            .any(|config| config.config_id == uploaded.config_id)
    );
}

const TOLERANT_CSV: &str = "date,open,high,low,close,volume\n\
2024-01-01 00:00:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:01:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:01:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:02:30+00:00,100,101,99,100.5,10\n\
2024-01-01 00:02:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:01:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:03:00+00:00,100,90,95,99,10\n\
2024-01-01 00:04:00+00:00,100,101,99,100.5,10\n";

#[tokio::test]
async fn a_tolerant_upload_round_trips_its_quality_through_sqlite() {
    let state = state().await;
    let response = request(
        Arc::clone(&state),
        Request::post("/catalog/datasets/tolerant?name=tolerant.csv")
            .header(PROFILE_HEADER, "default")
            .body(Body::from(TOLERANT_CSV))
            .expect("the request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let record: DatasetRecord =
        serde_json::from_slice(&body_bytes(response).await).expect("the record");
    assert_eq!(record.summary.candles, 4);
    let quality = record.quality.clone().expect("the quality report");
    assert_eq!(quality.rows_read, 8);
    assert_eq!(quality.accepted, 4);
    assert_eq!(quality.rejected, 4);

    let listed = list_datasets(Arc::clone(&state), "default").await;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0], record.summary);

    let response = request(
        state,
        Request::get(format!("/catalog/datasets/{}", record.summary.data_version))
            .header(PROFILE_HEADER, "default")
            .body(Body::empty())
            .expect("the request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let stored: DatasetRecord =
        serde_json::from_slice(&body_bytes(response).await).expect("the stored record");
    assert_eq!(stored.quality, Some(quality));
    assert_eq!(stored, record);
}
