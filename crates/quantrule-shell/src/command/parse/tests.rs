use super::{parse_argv, parse_line};
use crate::command::types::{Command, HelpTopic};

fn argv(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn every_family_parses_its_leaf() {
    let cases = [
        (vec!["health"], "health"),
        (vec!["dataset", "list"], "list"),
        (vec!["dataset", "show", "sha256:a"], "show"),
        (vec!["dataset", "upload", "c.csv"], "upload"),
        (vec!["config", "list"], "list"),
        (vec!["config", "show", "cfg"], "show"),
        (vec!["run", "submit", "d", "c"], "submit"),
        (vec!["run", "status", "r"], "status"),
        (vec!["run", "list"], "list"),
        (vec!["run", "fetch", "r", "out"], "fetch"),
        (vec!["run", "cancel", "r"], "cancel"),
        (vec!["stage", "run", "model", "d", "c"], "model"),
        (vec!["stage", "run", "strategy", "m", "c"], "strategy"),
        (vec!["stage", "explain", "s", "--bar", "2"], "explain"),
        (vec!["stage", "list"], "list"),
        (vec!["stage", "show", "s"], "show"),
        (vec!["stage", "events", "s", "--timeframe", "5m"], "events"),
        (vec!["stage", "fetch", "s", "out"], "fetch"),
        (vec!["stage", "cancel", "s"], "cancel"),
        (vec!["stage", "delete", "s"], "delete"),
        (vec!["inspect", "candles", "d"], "candles"),
        (vec!["inspect", "timeframes", "d"], "timeframes"),
        (
            vec!["inspect", "events", "d", "--timeframe", "5m"],
            "events",
        ),
        (vec!["verify", "r"], "verify"),
        (vec!["profile", "list"], "list"),
        (vec!["profile", "create", "Research"], "create"),
        (vec!["profile", "use", "research"], "use"),
        (vec!["simulation", "list"], "list"),
        (vec!["simulation", "windows", "s"], "windows"),
        (
            vec![
                "simulation",
                "mark",
                "s",
                "--chart",
                "w1",
                "--bar",
                "4",
                "--action",
                "entry",
                "--quality",
                "good",
                "--side",
                "long",
            ],
            "mark",
        ),
        (vec!["simulation", "marks", "s"], "marks"),
        (vec!["simulation", "oracle", "s"], "oracle"),
        (vec!["simulation", "check", "s"], "check"),
        (vec!["simulation", "decide", "s", "test"], "decide"),
    ];
    for (tokens, _label) in cases {
        let parsed = parse_argv(&argv(&tokens)).expect("the command parses");
        assert!(parsed.is_some(), "{tokens:?}");
    }
}

#[test]
fn a_global_profile_is_removed_from_the_command() {
    let parsed = parse_line("--profile research run list")
        .expect("the line parses")
        .expect("a command");
    assert_eq!(parsed.globals.profile.as_deref(), Some("research"));
    assert!(matches!(parsed.command, Command::RunList { .. }));
}

#[test]
fn a_global_json_flag_is_removed_from_the_command() {
    let parsed = parse_line("health --json")
        .expect("the line parses")
        .expect("a command");
    assert!(parsed.globals.json);
    assert!(matches!(parsed.command, Command::Health));
}

#[test]
fn help_topics_are_one_or_two_words() {
    assert_eq!(
        parse_line("help all").expect("parses").expect("command"),
        crate::command::types::Invocation {
            globals: Default::default(),
            command: Command::Help(HelpTopic::All),
        }
    );
    assert!(matches!(
        parse_line("help run status")
            .expect("parses")
            .expect("command")
            .command,
        Command::Help(HelpTopic::Command(_))
    ));
}

#[test]
fn a_simulation_create_carries_every_required_option() {
    let bundle = format!("sha256:{}", "a".repeat(64));
    let parsed = parse_line(&format!(
        "simulation create --premise p --bundle {bundle} --inputs context --seed 7 --criteria c"
    ))
    .expect("the line parses")
    .expect("a command");
    assert!(matches!(
        parsed.command,
        Command::SimulationCreate {
            seed: 7,
            ref inputs,
            ..
        } if inputs == "context"
    ));
}

#[test]
fn an_unknown_option_is_rejected() {
    assert!(parse_line("run list --path /tmp").is_err());
}

#[test]
fn a_duplicate_profile_is_rejected() {
    assert!(parse_line("--profile a --profile b health").is_err());
}

#[test]
fn a_global_token_is_removed_from_the_command() {
    let parsed = parse_line("--token secret-token health")
        .expect("the line parses")
        .expect("a command");
    assert_eq!(
        parsed
            .globals
            .token
            .as_ref()
            .map(super::BearerToken::as_str),
        Some("secret-token")
    );
    assert!(matches!(parsed.command, Command::Health));
}

#[test]
fn a_token_after_the_command_is_still_global() {
    let parsed = parse_line("dataset list --token=secret-token")
        .expect("the line parses")
        .expect("a command");
    assert_eq!(
        parsed
            .globals
            .token
            .as_ref()
            .map(super::BearerToken::as_str),
        Some("secret-token")
    );
    assert!(matches!(parsed.command, Command::DatasetList));
}

#[test]
fn a_duplicate_token_is_rejected() {
    assert!(parse_line("--token a --token b health").is_err());
}

#[test]
fn an_empty_or_spaced_token_is_rejected_without_echoing_it() {
    for line in ["--token= health", "--token 'two words' health"] {
        let error = parse_line(line).expect_err("the token is invalid");
        let rendered = error.to_string();
        assert!(!rendered.contains("two words"), "{rendered}");
        assert!(rendered.contains("--token"), "{rendered}");
    }
}
