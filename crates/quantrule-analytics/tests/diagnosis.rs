//! Integration tests for the cross-engine diagnosis.

use quantrule_analytics::diagnosis::diagnose;
use quantrule_analytics::diagnosis::text;
use quantrule_analytics::diagnosis::types::{
    DiagnosisIdentity, DiagnosisInput, EquityFacts, MetricsFacts, StrategyFacts,
};
use quantrule_data::Candle;
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::run::ModelRun;

struct Fixture {
    run: ModelRun,
}

impl Fixture {
    fn new(count: usize) -> Self {
        let candles: Vec<Candle> = (0..count)
            .map(|index| {
                let timestamp = 1_735_689_600 + index as i64 * 300;
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
            .collect();
        let run = ModelRun::compute(
            &[TimeframeInput {
                timeframe_secs: 300,
                candles: &candles,
            }],
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
            out_of_sample: false,
            rerun_verified: false,
            instrument_count: 1,
        }
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
