//! The deterministic full-pipeline runner.
//!
//! The runner computes the model once, walks the strategy timeframe bar by bar,
//! and applies the contract's per-bar order:
//!
//! 1. funding at an aligned boundary, before fills;
//! 2. `advance`: fill the pending order at the open and evaluate the bracket;
//! 3. feed the fills to the ledger;
//! 4. mark the position to the close;
//! 5. compute the decision and size an entry at the close;
//! 6. `submit` the decision for the next bar.
//!
//! The runner is pure: it reads the inputs and configuration it is given, and
//! performs no I/O, clock read, or random draw. The model is computed once and
//! exposed on the completed run, so the reports never recompute it.
//!
//! A sizing rejection is submitted as a missing quantity, so execution records
//! the blocked entry with [`crate::execution::RejectReason::MissingQuantity`];
//! the ledger's risk event and `halted` flag record the drawdown halt itself.

use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::run::ModelRun;

use crate::execution::{ExecutionBatch, ExecutionConfig, ExecutionEngine, ExecutionError};
use crate::portfolio::{PortfolioConfig, PortfolioLedger, PortfolioLedgerError, PortfolioMetrics};
use crate::strategy::{AxisAgreementStrategy, Decision, Strategy, StrategyConfig, StrategyInput};

/// The configuration of one full run.
///
/// The strategy timeframe is the one in [`StrategyConfig`]; it must equal the
/// execution fill timeframe, which the shell's config loader checks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BacktestConfig {
    /// The model's structure scale.
    pub model: MarketStateConfig,
    /// The strategy's timeframe and thresholds.
    pub strategy: StrategyConfig,
    /// Execution's costs and bracket multiples.
    pub execution: ExecutionConfig,
    /// The portfolio's capital, costs, and risk limits.
    pub portfolio: PortfolioConfig,
}

/// The result of one full run.
#[derive(Debug, Clone, PartialEq)]
pub struct BacktestRun {
    model: ModelRun,
    decisions: Vec<Decision>,
    batches: Vec<ExecutionBatch>,
    ledger: PortfolioLedger,
}

impl BacktestRun {
    /// The model run the pipeline computed once.
    #[must_use]
    pub const fn model_run(&self) -> &ModelRun {
        &self.model
    }

    /// One decision per strategy bar, in bar order.
    #[must_use]
    pub fn decisions(&self) -> &[Decision] {
        &self.decisions
    }

    /// The execution batches, in call order: advance then submit per bar, then
    /// finish.
    #[must_use]
    pub fn execution_batches(&self) -> &[ExecutionBatch] {
        &self.batches
    }

    /// The completed portfolio ledger.
    #[must_use]
    pub const fn ledger(&self) -> &PortfolioLedger {
        &self.ledger
    }

    /// The portfolio summary metrics.
    #[must_use]
    pub fn metrics(&self) -> PortfolioMetrics {
        self.ledger.metrics()
    }

    /// Whether the run produced no bars, decisions, batches, or trades.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.decisions.is_empty() && self.batches.is_empty() && self.ledger.trades().is_empty()
    }
}

/// Runs the model, strategy, execution, and portfolio over the inputs.
///
/// The model is computed once over every non-empty timeframe input. If the
/// strategy timeframe is absent, the run is empty and the ledger stays at its
/// initial cash; this is not an error.
///
/// # Errors
///
/// Returns [`BacktestError`] when execution or the ledger rejects its
/// configuration, when a fill cannot be applied, or when the model omits a bar
/// inside a non-empty timeframe.
pub fn run_backtest(
    inputs: &[TimeframeInput<'_>],
    config: &BacktestConfig,
) -> Result<BacktestRun, BacktestError> {
    let model = ModelRun::compute(inputs, &config.model);
    let mut execution = ExecutionEngine::new(config.execution).map_err(BacktestError::Execution)?;
    let mut ledger = PortfolioLedger::new(config.portfolio).map_err(BacktestError::Ledger)?;
    let mut strategy = AxisAgreementStrategy::new(config.strategy);
    let mut decisions = Vec::new();
    let mut batches = Vec::new();

    if let Some(timeframe) = model.timeframe(config.strategy.timeframe_secs()) {
        for bar_index in 0..timeframe.len() {
            let bar = timeframe
                .bar(bar_index)
                .ok_or(BacktestError::MissingBar { bar: bar_index })?;
            let candle = bar.candle;
            let time = candle.timestamp();

            ledger
                .apply_funding(bar_index, time, candle.open(), candle.volume())
                .map_err(BacktestError::Ledger)?;

            let batch = execution
                .advance(&candle)
                .map_err(BacktestError::Execution)?;
            for fill in &batch.fills {
                ledger.apply_fill(fill).map_err(BacktestError::Ledger)?;
            }
            batches.push(batch);

            ledger
                .mark(bar_index, time, candle.close())
                .map_err(BacktestError::Ledger)?;

            let decision = strategy.on_bar(&StrategyInput::from_model_bar(&bar));
            decisions.push(decision);

            let quantity = match decision {
                Decision::Enter(entry) => ledger.size_entry(entry.side, candle.close()).ok(),
                Decision::Exit(_) | Decision::Hold => None,
            };
            let atr14 = match decision {
                Decision::Enter(_) => bar.readings.atr14,
                Decision::Exit(_) | Decision::Hold => None,
            };
            let batch = execution
                .submit(decision, quantity, atr14)
                .map_err(BacktestError::Execution)?;
            batches.push(batch);
        }
    }

    let batch = execution.finish();
    if !batch.is_empty() {
        for fill in &batch.fills {
            ledger.apply_fill(fill).map_err(BacktestError::Ledger)?;
        }
        batches.push(batch);
    }

    Ok(BacktestRun {
        model,
        decisions,
        batches,
        ledger,
    })
}

/// The runner could not complete a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BacktestError {
    /// Execution rejected its configuration or a bar.
    Execution(ExecutionError),
    /// The ledger rejected its configuration, a fill, or a mark.
    Ledger(PortfolioLedgerError),
    /// The model published a non-empty timeframe but omitted one of its bars.
    MissingBar {
        /// The zero-based strategy bar.
        bar: usize,
    },
}

impl std::fmt::Display for BacktestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Execution(source) => source.fmt(formatter),
            Self::Ledger(source) => source.fmt(formatter),
            Self::MissingBar { bar } => write!(formatter, "the model has no bar {bar}"),
        }
    }
}

impl std::error::Error for BacktestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Execution(source) => Some(source),
            Self::Ledger(source) => Some(source),
            Self::MissingBar { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests;
