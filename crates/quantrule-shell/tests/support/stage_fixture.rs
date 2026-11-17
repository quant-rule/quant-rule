//! The default fixture's diagnostic stage surface.
//!
//! The stage is diagnostic: its detail carries `evidence: false` and no
//! canonical digest. The two artifacts are the published model files the shell
//! reads; the fixture serves their bytes unchanged.

use serde_json::{Value, json};

use super::{Response, config_hash, data_version, digest, image_digest};

/// The stage-run fixture.
pub fn stage_id() -> String {
    digest('f')
}

/// The model stage event artifact fixture.
pub fn stage_events_body() -> Vec<u8> {
    b"timeframe_secs,event_index,event_code\n300,0,1\n900,0,2\n300,1,3\n".to_vec()
}

/// The model stage report artifact fixture.
pub fn stage_report_body() -> Vec<u8> {
    br#"{"model_version":"1.2.0","view_contract":"quantrule.model-view/v4","data_version":"fixture","requested_period":{"from":null,"to":null},"flat_report":{"timeframes":[{"timeframe_secs":900,"bars":0,"expected_bars":1,"coverage_share":0.0},{"timeframe_secs":300,"bars":3,"expected_bars":3,"coverage_share":1.0}]}}"#.to_vec()
}

/// A stage detail with the two model artifacts.
pub fn stage_detail(state: &str) -> Value {
    let events = stage_events_body();
    let report = stage_report_body();
    json!({
        "stage_run_id": stage_id(),
        "kind": "model",
        "state": state,
        "owner": "default",
        "config_id": "cfg_abc",
        "input": { "kind": "data_version", "data_version": data_version() },
        "provenance": {
            "data_version": data_version(),
            "config_hash": config_hash(),
            "model_version": "M1.0",
            "image_digest": image_digest(),
        },
        "artifact": if state == "succeeded" {
            json!({
                "artifact_id": digest('9'),
                "files": [
                    { "name": "events.csv", "bytes": events.len(), "sha256": quantrule_api::Digest::sha256(&events).to_string() },
                    { "name": "report.json", "bytes": report.len(), "sha256": quantrule_api::Digest::sha256(&report).to_string() },
                ],
            })
        } else {
            Value::Null
        },
        "submitted_at": "2026-11-17T00:00:00Z",
        "error": if state == "failed" { json!("the model failed") } else { Value::Null },
        "attempts": 1,
        "evidence": false,
    })
}

/// The shared client-parity responses.
///
/// One file serves the shell fixture server and the browser Bun test, so the
/// two clients cannot drift onto different fixture values.
pub fn client_parity_api() -> Value {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/client-parity/api.json"
    ))
    .expect("the shared client-parity fixture is JSON")
}

/// The normalized view both clients must produce from [`client_parity_api`].
pub fn client_parity_view() -> Value {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/client-parity/view.json"
    ))
    .expect("the shared client-parity view is JSON")
}

/// Answer one default-fixture stage route.
pub(super) fn stage_route(request: &super::Request, segments: &[&str]) -> Option<Response> {
    match (request.method.as_str(), segments) {
        ("POST", ["stages", "model", "runs"]) => Some(Response::json(
            202,
            &json!({
                "stage_run_id": stage_id(),
                "state": "queued",
                "created": true,
            }),
        )),
        ("GET", ["stages", "runs"]) => Some(Response::json(
            200,
            &json!([{
                "stage_run_id": stage_id(),
                "kind": "model",
                "state": "succeeded",
                "config_id": "cfg_abc",
                "submitted_at": "2026-11-17T00:00:00Z",
                "artifact_id": digest('9'),
                "evidence": false,
            }]),
        )),
        ("GET", ["stages", "runs", id]) if id.starts_with("sha256:9") => {
            Some(Response::error(404, "stage.not_found", "stage is unknown"))
        }
        ("GET", ["stages", "runs", _]) => Some(Response::json(200, &stage_detail("succeeded"))),
        ("POST", ["stages", "runs", id, "cancel"]) if *id == stage_id() => {
            Some(Response::json(202, &stage_detail("cancelled")))
        }
        ("POST", ["stages", "runs", _, "cancel"]) => {
            Some(Response::error(404, "stage.not_found", "stage is unknown"))
        }
        ("DELETE", ["stages", "runs", id]) if *id == stage_id() => {
            Some(Response::bytes(204, "application/json", Vec::new()))
        }
        ("DELETE", ["stages", "runs", _]) => {
            Some(Response::error(404, "stage.not_found", "stage is unknown"))
        }
        ("GET", ["stages", "runs", _, "artifacts", "events.csv"]) => {
            Some(Response::bytes(200, "text/csv", stage_events_body()))
        }
        ("GET", ["stages", "runs", _, "artifacts", "report.json"]) => Some(Response::bytes(
            200,
            "application/json",
            stage_report_body(),
        )),
        ("GET", ["stages", "runs", _, "artifacts"]) => Some(Response::json(
            200,
            &json!({
                "artifact_id": digest('9'),
                "files": stage_detail("succeeded")["artifact"]["files"].clone(),
            }),
        )),
        _ => None,
    }
}
