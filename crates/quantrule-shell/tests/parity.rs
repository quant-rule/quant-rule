//! Parity between the shell and the browser fixture.
//!
//! The browser's run-detail fixture is the reference: both clients read the same
//! `RunDetail` from the same profile and must present the same numbers and
//! codes. The browser E2E suite owns the DOM comparison (`DIST-071`); this test
//! proves the shell prints the fixture's contract values for the same profile.

mod support;

use std::process::Output;

use support::{Fixture, canonical_digest, config_hash, data_version, digest, image_digest, run_id};

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

#[test]
fn the_shell_prints_the_browser_run_detail_fixture() {
    let fixture = Fixture::start_parity();
    let output = fixture.run(&["run", "status", &run_id()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let expected = format!(
        "run_id: {run_id}\n\
         status: succeeded\n\
         owner: default\n\
         data_version: {data}\n\
         config_hash: {config}\n\
         model_version: M1.0\n\
         image_digest: {image}\n\
         stages: 4\n\
         stage data: done 1200ms\n\
         stage model: done 2400ms\n\
         stage backtest: done 3600ms\n\
         stage evidence: done 120ms\n\
         artifacts: 1\n\
         artifact results.json: bytes=2048 sha256={artifact}\n\
         digest: {digest}\n\
         error: none\n",
        run_id = run_id(),
        data = data_version(),
        config = config_hash(),
        image = image_digest(),
        artifact = digest('f'),
        digest = canonical_digest(),
    );
    assert_eq!(stdout(&output), expected);

    let request = fixture.last_request();
    assert_eq!(request.path, format!("/runs/{}", run_id()));
    assert_eq!(
        request.header("x-quantrule-profile"),
        Some("default"),
        "the browser fixture uses the default profile"
    );
}
