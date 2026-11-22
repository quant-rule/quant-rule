//! The pre-change command contracts, checked byte for byte.
//!
//! RA-009 appends pack, signal, and lineage commands. The fixture holds the exact
//! `help <command>` text for the 49 commands that existed before the change, so a
//! drift in an existing usage line, summary, output, or note fails here.

mod support;

use std::collections::BTreeMap;

use support::Fixture;

fn expected() -> BTreeMap<String, String> {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/command-surface/help-49.json"
    ))
    .expect("the pre-change help fixture is JSON")
}

#[test]
fn the_49_existing_help_contracts_are_byte_identical() {
    let fixture = Fixture::start();
    let expected = expected();
    assert_eq!(expected.len(), 49);
    for (name, text) in expected {
        let output = fixture.run(&["help", &name]);
        assert!(
            output.status.success(),
            "help {name} failed; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let observed = String::from_utf8(output.stdout).expect("help is UTF-8");
        assert_eq!(observed, text, "the contract for '{name}' changed");
    }
}
