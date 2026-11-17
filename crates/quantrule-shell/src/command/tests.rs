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
fn a_usage_error_carries_the_registry_usage() {
    use super::parse::parse_line;

    for name in names() {
        // `help` answers an unknown topic instead of echoing a usage line.
        if name == "help" {
            continue;
        }
        let input = format!("{name} --bogus");
        let error = parse_line(&input).expect_err("an unknown option is rejected");
        let registry = super::help::usage(name).expect("the registry lists the command");
        assert!(
            error.to_string().contains(registry),
            "{name}: {error}\nregistry usage: {registry}"
        );
    }
}

#[test]
fn the_short_map_names_the_global_flags_and_environment() {
    let mut buffer = Vec::new();
    run_help(&HelpTopic::Short, &mut buffer).expect("help renders");
    let text = String::from_utf8(buffer).expect("help is UTF-8");
    for expected in [
        "--profile",
        "--token",
        "--json",
        "QUANTRULE_API_URL",
        "QUANTRULE_HOME",
        "QUANTRULE_TOKEN",
    ] {
        assert!(text.contains(expected), "{expected} is missing");
    }
}

#[test]
fn run_detail_prints_the_frozen_model_artifact_or_none() {
    use quantrule_api::{Digest, Provenance, RunDetail, RunId, RunState, StageProgress};

    let digest = Digest::sha256(b"fixture");
    let detail = RunDetail {
        run_id: RunId(digest.clone()),
        status: RunState::Queued,
        owner: "default".to_owned(),
        stages: Vec::<StageProgress>::new(),
        model_artifact_id: Some(digest.clone()),
        provenance: Provenance {
            data_version: digest.clone(),
            config_hash: digest.clone(),
            model_version: "M1.0".to_owned(),
            image_digest: digest.clone(),
        },
        artifacts: Vec::new(),
        digest: None,
        error: None,
    };
    let mut buffer = Vec::new();
    crate::output::run_detail(&mut buffer, &detail).expect("run detail");
    let text = String::from_utf8(buffer).expect("UTF-8");
    assert!(
        text.contains(&format!("model_artifact_id: {digest}")),
        "{text}"
    );

    let mut buffer = Vec::new();
    let mut plain = detail;
    plain.model_artifact_id = None;
    crate::output::run_detail(&mut buffer, &plain).expect("run detail");
    let text = String::from_utf8(buffer).expect("UTF-8");
    assert!(text.contains("model_artifact_id: none"), "{text}");
}
