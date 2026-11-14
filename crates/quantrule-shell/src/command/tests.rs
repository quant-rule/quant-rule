//! Command-layer tests that do not need a gateway.
//!
//! The HTTP surface is covered by the integration tests; these tests protect
//! the one registry that drives help, the README, and the surface checker.

use std::io::Write;

use super::help::{names, run_help};
use super::types::HelpTopic;

#[test]
fn help_renders_every_registered_command() {
    for name in names() {
        let mut buffer = Vec::new();
        run_help(&HelpTopic::Command(name.to_owned()), &mut buffer).expect("help renders");
        let text = String::from_utf8(buffer).expect("help is UTF-8");
        assert!(text.starts_with(name), "{name}: {text}");
        assert!(text.contains("usage:"), "{name}: {text}");
        assert!(text.contains("output:"), "{name}: {text}");
    }
}

#[test]
fn help_all_lists_every_usage_once() {
    let mut buffer = Vec::new();
    run_help(&HelpTopic::All, &mut buffer).expect("help renders");
    buffer.flush().expect("the buffer flushes");
    let text = String::from_utf8(buffer).expect("help is UTF-8");
    for name in names() {
        assert!(
            text.contains(&format!("{name}\n  usage:")),
            "{name} is missing"
        );
    }
}

#[test]
fn the_short_map_names_the_global_flags_and_environment() {
    let mut buffer = Vec::new();
    run_help(&HelpTopic::Short, &mut buffer).expect("help renders");
    let text = String::from_utf8(buffer).expect("help is UTF-8");
    for expected in ["--profile", "--json", "QUANTRULE_API_URL", "QUANTRULE_HOME"] {
        assert!(text.contains(expected), "{expected} is missing");
    }
}
