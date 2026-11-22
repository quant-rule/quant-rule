//! Pack command output tests.

use std::fs;

use crate::cases::assertions::{assert_failure, assert_success, stdout};
use crate::support::{Fixture, pack_body, pack_id};

#[test]
fn pack_create_sends_the_file_bytes_and_prints_the_record() {
    let fixture = Fixture::start();
    let path = fixture.home().join("strategy-pack.json");
    fs::create_dir_all(fixture.home()).expect("home");
    fs::write(&path, pack_body()).expect("pack file");

    let output = fixture.run(&["pack", "create", path.to_str().expect("the pack path")]);
    assert_success(&output);
    let text = stdout(&output);
    assert!(
        text.contains(&format!("pack_id: {}\n", pack_id())),
        "{text}"
    );
    assert!(text.contains("kind: strategy\n"), "{text}");
    assert!(text.contains("state: draft\n"), "{text}");
    assert!(text.contains("name: parity-pack\n"), "{text}");
    assert!(text.contains("validations: 0\n"), "{text}");
    assert!(text.contains("events: 1\n"), "{text}");

    let request = fixture.last_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/catalog/packs");
    assert_eq!(request.body, pack_body());
    assert_eq!(request.header("content-type"), Some("application/json"));
}

#[test]
fn pack_list_sends_the_kind_and_state_filters() {
    let fixture = Fixture::start();
    let output = fixture.run(&["pack", "list", "--kind", "strategy", "--state", "frozen"]);
    assert_success(&output);
    let text = stdout(&output);
    assert!(text.contains("packs: 1\n"), "{text}");
    assert!(text.contains("state=frozen"), "{text}");

    let request = fixture.last_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/catalog/packs");
    assert_eq!(
        request.query.get("kind").map(String::as_str),
        Some("strategy")
    );
    assert_eq!(
        request.query.get("state").map(String::as_str),
        Some("frozen")
    );
}

#[test]
fn pack_show_and_every_lifecycle_transition_print_the_record() {
    for (command, state) in [
        ("show", "frozen"),
        ("validate", "validated"),
        ("freeze", "frozen"),
        ("retire", "retired"),
    ] {
        let fixture = Fixture::start();
        let output = fixture.run(&["pack", command, &pack_id()]);
        assert_success(&output);
        let text = stdout(&output);
        assert!(text.contains(&format!("state: {state}\n")), "{text}");
        assert!(
            text.contains(&format!("pack_id: {}\n", pack_id())),
            "{text}"
        );
        let request = fixture.last_request();
        let expected = if command == "show" {
            assert_eq!(request.method, "GET");
            format!("/catalog/packs/{}", pack_id())
        } else {
            assert_eq!(request.method, "POST");
            format!("/catalog/packs/{}/{command}", pack_id())
        };
        assert_eq!(request.path, expected);
    }
}

#[test]
fn pack_fetch_verifies_and_writes_the_document() {
    let fixture = Fixture::start();
    let file = fixture.home().join("downloads/pack.json");
    let output = fixture.run(&[
        "pack",
        "fetch",
        &pack_id(),
        file.to_str().expect("the file path"),
    ]);
    assert_success(&output);
    let text = stdout(&output);
    assert!(
        text.contains(&format!("pack_id: {}\n", pack_id())),
        "{text}"
    );
    assert!(text.contains("bytes: "), "{text}");
    assert_eq!(fs::read(&file).expect("the pack download"), pack_body());
    let request = fixture.last_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, format!("/catalog/packs/{}/bytes", pack_id()));
}

#[test]
fn pack_json_mode_prints_the_wire_record() {
    let fixture = Fixture::start();
    let output = fixture.run(&["--json", "pack", "show", &pack_id()]);
    assert_success(&output);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("the pack JSON");
    assert_eq!(value["summary"]["pack_id"], pack_id());
    assert_eq!(value["summary"]["state"], "frozen");
}

#[test]
fn pack_commands_refuse_a_bad_filter_or_digest_before_the_gateway() {
    let fixture = Fixture::start();
    fixture.clear_requests();
    let bad_kind = fixture.run(&["pack", "list", "--kind", "book"]);
    assert_failure(&bad_kind);
    let bad_digest = fixture.run(&["pack", "show", "sha256:not-hex"]);
    assert_failure(&bad_digest);
    let missing = format!("sha256:{}", "9".repeat(64));
    let missing_pack = fixture.run(&["pack", "show", &missing]);
    assert_failure(&missing_pack);
    assert!(
        stdout(&missing_pack).is_empty(),
        "the gateway failure writes stderr"
    );
    let requests = fixture.requests();
    assert_eq!(
        requests.len(),
        1,
        "only the well-formed unknown id reaches the gateway"
    );
    assert_eq!(requests[0].path, format!("/catalog/packs/{}", missing));
}
