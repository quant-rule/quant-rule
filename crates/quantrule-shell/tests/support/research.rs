//! Research fixture routes and records.

use serde_json::{Value, json};

use super::{Request, Response, canonical_digest, data_version, run_id};

fn detail() -> Value {
    json!({
        "improvement_id": "MODEL-2026-11-17-01",
        "tier": "model",
        "state": "created",
        "decision": null,
        "decision_note": null,
        "manifest": {
            "schema": "quantrule.improvement-run/v3",
            "improvement_id": "MODEL-2026-11-17-01",
            "tier": "model",
            "hypothesis": "the fixture hypothesis",
            "created_utc": "2026-11-17T00:00:00Z",
            "changed": "model",
            "frozen": ["strategy", "portfolio"],
            "partition": "model_dev",
            "data_version": data_version(),
            "config_id": "cfg_abc",
            "criteria_id": null,
            "green_criteria": {"return_target": 0.1},
            "holdout_access": "none",
            "baseline": null,
            "code_commit": "c236539f",
            "seed": 0,
            "command": "run submit",
            "analytics": true
        },
        "pipeline_run_id": null,
        "canonical_digest": null,
        "created_at": "2026-11-17T00:00:00Z",
        "submitted_at": null,
        "decided_at": null
    })
}

fn summary() -> Value {
    json!({
        "improvement_id": "MODEL-2026-11-17-01",
        "tier": "model",
        "state": "created",
        "decision": null,
        "created_at": "2026-11-17T00:00:00Z",
        "submitted_at": null,
        "decided_at": null
    })
}

fn baseline_row() -> Value {
    json!({
        "baseline_id": "MODEL-2026-11-04-04..05",
        "tier": "model",
        "run_id": "MODEL-2026-11-04-04..05",
        "improvement_id": null,
        "accepted_by": "Human",
        "accepted_on": "2026-11-04",
        "scope": "1m, 4 ATR over 20 bars",
        "supersedes": null,
        "created_at": "2026-11-04T00:00:00Z"
    })
}

fn invalidation_row() -> Value {
    json!({
        "invalidation_id": "INVALIDATION-2026-11-10-01",
        "date": "2026-11-10",
        "commit": "worktree",
        "removed": "MODEL-2026-11-10-01",
        "reason": "the fixture tombstone",
        "superseded_by": null,
        "created_at": "2026-11-10T00:00:00Z"
    })
}

/// Answer one research route, or None when the path is not research.
pub(super) fn route(request: &Request, segments: &[&str]) -> Option<Response> {
    match (request.method.as_str(), segments) {
        ("GET", ["research", "runs"]) => Some(Response::json(200, &json!([summary()]))),
        ("POST", ["research", "runs"]) => Some(Response::json(201, &detail())),
        ("GET", ["research", "runs", _]) => Some(Response::json(200, &detail())),
        ("POST", ["research", "runs", id, "submit"]) if *id == "MODEL-CONFLICT" => Some(
            Response::error(409, "research.conflict", "the decision is already recorded"),
        ),
        ("POST", ["research", "runs", _, "submit"]) => {
            let mut detail = detail();
            detail["state"] = json!("submitted");
            detail["pipeline_run_id"] = json!(run_id());
            detail["submitted_at"] = json!("2026-11-17T00:05:00Z");
            Some(Response::json(202, &detail))
        }
        ("GET", ["research", "runs", _, "compare"]) => Some(Response::json(
            200,
            &json!({
                "improvement_id": "MODEL-2026-11-17-01",
                "tier": "model",
                "candidate_run_id": run_id(),
                "baseline_id": "MODEL-2026-11-04-04..05",
                "candidate_digest": canonical_digest(),
                "baseline_digest": null,
                "rows": [
                    {"metric": "1m.coverage_share", "candidate": 1.0, "baseline": null, "delta": null}
                ],
                "note": "the baseline has no pipeline run; baseline metrics are unavailable"
            }),
        )),
        ("POST", ["research", "runs", _, "decision"]) => {
            let body = request.json();
            let mut detail = detail();
            detail["state"] = json!("decided");
            detail["decision"] = body["decision"].clone();
            detail["decision_note"] = body["note"].clone();
            detail["canonical_digest"] = json!(canonical_digest());
            detail["decided_at"] = json!("2026-11-17T00:10:00Z");
            Some(Response::json(200, &detail))
        }
        ("GET", ["research", "baselines"]) => Some(Response::json(200, &json!([baseline_row()]))),
        ("GET", ["research", "baselines", _]) => Some(Response::json(200, &baseline_row())),
        ("GET", ["research", "invalidations"]) => {
            Some(Response::json(200, &json!([invalidation_row()])))
        }
        _ => None,
    }
}
