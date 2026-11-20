//! The internal model-bundle reference.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{body_bytes, seed, state};
use quantrule_api::{Digest, ModelObject, PROFILE_HEADER};
use quantrule_model_service::router;
use quantrule_service_api::{MODEL_BUNDLE_SCHEMA, ModelBundleManifest};
use tower::ServiceExt;

fn bundle_request(version: Digest) -> Request<Body> {
    Request::post("/internal/v1/model/bundle")
        .header(PROFILE_HEADER, "default")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "data_version": version,
                "config_id": "cfg_valid",
            })
            .to_string(),
        ))
        .expect("request")
}

#[tokio::test]
async fn the_manifest_lists_every_artifact_with_its_hash() {
    let seed = seed().await;
    let state = state(&seed);
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(bundle_request(seed.data_version.clone()))
        .await
        .expect("response");
    let status = response.status();
    if status != StatusCode::OK {
        let body = body_bytes(response).await;
        panic!("status {status}: {}", String::from_utf8_lossy(&body));
    }
    let manifest: ModelBundleManifest =
        serde_json::from_slice(&body_bytes(response).await).expect("manifest");

    assert_eq!(manifest.schema, MODEL_BUNDLE_SCHEMA);
    assert_eq!(manifest.data_version, seed.data_version);
    assert_eq!(manifest.config_id, "cfg_valid");
    assert_eq!(
        manifest.ladder,
        vec![
            "1m".to_owned(),
            "5m".to_owned(),
            "15m".to_owned(),
            "1h".to_owned()
        ]
    );
    assert!(
        manifest.manifest_key.ends_with("/manifest.json"),
        "{}",
        manifest.manifest_key
    );
    let names: Vec<&str> = manifest
        .artifacts
        .iter()
        .map(|artifact| artifact.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec![
            "events.csv",
            "legs.csv",
            "levels.csv",
            "provenance.json",
            "readings.csv",
            "report.json",
            "result.json",
            "simulation-15m.json",
            "simulation-1h.json",
            "simulation-1m.json",
            "simulation-5m.json",
            "summary.txt"
        ]
    );

    let prefix = manifest
        .manifest_key
        .strip_suffix("/manifest.json")
        .expect("bundle prefix");
    let objects = state.objects.as_ref().expect("objects");
    for artifact in &manifest.artifacts {
        let key = format!("{prefix}/{}", artifact.name);
        let bytes = objects.get(&key).await.expect("artifact");
        assert_eq!(bytes.len() as u64, artifact.bytes, "{}", artifact.name);
        assert_eq!(Digest::sha256(&bytes), artifact.sha256, "{}", artifact.name);
        if artifact.name.starts_with("simulation-") {
            let simulation_key = quantrule_api::simulation::bundle_key(&artifact.sha256);
            let stored = objects
                .get(&simulation_key)
                .await
                .expect("simulation bundle");
            assert_eq!(Digest::sha256(&stored), artifact.sha256);
        }
    }
    assert!(
        !names.iter().any(|name| name.contains("strategy")
            || name.contains("decision")
            || name.contains("execution")
            || name.contains("portfolio")),
        "the model bundle carries a strategy, execution, or portfolio artifact: {names:?}"
    );

    let manifest_bytes = objects.get(&manifest.manifest_key).await.expect("manifest");
    let stored: ModelBundleManifest =
        serde_json::from_slice(&manifest_bytes).expect("stored manifest");
    assert_eq!(stored, manifest);
}

#[tokio::test]
async fn the_bundle_report_carries_the_same_model_output_as_the_public_object() {
    let seed = seed().await;
    let state = state(&seed);
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::post("/model/compute")
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "data_version": seed.data_version.clone(),
                        "config_id": "cfg_valid",
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let object: ModelObject =
        serde_json::from_slice(&body_bytes(response).await).expect("model object");
    let public = state
        .objects
        .as_ref()
        .expect("objects")
        .get(&object.object_key)
        .await
        .expect("public model bytes");

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(bundle_request(seed.data_version.clone()))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let manifest: ModelBundleManifest =
        serde_json::from_slice(&body_bytes(response).await).expect("manifest");
    let prefix = manifest
        .manifest_key
        .strip_suffix("/manifest.json")
        .expect("bundle prefix");
    let report = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.name == "report.json")
        .expect("report artifact");
    let stored = state
        .objects
        .as_ref()
        .expect("objects")
        .get(&format!("{prefix}/report.json"))
        .await
        .expect("report bytes");
    assert_eq!(Digest::sha256(&stored), report.sha256);
    assert_eq!(stored.len() as u64, report.bytes);

    // The public object names the content digest; the coupled bundle names the
    // config-declared data version, exactly as the coupled `backtest run` writer
    // does. Every model number must still be identical.
    let public: serde_json::Value = serde_json::from_slice(&public).expect("public object");
    let bundle: serde_json::Value = serde_json::from_slice(&stored).expect("bundle report");
    assert_ne!(public["data_version"], bundle["data_version"]);
    assert_eq!(public["view_contract"], bundle["view_contract"]);
    assert_eq!(public["requested_period"], bundle["requested_period"]);
    assert_eq!(public["flat_report"], bundle["flat_report"]);
}

#[tokio::test]
async fn a_second_request_returns_the_same_manifest_and_artifacts() {
    let seed = seed().await;
    let state = state(&seed);
    let first = router(std::sync::Arc::clone(&state))
        .oneshot(bundle_request(seed.data_version.clone()))
        .await
        .expect("response");
    assert_eq!(first.status(), StatusCode::OK);
    let first: ModelBundleManifest =
        serde_json::from_slice(&body_bytes(first).await).expect("first manifest");
    let mut before_keys = seed.objects.keys();

    let second = router(state)
        .oneshot(bundle_request(seed.data_version.clone()))
        .await
        .expect("response");
    assert_eq!(second.status(), StatusCode::OK);
    let second: ModelBundleManifest =
        serde_json::from_slice(&body_bytes(second).await).expect("second manifest");
    assert_eq!(second, first);
    let mut after_keys = seed.objects.keys();
    before_keys.sort();
    after_keys.sort();
    assert_eq!(after_keys, before_keys);
}

#[tokio::test]
async fn an_unknown_dataset_is_not_found_internally() {
    let seed = seed().await;
    let response = router(state(&seed))
        .oneshot(bundle_request(Digest::sha256(b"missing")))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("dataset.not_found"), "{text}");
}
