//! Profile command output tests.

use serde_json::json;

use crate::cases::assertions::{assert_failure, assert_success, stderr, stdout};
use crate::support::Fixture;

#[test]
fn profile_list_marks_the_active_profile() {
    let fixture = Fixture::start();
    let output = fixture.run(&["profile", "list"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        "active: default\nprofiles: 2\n\
         alpha name=Alpha datasets=1 runs=0 simulations=0\n\
         default name=Default datasets=1 runs=1 simulations=1\n"
    );
}

#[test]
fn profile_create_derives_a_slug_and_sends_the_contract_body() {
    let fixture = Fixture::start();
    let output = fixture.run(&["profile", "create", "Research Lab"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        "created: true\nprofile_id: research-lab\nname: Research Lab\n\
         datasets: 0\nruns: 0\nsimulations: 0\n"
    );
    assert_eq!(
        fixture.last_request().json(),
        json!({ "profile_id": "research-lab", "name": "Research Lab" })
    );
}

#[test]
fn profile_use_confirms_the_profile_and_stores_it() {
    let fixture = Fixture::start();
    let output = fixture.run(&["profile", "use", "alpha"]);
    assert_success(&output);
    assert_eq!(stdout(&output), "active: alpha\n");
    assert_eq!(fixture.stored_profile().as_deref(), Some("alpha"));

    fixture.clear_requests();
    let listed = fixture.run(&["dataset", "list"]);
    assert_success(&listed);
    assert_eq!(
        fixture.last_request().header("x-quantrule-profile"),
        Some("alpha")
    );
}

#[test]
fn an_explicit_profile_wins_and_is_never_stored() {
    let fixture = Fixture::start();
    fixture.store_profile("alpha");
    let output = fixture.run(&["--profile", "default", "dataset", "list"]);
    assert_success(&output);
    assert_eq!(
        fixture.last_request().header("x-quantrule-profile"),
        Some("default")
    );
    assert_eq!(fixture.stored_profile().as_deref(), Some("alpha"));
}

#[test]
fn profile_use_refuses_a_global_profile_and_leaves_the_default() {
    let fixture = Fixture::start();
    fixture.store_profile("alpha");
    let output = fixture.run(&["--profile", "alpha", "profile", "use", "default"]);
    assert_failure(&output);
    assert!(stderr(&output).contains("profile use cannot run with --profile"));
    assert_eq!(fixture.stored_profile().as_deref(), Some("alpha"));
}

#[test]
fn an_unknown_stored_profile_is_not_replaced_by_the_shell() {
    let fixture = Fixture::start();
    fixture.store_profile("ghost");
    let output = fixture.run(&["dataset", "list"]);
    assert_failure(&output);
    assert!(
        stderr(&output).contains("profile.not_found: profile ghost is unknown"),
        "{}",
        stderr(&output)
    );
    assert_eq!(fixture.stored_profile().as_deref(), Some("ghost"));
}

#[test]
fn a_missing_profile_is_refused_by_profile_use_without_a_write() {
    let fixture = Fixture::start();
    let output = fixture.run(&["profile", "use", "ghost"]);
    assert_failure(&output);
    assert!(stderr(&output).contains("profile.not_found"));
    assert_eq!(fixture.stored_profile(), None);
}
