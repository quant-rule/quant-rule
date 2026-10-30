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
use quantrule_model::timeframe::TimeframeModel;

use crate::execution::{
    ExecutionBatch, ExecutionConfig, ExecutionEngine, ExecutionError, OrderIntent,
};
use crate::portfolio::{PortfolioConfig, PortfolioLedger, PortfolioLedgerError, PortfolioMetrics};
use crate::strategy::{
    Agreement, AgreementCounts, AxisAgreementStrategy, Decision, Strategy, StrategyConfig,
    StrategyInput, StrategyWalk,
};

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
    walk: StrategyWalk,
    batches: Vec<ExecutionBatch>,
    ledger: PortfolioLedger,
}

impl BacktestRun {
    /// The model run the pipeline computed once.
    #[must_use]
    pub const fn model_run(&self) -> &ModelRun {
        &self.model
    }

    /// The one strategy walk computed for this run.
    #[must_use]
    pub const fn strategy_walk(&self) -> &StrategyWalk {
        &self.walk
    }

    /// One decision per strategy bar, in bar order.
    #[must_use]
    pub fn decisions(&self) -> &[Decision] {
        self.walk.decisions()
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
        self.walk.is_empty() && self.batches.is_empty() && self.ledger.trades().is_empty()
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
    let strategy = AxisAgreementStrategy::new(config.strategy);
    let mut inputs_seen = Vec::new();
    let mut decisions = Vec::new();
    let mut positions = Vec::new();
    let mut agreements = AgreementCounts::default();
    let mut batches = Vec::new();

    if let Some(timeframe) = model.timeframe(config.strategy.timeframe_secs()) {
        inputs_seen.reserve(timeframe.len());
        decisions.reserve(timeframe.len());
        positions.reserve(timeframe.len());
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

            let input = StrategyInput::from_model_bar(&bar);
            match Agreement::classify(&input, &config.strategy) {
                Some(Agreement::Long) => agreements.long += 1,
                Some(Agreement::Short) => agreements.short += 1,
                Some(Agreement::Neutral) => agreements.neutral += 1,
                None => agreements.unavailable += 1,
            }
            let position = execution.position_side();
            let decision = strategy.decide(&input, position);
            inputs_seen.push(input);
            decisions.push(decision);
            positions.push(position);

            let quantity = match decision {
                Decision::Enter(entry) => ledger.size_entry(entry.side, candle.close()).ok(),
                Decision::Exit(_) | Decision::Hold => None,
            };
            let atr14 = match decision {
                Decision::Enter(_) => bar.readings.atr14,
                Decision::Exit(_) | Decision::Hold => None,
            };
            let intent = match decision {
                Decision::Hold => OrderIntent::Hold,
                Decision::Enter(entry) => OrderIntent::Enter {
                    side: entry.side,
                    bar_index: entry.bar.bar_index,
                    open_time: entry.bar.open_time,
                },
                Decision::Exit(exit) => OrderIntent::Exit {
                    side: exit.side,
                    bar_index: exit.bar.bar_index,
                    open_time: exit.bar.open_time,
                },
            };
            let batch = execution
                .submit(intent, quantity, atr14)
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

    let walk = StrategyWalk::new(
        inputs_seen,
        decisions,
        positions,
        agreements,
        execution.position_side(),
    );

    Ok(BacktestRun {
        model,
        walk,
        batches,
        ledger,
    })
}

/// Walks one timeframe assuming every decision is filled immediately.
///
/// This is the diagnostic walk used by `strategy-run`; it is not the executed
/// walk. The backtest runner passes the real execution position into
/// [`Strategy::decide`] on every bar instead. The virtual position here is
/// labeled as such by the command envelope.
///
/// # Errors
///
/// Returns [`BacktestError::MissingBar`] when the model omits a bar inside a
/// non-empty timeframe.
pub fn walk_strategy(
    timeframe: &TimeframeModel,
    config: &StrategyConfig,
) -> Result<StrategyWalk, BacktestError> {
    let strategy = AxisAgreementStrategy::new(*config);
    let mut position = None;
    let mut inputs = Vec::with_capacity(timeframe.len());
    let mut decisions = Vec::with_capacity(timeframe.len());
    let mut positions = Vec::with_capacity(timeframe.len());
    let mut agreements = AgreementCounts::default();

    for bar_index in 0..timeframe.len() {
        let bar = timeframe
            .bar(bar_index)
            .ok_or(BacktestError::MissingBar { bar: bar_index })?;
        let input = StrategyInput::from_model_bar(&bar);
        match Agreement::classify(&input, config) {
            Some(Agreement::Long) => agreements.long += 1,
            Some(Agreement::Short) => agreements.short += 1,
            Some(Agreement::Neutral) => agreements.neutral += 1,
            None => agreements.unavailable += 1,
        }
        let decision = strategy.decide(&input, position);
        positions.push(position);
        match decision {
            Decision::Enter(entry) => position = Some(entry.side),
            Decision::Exit(_) => position = None,
            Decision::Hold => {}
        }
        inputs.push(input);
        decisions.push(decision);
    }

    Ok(StrategyWalk::new(
        inputs, decisions, positions, agreements, position,
    ))
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
