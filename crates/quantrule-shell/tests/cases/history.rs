//! History command output tests.

use crate::cases::assertions::{assert_success, stdout};
use crate::support::{Fixture, run_id};

#[test]
fn history_list_prints_the_run_kind() {
    let fixture = Fixture::start();
    let output = fixture.run(&["history", "list", "--kind", "run"]);
    assert_success(&output);
    let text = stdout(&output);
    assert!(text.starts_with("runs: 1\n"), "{text}");
    assert!(
        text.contains(&format!("{} status=succeeded", run_id())),
        "{text}"
    );
    assert!(text.contains("archived=no"), "{text}");
}

#[test]
fn history_archive_prints_the_receipt() {
    let fixture = Fixture::start();
    let output = fixture.run(&["history", "archive", "--kind", "run", "--id", &run_id()]);
    assert_success(&output);
    assert_eq!(stdout(&output), format!("archived run {}\n", run_id()));
    let request = fixture.last_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, format!("/runs/{}/archive", run_id()));
}

#[test]
fn history_erase_requires_confirmation_and_prints_bytes() {
    let fixture = Fixture::start();
    let rejected = fixture.run(&[
        "history",
        "erase",
        "--kind",
        "run",
        "--id",
        &run_id(),
        "--confirm",
        "sha256:other",
    ]);
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("--confirm"));

    let output = fixture.run(&[
        "history",
        "erase",
        "--kind",
        "run",
        "--id",
        &run_id(),
        "--confirm",
        &run_id(),
    ]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        format!("erased run {} bytes=0\n", run_id())
    );
    let request = fixture.last_request();
    assert_eq!(request.method, "DELETE");
    assert_eq!(request.path, format!("/runs/{}", run_id()));
}

#[test]
fn history_dry_run_calls_nothing() {
    let fixture = Fixture::start();
    let before = fixture.requests().len();
    let output = fixture.run(&[
        "history",
        "archive",
        "--kind",
        "run",
        "--id",
        &run_id(),
        "--dry-run",
    ]);
    assert_success(&output);
    assert_eq!(stdout(&output), format!("would archive run {}\n", run_id()));
    assert_eq!(fixture.requests().len(), before);
}

#[test]
fn history_archive_refuses_a_stage() {
    let fixture = Fixture::start();
    let output = fixture.run(&["history", "archive", "--kind", "stage", "--id", "x"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no archive"));
}
