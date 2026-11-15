//! Printed-output tests for the shell's bearer-token seam.
//!
//! The fixture requires `Authorization: Bearer` on every gated route, mirroring
//! the gateway. Each test runs the real binary and asserts on stdout, stderr,
//! the exit status, the captured header, and the absence of the token from
//! printed output and local state.

mod support;

use std::fs;
use std::path::Path;
use std::process::Output;

use support::Fixture;

const TOKEN: &str = "qr_test_token_0123456789abcdef0123456789abcdef";
const OTHER: &str = "qr_other_token_fedcba9876543210fedcba9876543210";

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

/// The exact header value the gateway compares.
fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

/// Whether any file under the shell's state directory holds the token.
fn state_contains(fixture: &Fixture, needle: &str) -> bool {
    fn walk(path: &Path, needle: &str) -> bool {
        let Ok(entries) = fs::read_dir(path) else {
            return false;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if walk(&path, needle) {
                    return true;
                }
            } else if fs::read_to_string(&path).is_ok_and(|text| text.contains(needle)) {
                return true;
            }
        }
        false
    }
    walk(fixture.home(), needle)
}

#[test]
fn a_gated_route_is_refused_without_a_token() {
    let fixture = Fixture::start();
    fixture.require_token(TOKEN);
    let output = fixture.run(&["dataset", "list"]);
    assert_failure(&output);
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).contains("auth.unauthorized"),
        "{}",
        stderr(&output)
    );
    assert_eq!(fixture.last_request().header("authorization"), None);
}

#[test]
fn an_explicit_token_is_sent_on_every_request() {
    let fixture = Fixture::start();
    let baseline = fixture.run(&["dataset", "list"]);
    assert_success(&baseline);

    fixture.require_token(TOKEN);
    let output = fixture.run(&["--token", TOKEN, "dataset", "list"]);
    assert_success(&output);
    assert_eq!(stdout(&output), stdout(&baseline));
    assert_eq!(stderr(&output), "");
    let expected = bearer(TOKEN);
    assert_eq!(
        fixture.last_request().header("authorization"),
        Some(expected.as_str())
    );
    assert!(!stdout(&output).contains(TOKEN));
    assert!(!stderr(&output).contains(TOKEN));
    assert!(!state_contains(&fixture, TOKEN));
}

#[test]
fn the_environment_token_is_sent() {
    let fixture = Fixture::start();
    fixture.require_token(TOKEN);
    let output = fixture.run_with_env(&["dataset", "list"], "QUANTRULE_TOKEN", TOKEN);
    assert_success(&output);
    let expected = bearer(TOKEN);
    assert_eq!(
        fixture.last_request().header("authorization"),
        Some(expected.as_str())
    );
    assert!(!stdout(&output).contains(TOKEN));
    assert!(!stderr(&output).contains(TOKEN));
}

#[test]
fn the_flag_wins_over_the_environment_token() {
    let fixture = Fixture::start();
    fixture.require_token(TOKEN);
    let output = fixture.run_with_env(
        &["--token", TOKEN, "dataset", "list"],
        "QUANTRULE_TOKEN",
        OTHER,
    );
    assert_success(&output);
    let expected = bearer(TOKEN);
    assert_eq!(
        fixture.last_request().header("authorization"),
        Some(expected.as_str())
    );
}

#[test]
fn a_wrong_token_is_refused_without_echoing_it() {
    let fixture = Fixture::start();
    fixture.require_token(TOKEN);
    let output = fixture.run(&["--token", OTHER, "dataset", "list"]);
    assert_failure(&output);
    assert!(stderr(&output).contains("auth.unauthorized"));
    assert!(!stderr(&output).contains(OTHER));
    assert!(!stdout(&output).contains(OTHER));
    let expected = bearer(OTHER);
    assert_eq!(
        fixture.last_request().header("authorization"),
        Some(expected.as_str())
    );
}

#[test]
fn the_json_view_does_not_print_the_token() {
    let fixture = Fixture::start();
    fixture.require_token(TOKEN);
    let output = fixture.run(&["--json", "--token", TOKEN, "dataset", "list"]);
    assert_success(&output);
    assert!(!stdout(&output).contains(TOKEN));
    assert!(stdout(&output).starts_with('['), "{}", stdout(&output));
}

#[test]
fn an_interactive_line_carries_the_token() {
    let fixture = Fixture::start();
    fixture.require_token(TOKEN);
    let output = fixture.run_interactive(&format!("dataset list --token {TOKEN}\nquit\n"));
    assert_success(&output);
    let expected = bearer(TOKEN);
    assert_eq!(
        fixture.last_request().header("authorization"),
        Some(expected.as_str())
    );
    assert!(!stdout(&output).contains(TOKEN));
    assert!(!stderr(&output).contains(TOKEN));
}

#[test]
fn health_stays_open_on_a_token_protected_gateway() {
    let fixture = Fixture::start();
    fixture.require_token(TOKEN);
    let output = fixture.run(&["health"]);
    assert_success(&output);
    assert_eq!(fixture.last_request().header("authorization"), None);
}

#[test]
fn an_invalid_token_is_rejected_before_a_request_and_not_echoed() {
    let fixture = Fixture::start();
    let output = fixture.run(&["--token", "two words", "dataset", "list"]);
    assert_failure(&output);
    let rendered = stderr(&output);
    assert!(rendered.contains("--token"), "{rendered}");
    assert!(!rendered.contains("two words"), "{rendered}");
    assert!(fixture.requests().is_empty());
}
