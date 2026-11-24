//! Remote command construction tests for the shell.
//!
//! The remote client path is the environment path: `QUANTRULE_API_URL`,
//! `QUANTRULE_CA_CERT`, and `QUANTRULE_TOKEN` (or `QUANTRULE_TOKEN_FILE`), with
//! `--profile` and `--token` as explicit overrides. The shell has no `--remote`,
//! `--cacert`, or `--url` flag; `--remote` belongs to `qr-deploy check`.
//!
//! These tests use the in-process HTTP fixture, never a live gateway. They prove
//! precedence, a missing or malformed CA fails closed, and no token or CA body
//! reaches stdout or stderr.

mod support;

use std::process::{Command, Output};

use support::{BIN, Fixture};

const TOKEN: &str = "qr_remote_test_token_0123456789abcdef";

fn run(fixture: &Fixture, args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(BIN);
    command
        .args(args)
        .env("QUANTRULE_API_URL", fixture.url())
        .env("QUANTRULE_HOME", fixture.home())
        .env_remove("QUANTRULE_TOKEN")
        .env_remove("QUANTRULE_TOKEN_FILE")
        .env_remove("QUANTRULE_CA_CERT")
        .env_remove("CURL_CA_BUNDLE");
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("the quantrule binary runs")
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn the_api_url_and_token_environment_reach_the_gateway() {
    let fixture = Fixture::start();
    fixture.require_token(TOKEN);
    let output = run(
        &fixture,
        &["profile", "list"],
        &[("QUANTRULE_TOKEN", TOKEN)],
    );
    assert!(output.status.success(), "{}", text(&output));
    let request = fixture.last_request();
    let expected = format!("Bearer {TOKEN}");
    assert_eq!(request.path, "/profiles");
    assert_eq!(request.header("authorization"), Some(expected.as_str()));
    assert!(!text(&output).contains(TOKEN));
}

#[test]
fn a_trailing_slash_in_the_api_url_is_tolerated() {
    let fixture = Fixture::start();
    let base = format!("{}/", fixture.url());
    let output = run(
        &fixture,
        &["health"],
        &[("QUANTRULE_API_URL", base.as_str())],
    );
    assert!(output.status.success(), "{}", text(&output));
    assert_eq!(fixture.last_request().path, "/health");
}

#[test]
fn an_explicit_token_wins_over_the_environment() {
    let fixture = Fixture::start();
    let explicit = "qr_explicit_token_0123456789";
    fixture.require_token(explicit);
    let output = run(
        &fixture,
        &["--token", explicit, "profile", "list"],
        &[("QUANTRULE_TOKEN", "qr_wrong_token_0123456789")],
    );
    assert!(output.status.success(), "{}", text(&output));
    let expected = format!("Bearer {explicit}");
    assert_eq!(
        fixture.last_request().header("authorization"),
        Some(expected.as_str())
    );
    let printed = text(&output);
    assert!(!printed.contains(explicit));
    assert!(!printed.contains("qr_wrong_token"));
}

#[test]
fn an_explicit_profile_wins_over_the_stored_default() {
    let fixture = Fixture::start();
    fixture.add_profile("beta", "Beta");
    fixture.store_profile("alpha");
    let output = run(&fixture, &["--profile", "beta", "profile", "list"], &[]);
    assert!(output.status.success(), "{}", text(&output));
    assert_eq!(
        fixture.last_request().header("x-quantrule-profile"),
        Some("beta")
    );
    assert_eq!(fixture.stored_profile().as_deref(), Some("alpha"));
}

#[test]
fn a_missing_ca_file_fails_closed_and_does_not_reach_the_gateway() {
    let fixture = Fixture::start();
    let missing = fixture.home().join("missing-ca.pem");
    let output = run(
        &fixture,
        &["health"],
        &[
            ("QUANTRULE_CA_CERT", missing.to_str().expect("path")),
            ("QUANTRULE_TOKEN", TOKEN),
        ],
    );
    assert!(!output.status.success());
    let printed = text(&output);
    assert!(printed.contains("cannot use"), "{printed}");
    assert!(!printed.contains(TOKEN), "{printed}");
    assert!(fixture.requests().is_empty());
}

#[test]
fn a_malformed_ca_fails_closed_without_printing_its_body() {
    let fixture = Fixture::start();
    std::fs::create_dir_all(fixture.home()).expect("home");
    let ca = fixture.home().join("ca.pem");
    std::fs::write(&ca, "CA-BODY-MARKER\n").expect("fixture CA");
    let output = run(
        &fixture,
        &["health"],
        &[
            ("QUANTRULE_CA_CERT", ca.to_str().expect("path")),
            ("QUANTRULE_TOKEN", TOKEN),
        ],
    );
    assert!(!output.status.success());
    let printed = text(&output);
    assert!(printed.contains("PEM CA certificate"), "{printed}");
    assert!(!printed.contains("CA-BODY-MARKER"), "{printed}");
    assert!(!printed.contains(TOKEN), "{printed}");
    assert!(fixture.requests().is_empty());
}

#[test]
fn the_shell_rejects_remote_url_and_ca_flags() {
    let fixture = Fixture::start();
    for (args, expected) in [
        (&["--remote", "health"][..], "unknown command '--remote'"),
        (
            &["health", "--url", "https://quantrule.dev/api/v1"][..],
            "unexpected argument '--url'",
        ),
        (
            &["health", "--cacert", "/tmp/ca.pem"][..],
            "unexpected argument '--cacert'",
        ),
    ] {
        let output = run(&fixture, args, &[]);
        assert!(!output.status.success(), "{args:?}");
        let printed = text(&output);
        assert!(printed.contains(expected), "{expected}: {printed}");
        assert!(
            fixture.requests().is_empty(),
            "{args:?} contacted the gateway"
        );
    }
}
