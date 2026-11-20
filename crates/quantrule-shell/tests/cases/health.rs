//! Health command output tests.

use crate::cases::assertions::{assert_success, stdout};
use crate::support::{Fixture, image_digest};

#[test]
fn health_prints_the_aggregate_and_sorted_services() {
    let fixture = Fixture::start();
    let output = fixture.run(&["health"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        format!(
            "api: quantrule.api/v1\nstate: degraded\nqueue_depth: 2\nworkers: 1\n\
             image_digest: {}\nservice minio: degraded (slow)\nservice postgres: healthy\n",
            image_digest()
        )
    );
    let request = fixture.last_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/health");
    assert_eq!(request.header("x-quantrule-profile"), Some("default"));
}
