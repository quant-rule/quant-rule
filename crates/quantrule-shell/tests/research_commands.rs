//! Printed-output tests for the research and registry commands.

mod support;

use std::process::Output;

use support::{Fixture, data_version, run_id};

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success, got {:?}; stderr: {}",
        output.status.code(),
        stderr(output)
    );
}

fn assert_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "expected failure; stdout: {}",
        stdout(output)
    );
}

#[test]
fn research_create_sends_the_gateway_references_and_prints_the_record() {
    let fixture = Fixture::start();
    let output = fixture.run(&[
        "research",
        "create",
        "--tier",
        "model",
        "--hypothesis",
        "the fixture hypothesis",
        "--partition",
        "model_dev",
        "--data-version",
        &data_version(),
        "--config",
        "cfg_abc",
        "--green-criteria",
        r#"{"return_target":0.1}"#,
        "--code-commit",
        "c236539f",
        "--seed",
        "0",
    ]);
    assert_success(&output);
    let text = stdout(&output);
    assert!(
        text.contains("improvement_id: MODEL-2026-11-17-01"),
        "{text}"
    );
    assert!(text.contains("state: created"), "{text}");
    assert!(
        text.contains("hypothesis: the fixture hypothesis"),
        "{text}"
    );
    let request = fixture.last_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/research/runs");
    let body = request.json();
    assert_eq!(body["tier"], "model");
    assert_eq!(body["partition"], "model_dev");
    assert_eq!(body["data_version"], data_version());
    assert_eq!(body["config_id"], "cfg_abc");
    assert_eq!(body["green_criteria"]["return_target"], 0.1);
    assert_eq!(body["code_commit"], "c236539f");
}

#[test]
fn research_submit_and_show_print_the_state_and_digest() {
    let fixture = Fixture::start();
    let submit = fixture.run(&["research", "submit", "MODEL-2026-11-17-01"]);
    assert_success(&submit);
    let text = stdout(&submit);
    assert!(text.contains("state: submitted"), "{text}");
    assert!(
        text.contains(&format!("pipeline_run_id: {}", run_id())),
        "{text}"
    );
    assert_eq!(
        fixture.last_request().path,
        "/research/runs/MODEL-2026-11-17-01/submit"
    );

    let show = fixture.run(&["research", "show", "MODEL-2026-11-17-01"]);
    assert_success(&show);
    let text = stdout(&show);
    assert!(
        text.contains("improvement_id: MODEL-2026-11-17-01"),
        "{text}"
    );
    assert!(text.contains("canonical_digest: none"), "{text}");
}

#[test]
fn research_list_filters_and_prints_the_table() {
    let fixture = Fixture::start();
    let output = fixture.run(&[
        "research", "list", "--tier", "model", "--state", "created", "--limit", "5",
    ]);
    assert_success(&output);
    assert!(
        stdout(&output).contains("research_runs: 1"),
        "{}",
        stdout(&output)
    );
    assert!(stdout(&output).contains("MODEL-2026-11-17-01 tier=model state=created"));
    let request = fixture.last_request();
    assert_eq!(request.path, "/research/runs");
    assert_eq!(request.query.get("tier").map(String::as_str), Some("model"));
    assert_eq!(
        request.query.get("state").map(String::as_str),
        Some("created")
    );
    assert_eq!(request.query.get("limit").map(String::as_str), Some("5"));
}

#[test]
fn research_compare_and_decide_print_the_published_numbers_and_decision() {
    let fixture = Fixture::start();
    let compare = fixture.run(&[
        "research",
        "compare",
        "MODEL-2026-11-17-01",
        "--baseline",
        "MODEL-2026-11-04-04..05",
    ]);
    assert_success(&compare);
    let text = stdout(&compare);
    assert!(text.contains("metric candidate baseline delta"), "{text}");
    assert!(text.contains("1m.coverage_share 1 none none"), "{text}");
    assert!(
        text.contains("baseline_id: MODEL-2026-11-04-04..05"),
        "{text}"
    );
    assert_eq!(
        fixture
            .last_request()
            .query
            .get("baseline")
            .map(String::as_str),
        Some("MODEL-2026-11-04-04..05")
    );

    let decide = fixture.run(&[
        "research",
        "decide",
        "MODEL-2026-11-17-01",
        "--decision",
        "accepted",
        "--note",
        "the fixture passed",
    ]);
    assert_success(&decide);
    let text = stdout(&decide);
    assert!(text.contains("state: decided"), "{text}");
    assert!(text.contains("decision: accepted"), "{text}");
    assert!(text.contains("decision_note: the fixture passed"), "{text}");
    let request = fixture.last_request();
    assert_eq!(request.path, "/research/runs/MODEL-2026-11-17-01/decision");
    let body = request.json();
    assert_eq!(body["decision"], "accepted");
    assert_eq!(body["note"], "the fixture passed");
}

#[test]
fn baseline_and_invalidation_commands_print_their_registries() {
    let fixture = Fixture::start();
    let list = fixture.run(&["baseline", "list"]);
    assert_success(&list);
    let text = stdout(&list);
    assert!(text.contains("baselines: 1"), "{text}");
    assert!(
        text.contains("MODEL-2026-11-04-04..05 tier=model"),
        "{text}"
    );
    assert_eq!(fixture.last_request().path, "/research/baselines");

    let show = fixture.run(&["baseline", "show", "model"]);
    assert_success(&show);
    let text = stdout(&show);
    assert!(
        text.contains("baseline_id: MODEL-2026-11-04-04..05"),
        "{text}"
    );
    assert!(text.contains("accepted_by: Human"), "{text}");
    assert_eq!(fixture.last_request().path, "/research/baselines/model");

    let invalidations = fixture.run(&["invalidation", "list"]);
    assert_success(&invalidations);
    let text = stdout(&invalidations);
    assert!(text.contains("invalidations: 1"), "{text}");
    assert!(text.contains("INVALIDATION-2026-11-10-01"), "{text}");
    assert_eq!(fixture.last_request().path, "/research/invalidations");
}

#[test]
fn research_decide_requires_a_note() {
    let fixture = Fixture::start();
    let output = fixture.run(&[
        "research",
        "decide",
        "MODEL-2026-11-17-01",
        "--decision",
        "accepted",
    ]);
    assert_failure(&output);
    assert!(
        stderr(&output).contains("--note is required"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn research_conflicts_keep_the_research_error_code() {
    let fixture = Fixture::start();
    let output = fixture.run(&["research", "submit", "MODEL-CONFLICT"]);
    assert_failure(&output);
    assert!(
        stderr(&output).contains("research.conflict"),
        "{}",
        stderr(&output)
    );
    assert_eq!(fixture.last_request().method, "POST");
    assert_eq!(
        fixture.last_request().path,
        "/research/runs/MODEL-CONFLICT/submit"
    );
}
