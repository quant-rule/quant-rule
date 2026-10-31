use super::chart::parse_market_chart_arguments;
use super::fetch::{parse_fetch_arguments, write_fetch_cap_warning};
use super::parse::parse;
use super::types::{
    Axis, Command, DashboardArgs, FetchArgs, HelpTopic, LabDiagnoseArgs, LabListArgs, LabOperator,
    LabRole, LabSummaryArgs, MarketChartArgs, ModelArgs, ModelSeriesArgs, ParseError,
    ResearchExportArgs, StoreIngestArgs, StoreRemoveArgs, WindowMode,
};
use std::path::PathBuf;

use crate::source::{DateRange, Symbol};
use quantrule_data::Timeframe;

fn fetch_args(csv: Option<PathBuf>, png: Option<PathBuf>, candles: usize) -> FetchArgs {
    fetch_args_with_timeframe(Timeframe::M1, csv, png, candles)
}

fn fetch_args_with_timeframe(
    timeframe: Timeframe,
    csv: Option<PathBuf>,
    png: Option<PathBuf>,
    candles: usize,
) -> FetchArgs {
    FetchArgs {
        symbol: Symbol::resolve("gold").expect("gold is known"),
        range: DateRange::parse("2025-01-01", "2025-12-31").expect("the dates are valid"),
        timeframe,
        csv,
        png,
        candles,
    }
}

fn market_chart_args(timeframe: Timeframe, candles: usize) -> MarketChartArgs {
    MarketChartArgs {
        path: PathBuf::from("data/m1.csv"),
        png: PathBuf::from("data/market.png"),
        timeframe,
        candles,
        window: WindowMode::Full,
    }
}

#[test]
fn blank_lines_are_ignored() {
    assert_eq!(parse(""), Ok(None));
    assert_eq!(parse("   \t "), Ok(None));
}

#[test]
fn parses_candles_with_a_path() {
    assert_eq!(
        parse("candles data/example.csv"),
        Ok(Some(Command::Candles(PathBuf::from("data/example.csv"))))
    );
}

#[test]
fn parses_tolerant_candles_with_a_path() {
    assert_eq!(
        parse("candles-tolerant data/example.csv"),
        Ok(Some(Command::CandlesTolerant(PathBuf::from(
            "data/example.csv"
        ))))
    );
}

#[test]
fn preserves_spaces_in_a_candles_path() {
    assert_eq!(
        parse("candles my file.csv"),
        Ok(Some(Command::Candles(PathBuf::from("my file.csv"))))
    );
}

#[test]
fn preserves_spaces_in_a_tolerant_candles_path() {
    assert_eq!(
        parse("candles-tolerant my file.csv"),
        Ok(Some(Command::CandlesTolerant(PathBuf::from("my file.csv"))))
    );
}

#[test]
fn parses_timeframes_with_a_path() {
    assert_eq!(
        parse("timeframes data/example.csv"),
        Ok(Some(Command::Timeframes(PathBuf::from("data/example.csv"))))
    );
}

#[test]
fn preserves_spaces_in_a_timeframes_path() {
    assert_eq!(
        parse("timeframes my file.csv"),
        Ok(Some(Command::Timeframes(PathBuf::from("my file.csv"))))
    );
}

#[test]
fn rejects_candles_without_a_path() {
    assert_eq!(parse("candles"), Err(ParseError::MissingPath("candles")));
    assert_eq!(parse("candles   "), Err(ParseError::MissingPath("candles")));
}

#[test]
fn rejects_tolerant_candles_without_a_path() {
    assert_eq!(
        parse("candles-tolerant"),
        Err(ParseError::MissingPath("candles-tolerant"))
    );
    assert_eq!(
        parse("candles-tolerant   "),
        Err(ParseError::MissingPath("candles-tolerant"))
    );
}

#[test]
fn rejects_timeframes_without_a_path() {
    assert_eq!(
        parse("timeframes"),
        Err(ParseError::MissingPath("timeframes"))
    );
    assert_eq!(
        parse("timeframes   "),
        Err(ParseError::MissingPath("timeframes"))
    );
}

#[test]
fn parses_help_and_quit() {
    assert_eq!(parse("help"), Ok(Some(Command::Help(HelpTopic::Short))));
    assert_eq!(parse("quit"), Ok(Some(Command::Quit)));
}

#[test]
fn parses_help_topics() {
    assert_eq!(parse("help all"), Ok(Some(Command::Help(HelpTopic::All))));
    assert_eq!(
        parse("help glossary"),
        Ok(Some(Command::Help(HelpTopic::Glossary)))
    );
    assert_eq!(
        parse("help tutorial"),
        Ok(Some(Command::Help(HelpTopic::Tutorial)))
    );
    assert_eq!(
        parse("help roles"),
        Ok(Some(Command::Help(HelpTopic::Roles)))
    );
    assert_eq!(
        parse("help candles"),
        Ok(Some(Command::Help(HelpTopic::Command(String::from(
            "candles"
        )))))
    );
    assert_eq!(
        parse("help store ingest"),
        Ok(Some(Command::Help(HelpTopic::Command(String::from(
            "store ingest"
        )))))
    );
    assert_eq!(
        parse("help research export"),
        Ok(Some(Command::Help(HelpTopic::Command(String::from(
            "research export"
        )))))
    );
    assert_eq!(
        parse("help lab summary"),
        Ok(Some(Command::Help(HelpTopic::Command(String::from(
            "lab summary"
        )))))
    );
    assert_eq!(
        parse("help \"store ingest\""),
        Ok(Some(Command::Help(HelpTopic::Command(String::from(
            "store ingest"
        )))))
    );
}

#[test]
fn rejects_invalid_help_topics() {
    assert_eq!(
        parse("help now"),
        Err(ParseError::UnknownHelpTopic(String::from("now")))
    );
    assert_eq!(
        parse("help all extra"),
        Err(ParseError::Usage(String::from(
            "usage: help [<command>|all|glossary|tutorial|roles]"
        )))
    );
    assert_eq!(
        parse("help a b c"),
        Err(ParseError::Usage(String::from(
            "usage: help [<command>|all|glossary|tutorial|roles]"
        )))
    );
    assert_eq!(
        parse("quit now"),
        Err(ParseError::UnexpectedArguments("quit"))
    );
}

#[test]
fn parses_the_market_state_commands() {
    assert_eq!(
        parse("market-state data/example.csv"),
        Ok(Some(Command::MarketState(ModelArgs::new(
            PathBuf::from("data/example.csv"),
            WindowMode::Full,
        ))))
    );
    assert_eq!(
        parse("events data/example.csv --window latest"),
        Ok(Some(Command::Events(ModelArgs::new(
            PathBuf::from("data/example.csv"),
            WindowMode::Latest,
        ))))
    );
    assert_eq!(
        parse("legs data/example.csv --window full"),
        Ok(Some(Command::Legs(ModelArgs::new(
            PathBuf::from("data/example.csv"),
            WindowMode::Full,
        ))))
    );
}

#[test]
fn parses_the_market_dashboard_with_window_and_json() {
    assert_eq!(
        parse("market-dashboard data/example.csv --json --window latest"),
        Ok(Some(Command::MarketDashboard(DashboardArgs {
            path: PathBuf::from("data/example.csv"),
            window: WindowMode::Latest,
            json: true,
        })))
    );
    assert_eq!(
        parse("market-dashboard \"my data.csv\""),
        Ok(Some(Command::MarketDashboard(DashboardArgs {
            path: PathBuf::from("my data.csv"),
            window: WindowMode::Full,
            json: false,
        })))
    );
    assert_eq!(
        parse("market-dashboard"),
        Err(ParseError::MissingPath("market-dashboard"))
    );
}

#[test]
fn parses_an_axis_with_a_path_and_a_name() {
    assert_eq!(
        parse("axis data/example.csv direction"),
        Ok(Some(Command::Axis {
            args: ModelArgs::new(PathBuf::from("data/example.csv"), WindowMode::Full),
            axis: Axis::Direction,
        }))
    );
    assert_eq!(
        parse("axis my data.csv progression --window latest"),
        Ok(Some(Command::Axis {
            args: ModelArgs::new(PathBuf::from("my data.csv"), WindowMode::Latest),
            axis: Axis::Progression,
        }))
    );
}

#[test]
fn parses_a_model_series_with_a_tail_and_timeframe() {
    assert_eq!(
        parse("model-series data/example.csv --timeframe 1m --bars 3 --window latest"),
        Ok(Some(Command::ModelSeries(ModelSeriesArgs {
            path: PathBuf::from("data/example.csv"),
            window: WindowMode::Latest,
            timeframe: Some(quantrule_data::Timeframe::M1),
            bars: 3,
        })))
    );
}

#[test]
fn rejects_an_invalid_model_window() {
    assert!(matches!(
        parse("market-state data/example.csv --window 300"),
        Err(ParseError::Window(_))
    ));
    assert!(matches!(
        parse("axis data/example.csv direction --window"),
        Err(ParseError::Window(_))
    ));
}

#[test]
fn the_fetch_cap_warning_is_only_written_over_the_cap() {
    let mut under = Vec::new();
    write_fetch_cap_warning(100_000, &mut under).expect("the warning writes");
    assert!(under.is_empty());

    let mut over = Vec::new();
    write_fetch_cap_warning(100_001, &mut over).expect("the warning writes");
    let text = String::from_utf8(over).expect("the warning is UTF-8");
    assert!(text.contains("accepted=100001"));
    assert!(text.contains("100000-row batch cap"));
    assert!(text.contains("--window latest"));
    assert!(text.contains("market-report accepts up to 600000 rows"));

    let mut real_world = Vec::new();
    write_fetch_cap_warning(354_442, &mut real_world).expect("the warning writes");
    let text = String::from_utf8(real_world).expect("the warning is UTF-8");
    assert!(text.contains("accepted=354442"));
    assert!(text.contains("100000-row batch cap"));
}

#[test]
fn rejects_a_bad_axis_command() {
    assert_eq!(parse("axis"), Err(ParseError::AxisUsage));
    assert_eq!(parse("axis data/example.csv"), Err(ParseError::AxisUsage));
    assert_eq!(
        parse("axis data/example.csv bogus"),
        Err(ParseError::UnknownAxis(String::from("bogus")))
    );
}

#[test]
fn rejects_an_unknown_command() {
    assert_eq!(
        parse("bogus"),
        Err(ParseError::Unknown(String::from("bogus")))
    );
}

#[test]
fn parses_fetch_with_defaults() {
    assert_eq!(
        parse("fetch gold 2025-01-01 2025-12-31"),
        Ok(Some(Command::Fetch(fetch_args(None, None, 1_000))))
    );
}

#[test]
fn parses_each_supported_timeframe() {
    for (code, timeframe) in [
        ("1m", Timeframe::M1),
        ("5m", Timeframe::M5),
        ("15m", Timeframe::M15),
        ("1h", Timeframe::H1),
    ] {
        let line = format!("fetch gold 2025-01-01 2025-12-31 --timeframe {code}");
        assert_eq!(
            parse(&line),
            Ok(Some(Command::Fetch(fetch_args_with_timeframe(
                timeframe, None, None, 1_000
            )))),
            "timeframe {code} is parsed"
        );
    }
}

#[test]
fn parses_fetch_options_in_any_order() {
    assert_eq!(
        parse("fetch gold 2025-01-01 2025-12-31 --candles 250 --png out.png --csv out.csv"),
        Ok(Some(Command::Fetch(fetch_args(
            Some(PathBuf::from("out.csv")),
            Some(PathBuf::from("out.png")),
            250,
        ))))
    );
    assert_eq!(
        parse("fetch gold 2025-01-01 2025-12-31 --png out.png --timeframe 5m --csv out.csv"),
        Ok(Some(Command::Fetch(fetch_args_with_timeframe(
            Timeframe::M5,
            Some(PathBuf::from("out.csv")),
            Some(PathBuf::from("out.png")),
            1_000,
        ))))
    );
}

#[test]
fn parses_a_quoted_path_with_spaces() {
    assert_eq!(
        parse("fetch gold 2025-01-01 2025-12-31 --csv \"my data/out.csv\""),
        Ok(Some(Command::Fetch(fetch_args(
            Some(PathBuf::from("my data/out.csv")),
            None,
            1_000,
        ))))
    );
}

#[test]
fn parses_fetch_arguments_from_an_already_split_argv() {
    let arguments = [
        String::from("gold"),
        String::from("2025-01-01"),
        String::from("2025-12-31"),
        String::from("--csv"),
        String::from("my data/out.csv"),
    ];

    assert_eq!(
        parse_fetch_arguments(&arguments),
        Ok(fetch_args(
            Some(PathBuf::from("my data/out.csv")),
            None,
            1_000
        ))
    );
}

#[test]
fn rejects_fetch_without_positionals() {
    assert!(matches!(parse("fetch"), Err(ParseError::Fetch(_))));
    assert!(matches!(
        parse("fetch gold 2025-01-01"),
        Err(ParseError::Fetch(_))
    ));
}

#[test]
fn rejects_an_invalid_fetch_symbol_or_date() {
    assert!(matches!(
        parse("fetch notreal 2025-01-01 2025-12-31"),
        Err(ParseError::Fetch(_))
    ));
    assert!(matches!(
        parse("fetch gold 2025-02-30 2025-12-31"),
        Err(ParseError::Fetch(_))
    ));
    assert!(matches!(
        parse("fetch gold 2025-12-31 2025-01-01"),
        Err(ParseError::Fetch(_))
    ));
}

#[test]
fn rejects_bad_fetch_options() {
    assert!(matches!(
        parse("fetch gold 2025-01-01 2025-12-31 --csv"),
        Err(ParseError::Fetch(_))
    ));
    assert!(matches!(
        parse("fetch gold 2025-01-01 2025-12-31 --csv a.csv --csv b.csv"),
        Err(ParseError::Fetch(_))
    ));
    assert!(matches!(
        parse("fetch gold 2025-01-01 2025-12-31 --bogus x"),
        Err(ParseError::Fetch(_))
    ));
    assert!(matches!(
        parse("fetch gold 2025-01-01 2025-12-31 extra"),
        Err(ParseError::Fetch(_))
    ));
    assert!(matches!(
        parse("fetch gold 2025-01-01 2025-12-31 --csv same.csv --png same.csv"),
        Err(ParseError::Fetch(_))
    ));
    assert!(matches!(
        parse("fetch gold 2025-01-01 2025-12-31 --timeframe 30m"),
        Err(ParseError::Fetch(_))
    ));
    assert!(matches!(
        parse("fetch gold 2025-01-01 2025-12-31 --timeframe 5m --timeframe 15m"),
        Err(ParseError::Fetch(_))
    ));
    assert!(matches!(
        parse("fetch gold 2025-01-01 2025-12-31 --timeframe"),
        Err(ParseError::Fetch(_))
    ));
}

#[test]
fn rejects_an_out_of_range_chart_count() {
    for count in ["0", "1001", "many", "-1"] {
        let line = format!("fetch gold 2025-01-01 2025-12-31 --candles {count}");
        assert!(
            matches!(parse(&line), Err(ParseError::Fetch(_))),
            "count {count} should be rejected"
        );
    }
}

#[test]
fn rejects_the_same_csv_and_png_path() {
    assert!(matches!(
        parse("fetch gold 2025-01-01 2025-12-31 --csv out --png out"),
        Err(ParseError::Fetch(_))
    ));
}

#[test]
fn rejects_an_unterminated_quote() {
    assert!(matches!(
        parse("fetch gold 2025-01-01 2025-12-31 --csv \"out.csv"),
        Err(ParseError::Fetch(_))
    ));
}

#[test]
fn parses_market_chart_with_defaults() {
    assert_eq!(
        parse("market-chart data/m1.csv --png data/market.png"),
        Ok(Some(Command::MarketChart(market_chart_args(
            Timeframe::M1,
            1_000
        ))))
    );
}

#[test]
fn parses_market_chart_options_in_any_order() {
    assert_eq!(
        parse("market-chart --candles 250 --png data/market.png data/m1.csv --timeframe 5m"),
        Ok(Some(Command::MarketChart(market_chart_args(
            Timeframe::M5,
            250
        ))))
    );
}

#[test]
fn parses_market_chart_arguments_from_an_already_split_argv() {
    let arguments = [
        String::from("data/m1.csv"),
        String::from("--png"),
        String::from("data/market.png"),
        String::from("--timeframe"),
        String::from("15m"),
        String::from("--candles"),
        String::from("100"),
    ];

    assert_eq!(
        parse_market_chart_arguments(&arguments),
        Ok(market_chart_args(Timeframe::M15, 100))
    );
}

#[test]
fn rejects_bad_market_chart_arguments() {
    for line in [
        "market-chart",
        "market-chart data/m1.csv",
        "market-chart data/m1.csv --png",
        "market-chart data/m1.csv --png data/market.png --png other.png",
        "market-chart data/m1.csv --png data/market.png --timeframe 30m",
        "market-chart data/m1.csv --png data/market.png --timeframe 5m --timeframe 15m",
        "market-chart data/m1.csv --png data/market.png --bogus x",
        "market-chart data/m1.csv data/other.csv --png data/market.png",
        "market-chart data/m1.csv --png data/m1.csv",
    ] {
        assert!(
            matches!(parse(line), Err(ParseError::MarketChart(_))),
            "line should be rejected: {line}"
        );
    }
}

#[test]
fn rejects_an_out_of_range_market_chart_count() {
    for count in ["0", "1001", "many", "-1"] {
        let line = format!("market-chart data/m1.csv --png data/market.png --candles {count}");
        assert!(
            matches!(parse(&line), Err(ParseError::MarketChart(_))),
            "count {count} should be rejected"
        );
    }
}

#[test]
fn parses_a_research_export() {
    assert_eq!(
        parse("research export data/m1.csv demo --data-version source:1 --root out"),
        Ok(Some(Command::ResearchExport(ResearchExportArgs {
            path: PathBuf::from("data/m1.csv"),
            dataset: "demo".to_string(),
            data_version: "source:1".to_string(),
            root: PathBuf::from("out"),
        })))
    );
}

#[test]
fn a_research_export_requires_a_data_version() {
    assert!(matches!(
        parse("research export data/m1.csv demo"),
        Err(ParseError::Research(_))
    ));
}

#[test]
fn a_research_export_rejects_an_unknown_option() {
    assert!(matches!(
        parse("research export data/m1.csv demo --data-version x --nope y"),
        Err(ParseError::Research(_))
    ));
}

#[test]
fn parses_a_store_ingest_with_defaults() {
    assert_eq!(
        parse("store ingest data/m1.csv --dataset demo --data-version source:1"),
        Ok(Some(Command::StoreIngest(StoreIngestArgs {
            path: PathBuf::from("data/m1.csv"),
            dataset: "demo".to_string(),
            data_version: "source:1".to_string(),
            database: PathBuf::from("data/db/quantrule.db"),
        })))
    );
}

#[test]
fn parses_a_store_ingest_in_any_option_order() {
    assert_eq!(
        parse("store ingest --db other.db --data-version source:1 data/m1.csv --dataset demo"),
        Ok(Some(Command::StoreIngest(StoreIngestArgs {
            path: PathBuf::from("data/m1.csv"),
            dataset: "demo".to_string(),
            data_version: "source:1".to_string(),
            database: PathBuf::from("other.db"),
        })))
    );
}

#[test]
fn a_store_ingest_requires_a_dataset_and_a_data_version() {
    assert!(matches!(
        parse("store ingest data/m1.csv --data-version source:1"),
        Err(ParseError::Store(_))
    ));
    assert!(matches!(
        parse("store ingest data/m1.csv --dataset demo"),
        Err(ParseError::Store(_))
    ));
    assert!(matches!(
        parse("store ingest --dataset demo --data-version source:1"),
        Err(ParseError::Store(_))
    ));
}

#[test]
fn a_store_ingest_rejects_repeated_options() {
    for line in [
        "store ingest data/m1.csv --dataset a --dataset b --data-version v",
        "store ingest data/m1.csv --dataset a --data-version v --data-version w",
    ] {
        assert!(
            matches!(parse(line), Err(ParseError::Store(_))),
            "{line} should be rejected"
        );
    }
}

#[test]
fn a_store_ingest_rejects_an_unknown_option() {
    assert!(matches!(
        parse("store ingest data/m1.csv --dataset demo --data-version v --nope x"),
        Err(ParseError::Store(_))
    ));
}

#[test]
fn parses_a_store_init_with_defaults() {
    assert_eq!(
        parse("store init"),
        Ok(Some(Command::StoreInit(PathBuf::from(
            "data/db/quantrule.db"
        ))))
    );
}

#[test]
fn parses_store_removal() {
    assert_eq!(
        parse("store remove demo --db other.db"),
        Ok(Some(Command::StoreRemove(StoreRemoveArgs {
            dataset: "demo".to_string(),
            database: PathBuf::from("other.db"),
        })))
    );
}

fn lab_summary_args() -> LabSummaryArgs {
    LabSummaryArgs {
        lab_dir: PathBuf::from("data/labs/lab-result-2026-10-29T12-00-00Z"),
        role: LabRole::Researcher,
        data_version: String::from("test:v1"),
        config: PathBuf::from("config/examples/solusd-2025/run.json"),
        created_utc: String::from("2026-10-29T12:00:00Z"),
        operator: LabOperator::Human,
        notes: None,
        force: false,
    }
}

#[test]
fn parses_lab_summary_with_defaults() {
    assert_eq!(
        parse(
            "lab summary data/labs/lab-result-2026-10-29T12-00-00Z \
--role researcher --data-version test:v1 \
--config config/examples/solusd-2025/run.json --created-utc 2026-10-29T12:00:00Z"
        ),
        Ok(Some(Command::LabSummary(lab_summary_args())))
    );
}

#[test]
fn parses_lab_summary_with_every_option() {
    let mut expected = lab_summary_args();
    expected.role = LabRole::Validator;
    expected.operator = LabOperator::Agent;
    expected.notes = Some(PathBuf::from("data/notes.md"));
    expected.force = true;

    assert_eq!(
        parse(
            "lab summary --force data/labs/lab-result-2026-10-29T12-00-00Z \
--role validator --data-version test:v1 \
--config config/examples/solusd-2025/run.json --created-utc 2026-10-29T12:00:00Z \
--operator agent --notes data/notes.md"
        ),
        Ok(Some(Command::LabSummary(expected)))
    );
}

#[test]
fn lab_requires_a_summary_subcommand() {
    assert!(matches!(parse("lab"), Err(ParseError::Lab(_))));
    assert!(matches!(parse("lab report"), Err(ParseError::Lab(_))));
}

#[test]
fn lab_summary_requires_the_lab_directory_and_every_required_option() {
    assert!(matches!(parse("lab summary"), Err(ParseError::Lab(_))));
    assert!(matches!(
        parse(
            "lab summary data/lab --data-version v --config c --created-utc 2026-10-29T12:00:00Z"
        ),
        Err(ParseError::Lab(_))
    ));
    assert!(matches!(
        parse(
            "lab summary data/lab --role researcher --config c --created-utc 2026-10-29T12:00:00Z"
        ),
        Err(ParseError::Lab(_))
    ));
    assert!(matches!(
        parse(
            "lab summary data/lab --role researcher --data-version v --created-utc 2026-10-29T12:00:00Z"
        ),
        Err(ParseError::Lab(_))
    ));
    assert!(matches!(
        parse("lab summary data/lab --role researcher --data-version v --config c"),
        Err(ParseError::Lab(_))
    ));
}

#[test]
fn lab_summary_rejects_an_unknown_role() {
    let error = parse(
        "lab summary data/lab --role wizard --data-version v --config c \
--created-utc 2026-10-29T12:00:00Z",
    );
    assert!(matches!(error, Err(ParseError::Lab(_))));
}

#[test]
fn lab_summary_rejects_an_unknown_operator() {
    let error = parse(
        "lab summary data/lab --role researcher --data-version v --config c \
--created-utc 2026-10-29T12:00:00Z --operator robot",
    );
    assert!(matches!(error, Err(ParseError::Lab(_))));
}

#[test]
fn lab_summary_rejects_a_non_canonical_created_utc() {
    for value in [
        "2026-10-29T12:00:00+00:00",
        "2026-10-29 12:00:00Z",
        "2026-10-29T12:00Z",
        "2026-10-29T12:00:00",
    ] {
        let line = format!(
            "lab summary data/lab --role researcher --data-version v --config c --created-utc {value}"
        );
        assert!(matches!(parse(&line), Err(ParseError::Lab(_))), "{value}");
    }
}

#[test]
fn lab_summary_rejects_an_impossible_created_utc() {
    let error = parse(
        "lab summary data/lab --role researcher --data-version v --config c \
--created-utc 2025-02-30T00:00:00Z",
    );
    assert!(matches!(error, Err(ParseError::Lab(_))));
}

#[test]
fn lab_summary_rejects_repeated_options_and_extra_arguments() {
    let base = "lab summary data/lab --role researcher --data-version v --config c \
--created-utc 2026-10-29T12:00:00Z";
    assert!(matches!(
        parse(&format!("{base} --role developer")),
        Err(ParseError::Lab(_))
    ));
    assert!(matches!(
        parse(&format!("{base} --force --force")),
        Err(ParseError::Lab(_))
    ));
    assert!(matches!(
        parse(&format!("{base} other")),
        Err(ParseError::Lab(_))
    ));
    assert!(matches!(
        parse(&format!("{base} --bogus x")),
        Err(ParseError::Lab(_))
    ));
}

#[test]
fn parses_lab_diagnose_with_every_option() {
    assert_eq!(
        parse(
            "lab diagnose data/lab --config config/run.json --out data/diagnosis --json --strict"
        ),
        Ok(Some(Command::LabDiagnose(LabDiagnoseArgs {
            lab_dir: PathBuf::from("data/lab"),
            config: PathBuf::from("config/run.json"),
            out: Some(PathBuf::from("data/diagnosis")),
            json: true,
            strict: true,
        })))
    );
}

#[test]
fn lab_diagnose_requires_the_lab_and_config() {
    assert!(matches!(parse("lab diagnose"), Err(ParseError::Lab(_))));
    assert!(matches!(
        parse("lab diagnose data/lab"),
        Err(ParseError::Lab(_))
    ));
    assert!(matches!(
        parse("lab diagnose data/lab --config a --config b"),
        Err(ParseError::Lab(_))
    ));
    assert!(matches!(
        parse("lab diagnose data/lab --config a extra"),
        Err(ParseError::Lab(_))
    ));
}

#[test]
fn parses_lab_list() {
    assert_eq!(
        parse("lab list --root data/labs"),
        Ok(Some(Command::LabList(LabListArgs {
            root: PathBuf::from("data/labs"),
            json: false,
        })))
    );
    assert_eq!(
        parse("lab list --root data/labs --json"),
        Ok(Some(Command::LabList(LabListArgs {
            root: PathBuf::from("data/labs"),
            json: true,
        })))
    );
}

#[test]
fn lab_list_requires_a_root() {
    assert!(matches!(parse("lab list"), Err(ParseError::Lab(_))));
    assert!(matches!(
        parse("lab list --root a --root b"),
        Err(ParseError::Lab(_))
    ));
    assert!(matches!(
        parse("lab list --root a extra"),
        Err(ParseError::Lab(_))
    ));
}
