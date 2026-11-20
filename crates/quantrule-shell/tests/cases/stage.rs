//! Stage command output tests.

use std::fs;

use serde_json::json;

use crate::cases::assertions::{assert_failure, assert_success, stderr, stdout};
use crate::support::{self, Fixture, data_version, digest, stage_id, strategy_stage_id};

#[test]
fn stage_run_model_sends_the_contract_body() {
    let fixture = Fixture::start();
    let output = fixture.run(&["stage", "run", "model", &data_version(), "cfg_abc"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        format!(
            "stage_run_id: {}\nstate: queued\ncreated: true\n",
            stage_id()
        )
    );
    let request = fixture.last_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/stages/model/runs");
    assert_eq!(
        request.json(),
        json!({
            "kind": "model",
            "config_id": "cfg_abc",
            "input": { "kind": "data_version", "data_version": data_version() },
        })
    );
}

#[test]
fn stage_run_strategy_and_explain_use_the_published_decisions() {
    let fixture = Fixture::start();
    let model_artifact = digest('9');
    let run = fixture.run(&["stage", "run", "strategy", &model_artifact, "cfg_abc"]);
    assert_success(&run);
    assert_eq!(
        stdout(&run),
        format!(
            "stage_run_id: {}\nstate: queued\ncreated: true\n",
            strategy_stage_id()
        )
    );
    assert_eq!(fixture.last_request().path, "/stages/strategy/runs");
    assert_eq!(
        fixture.last_request().json(),
        json!({
            "kind": "strategy",
            "config_id": "cfg_abc",
            "input": { "kind": "model_artifact", "model_artifact_id": model_artifact },
        })
    );

    fixture.clear_requests();
    let explain = fixture.run(&["stage", "explain", &strategy_stage_id(), "--bar", "1"]);
    assert_success(&explain);
    let text = stdout(&explain);
    assert!(text.contains("bar_index: 1"));
    assert!(text.contains("decision: hold"));
    assert!(text.contains("position_before: long"));
    assert!(text.contains("position_after: long"));
    assert!(text.contains("atr14: 1.5"));
    let paths: Vec<String> = fixture
        .requests()
        .iter()
        .map(|request| request.path.clone())
        .collect();
    assert_eq!(
        paths,
        vec![
            format!("/stages/runs/{}", strategy_stage_id()),
            format!(
                "/stages/runs/{}/artifacts/decisions.ndjson",
                strategy_stage_id()
            ),
        ]
    );

    let json_output = fixture.run(&[
        "--json",
        "--profile",
        "default",
        "stage",
        "explain",
        &strategy_stage_id(),
        "--bar",
        "0",
    ]);
    assert_success(&json_output);
    let value: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("one JSON value");
    assert_eq!(value["bar_index"], 0);
    assert_eq!(value["decision"], "enter");
    assert_eq!(value["side"], "long");
}

#[test]
fn stage_list_and_show_print_the_diagnostic_record() {
    let fixture = Fixture::start();
    let list = fixture.run(&["stage", "list", "--kind", "model", "--status", "succeeded"]);
    assert_success(&list);
    let text = stdout(&list);
    assert!(text.starts_with("stages: 1\n"));
    assert!(text.contains(&format!(
        "{} kind=model state=succeeded config=cfg_abc submitted=2026-11-17T00:00:00Z artifact={}",
        stage_id(),
        digest('9')
    )));
    let query = fixture.last_request().query;
    assert_eq!(query.get("kind").map(String::as_str), Some("model"));
    assert_eq!(query.get("status").map(String::as_str), Some("succeeded"));

    let show = fixture.run(&["stage", "show", &stage_id()]);
    assert_success(&show);
    let text = stdout(&show);
    for expected in [
        "kind: model",
        "state: succeeded",
        "owner: default",
        "config_id: cfg_abc",
        &format!("input: data_version {}", data_version()),
        "model_version: M1.0",
        "attempts: 1",
        "error: none",
        "artifacts: 2",
        "artifact events.csv:",
        "artifact report.json:",
        "evidence: false",
    ] {
        assert!(text.contains(expected), "{expected} missing from {text}");
    }
    assert_eq!(
        fixture.last_request().path,
        format!("/stages/runs/{}", stage_id())
    );
}

#[test]
fn stage_events_filters_the_artifact_and_prints_coverage() {
    let fixture = Fixture::start();
    let output = fixture.run(&["stage", "events", &stage_id(), "--timeframe", "5m"]);
    assert_success(&output);
    let text = stdout(&output);
    assert!(text.contains("events: 2\n"));
    assert!(text.contains("timeframe_secs,event_index,event_code\n300,0,1\n300,1,3\n"));
    assert!(text.contains("coverage: 2\n"));
    assert!(text.contains("timeframe_secs=300 bars=3 expected_bars=3 coverage_share=1.0"));
    assert!(text.contains("timeframe_secs=900 bars=0 expected_bars=1 coverage_share=0.0"));
    let paths: Vec<String> = fixture
        .requests()
        .iter()
        .map(|request| request.path.clone())
        .collect();
    assert_eq!(
        paths,
        vec![
            format!("/stages/runs/{}", stage_id()),
            format!("/stages/runs/{}/artifacts/events.csv", stage_id()),
            format!("/stages/runs/{}/artifacts/report.json", stage_id()),
        ]
    );

    let json_output = fixture.run(&[
        "--json",
        "--profile",
        "default",
        "stage",
        "events",
        &stage_id(),
        "--timeframe",
        "5m",
    ]);
    assert_success(&json_output);
    let value: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("one JSON value");
    assert_eq!(value["timeframe"], "5m");
    assert_eq!(value["events"].as_array().map(Vec::len), Some(2));
    assert_eq!(value["events"][0]["event_code"], "1");
    assert_eq!(value["coverage"][0]["timeframe_secs"], 300);
}

#[test]
fn stage_fetch_writes_and_verifies_every_artifact() {
    let fixture = Fixture::start();
    let directory = fixture.home().join("stage");
    let output = fixture.run(&[
        "stage",
        "fetch",
        &stage_id(),
        directory.to_str().expect("UTF-8 path"),
    ]);
    assert_success(&output);
    assert_eq!(
        fs::read(directory.join("events.csv")).expect("events"),
        support::stage_events_body()
    );
    assert_eq!(
        fs::read(directory.join("report.json")).expect("report"),
        support::stage_report_body()
    );
    let text = stdout(&output);
    assert!(text.contains(&format!("stage_run_id: {}", stage_id())));
    assert!(text.contains("artifact events.csv: bytes="));
    assert!(text.contains("artifact report.json: bytes="));
}

#[test]
fn a_strategy_stage_show_and_fetch_render_the_decision_bundle() {
    let fixture = Fixture::start();
    let show = fixture.run(&["stage", "show", &strategy_stage_id()]);
    assert_success(&show);
    let text = stdout(&show);
    assert!(text.contains("kind: strategy"));
    assert!(text.contains(&format!("input: model_artifact {}", digest('9'))));
    assert!(text.contains("artifact decisions.ndjson:"));

    let directory = fixture.home().join("strategy-stage");
    let fetch = fixture.run(&[
        "stage",
        "fetch",
        &strategy_stage_id(),
        directory.to_str().expect("UTF-8 path"),
    ]);
    assert_success(&fetch);
    assert_eq!(
        fs::read(directory.join("decisions.ndjson")).expect("decisions"),
        support::strategy_decisions_body()
    );
    let text = stdout(&fetch);
    assert!(text.contains("artifact decisions.ndjson:"));
}

#[test]
fn stage_cancel_and_delete_print_their_result() {
    let fixture = Fixture::start();
    let cancel = fixture.run(&["stage", "cancel", &stage_id()]);
    assert_success(&cancel);
    assert!(stdout(&cancel).contains("state: cancelled"));
    assert_eq!(
        fixture.last_request().path,
        format!("/stages/runs/{}/cancel", stage_id())
    );

    let delete = fixture.run(&["stage", "delete", &stage_id()]);
    assert_success(&delete);
    assert_eq!(stdout(&delete), format!("deleted: {}\n", stage_id()));
    let request = fixture.last_request();
    assert_eq!(request.method, "DELETE");
    assert_eq!(request.path, format!("/stages/runs/{}", stage_id()));
}

#[test]
fn stage_commands_refuse_an_unknown_kind_and_an_unknown_stage() {
    let fixture = Fixture::start();
    let unknown_kind = fixture.run(&["stage", "run", "research", &stage_id(), "cfg_abc"]);
    assert_failure(&unknown_kind);
    assert!(stderr(&unknown_kind).contains("stage run expects model, strategy, or portfolio"));

    let unknown = fixture.run(&["stage", "show", &digest('9')]);
    assert_failure(&unknown);
    assert!(stderr(&unknown).contains("stage.not_found"));

    let bad_status = fixture.run(&["stage", "list", "--status", "done"]);
    assert_failure(&bad_status);
    assert!(stderr(&bad_status).contains("--status expects"));

    let bad_digest = fixture.run(&["stage", "run", "model", "not-a-digest", "cfg_abc"]);
    assert_failure(&bad_digest);
    assert!(stderr(&bad_digest).contains("data_version"));

    let unknown_id = digest('9');
    for command in [
        vec!["stage", "cancel", &unknown_id],
        vec!["stage", "delete", &unknown_id],
        vec!["stage", "fetch", &unknown_id, "."],
        vec!["stage", "events", &unknown_id, "--timeframe", "5m"],
    ] {
        let output = fixture.run(&command);
        assert_failure(&output);
        assert!(
            stderr(&output).contains("stage.not_found"),
            "{command:?}: {}",
            stderr(&output)
        );
    }
}
