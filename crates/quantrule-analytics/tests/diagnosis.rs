//! Integration tests for the cross-engine diagnosis.

use quantrule_analytics::diagnosis::diagnose;
use quantrule_analytics::diagnosis::text;
use quantrule_analytics::diagnosis::types::{
    CheckStatus, DiagnosisIdentity, DiagnosisInput, EquityFacts, FillFacts, MetricsFacts,
    OutOfSampleStatus, ReportVersion, StrategyFacts, TradeFacts,
};
use quantrule_data::Candle;
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::run::ModelRun;

struct Fixture {
    run: ModelRun,
}

impl Fixture {
    fn new(count: usize) -> Self {
        let candles = candles(count, 300);
        let run = ModelRun::compute(
            &[TimeframeInput {
                timeframe_secs: 300,
                candles: &candles,
            }],
            &MarketStateConfig::default(),
        );
        Self { run }
    }

    fn with_one_minute(count: usize) -> Self {
        let minute = candles(count, 60);
        let five_minute = candles(count, 300);
        let run = ModelRun::compute(
            &[
                TimeframeInput {
                    timeframe_secs: 60,
                    candles: &minute,
                },
                TimeframeInput {
                    timeframe_secs: 300,
                    candles: &five_minute,
                },
            ],
            &MarketStateConfig::default(),
        );
        Self { run }
    }

    fn input(&self) -> DiagnosisInput<'_> {
        DiagnosisInput {
            identity: DiagnosisIdentity {
                lab: String::from("test-lab"),
                data_version: String::from("test:v1"),
                config_hash: String::from("0123456789abcdef"),
                model_version: String::from(env!("CARGO_PKG_VERSION")),
                requested_period: Some(String::from("2025-01-01..2025-01-02")),
                actual_period: Some(String::from("2025-01-01T00:00:00Z..2025-01-01T00:20:00Z")),
                report_versions: Vec::new(),
                created_utc: None,
                role: None,
                operator: None,
            },
            model_run: &self.run,
            execution_timeframe_secs: 300,
            trades: Vec::new(),
            orders: Vec::new(),
            fills: Vec::new(),
            rejections: Vec::new(),
            metrics: MetricsFacts {
                initial_cash: 10_000.0,
                final_equity: 10_000.0,
                total_return: 0.0,
                gross_pnl: 0.0,
                net_pnl: 0.0,
                commission: 0.0,
                execution_fees: 0.0,
                funding_paid: 0.0,
                funding_received: 0.0,
                max_drawdown: 0.0,
                max_drawdown_duration_bars: 0,
                trades: 0,
                wins: 0,
                losses: 0,
                win_rate: None,
                profit_factor: None,
                average_trade: None,
                average_hold_bars: None,
                exposure: None,
                halted: false,
                max_drawdown_limit: Some(0.2),
                leverage: 1.0,
                commission_rate: 0.001,
                configured_slippage_bps: 2.0,
            },
            equity: vec![EquityFacts {
                date: 1_735_689_600,
                equity: 10_000.0,
                drawdown: 0.0,
            }],
            benchmark: vec![(1_735_689_600, 10_000.0)],
            risk_events: Vec::new(),
            monthly_returns: Vec::new(),
            strategy: StrategyFacts {
                bars: self
                    .run
                    .timeframe(300)
                    .map_or(0, |timeframe| timeframe.len()),
                decisions: 0,
                entry_attempts: 0,
                exit_attempts: 0,
                executed_entries: 0,
                executed_exits: 0,
                long_entries: 0,
                short_entries: 0,
                context_complete_bars: 0,
                context_missing_bars: 0,
                context_conflict_bars: 0,
                context_late_or_terminal_bars: 0,
            },
            entry_contexts: Vec::new(),
            out_of_sample: OutOfSampleStatus::Absent,
            rerun_verified: false,
            instrument_count: 1,
        }
    }
}

fn candles(count: usize, step: i64) -> Vec<Candle> {
    (0..count)
        .map(|index| {
            let timestamp = 1_735_689_600 + index as i64 * step;
            let open = 100.0 + index as f64;
            Candle::new(
                index as u64,
                timestamp,
                open,
                open + 1.0,
                open - 1.0,
                open + 0.5,
                Some(10.0 + index as f64),
            )
            .expect("the test candle is valid")
        })
        .collect()
}

fn report_versions(bundles: &[&str]) -> Vec<ReportVersion> {
    bundles
        .iter()
        .map(|bundle| ReportVersion {
            bundle: (*bundle).to_owned(),
            version: String::from("2026-10-31"),
        })
        .collect()
}

fn report_version_status(fixture: &Fixture, versions: Vec<ReportVersion>) -> CheckStatus {
    let mut input = fixture.input();
    input.identity.report_versions = versions;
    let diagnosis = diagnose(&input);
    diagnosis
        .checks
        .iter()
        .find(|check| check.id == "identity.report_versions")
        .expect("the report-version check exists")
        .status
}

fn trade(exit_reason: &str, hold_bars: usize, entry_price: f64) -> TradeFacts {
    TradeFacts {
        trade_id: 1,
        side: 1,
        quantity: 1.0,
        entry_bar: 10,
        entry_time: 1_735_689_600 + 10 * 300,
        entry_price,
        exit_bar: 10 + hold_bars,
        exit_time: 1_735_689_600 + (10 + hold_bars as i64) * 300,
        exit_price: 101.0,
        gross_pnl: 1.0,
        commission: 0.0,
        execution_fees: 0.0,
        funding: 0.0,
        net_pnl: 1.0,
        return_fraction: None,
        hold_bars,
        exit_reason: exit_reason.to_owned(),
    }
}

#[test]
fn the_same_input_produces_the_same_diagnosis_and_text() {
    let fixture = Fixture::new(80);
    let first = diagnose(&fixture.input());
    let second = diagnose(&fixture.input());
    assert_eq!(first, second);

    let first_summary = text::summary(&first).expect("the first summary renders");
    let second_summary = text::summary(&second).expect("the second summary renders");
    assert_eq!(first_summary, second_summary);

    let first_checks = text::checks_csv(&first).expect("the first checks render");
    let second_checks = text::checks_csv(&second).expect("the second checks render");
    assert_eq!(first_checks, second_checks);
}

#[test]
fn check_counts_always_sum_to_the_check_total() {
    let fixture = Fixture::new(80);
    let diagnosis = diagnose(&fixture.input());
    let counts = diagnosis.check_counts;
    assert_eq!(
        counts.pass + counts.fail + counts.unknown,
        diagnosis.checks.len()
    );
}

#[test]
fn out_of_sample_evidence_requires_a_distinct_data_version() {
    let fixture = Fixture::new(80);
    let cases = [
        (
            OutOfSampleStatus::Absent,
            CheckStatus::Unknown,
            "no out-of-sample lab",
        ),
        (
            OutOfSampleStatus::Unreadable,
            CheckStatus::Unknown,
            "cannot be read",
        ),
        (
            OutOfSampleStatus::SameDataVersion,
            CheckStatus::Unknown,
            "same data version",
        ),
        (
            OutOfSampleStatus::DistinctDataVersion,
            CheckStatus::Pass,
            "different data version",
        ),
    ];
    for (status, expected, evidence) in cases {
        let mut input = fixture.input();
        input.out_of_sample = status;
        let diagnosis = diagnose(&input);
        let check = diagnosis
            .checks
            .iter()
            .find(|check| check.id == "evidence.out_of_sample")
            .expect("the out-of-sample check exists");
        assert_eq!(check.status, expected, "{status:?}");
        assert!(
            check.evidence.contains(evidence),
            "{status:?}: {}",
            check.evidence
        );
        assert_eq!(check.value, None, "{status:?}");
    }
}

#[test]
fn a_run_without_1m_reports_the_gap_check_unknown() {
    let fixture = Fixture::new(80);
    let diagnosis = diagnose(&fixture.input());
    let check = diagnosis
        .checks
        .iter()
        .find(|check| check.id == "data.long_gap")
        .expect("the gap check exists");
    assert_eq!(check.status, CheckStatus::Unknown);
    assert_eq!(check.value, None);
    assert!(check.evidence.contains("no 1m series"));
    assert!(
        diagnosis
            .triage
            .iter()
            .any(|action| action.contains("Add a 1m series"))
    );
}

#[test]
fn a_present_1m_series_with_no_gaps_passes_with_zero() {
    let fixture = Fixture::with_one_minute(80);
    let diagnosis = diagnose(&fixture.input());
    let check = diagnosis
        .checks
        .iter()
        .find(|check| check.id == "data.long_gap")
        .expect("the gap check exists");
    assert_eq!(check.status, CheckStatus::Pass);
    assert_eq!(check.value, Some(0.0));
    assert!(check.evidence.contains("no 1m gap"));
}

#[test]
fn report_versions_require_the_five_expected_bundles() {
    let fixture = Fixture::new(80);
    let all = ["model", "strategy", "execution", "portfolio", "analytical"];

    assert_eq!(
        report_version_status(&fixture, report_versions(&all)),
        CheckStatus::Pass
    );
    assert_eq!(
        report_version_status(&fixture, report_versions(&all[..4])),
        CheckStatus::Fail
    );
    assert_eq!(
        report_version_status(
            &fixture,
            report_versions(&["model", "strategy", "execution", "portfolio", "wrong"])
        ),
        CheckStatus::Fail
    );

    let mut empty = report_versions(&all);
    empty[0].version.clear();
    assert_eq!(report_version_status(&fixture, empty), CheckStatus::Fail);
}

#[test]
fn a_zero_trade_exit_reason_has_no_average_hold() {
    let fixture = Fixture::new(80);
    let diagnosis = diagnose(&fixture.input());
    for reason in &diagnosis.strategy.exit_reasons {
        assert_eq!(reason.trades, 0);
        assert_eq!(reason.average_hold_bars, None);
    }
    let csv = String::from_utf8(
        text::exit_reasons_csv(&diagnosis.strategy).expect("the exit reasons render"),
    )
    .expect("the CSV is UTF-8");
    assert!(
        csv.lines()
            .any(|line| line == "strategy,0,0.000000,0.000000,0,")
    );
}

#[test]
fn a_traded_exit_reason_keeps_its_average_hold() {
    let fixture = Fixture::new(80);
    let mut input = fixture.input();
    input.trades = vec![trade("stop", 4, 100.0)];
    let diagnosis = diagnose(&input);
    let stop = diagnosis
        .strategy
        .exit_reasons
        .iter()
        .find(|reason| reason.reason == "stop")
        .expect("the stop reason exists");
    assert_eq!(stop.trades, 1);
    assert_eq!(stop.average_hold_bars, Some(4.0));
    let target = diagnosis
        .strategy
        .exit_reasons
        .iter()
        .find(|reason| reason.reason == "target")
        .expect("the target reason exists");
    assert_eq!(target.average_hold_bars, None);
    let csv = String::from_utf8(
        text::exit_reasons_csv(&diagnosis.strategy).expect("the exit reasons render"),
    )
    .expect("the CSV is UTF-8");
    assert!(
        csv.lines()
            .any(|line| line == "stop,1,1.000000,1.000000,1,4.000000")
    );
}

#[test]
fn a_long_fill_above_the_entry_bar_clamps_mfe_to_zero() {
    let fixture = Fixture::new(80);
    let mut input = fixture.input();
    input.trades = vec![trade("stop", 0, 112.0)];
    let diagnosis = diagnose(&input);
    let enriched = &diagnosis.trades[0];
    assert_eq!(enriched.mfe, Some(0.0));
    assert_eq!(enriched.mae, Some(-3.0));
    assert_eq!(enriched.max_favorable_price, Some(111.0));
    assert_eq!(enriched.max_adverse_price, Some(109.0));
}

#[test]
fn a_short_fill_below_the_entry_bar_clamps_mfe_to_zero() {
    let fixture = Fixture::new(80);
    let mut short = trade("stop", 0, 108.0);
    short.side = -1;
    let mut input = fixture.input();
    input.trades = vec![short];
    let diagnosis = diagnose(&input);
    let enriched = &diagnosis.trades[0];
    assert_eq!(enriched.mfe, Some(0.0));
    assert_eq!(enriched.mae, Some(-3.0));
    assert_eq!(enriched.max_favorable_price, Some(109.0));
    assert_eq!(enriched.max_adverse_price, Some(111.0));
}

#[test]
fn an_in_range_fill_keeps_its_measured_excursions() {
    let fixture = Fixture::new(80);
    let mut input = fixture.input();
    input.trades = vec![trade("stop", 0, 110.0)];
    let diagnosis = diagnose(&input);
    let enriched = &diagnosis.trades[0];
    assert_eq!(enriched.mfe, Some(1.0));
    assert_eq!(enriched.mae, Some(-1.0));
    assert_eq!(enriched.max_favorable_price, Some(111.0));
    assert_eq!(enriched.max_adverse_price, Some(109.0));
}

fn fill(time: i64, reason: &str) -> FillFacts {
    FillFacts {
        side: 1,
        quantity: 1.0,
        price: 100.0,
        reference_price: 100.0,
        fee: 0.0,
        slippage: 0.0,
        bar: 10,
        time,
        reason: reason.to_owned(),
    }
}

#[test]
fn the_daily_funnel_counts_open_entry_fills() {
    let fixture = Fixture::new(80);
    let mut input = fixture.input();
    let closed = trade("stop", 4, 100.0);
    let entry_time = closed.entry_time;
    let next_day = entry_time + 86_400;
    input.trades = vec![closed];
    input.fills = vec![
        fill(entry_time, "entry"),
        fill(next_day, "entry"),
        fill(next_day + 300, "exit"),
    ];
    let diagnosis = diagnose(&input);
    let series = &diagnosis.strategy.funnel_series;
    let daily: usize = series.iter().map(|point| point.entry_fills).sum();
    assert_eq!(daily, 2);
    assert_eq!(daily, diagnosis.funnel.entry_fills);
    assert!(series.iter().any(|point| {
        point.time == next_day.div_euclid(86_400) * 86_400 && point.entry_fills == 1
    }));
}

#[test]
fn an_empty_series_is_unknown_not_a_panic() {
    let fixture = Fixture::new(0);
    let diagnosis = diagnose(&fixture.input());
    assert!(diagnosis.data_quality.timeframes.is_empty());
    assert!(diagnosis.model_health.is_empty());
    assert!(
        diagnosis.checks.iter().any(
            |check| check.status == quantrule_analytics::diagnosis::types::CheckStatus::Unknown
        )
    );
    assert!(text::summary(&diagnosis).is_ok());
}
