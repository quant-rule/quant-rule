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

#[test]
fn history_list_parses_a_kind_and_the_history_filters() {
    let parsed = parse_line("history list --kind run --include-archived --limit 5")
        .expect("the line parses")
        .expect("a command");
    match parsed.command {
        Command::HistoryList {
            kind,
            include_archived,
            limit,
        } => {
            assert_eq!(kind.as_str(), "run");
            assert!(include_archived);
            assert_eq!(limit, Some(5));
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn history_archive_refuses_a_stage() {
    let error =
        parse_line("history archive --kind stage --id x").expect_err("a stage has no archive");
    assert!(error.to_string().contains("no archive"), "{error}");
}

#[test]
fn history_erase_requires_the_exact_confirmation() {
    assert!(parse_line("history erase --kind run --id a --confirm b").is_err());
    let parsed = parse_line("history erase --kind run --id a --confirm a --dry-run")
        .expect("the line parses")
        .expect("a command");
    assert!(matches!(
        parsed.command,
        Command::HistoryErase { dry_run: true, .. }
    ));
}

#[test]
fn history_rejects_an_unknown_kind() {
    assert!(parse_line("history list --kind config").is_err());
}

const PACK_ID: &str = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const SIGNAL_ID: &str = "sha256:2222222222222222222222222222222222222222222222222222222222222222";
const MODEL_ID: &str = "sha256:3333333333333333333333333333333333333333333333333333333333333333";
const DATA_ID: &str = "sha256:4444444444444444444444444444444444444444444444444444444444444444";

#[test]
fn pack_family_parses_every_leaf() {
    for line in [
        "pack create ./packs/strategy.json",
        "pack list --kind strategy --state frozen",
        &format!("pack show {PACK_ID}"),
        &format!("pack validate {PACK_ID}"),
        &format!("pack freeze {PACK_ID}"),
        &format!("pack retire {PACK_ID}"),
        &format!("pack fetch {PACK_ID} ./packs/out.json"),
    ] {
        assert!(
            parse_line(line).expect("the line parses").is_some(),
            "{line}"
        );
    }
}

#[test]
fn signal_family_parses_every_leaf() {
    let create = format!(
        "signal create --model-artifact {MODEL_ID} --data-version {DATA_ID} --config cfg_abc --strategy-pack {PACK_ID}"
    );
    for line in [
        create.as_str(),
        "signal list --limit 5",
        &format!("signal show {SIGNAL_ID}"),
        &format!("signal fetch {SIGNAL_ID} ./signals/out.json"),
    ] {
        assert!(
            parse_line(line).expect("the line parses").is_some(),
            "{line}"
        );
    }
    assert!(matches!(
        parse_line(&create).expect("parses").expect("command").command,
        Command::SignalCreate { ref config_id, .. } if config_id == "cfg_abc"
    ));
}

#[test]
fn a_bad_pack_kind_state_or_digest_is_a_usage_error() {
    for line in [
        "pack list --kind book",
        "pack list --state deleted",
        &format!("pack show sha256:{}", "z".repeat(64)),
        &format!("pack fetch {PACK_ID} ./out.json --extra"),
    ] {
        assert!(parse_line(line).is_err(), "{line}");
    }
}

#[test]
fn signal_create_requires_every_reference() {
    assert!(parse_line("signal create --config cfg_abc").is_err());
    assert!(
        parse_line(&format!(
            "signal create --model-artifact {MODEL_ID} --data-version {DATA_ID} --config cfg_abc"
        ))
        .is_err(),
        "the strategy pack is required"
    );
    assert!(
        parse_line(&format!(
            "signal create --model-artifact not-a-digest --data-version {DATA_ID} --config cfg_abc --strategy-pack {PACK_ID}"
        ))
        .is_err()
    );
}

#[test]
fn run_lineage_parses_one_run_id() {
    assert!(matches!(
        parse_line("run lineage run-1").expect("parses").expect("command").command,
        Command::RunLineage { ref run_id } if run_id == "run-1"
    ));
    assert!(parse_line("run lineage").is_err());
}
