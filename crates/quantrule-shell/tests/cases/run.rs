//! Run command output tests.

use std::fs;

use serde_json::json;

use crate::cases::assertions::{assert_success, stdout};
use crate::support::{Fixture, canonical_digest, data_version, digest, run_id};

#[test]
fn run_submit_sends_the_contract_body() {
    let fixture = Fixture::start();
    let output = fixture.run(&["run", "submit", &data_version(), "cfg_abc"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        format!("run_id: {}\nstatus: queued\ncreated: true\n", run_id())
    );
    let request = fixture.last_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/runs");
    assert_eq!(
        request.json(),
        json!({ "data_version": data_version(), "config_id": "cfg_abc" })
    );
}

#[test]
fn run_submit_sends_the_optional_model_artifact() {
    let fixture = Fixture::start();
    let artifact = digest('9');
    let output = fixture.run(&[
        "run",
        "submit",
        &data_version(),
        "cfg_abc",
        "--model-artifact",
        &artifact,
    ]);
    assert_success(&output);
    let request = fixture.last_request();
    assert_eq!(request.path, "/runs");
    assert_eq!(
        request.json(),
        json!({
            "data_version": data_version(),
            "config_id": "cfg_abc",
            "model_artifact_id": artifact,
        })
    );
}

#[test]
fn run_status_prints_stages_provenance_artifacts_and_digest() {
    let fixture = Fixture::start();
    let output = fixture.run(&["run", "status", &run_id()]);
    assert_success(&output);
    let text = stdout(&output);
    assert!(text.starts_with(&format!("run_id: {}\nstatus: succeeded\n", run_id())));
    assert!(text.contains("stage data: done 1200ms"));
    assert!(text.contains("stage evidence: done 120ms"));
    assert!(text.contains(&format!("digest: {}", canonical_digest())));
    assert!(text.ends_with("error: none\n"));
}

#[test]
fn run_list_passes_the_filters_and_sorts_the_rows() {
    let fixture = Fixture::start();
    let output = fixture.run(&["run", "list", "--status", "succeeded", "--limit", "5"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        format!(
            "runs: 1\n{} status=succeeded config=cfg_abc data={} submitted=2026-11-13T12:00:00Z elapsed=1200ms\n",
            run_id(),
            data_version()
        )
    );
    assert_eq!(
        fixture
            .last_request()
            .query
            .get("status")
            .map(String::as_str),
        Some("succeeded")
    );
    assert_eq!(
        fixture
            .last_request()
            .query
            .get("limit")
            .map(String::as_str),
        Some("5")
    );
}

#[test]
fn run_fetch_writes_every_artifact_and_verifies_its_digest() {
    let fixture = Fixture::start();
    let directory = fixture.home().join("artifacts");
    let output = fixture.run(&[
        "run",
        "fetch",
        &run_id(),
        directory.to_str().expect("UTF-8 path"),
    ]);
    assert_success(&output);
    let written = fs::read(directory.join("results.json")).expect("the artifact is written");
    assert_eq!(written, b"{\"ok\":true}\n");
    let text = stdout(&output);
    assert!(text.contains("artifact results.json: bytes="));
    assert!(text.contains("written="));
    assert_eq!(
        fixture.last_request().path,
        format!("/runs/{}/artifacts/results.json", run_id())
    );
}

#[test]
fn run_cancel_prints_the_new_state() {
    let fixture = Fixture::start();
    let output = fixture.run(&["run", "cancel", &run_id()]);
    assert_success(&output);
    assert!(stdout(&output).contains("status: cancelled"));
    assert_eq!(
        fixture.last_request().path,
        format!("/runs/{}/cancel", run_id())
    );
}
