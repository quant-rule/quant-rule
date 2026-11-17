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

/// The strategy-stage fixture id.
pub fn strategy_stage_id() -> String {
    digest('8')
}

/// The strategy decisions fixture.
pub fn strategy_decisions_body() -> Vec<u8> {
    b"{\"schema\":\"quantrule.strategy-decision/v1\",\"timeframe_secs\":60,\"bar_index\":0,\"decision\":\"enter\",\"side\":\"long\",\"reason\":\"entry\",\"position_before\":\"flat\",\"position_after\":\"long\",\"atr14\":1.5}\n{\"schema\":\"quantrule.strategy-decision/v1\",\"timeframe_secs\":60,\"bar_index\":1,\"decision\":\"hold\",\"side\":null,\"reason\":null,\"position_before\":\"long\",\"position_after\":\"long\",\"atr14\":1.5}\n".to_vec()
}

/// A strategy stage detail with the decision bundle.
pub fn strategy_stage_detail(state: &str) -> Value {
    let decisions = strategy_decisions_body();
    let summary = b"QuantRule strategy run\n".to_vec();
    let provenance = b"{\"schema\":\"quantrule.stage-provenance/v1\",\"stage\":\"strategy\",\"artifact\":\"decisions\"}\n".to_vec();
    json!({
        "stage_run_id": strategy_stage_id(),
        "kind": "strategy",
        "state": state,
        "owner": "default",
        "config_id": "cfg_abc",
        "input": {
            "kind": "model_artifact",
            "model_artifact_id": digest('9'),
        },
        "provenance": {
            "data_version": data_version(),
            "config_hash": config_hash(),
            "model_version": "M1.0",
            "image_digest": image_digest(),
        },
        "artifact": if state == "succeeded" {
            json!({
                "artifact_id": digest('7'),
                "files": [
                    { "name": "decisions.ndjson", "bytes": decisions.len(), "sha256": quantrule_api::Digest::sha256(&decisions).to_string() },
                    { "name": "provenance.json", "bytes": provenance.len(), "sha256": quantrule_api::Digest::sha256(&provenance).to_string() },
                    { "name": "summary.txt", "bytes": summary.len(), "sha256": quantrule_api::Digest::sha256(&summary).to_string() },
                ],
            })
        } else {
            Value::Null
        },
        "submitted_at": "2026-11-17T00:00:01Z",
        "error": Value::Null,
        "attempts": 1,
        "evidence": false,
    })
}

/// The portfolio-stage fixture id.
pub fn portfolio_stage_id() -> String {
    digest('6')
}

/// The decisions artifact the portfolio fixture consumes.
pub fn portfolio_decisions_artifact_id() -> String {
    digest('7')
}

/// The published execution and portfolio fixture files.
pub fn portfolio_files() -> Vec<(String, Vec<u8>)> {
    vec![
        (
            "execution/orders.csv".to_owned(),
            b"order_id,submitted_bar,submitted_time,side,quantity,reason,trigger_kind,trigger_price,state,filled_bar,filled_time,fill_id,rejection_reason
0,0,1704067200,buy,10,entry,market,,filled,1,1704067260,0,
".to_vec(),
        ),
        (
            "execution/fills.csv".to_owned(),
            b"fill_id,side,quantity,reference_price,price,slippage,fee,bar,time,reason
0,buy,10,100.0,100.0,0.0,0.0,1,1704067260,entry
".to_vec(),
        ),
        (
            "execution/rejections.csv".to_owned(),
            b"order_id,bar,time,reason
".to_vec(),
        ),
        (
            "execution/summary.txt".to_owned(),
            b"QuantRule execution run\nfills: count=1\ncosts: fee=0 slippage=0\n".to_vec(),
        ),
        (
            "execution/provenance.json".to_owned(),
            b"{\"schema\":\"quantrule.stage-provenance/v1\",\"stage\":\"execution\",\"artifact\":\"fills\",\"data_version\":\"fixture\",\"config_id\":\"fixture\"}\n".to_vec(),
        ),
        (
            "portfolio/trades.csv".to_owned(),
            b"trade_id,side,quantity,entry_bar,entry_time,entry_price,exit_bar,exit_time,exit_price,gross_pnl,execution_fees,commission,funding,net_pnl,return_fraction,hold_bars,exit_reason
".to_vec(),
        ),
        (
            "portfolio/equity.csv".to_owned(),
            b"date,cash,position_value,equity,peak_equity,drawdown\n".to_vec(),
        ),
        (
            "portfolio/funding.csv".to_owned(),
            b"bar,time,rate,price,quantity,amount\n".to_vec(),
        ),
        (
            "portfolio/risk_events.csv".to_owned(),
            b"bar,time,drawdown,equity,peak_equity,limit\n".to_vec(),
        ),
        (
            "portfolio/metrics.json".to_owned(),
            b"{\"schema\":\"quantrule.portfolio-metrics/v1\",\"initial_cash\":10000.0,\"final_equity\":10000.0,\"net_pnl\":0.0,\"execution_fees\":0.0,\"commission\":0.0,\"funding_paid\":0.0,\"funding_received\":0.0,\"max_drawdown\":0.0,\"halted\":false}\n".to_vec(),
        ),
        (
            "portfolio/summary.txt".to_owned(),
            b"QuantRule portfolio replay\nnet_pnl: 0\nfunding_paid: 0\nhalted: false\n".to_vec(),
        ),
        (
            "portfolio/provenance.json".to_owned(),
            b"{\"schema\":\"quantrule.stage-provenance/v1\",\"stage\":\"portfolio\",\"artifact\":\"portfolio\",\"data_version\":\"fixture\",\"config_id\":\"fixture\"}\n".to_vec(),
        ),
    ]
}

/// A portfolio stage detail with prefixed artifact names.
pub fn portfolio_stage_detail(state: &str) -> Value {
    let files = portfolio_files();
    json!({
        "stage_run_id": portfolio_stage_id(),
        "kind": "portfolio",
        "state": state,
        "owner": "default",
        "config_id": "cfg_abc",
        "input": {
            "kind": "decisions_artifact",
            "decisions_artifact_id": portfolio_decisions_artifact_id(),
        },
        "provenance": {
            "data_version": data_version(),
            "config_hash": config_hash(),
            "model_version": "M1.0",
            "image_digest": image_digest(),
        },
        "artifact": if state == "succeeded" {
            json!({
                "artifact_id": digest('5'),
                "files": files.iter().map(|(name, bytes)| json!({
                    "name": name,
                    "bytes": bytes.len(),
                    "sha256": quantrule_api::Digest::sha256(bytes).to_string(),
                })).collect::<Vec<Value>>(),
            })
        } else {
            Value::Null
        },
        "submitted_at": "2026-11-17T00:00:02Z",
        "error": Value::Null,
        "attempts": 1,
        "evidence": false,
    })
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
        ("POST", ["stages", "strategy", "runs"]) => Some(Response::json(
            202,
            &json!({
                "stage_run_id": strategy_stage_id(),
                "state": "queued",
                "created": true,
            }),
        )),
        ("POST", ["stages", "portfolio", "runs"]) => Some(Response::json(
            202,
            &json!({
                "stage_run_id": portfolio_stage_id(),
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
        ("GET", ["stages", "runs", id]) if *id == strategy_stage_id() => {
            Some(Response::json(200, &strategy_stage_detail("succeeded")))
        }
        ("GET", ["stages", "runs", id]) if *id == portfolio_stage_id() => {
            Some(Response::json(200, &portfolio_stage_detail("succeeded")))
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
        ("GET", ["stages", "runs", id, "artifacts", "decisions.ndjson"])
            if *id == strategy_stage_id() =>
        {
            Some(Response::bytes(200, "application/x-ndjson", strategy_decisions_body()))
        }
        ("GET", ["stages", "runs", id, "artifacts", "summary.txt"])
            if *id == strategy_stage_id() =>
        {
            Some(Response::bytes(
                200,
                "text/plain; charset=utf-8",
                b"QuantRule strategy run\n".to_vec(),
            ))
        }
        ("GET", ["stages", "runs", id, "artifacts", "provenance.json"])
            if *id == strategy_stage_id() =>
        {
            Some(Response::bytes(
                200,
                "application/json",
                b"{\"schema\":\"quantrule.stage-provenance/v1\",\"stage\":\"strategy\",\"artifact\":\"decisions\"}\n".to_vec(),
            ))
        }
        ("GET", ["stages", "runs", id, "artifacts", prefix, name])
            if *id == portfolio_stage_id() =>
        {
            let artifact = format!("{prefix}/{name}");
            portfolio_files()
                .into_iter()
                .find(|(artifact_name, _)| *artifact_name == artifact)
                .map(|(_, bytes)| Response::bytes(200, "application/octet-stream", bytes))
        }
        ("GET", ["stages", "runs", id, "artifacts", name])
            if *id == portfolio_stage_id() =>
        {
            portfolio_files()
                .into_iter()
                .find(|(artifact_name, _)| artifact_name == name)
                .map(|(_, bytes)| Response::bytes(200, "application/octet-stream", bytes))
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
