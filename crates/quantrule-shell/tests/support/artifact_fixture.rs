//! The default fixture's pack, signal and lineage routes.
//!
//! The bodies are content-addressed: the pack and signal ids are the SHA-256 of
//! the exact bytes the byte route serves, so `pack fetch` and `signal fetch` can
//! verify them the same way the real clients do.

use quantrule_api::Digest;
use serde_json::{Value, json};

use super::{Request, Response, digest, run_id};

const PACK_BODY: &str = "{\"kind\":\"strategy\",\"name\":\"parity-pack\",\"owner\":\"operator\",\"payload\":{},\"schema\":\"quantrule.pack/v1\",\"version\":\"1.0.0\"}\n";
const SIGNAL_BODY: &str = "{\"schema\":\"quantrule.signal-artifact/v1\",\"rows\":[]}\n";

/// The pack's content id.
pub fn pack_id() -> String {
    Digest::sha256(PACK_BODY.as_bytes()).to_string()
}

/// The pack's canonical bytes.
pub fn pack_body() -> Vec<u8> {
    PACK_BODY.as_bytes().to_vec()
}

/// The signal's content id.
pub fn signal_id() -> String {
    Digest::sha256(SIGNAL_BODY.as_bytes()).to_string()
}

/// The signal's stored bytes.
pub fn signal_body() -> Vec<u8> {
    SIGNAL_BODY.as_bytes().to_vec()
}

/// A pack summary in one lifecycle state.
fn pack_summary(state: &str) -> Value {
    json!({
        "pack_id": pack_id(),
        "kind": "strategy",
        "name": "parity-pack",
        "version": "1.0.0",
        "owner": "operator",
        "state": state,
        "bytes": pack_body().len(),
        "created_at": "2026-11-22T00:00:00Z",
        "validated_at": (state != "draft").then_some("2026-11-22T00:01:00Z"),
        "frozen_at": matches!(state, "frozen" | "retired").then_some("2026-11-22T00:02:00Z"),
        "retired_at": (state == "retired").then_some("2026-11-22T00:03:00Z"),
    })
}

/// A pack record in one lifecycle state.
fn pack_record(state: &str) -> Value {
    let mut events = vec![json!({
        "to": "draft",
        "action": "create",
        "created_at": "2026-11-22T00:00:00Z",
    })];
    if state != "draft" {
        events.push(json!({
            "from": "draft",
            "to": "validated",
            "action": "validate",
            "created_at": "2026-11-22T00:01:00Z",
        }));
    }
    if matches!(state, "frozen" | "retired") {
        events.push(json!({
            "from": "validated",
            "to": "frozen",
            "action": "freeze",
            "created_at": "2026-11-22T00:02:00Z",
        }));
    }
    if state == "retired" {
        events.push(json!({
            "from": "frozen",
            "to": "retired",
            "action": "retire",
            "created_at": "2026-11-22T00:03:00Z",
        }));
    }
    let validations = if state == "draft" {
        Vec::new()
    } else {
        vec![json!({
            "validator": "pack.schema/v1",
            "valid": true,
            "created_at": "2026-11-22T00:01:00Z",
        })]
    };
    json!({
        "summary": pack_summary(state),
        "object_key": format!("packs/{}", Digest::sha256(PACK_BODY.as_bytes()).as_hex()),
        "sha256": pack_id(),
        "validations": validations,
        "events": events,
    })
}

/// The fixture's signal record.
pub fn signal_record() -> Value {
    json!({
        "summary": {
            "artifact_id": signal_id(),
            "kind": "signal",
            "bytes": signal_body().len(),
            "created_at": "2026-11-22T00:04:00Z",
        },
        "object_key": format!("artifacts/{}", Digest::sha256(SIGNAL_BODY.as_bytes()).as_hex()),
        "sha256": signal_id(),
    })
}

/// The fixture's signal create response.
fn signal_created() -> Value {
    json!({
        "schema": "quantrule.signal-response/v1",
        "artifact": signal_record(),
        "created": true,
    })
}

/// The fixture's run lineage.
fn lineage() -> Value {
    json!({
        "run_id": run_id(),
        "references": [
            {
                "kind": "model_artifact",
                "id": digest('9'),
                "hash": digest('9'),
            },
            {
                "kind": "strategy_pack",
                "id": pack_id(),
                "hash": pack_id(),
                "pack": {
                    "kind": "strategy",
                    "name": "parity-pack",
                    "version": "1.0.0",
                    "owner": "operator",
                    "state": "frozen",
                },
            },
            {
                "kind": "signal_artifact",
                "id": signal_id(),
                "hash": signal_id(),
            },
        ],
        "recorded_at": "2026-11-22T00:05:00Z",
    })
}

/// Answer one pack, signal, or lineage route.
pub(super) fn route(request: &Request, segments: &[&str]) -> Option<Response> {
    match (request.method.as_str(), segments) {
        ("GET", ["catalog", "packs"]) => {
            let kind = request.query.get("kind").map(String::as_str);
            let state = request.query.get("state").map(String::as_str);
            let matches = kind.is_none_or(|kind| kind == "strategy")
                && state.is_none_or(|state| state == "frozen");
            let rows = if matches {
                vec![pack_summary("frozen")]
            } else {
                Vec::new()
            };
            Some(Response::json(200, &Value::Array(rows)))
        }
        ("POST", ["catalog", "packs"]) => Some(Response::json(201, &pack_record("draft"))),
        ("GET", ["catalog", "packs", id]) if *id == pack_id() => {
            Some(Response::json(200, &pack_record("frozen")))
        }
        ("GET", ["catalog", "packs", id, "bytes"]) if *id == pack_id() => {
            let mut response = Response::bytes(200, "application/json", pack_body());
            response.headers.push(("etag", "\"pack-fixture\""));
            Some(response)
        }
        ("GET", ["catalog", "packs", id]) | ("GET", ["catalog", "packs", id, "bytes"]) => {
            let _ = id;
            Some(Response::error(
                404,
                "pack.not_found",
                "the pack is unknown",
            ))
        }
        ("POST", ["catalog", "packs", id, action]) if *id == pack_id() => {
            let state = match *action {
                "validate" => "validated",
                "freeze" => "frozen",
                "retire" => "retired",
                _ => return None,
            };
            Some(Response::json(200, &pack_record(state)))
        }
        ("POST", ["catalog", "packs", id, _]) => {
            let _ = id;
            Some(Response::error(
                404,
                "pack.not_found",
                "the pack is unknown",
            ))
        }
        ("GET", ["signals"]) => Some(Response::json(200, &Value::Array(vec![signal_record()]))),
        ("POST", ["signals"]) => Some(Response::json(201, &signal_created())),
        ("GET", ["signals", id]) if *id == signal_id() => {
            Some(Response::json(200, &signal_record()))
        }
        ("GET", ["signals", id, "bytes"]) if *id == signal_id() => {
            let mut response = Response::bytes(200, "application/octet-stream", signal_body());
            response.headers.push(("etag", "\"signal-fixture\""));
            Some(response)
        }
        ("GET", ["signals", id]) | ("GET", ["signals", id, "bytes"]) => {
            let _ = id;
            Some(Response::error(
                404,
                "artifact.not_found",
                "the artifact is unknown",
            ))
        }
        ("GET", ["runs", id, "lineage"]) if *id == run_id() => {
            Some(Response::json(200, &lineage()))
        }
        _ => None,
    }
}
