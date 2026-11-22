//! The default fixture routes.
//!
//! This module answers the command families that do not need their own
//! fixture module. It reads only the request and the mutable fixture state.

use serde_json::{Value, json};

use super::parity::parity_route;
use super::research;
use super::stage_fixture::stage_route;
use super::{
    FixtureState, Request, Response, canonical_digest, config_hash, data_version, image_digest,
    run_id,
};

pub(super) fn handle(state: &mut FixtureState, request: &Request) -> Response {
    let path = request.path.as_str();
    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();

    if let Some(response) = parity_route(state, request, &segments) {
        return response;
    }

    if path == "/health" && request.method == "GET" {
        return Response::json(
            200,
            &json!({
                "v": "quantrule.api/v1",
                "state": "degraded",
                "services": [
                    { "name": "postgres", "state": "healthy", "detail": null },
                    { "name": "minio", "state": "degraded", "detail": "slow" },
                ],
                "queue_depth": 2,
                "workers": 1,
                "image_digest": image_digest(),
            }),
        );
    }
    if path == "/version" && request.method == "GET" {
        return Response::json(
            200,
            &json!({
                "product": "1.0.0",
                "api": "quantrule.api/v1",
                "image_digest": image_digest(),
                "contracts": [["api", "quantrule.api/v1"]],
            }),
        );
    }

    if !matches!(path, "/profiles" | "/health" | "/version") {
        let profile = request.header("x-quantrule-profile").unwrap_or("");
        if !state
            .profiles
            .iter()
            .any(|item| item["profile_id"].as_str() == Some(profile))
        {
            return Response::error(
                404,
                "profile.not_found",
                &format!("profile {profile} is unknown"),
            );
        }
    }

    if let Some(response) = super::artifact_fixture::route(request, &segments) {
        return response;
    }
    if let Some(response) = stage_route(request, &segments) {
        return response;
    }
    if let Some(response) = research::route(request, &segments) {
        return response;
    }

    match (request.method.as_str(), segments.as_slice()) {
        ("GET", ["profiles"]) => Response::json(200, &Value::Array(state.profiles.clone())),
        ("POST", ["profiles"]) => {
            let body = request.json();
            let id = body["profile_id"].as_str().unwrap_or("").to_owned();
            if let Some(existing) = state.profiles.iter().find(|item| item["profile_id"] == id) {
                return Response::json(200, existing);
            }
            state.next_profile += 1;
            let created = json!({
                "profile_id": id,
                "name": body["name"],
                "datasets": 0,
                "runs": 0,
                "simulations": 0,
            });
            state.profiles.push(created.clone());
            Response::json(201, &created)
        }
        ("GET", ["catalog", "datasets"]) => {
            Response::json(200, &json!([dataset_summary("candles.csv")]))
        }
        ("POST", ["catalog", "datasets"]) | ("POST", ["catalog", "datasets", "tolerant"]) => {
            let name = request
                .query
                .get("name")
                .cloned()
                .unwrap_or_else(|| String::from("upload.csv"));
            let mut record = json!({
                "summary": dataset_summary(&name),
                "object_key": format!("datasets/{}", quantrule_api::Digest::sha256(&request.body)),
                "sha256": data_version(),
                "quality": null,
                "created_at": "2026-11-14T00:00:00Z",
            });
            if path.ends_with("/tolerant") {
                record["quality"] = json!({
                    "rows_read": 8,
                    "accepted": 4,
                    "rejected": 4,
                    "duplicates": 1,
                    "out_of_order": 1,
                    "invalid": 2,
                    "skipped": [
                        { "line": 4, "reason": "duplicate" },
                        { "line": 5, "reason": "misaligned" },
                    ],
                });
            }
            Response::json(201, &record)
        }
        ("GET", ["catalog", "datasets", _]) => Response::json(
            200,
            &json!({
                "summary": dataset_summary("candles.csv"),
                "object_key": format!("datasets/{}", data_version()),
                "sha256": data_version(),
                "quality": null,
                "created_at": "2026-11-14T00:00:00Z",
            }),
        ),
        ("GET", ["catalog", "configs"]) => Response::json(
            200,
            &json!([
                {
                    "config_id": "cfg_abc",
                    "name": "run",
                    "origin": "repository",
                    "valid": true,
                    "message": null,
                    "config_hash": config_hash(),
                },
                {
                    "config_id": "cfg_bad",
                    "name": "broken",
                    "origin": "upload",
                    "valid": false,
                    "message": "portfolio.initial_cash must be positive",
                    "config_hash": null,
                }
            ]),
        ),
        ("POST", ["runs", id, "archive"]) => Response::json(
            200,
            &json!({
                "run_id": id,
                "status": "failed",
                "config_id": "cfg_abc",
                "data_version": data_version(),
                "submitted_at": "2026-11-13T12:00:00Z",
                "elapsed_ms": null,
                "archived_at": "2026-11-20T00:00:00Z",
            }),
        ),
        ("DELETE", ["runs", id]) if *id == format!("sha256:{}", "f".repeat(64)) => Response::error(
            409,
            "history.protected",
            "a succeeded run is evidence and cannot be erased",
        ),
        ("DELETE", ["runs", id]) => Response::json(200, &json!({ "record_id": id, "bytes": 0 })),
        ("GET", ["runs"]) => Response::json(
            200,
            &json!([{
                "run_id": run_id(),
                "status": "succeeded",
                "config_id": "cfg_abc",
                "data_version": data_version(),
                "submitted_at": "2026-11-13T12:00:00Z",
                "elapsed_ms": 1_200,
            }]),
        ),
        ("POST", ["runs"]) => Response::json(
            202,
            &json!({ "run_id": run_id(), "status": "queued", "created": true }),
        ),
        ("GET", ["runs", id]) if id.starts_with("sha256:9") => {
            Response::error(404, "run.not_found", "run is unknown")
        }
        ("GET", ["runs", _]) => Response::json(200, &state.run_detail),
        ("POST", ["runs", _, "cancel"]) => {
            let mut detail = state.run_detail.clone();
            detail["status"] = json!("cancelled");
            Response::json(202, &detail)
        }
        ("GET", ["runs", _, "digest"]) => Response::json(
            200,
            &json!({ "run_id": run_id(), "digest": canonical_digest() }),
        ),
        ("GET", ["runs", _, "artifacts", ..]) => Response::bytes(
            200,
            "application/octet-stream",
            state.artifact_bytes.clone(),
        ),
        ("GET", ["inspect", "candles"]) => Response::json(200, &dataset_summary("candles.csv")),
        ("GET", ["inspect", "timeframes"]) => Response::json(
            200,
            &json!({
                "data_version": data_version(),
                "timeframes": [
                    { "timeframe": "1m", "bars": 3, "partial_buckets": 0, "first_bar": 1_704_067_200, "last_bar": 1_704_067_320 },
                    { "timeframe": "5m", "bars": 1, "partial_buckets": 1, "first_bar": 1_704_067_200, "last_bar": 1_704_067_200 },
                    { "timeframe": "15m", "bars": 0, "partial_buckets": 1, "first_bar": null, "last_bar": null },
                ],
            }),
        ),
        ("GET", ["inspect", "events"]) => Response::json(
            200,
            &json!({
                "data_version": data_version(),
                "timeframe": request.query.get("timeframe").cloned().unwrap_or_default(),
                "events": [
                    { "type": 1, "source": 2, "bar": 10 },
                    { "type": 2, "source": 3, "bar": 11 },
                ],
            }),
        ),
        ("GET", ["simulation", "runs"]) => Response::json(
            200,
            &json!([
                {
                    "sim_id": "sim-2",
                    "premise": "second",
                    "status": "marking",
                    "windows": 4,
                    "marks": 2,
                    "bundle": image_digest(),
                    "marked_windows": 2,
                },
                {
                    "sim_id": "sim-1",
                    "premise": "first",
                    "status": "draft",
                    "windows": 3,
                    "marks": 0,
                    "bundle": image_digest(),
                    "marked_windows": 0,
                }
            ]),
        ),
        ("POST", ["simulation", "runs"]) => Response::json(201, &json!({ "sim_id": "sim-3" })),
        ("GET", ["simulation", "runs", _]) => Response::json(
            200,
            &json!({
                "sim_id": "sim-1",
                "premise": "first",
                "status": "draft",
                "inputs_digest": config_hash(),
                "windows": 3,
                "marks": 0,
            }),
        ),
        ("POST", ["simulation", "runs", _, "windows"]) => Response::json(
            200,
            &json!({
                "sim_id": "sim-1",
                "windows": [
                    { "window": 1, "start": 0, "end": 120 },
                    { "window": 2, "start": 121, "end": 240 },
                ],
            }),
        ),
        ("POST", ["simulation", "runs", _, "marks"]) => {
            let body = request.json();
            let mark_id = format!("mark-{}", state.marks.len() + 1);
            let mut mark = body;
            mark["mark_id"] = json!(mark_id);
            mark["marked_time_utc"] = json!("2026-11-14T10:00:00Z");
            mark["origin"] = json!("human");
            state.marks.push(mark);
            Response::json(201, &json!({ "mark_id": mark_id }))
        }
        ("GET", ["simulation", "runs", _, "marks"]) => {
            let origin = request.query.get("origin");
            let marks: Vec<Value> = state
                .marks
                .iter()
                .filter(|mark| {
                    origin.is_none_or(|origin| mark["origin"].as_str() == Some(origin.as_str()))
                })
                .cloned()
                .collect();
            Response::json(200, &json!({ "sim_id": "sim-1", "marks": marks }))
        }
        ("POST", ["simulation", "runs", _, "oracle"]) => Response::json(
            200,
            &json!({
                "sim_id": "sim-1",
                "marks": [
                    {
                        "mark_id": "oracle-1",
                        "chart": "w1",
                        "bar": 12,
                        "marked_time_utc": "2026-11-14T10:00:00Z",
                        "action": "entry",
                        "quality": "good",
                        "side": "long",
                        "origin": "oracle",
                        "note": ""
                    }
                ],
            }),
        ),
        ("POST", ["simulation", "runs", _, "check"]) => Response::json(
            200,
            &json!({
                "sim_id": "sim-1",
                "events": 31,
                "mean_net": 0.01,
                "hit_rate": 0.6,
                "control": 60,
                "control_mean_net": 0.0,
                "control_hit_rate": 0.5,
                "positive_half_years": 3,
                "passed": true,
                "failures": [],
                "verdict": "pass",
                "sides": [{ "side": "long", "verdict": "survives" }],
            }),
        ),
        ("POST", ["simulation", "runs", _, "decision"]) => {
            let body = request.json();
            Response::json(
                201,
                &json!({
                    "sim_id": "sim-1",
                    "decision": body["decision"],
                    "note": body["note"],
                }),
            )
        }
        _ => Response::error(
            404,
            "test.unhandled",
            &format!("{} {} is not fixtured", request.method, path),
        ),
    }
}

fn dataset_summary(name: &str) -> Value {
    json!({
        "data_version": data_version(),
        "name": name,
        "candles": 3,
        "first_bar": 1_704_067_200,
        "last_bar": 1_704_067_320,
        "bytes": 96,
    })
}

pub(super) fn run_detail(artifact_digest: &str, artifact_bytes: u64) -> Value {
    json!({
        "run_id": run_id(),
        "status": "succeeded",
        "owner": "default",
        "stages": [
            { "name": "data", "state": "done", "elapsed_ms": 1_200 },
            { "name": "model", "state": "done", "elapsed_ms": 2_400 },
            { "name": "backtest", "state": "done", "elapsed_ms": 3_600 },
            { "name": "evidence", "state": "done", "elapsed_ms": 120 },
        ],
        "provenance": {
            "data_version": data_version(),
            "config_hash": config_hash(),
            "model_version": "M1.0",
            "image_digest": image_digest(),
        },
        "artifacts": [
            { "name": "results.json", "bytes": artifact_bytes, "sha256": artifact_digest }
        ],
        "digest": canonical_digest(),
        "error": null,
    })
}
