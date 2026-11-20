//! Verify command output tests.

use crate::cases::assertions::{assert_success, stdout};
use crate::support::{Fixture, canonical_digest, run_id};

#[test]
fn verify_prints_the_server_digest() {
    let fixture = Fixture::start();
    let output = fixture.run(&["verify", &run_id()]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        format!("run_id: {}\ndigest: {}\n", run_id(), canonical_digest())
    );
    assert_eq!(
        fixture.last_request().path,
        format!("/runs/{}/digest", run_id())
    );
}
