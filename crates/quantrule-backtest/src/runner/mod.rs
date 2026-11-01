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
use quantrule_model::timeframe::ModelBar;

use crate::context::{LadderContext, LadderContextBuilder, LadderContextError};
use crate::execution::{
    ExecutionBatch, ExecutionConfig, ExecutionEngine, ExecutionError, OrderIntent,
};
use crate::intent::StrategyPosition;
use crate::portfolio::{
    PortfolioConfig, PortfolioLedger, PortfolioLedgerError, PortfolioMetrics, SizingRejection,
};
use crate::strategy::{
    Decision, LadderContextStrategy, Strategy, StrategyConfig, StrategyInput, StrategyWalk,
};

/// The configuration of one full run.
///
/// The strategy timeframe is the one in [`StrategyConfig`]; it must equal the
/// execution fill timeframe, which the shell's config loader checks.
#[derive(Debug, Clone, PartialEq)]
pub struct BacktestConfig {
    /// The model's structure scale.
    pub model: MarketStateConfig,
    /// The strategy's active ladder, context weights, rule, and risk policy.
    pub strategy: StrategyConfig,
    /// Execution's fill mechanics and costs.
    pub execution: ExecutionConfig,
    /// The portfolio's capital, costs, and risk limits.
    pub portfolio: PortfolioConfig,
    /// The policy's maximum position notional as a fraction of current equity.
    pub max_position_fraction: f64,
}

/// The result of one full run.
#[derive(Debug, Clone, PartialEq)]
pub struct BacktestRun {
    model: ModelRun,
    walk: StrategyWalk,
    batches: Vec<ExecutionBatch>,
    ledger: PortfolioLedger,
    sizing_rejections: Vec<(usize, SizingRejection)>,
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

    /// The sizing rejections, with the decision bar that produced each.
    #[must_use]
    pub fn sizing_rejections(&self) -> &[(usize, SizingRejection)] {
        &self.sizing_rejections
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
    if !config.max_position_fraction.is_finite()
        || config.max_position_fraction <= 0.0
        || config.max_position_fraction > 1.0
    {
        return Err(BacktestError::RiskEnvelope);
    }

    let model = ModelRun::compute(inputs, &config.model);
    let mut execution = ExecutionEngine::new(config.execution).map_err(BacktestError::Execution)?;
    let mut ledger = PortfolioLedger::new(config.portfolio).map_err(BacktestError::Ledger)?;
    let strategy = LadderContextStrategy::new(config.strategy.clone());
    let mut inputs_seen = Vec::new();
    let mut contexts: Vec<LadderContext> = Vec::new();
    let mut decisions = Vec::new();
    let mut positions: Vec<StrategyPosition> = Vec::new();
    let mut batches = Vec::new();
    let mut sizing_rejections = Vec::new();
    let mut previous: Option<ModelBar> = None;

    if let Some(timeframe) = model.timeframe(config.strategy.timeframe_secs()) {
        let execution_secs = i64::from(timeframe.timeframe_secs() as u32);
        let mut context_builder =
            LadderContextBuilder::new(&model, config.strategy.ladder(), config.strategy.weights())
                .map_err(BacktestError::Context)?;
        inputs_seen.reserve(timeframe.len());
        contexts.reserve(timeframe.len());
        decisions.reserve(timeframe.len());
        positions.reserve(timeframe.len());
        for bar_index in 0..timeframe.len() {
            let bar = timeframe
                .bar(bar_index)
                .ok_or(BacktestError::MissingBar { bar: bar_index })?;
            let candle = bar.candle;
            let time = candle.timestamp();
            let context = context_builder
                .next_context()
                .map_err(BacktestError::Context)?
                .ok_or(BacktestError::MissingContext { bar: bar_index })?;

            ledger
                .apply_funding(bar_index, time, candle.open(), candle.volume())
                .map_err(BacktestError::Ledger)?;

            let contiguous = previous.as_ref().is_none_or(|previous| {
                previous.candle.timestamp().checked_add(execution_secs) == Some(time)
            });

            let batch = execution
                .advance(&candle, contiguous)
                .map_err(BacktestError::Execution)?;
            for fill in &batch.fills {
                ledger.apply_fill(fill).map_err(BacktestError::Ledger)?;
            }
            batches.push(batch);

            ledger
                .mark(bar_index, time, candle.close())
                .map_err(BacktestError::Ledger)?;

            let input = StrategyInput::from_model_bar(&bar, previous.as_ref(), contiguous);
            let position = execution.position_view();
            let decision = strategy.decide(&input, &context, position);
            inputs_seen.push(input);
            contexts.push(context);
            decisions.push(decision);
            positions.push(position);

            let quantity = match decision {
                Decision::Enter(entry) => {
                    match ledger.size_entry(entry.intent.side, candle.close()) {
                        Ok(quantity) => Some(quantity),
                        Err(reason) => {
                            sizing_rejections.push((bar_index, reason));
                            None
                        }
                    }
                }
                Decision::Exit(_) | Decision::Hold => None,
            };
            let atr14 = match decision {
                Decision::Enter(_) => bar.readings.atr14,
                Decision::Exit(_) | Decision::Hold => None,
            };
            let max_entry_notional =
                ledger.state().equity * config.portfolio.leverage * config.max_position_fraction;
            let intent = match decision {
                Decision::Hold => OrderIntent::Hold,
                Decision::Enter(entry) => OrderIntent::Enter {
                    intent: entry.intent,
                    bar_index: entry.bar.bar_index,
                    open_time: entry.bar.open_time,
                    max_notional: Some(max_entry_notional),
                },
                Decision::Exit(exit) => OrderIntent::Exit {
                    side: exit.side,
                    bar_index: exit.bar.bar_index,
                    open_time: exit.bar.open_time,
                },
            };
            let batch = execution
                .submit_with_step(intent, quantity, atr14, Some(ledger.quantity_step()))
                .map_err(BacktestError::Execution)?;
            batches.push(batch);
            previous = Some(bar);
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
        contexts,
        decisions,
        positions,
        execution.position_view(),
    );

    Ok(BacktestRun {
        model,
        walk,
        batches,
        ledger,
        sizing_rejections,
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
/// non-empty timeframe, [`BacktestError::Context`] when the as-of context
/// cannot be built, and [`BacktestError::MissingContext`] when the context
/// stream ends before the execution series.
pub fn walk_strategy(
    run: &ModelRun,
    config: &StrategyConfig,
) -> Result<StrategyWalk, BacktestError> {
    let Some(timeframe) = run.timeframe(config.timeframe_secs()) else {
        return Ok(StrategyWalk::default());
    };
    let strategy = LadderContextStrategy::new(config.clone());
    let execution_secs = i64::from(timeframe.timeframe_secs() as u32);
    let mut context_builder = LadderContextBuilder::new(run, config.ladder(), config.weights())
        .map_err(BacktestError::Context)?;
    let mut position = StrategyPosition::flat();
    let mut inputs = Vec::with_capacity(timeframe.len());
    let mut contexts = Vec::with_capacity(timeframe.len());
    let mut decisions = Vec::with_capacity(timeframe.len());
    let mut positions = Vec::with_capacity(timeframe.len());
    let mut previous: Option<ModelBar> = None;

    for bar_index in 0..timeframe.len() {
        let bar = timeframe
            .bar(bar_index)
            .ok_or(BacktestError::MissingBar { bar: bar_index })?;
        let time = bar.candle.timestamp();
        let context = context_builder
            .next_context()
            .map_err(BacktestError::Context)?
            .ok_or(BacktestError::MissingContext { bar: bar_index })?;
        let contiguous = previous
            .as_ref()
            .is_none_or(|previous| previous.candle.timestamp() + execution_secs == time);
        let input = StrategyInput::from_model_bar(&bar, previous.as_ref(), contiguous);
        if position.is_open() {
            position.bars_held += 1;
        }
        let decision = strategy.decide(&input, &context, position);
        positions.push(position);
        match decision {
            Decision::Enter(entry) => {
                position = StrategyPosition::opened(entry.intent.side, input.bar_index);
            }
            Decision::Exit(_) => position = StrategyPosition::flat(),
            Decision::Hold => {}
        }
        inputs.push(input);
        contexts.push(context);
        decisions.push(decision);
        previous = Some(bar);
    }

    Ok(StrategyWalk::new(
        inputs, contexts, decisions, positions, position,
    ))
}

/// The runner could not complete a run.
#[derive(Debug, Clone, PartialEq)]
pub enum BacktestError {
    /// The policy risk envelope is not a finite fraction in `(0, 1]`.
    RiskEnvelope,
    /// Execution rejected its configuration or a bar.
    Execution(ExecutionError),
    /// The ledger rejected its configuration, a fill, or a mark.
    Ledger(PortfolioLedgerError),
    /// The as-of context could not be built.
    Context(LadderContextError),
    /// The model published a non-empty timeframe but omitted one of its bars.
    MissingBar {
        /// The zero-based strategy bar.
        bar: usize,
    },
    /// The context stream ended before the execution series.
    MissingContext {
        /// The zero-based strategy bar.
        bar: usize,
    },
}

impl std::fmt::Display for BacktestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RiskEnvelope => {
                formatter.write_str("the max position fraction must be finite and in (0, 1]")
            }
            Self::Execution(source) => source.fmt(formatter),
            Self::Ledger(source) => source.fmt(formatter),
            Self::Context(source) => source.fmt(formatter),
            Self::MissingBar { bar } => write!(formatter, "the model has no bar {bar}"),
            Self::MissingContext { bar } => {
                write!(formatter, "the context stream has no bar {bar}")
            }
        }
    }
}

impl std::error::Error for BacktestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::RiskEnvelope => None,
            Self::Execution(source) => Some(source),
            Self::Ledger(source) => Some(source),
            Self::Context(source) => Some(source),
            Self::MissingBar { .. } | Self::MissingContext { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests;
