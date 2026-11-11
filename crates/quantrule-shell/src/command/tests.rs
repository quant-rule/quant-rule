//! Parser and command-surface tests for the surviving shell.

use super::fetch::{parse_fetch_arguments, write_fetch_cap_warning};
use super::parse::parse;
use super::parse_argv;
use super::types::{Command, HelpTopic, ParseError};
use crate::date::DateRange;
use crate::pipeline::WindowMode;
use crate::request::{FetchArgs, ModelArgs};
use crate::source::Symbol;
use quantrule_data::Timeframe;
use std::path::PathBuf;

fn argv(tokens: &[&str]) -> Vec<String> {
    tokens.iter().map(|token| (*token).to_owned()).collect()
}

fn fetch_args(csv: Option<PathBuf>) -> FetchArgs {
    FetchArgs {
        symbol: Symbol::resolve("gold").expect("gold is known"),
        range: DateRange::parse("2025-01-01", "2025-12-31").expect("the dates are valid"),
        timeframe: Timeframe::M1,
        csv,
    }
}

#[test]
fn blank_lines_are_ignored() {
    assert_eq!(parse(""), Ok(None));
    assert_eq!(parse("   \t "), Ok(None));
}

#[test]
fn parses_the_data_commands() {
    assert_eq!(
        parse("candles data/example.csv"),
        Ok(Some(Command::Candles(PathBuf::from("data/example.csv"))))
    );
    assert_eq!(
        parse("candles-tolerant data/example.csv"),
        Ok(Some(Command::CandlesTolerant(PathBuf::from(
            "data/example.csv"
        ))))
    );
    assert_eq!(
        parse("timeframes data/example.csv"),
        Ok(Some(Command::Timeframes(PathBuf::from("data/example.csv"))))
    );
}

#[test]
fn a_data_path_may_contain_spaces() {
    assert_eq!(
        parse("candles data/my file.csv"),
        Ok(Some(Command::Candles(PathBuf::from("data/my file.csv"))))
    );
}

#[test]
fn parses_events_with_a_window() {
    assert_eq!(
        parse("events data/example.csv --window latest"),
        Ok(Some(Command::Events(ModelArgs::new(
            PathBuf::from("data/example.csv"),
            WindowMode::Latest
        ))))
    );
    assert!(parse("events data/example.csv --window full --window latest").is_err());
    assert!(parse("events data/example.csv --window").is_err());
    assert!(parse("events data/example.csv --window hourly").is_err());
}

#[test]
fn parses_report_model() {
    let parsed = parse("report model --config config/examples/btcusd-2024/run.json");
    let Ok(Some(Command::Report(args))) = parsed else {
        panic!("the report command must parse");
    };
    assert_eq!(
        args.config,
        PathBuf::from("config/examples/btcusd-2024/run.json")
    );
    assert!(args.spec.is_none());
}

#[test]
fn rejects_other_report_engines() {
    assert!(parse("report strategy --config run.json").is_err());
    assert!(parse("report model").is_err());
}

#[test]
fn parses_run_check() {
    assert!(matches!(
        parse("run check --config run.json"),
        Ok(Some(Command::RunCheck(_)))
    ));
    assert!(parse("run --config run.json").is_err());
}

#[test]
fn parses_fetch_and_rejects_removed_options() {
    let parsed =
        parse_fetch_arguments(&argv(&["gold", "2025-01-01", "2025-12-31"])).expect("fetch");
    assert_eq!(parsed.timeframe, Timeframe::M1);
    assert!(parsed.csv.is_none());

    let parsed = parse_fetch_arguments(&argv(&[
        "gold",
        "2025-01-01",
        "2025-12-31",
        "--timeframe",
        "1h",
        "--csv",
        "data/gold.csv",
    ]))
    .expect("fetch with options");
    assert_eq!(parsed.timeframe, Timeframe::H1);
    assert_eq!(parsed.csv, Some(PathBuf::from("data/gold.csv")));

    assert!(
        parse_fetch_arguments(&argv(&["gold", "2025-01-01", "2025-12-31", "--png", "x"])).is_err()
    );
    assert!(
        parse_fetch_arguments(&argv(&[
            "gold",
            "2025-01-01",
            "2025-12-31",
            "--candles",
            "10"
        ]))
        .is_err()
    );
}

#[test]
fn parses_help_topics() {
    assert_eq!(parse("help"), Ok(Some(Command::Help(HelpTopic::Short))));
    assert_eq!(parse("help all"), Ok(Some(Command::Help(HelpTopic::All))));
    assert_eq!(
        parse("help glossary"),
        Ok(Some(Command::Help(HelpTopic::Glossary)))
    );
    assert_eq!(
        parse("help workflows"),
        Ok(Some(Command::Help(HelpTopic::Workflows)))
    );
    assert_eq!(
        parse("help report model"),
        Ok(Some(Command::Help(HelpTopic::Command(
            "report model".to_owned()
        ))))
    );
    assert!(parse("help market-state").is_err());
}

#[test]
fn rejects_deleted_commands() {
    for line in [
        "market-state data/example.csv",
        "market-dashboard data/example.csv",
        "market-chart data/example.csv",
        "market-report data/example.csv",
        "axis data/example.csv direction",
        "legs data/example.csv",
        "model-series data/example.csv",
        "strategy-run data/example.csv --strategy s.json",
        "store list",
        "db bars",
        "research export data/example.csv",
        "lab list",
    ] {
        assert!(
            matches!(parse(line), Err(ParseError::Unknown(_))),
            "{line} must be rejected"
        );
    }
}

#[test]
fn interactive_and_argv_paths_agree() {
    let cases = [
        "candles data/example.csv",
        "events data/example.csv --window latest",
        "report model --config run.json --out target/report",
        "run check --config run.json",
        "fetch gold 2025-01-01 2025-12-31 --timeframe 1h",
        "help all",
        "quit",
    ];
    for line in cases {
        let interactive = parse(line).expect("the interactive line parses");
        let tokens: Vec<String> = line.split_whitespace().map(str::to_owned).collect();
        let non_interactive = parse_argv(&tokens).expect("the argv line parses");
        assert_eq!(interactive, non_interactive, "{line}");
    }
}

#[test]
fn a_quoted_fetch_option_is_one_token() {
    let parsed = parse("fetch gold 2025-01-01 2025-12-31 --csv 'data/my gold.csv'")
        .expect("the quoted path parses");
    let Some(Command::Fetch(args)) = parsed else {
        panic!("the fetch command must parse");
    };
    assert_eq!(args.csv, Some(PathBuf::from("data/my gold.csv")));
}

#[test]
fn the_fetch_cap_warning_is_emitted_above_the_batch_cap() {
    let mut warnings = Vec::new();
    write_fetch_cap_warning(quantrule_data::MAX_SERIES + 1, &mut warnings)
        .expect("the warning writes");
    let text = String::from_utf8(warnings).expect("the warning is UTF-8");
    assert!(text.contains("exceeds"));
    assert!(text.contains(&quantrule_data::MAX_SERIES.to_string()));
}

#[test]
fn the_fetch_args_helper_matches_the_parser() {
    let expected = fetch_args(Some(PathBuf::from("data/gold.csv")));
    let parsed = parse_fetch_arguments(&argv(&[
        "gold",
        "2025-01-01",
        "2025-12-31",
        "--csv",
        "data/gold.csv",
    ]))
    .expect("fetch with a csv target");
    assert_eq!(parsed, expected);
}
