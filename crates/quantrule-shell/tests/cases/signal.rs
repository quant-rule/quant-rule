//! Signal command output tests.

use std::fs;

use serde_json::json;

use crate::cases::assertions::{assert_failure, assert_success, stdout};
use crate::support::{Fixture, data_version, digest, pack_id, signal_body, signal_id};

#[test]
fn signal_create_sends_the_four_references_and_prints_the_artifact() {
    let fixture = Fixture::start();
    let model_artifact = digest('9');
    let output = fixture.run(&[
        "signal",
        "create",
        "--model-artifact",
        &model_artifact,
        "--data-version",
        &data_version(),
        "--config",
        "cfg_abc",
        "--strategy-pack",
        &pack_id(),
    ]);
    assert_success(&output);
    let text = stdout(&output);
    assert!(
        text.contains("schema: quantrule.signal-response/v1\n"),
        "{text}"
    );
    assert!(text.contains("created: true\n"), "{text}");
    assert!(
        text.contains(&format!("signal_id: {}\n", signal_id())),
        "{text}"
    );
    assert!(text.contains("kind: signal\n"), "{text}");
    assert!(text.contains("bytes: "), "{text}");

    let request = fixture.last_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/signals");
    assert_eq!(
        request.json(),
        json!({
            "model_artifact_id": model_artifact,
            "data_version": data_version(),
            "config_id": "cfg_abc",
            "strategy_pack_id": pack_id(),
        })
    );
}

#[test]
fn signal_list_sends_the_limit_and_prints_metadata_only() {
    let fixture = Fixture::start();
    let output = fixture.run(&["signal", "list", "--limit", "5"]);
    assert_success(&output);
    let text = stdout(&output);
    assert!(text.contains("signals: 1\n"), "{text}");
    assert!(
        text.contains(&format!("{} kind=signal", signal_id())),
        "{text}"
    );
    assert!(!text.contains("\"rows\""), "{text}");
    let request = fixture.last_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/signals");
    assert_eq!(request.query.get("limit").map(String::as_str), Some("5"));
}

#[test]
fn signal_show_prints_one_record_and_json_prints_the_wire_type() {
    let fixture = Fixture::start();
    let output = fixture.run(&["--json", "signal", "show", &signal_id()]);
    assert_success(&output);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("the signal JSON");
    assert_eq!(value["summary"]["artifact_id"], signal_id());
    assert_eq!(value["summary"]["kind"], "signal");
    let request = fixture.last_request();
    assert_eq!(request.path, format!("/signals/{}", signal_id()));

    let text = fixture.run(&["signal", "show", &signal_id()]);
    assert_success(&text);
    let text = stdout(&text);
    assert!(
        text.contains(&format!("signal_id: {}\n", signal_id())),
        "{text}"
    );
    assert!(text.contains("object_key: artifacts/"), "{text}");
}

#[test]
fn signal_fetch_verifies_and_writes_the_document() {
    let fixture = Fixture::start();
    let file = fixture.home().join("signals/signal-1.json");
    let output = fixture.run(&[
        "signal",
        "fetch",
        &signal_id(),
        file.to_str().expect("the signal path"),
    ]);
    assert_success(&output);
    let text = stdout(&output);
    assert!(
        text.contains(&format!("signal_id: {}\n", signal_id())),
        "{text}"
    );
    assert_eq!(fs::read(&file).expect("the signal download"), signal_body());
    let request = fixture.last_request();
    assert_eq!(request.path, format!("/signals/{}/bytes", signal_id()));
}

#[test]
fn signal_commands_refuse_a_bad_digest_or_missing_option_before_the_gateway() {
    let fixture = Fixture::start();
    fixture.clear_requests();
    for line in [
        vec!["signal", "show", "sha256:not-hex"],
        vec!["signal", "create", "--config", "cfg_abc"],
    ] {
        let output = fixture.run(&line);
        assert_failure(&output);
    }
    let model = digest('9');
    let data = data_version();
    let missing_option = vec![
        "signal",
        "create",
        "--model-artifact",
        model.as_str(),
        "--data-version",
        data.as_str(),
        "--config",
        "cfg_abc",
    ];
    let output = fixture.run(&missing_option);
    assert_failure(&output);
    assert!(
        fixture.requests().is_empty(),
        "a parser failure must not reach the gateway"
    );

    let missing = format!("sha256:{}", "9".repeat(64));
    let output = fixture.run(&["signal", "show", &missing]);
    assert_failure(&output);
    let requests = fixture.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, format!("/signals/{}", missing));
}
