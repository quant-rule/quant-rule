//! Evaluation benchmarks for the analytics bundle.
//!
//! The benchmarks are descriptive: buy-and-hold measures market beta, and the
//! matched random-timing replay measures whether the strategy's entries beat a
//! null with the same trade count, sides, holding periods, costs, and sizing.
//! No benchmark value enters a model, a strategy, or a decision.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use quantrule_backtest::benchmark::{
    BuyAndHold, Distribution, RandomBenchmark, RandomBenchmarkConfig, buy_and_hold,
    random_trade_paths,
};
use serde_json::{Value, json};

use super::evidence::AnalyticsEvidence;
use super::metrics::Table;

/// The number of seeded random-timing paths.
pub const BENCHMARK_PATHS: usize = 1_000;

/// A buy-and-hold summary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuyHoldSummary {
    /// The final equity.
    pub final_equity: f64,
    /// `final_equity / initial_cash - 1`.
    pub total_return: f64,
    /// The maximum drawdown.
    pub max_drawdown: f64,
    /// The longest underwater period in seconds.
    pub longest_underwater_secs: i64,
}

/// A random-timing summary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RandomSummary {
    /// The explicit seed.
    pub seed: u64,
    /// The number of paths that produced a metric.
    pub paths: usize,
    /// The benchmark method identifier.
    pub method: &'static str,
    /// The final-return distribution.
    pub returns: Distribution,
    /// The maximum-drawdown distribution.
    pub drawdowns: Distribution,
    /// The strategy's final-return percentile among the paths.
    pub strategy_percentile: f64,
    /// The share of paths whose final return beats the strategy.
    pub probability_random_beats_strategy: f64,
}

/// The benchmark section of the analytics bundle.
#[derive(Debug, Clone)]
pub struct BenchmarkSummary {
    /// The strategy's total return.
    pub strategy_return: f64,
    /// The strategy's maximum drawdown.
    pub strategy_max_drawdown: f64,
    /// The buy-and-hold baseline, when the candles support it.
    pub buy_and_hold: Option<BuyHoldSummary>,
    /// The random-timing baseline, when the trades and candles support it.
    pub random: Option<RandomSummary>,
    /// The strategy return minus the buy-and-hold return.
    pub alpha_vs_buy_and_hold: Option<f64>,
    /// The strategy's initial cash.
    pub initial_cash: f64,
    /// The commission rate replayed by both baselines.
    pub commission_rate: f64,
    /// The explicit random seed.
    pub seed: u64,
    /// The requested path count.
    pub paths: usize,
    /// Stable reasons for a missing benchmark.
    pub null_reasons: BTreeMap<&'static str, &'static str>,
}

impl BenchmarkSummary {
    /// Builds the benchmark summary from the recorded evidence.
    #[must_use]
    pub fn build(evidence: &AnalyticsEvidence) -> Self {
        let seed = evidence.bundle.seed.as_u64().unwrap_or(0);
        let mut null_reasons = BTreeMap::new();
        let buy = if evidence.candles.len() < 2 {
            null_reasons.insert("buy_and_hold", "candles_unavailable");
            None
        } else {
            buy_and_hold(
                &evidence.candles,
                evidence.portfolio.initial_cash,
                evidence.commission_rate,
                0.0,
                0,
            )
            .map(BuyHoldSummary::from)
        };
        if buy.is_none() && !null_reasons.contains_key("buy_and_hold") {
            null_reasons.insert("buy_and_hold", "buy_and_hold_unavailable");
        }

        let random = if evidence.trades.is_empty() {
            null_reasons.insert("random", "trades_unavailable");
            None
        } else if evidence.candles.len() < 2 {
            null_reasons.insert("random", "candles_unavailable");
            None
        } else {
            let config = RandomBenchmarkConfig {
                initial_cash: evidence.portfolio.initial_cash,
                commission_rate: evidence.commission_rate,
                max_drawdown_limit: None,
                seed,
                paths: BENCHMARK_PATHS,
                target: 0.0,
                evaluation_days: 0,
                strategy_return: evidence.portfolio.total_return,
            };
            random_trade_paths(&evidence.trades, &evidence.candles, &config)
                .map(RandomSummary::from)
        };
        if random.is_none() && !null_reasons.contains_key("random") {
            null_reasons.insert("random", "random_timing_unavailable");
        }

        let alpha = buy.map(|baseline| evidence.portfolio.total_return - baseline.total_return);
        Self {
            strategy_return: evidence.portfolio.total_return,
            strategy_max_drawdown: evidence.portfolio.max_drawdown,
            buy_and_hold: buy,
            random,
            alpha_vs_buy_and_hold: alpha,
            initial_cash: evidence.portfolio.initial_cash,
            commission_rate: evidence.commission_rate,
            seed,
            paths: BENCHMARK_PATHS,
            null_reasons,
        }
    }

    /// The benchmark table.
    #[must_use]
    pub fn table(&self) -> Table {
        let header = "baseline,paths,seed,total_return,max_drawdown,p05_return,p50_return,p95_return,strategy_percentile,probability_random_beats_strategy,alpha_vs_buy_and_hold";
        let mut text = format!("{header}\n");
        let mut rows = 0;
        let _ = writeln!(
            text,
            "strategy,null,null,{},{},null,null,null,null,null,{}",
            number(self.strategy_return),
            number(self.strategy_max_drawdown),
            optional_number(self.alpha_vs_buy_and_hold),
        );
        rows += 1;
        if let Some(buy) = self.buy_and_hold {
            let _ = writeln!(
                text,
                "buy_and_hold,null,null,{},{},null,null,null,null,null,{}",
                number(buy.total_return),
                number(buy.max_drawdown),
                optional_number(self.alpha_vs_buy_and_hold),
            );
        } else {
            text.push_str("buy_and_hold,null,null,null,null,null,null,null,null,null,null\n");
        }
        rows += 1;
        if let Some(random) = self.random {
            let _ = writeln!(
                text,
                "random_timing,{},{},{},{},{},{},{},{},{},",
                random.paths,
                random.seed,
                number(random.returns.median),
                number(random.drawdowns.median),
                number(random.returns.p05),
                number(random.returns.median),
                number(random.returns.p95),
                number(random.strategy_percentile),
                number(random.probability_random_beats_strategy),
            );
        } else {
            text.push_str("random_timing,null,null,null,null,null,null,null,null,null,null\n");
        }
        rows += 1;
        Table {
            name: "benchmarks",
            file: "tables/benchmarks.csv".to_owned(),
            header,
            text,
            rows,
        }
    }

    /// The JSON value for `metrics.json`.
    #[must_use]
    pub fn json(&self) -> Value {
        json!({
            "strategy": {
                "total_return": self.strategy_return,
                "max_drawdown": self.strategy_max_drawdown,
            },
            "buy_and_hold": self.buy_and_hold.map(|buy| json!({
                "final_equity": buy.final_equity,
                "total_return": buy.total_return,
                "max_drawdown": buy.max_drawdown,
                "longest_underwater_secs": buy.longest_underwater_secs,
            })),
            "random_timing": self.random.map(|random| json!({
                "seed": random.seed,
                "paths": random.paths,
                "method": random.method,
                "return_distribution": distribution_json(random.returns),
                "max_drawdown_distribution": distribution_json(random.drawdowns),
                "strategy_percentile": random.strategy_percentile,
                "probability_random_beats_strategy": random.probability_random_beats_strategy,
            })),
            "alpha_vs_buy_and_hold": self.alpha_vs_buy_and_hold,
            "assumptions": {
                "initial_cash": self.initial_cash,
                "commission_rate": self.commission_rate,
                "max_drawdown_limit": null,
                "execution_fees_replayed": false,
                "seed": self.seed,
                "paths": self.paths,
            },
            "null_reasons": self.null_reasons,
        })
    }

    /// The summary lines for `summary.txt`.
    #[must_use]
    pub fn summary_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!(
            "benchmark_buy_and_hold: {}",
            self.buy_and_hold.map_or_else(
                || "null".to_owned(),
                |buy| format!(
                    "return={} max_drawdown={} final_equity={}",
                    number(buy.total_return),
                    number(buy.max_drawdown),
                    number(buy.final_equity)
                )
            )
        ));
        lines.push(format!(
            "benchmark_random_timing: {}",
            self.random.map_or_else(
                || "null".to_owned(),
                |random| format!(
                    "paths={} seed={} return_p50={} drawdown_p50={} strategy_percentile={} probability_random_beats_strategy={}",
                    random.paths,
                    random.seed,
                    number(random.returns.median),
                    number(random.drawdowns.median),
                    number(random.strategy_percentile),
                    number(random.probability_random_beats_strategy)
                )
            )
        ));
        lines.push(format!(
            "benchmark_alpha_vs_buy_and_hold: {}",
            optional_number(self.alpha_vs_buy_and_hold)
        ));
        lines.push(format!(
            "benchmark_assumptions: initial_cash={} commission_rate={} max_drawdown_limit=null execution_fees_replayed=false",
            number(self.initial_cash),
            number(self.commission_rate)
        ));
        lines
    }
}

impl From<BuyAndHold> for BuyHoldSummary {
    fn from(value: BuyAndHold) -> Self {
        Self {
            final_equity: value.metrics.final_equity,
            total_return: value.metrics.total_return,
            max_drawdown: value.metrics.max_drawdown,
            longest_underwater_secs: value.metrics.longest_underwater_secs,
        }
    }
}

impl From<RandomBenchmark> for RandomSummary {
    fn from(value: RandomBenchmark) -> Self {
        Self {
            seed: value.seed,
            paths: value.paths,
            method: value.method,
            returns: value.return_distribution,
            drawdowns: value.max_drawdown_distribution,
            strategy_percentile: value.strategy_percentile,
            probability_random_beats_strategy: value.probability_random_beats_strategy,
        }
    }
}

fn distribution_json(value: Distribution) -> Value {
    json!({
        "mean": value.mean,
        "median": value.median,
        "p05": value.p05,
        "p95": value.p95,
        "min": value.min,
        "max": value.max,
    })
}

fn number(value: f64) -> String {
    value.to_string()
}

fn optional_number(value: Option<f64>) -> String {
    value.map_or_else(|| "null".to_owned(), number)
}

#[cfg(test)]
mod tests {
    use super::{BenchmarkSummary, BuyHoldSummary, RandomSummary};
    use quantrule_backtest::benchmark::Distribution;

    fn summary() -> BenchmarkSummary {
        BenchmarkSummary {
            strategy_return: 0.1,
            strategy_max_drawdown: 0.05,
            buy_and_hold: Some(BuyHoldSummary {
                final_equity: 11_000.0,
                total_return: 0.1,
                max_drawdown: 0.2,
                longest_underwater_secs: 10,
            }),
            random: Some(RandomSummary {
                seed: 0,
                paths: 1_000,
                method: "random_timing_daily_replay",
                returns: Distribution {
                    mean: 0.0,
                    median: 0.0,
                    p05: -0.1,
                    p95: 0.1,
                    min: -0.2,
                    max: 0.2,
                },
                drawdowns: Distribution {
                    mean: 0.05,
                    median: 0.05,
                    p05: 0.01,
                    p95: 0.1,
                    min: 0.0,
                    max: 0.2,
                },
                strategy_percentile: 75.0,
                probability_random_beats_strategy: 0.25,
            }),
            alpha_vs_buy_and_hold: Some(0.0),
            initial_cash: 10_000.0,
            commission_rate: 0.001,
            seed: 0,
            paths: 1_000,
            null_reasons: Default::default(),
        }
    }

    #[test]
    fn the_table_has_one_row_per_baseline() {
        let table = summary().table();
        assert_eq!(table.header.split(',').count(), 11);
        assert_eq!(table.rows, 3);
        assert!(table.text.contains("strategy,null,null,0.1,0.05"));
        assert!(table.text.contains("buy_and_hold,null,null,0.1,0.2"));
        assert!(table.text.contains("random_timing,1000,0"));
    }

    #[test]
    fn a_missing_baseline_is_null() {
        let mut value = summary();
        value.buy_and_hold = None;
        value.random = None;
        value.alpha_vs_buy_and_hold = None;
        let table = value.table();
        assert!(table.text.contains("buy_and_hold,null,null,null,null"));
        assert!(table.text.contains("random_timing,null,null,null,null"));
    }
}
