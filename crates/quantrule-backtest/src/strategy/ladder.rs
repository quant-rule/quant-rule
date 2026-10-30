//! The unified ladder-context strategy.
//!
//! The rule combines an execution-timeframe setup with the as-of
//! higher-timeframe context. It enters long when Direction and Momentum are
//! positive, Structure has turned up, and the context is bullish without
//! conflicts or late legs. It enters short on the mirrored conditions. It exits
//! when the context that justified the entry is lost, when execution Structure
//! flips against the position, or when execution fills an ATR bracket.
//!
//! The strategy is stateless with respect to position. Execution passes the
//! position it actually holds; a rejected entry cannot become a virtual
//! position.

use crate::context::{AgreementCode, AxisContext, LadderContext};
use crate::intent::StrategyPosition;
use crate::side::Side;

use super::config::StrategyConfig;
use super::decision::{Decision, Entry, Exit, ExitReason, Strategy};
use super::input::StrategyInput;

/// The unified ladder-context rule.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::context::{
///     AgreementCode, AxisContext, LadderContext, ProgressionContext, StructureContext,
/// };
/// use quantrule_backtest::intent::StrategyPosition;
/// use quantrule_backtest::strategy::{
///     Decision, LadderContextStrategy, Strategy, StrategyConfig, StrategyInput,
/// };
///
/// let strategy = LadderContextStrategy::new(StrategyConfig::default());
/// let bar = StrategyInput::new(300, 0, 0, Some(0.2), Some(0.2), Some(0.0))
///     .with_structure(Some(1), Some(0));
/// let context = LadderContext {
///     execution_timeframe_secs: 300,
///     execution_bar_index: 0,
///     execution_open_time: 0,
///     execution_close_time: 300,
///     context: Vec::new(),
///     missing_timeframes: Vec::new(),
///     direction: AxisContext {
///         timeframes: vec![900, 3_600],
///         bullish_count: 2,
///         bearish_count: 0,
///         neutral_count: 0,
///         weighted_value: Some(0.5),
///         bullish_share: Some(1.0),
///         bearish_share: Some(0.0),
///         neutral_share: Some(0.0),
///         agreement: AgreementCode::Bullish,
///     },
///     momentum: AxisContext {
///         timeframes: vec![900, 3_600],
///         bullish_count: 1,
///         bearish_count: 0,
///         neutral_count: 1,
///         weighted_value: Some(0.2),
///         bullish_share: Some(0.5),
///         bearish_share: Some(0.0),
///         neutral_share: Some(0.5),
///         agreement: AgreementCode::Bullish,
///     },
///     structure: StructureContext {
///         score: AxisContext::unavailable(),
///         conflict_count: 0,
///     },
///     progression: ProgressionContext {
///         direction: AxisContext {
///             timeframes: vec![900, 3_600],
///             bullish_count: 2,
///             bearish_count: 0,
///             neutral_count: 0,
///             weighted_value: Some(1.0),
///             bullish_share: Some(1.0),
///             bearish_share: Some(0.0),
///             neutral_share: Some(0.0),
///             agreement: AgreementCode::Bullish,
///         },
///         stage_histogram: [0.0; 6],
///         late_or_terminal_count: 0,
///     },
/// };
///
/// assert!(matches!(
///     strategy.decide(&bar, &context, StrategyPosition::flat()),
///     Decision::Enter(_)
/// ));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct LadderContextStrategy {
    config: StrategyConfig,
}

impl LadderContextStrategy {
    /// Builds the rule for a configuration.
    #[must_use]
    pub const fn new(config: StrategyConfig) -> Self {
        Self { config }
    }

    /// The configuration the strategy applies.
    #[must_use]
    pub const fn config(&self) -> &StrategyConfig {
        &self.config
    }

    fn long_entry(&self, input: &StrategyInput, context: &LadderContext) -> bool {
        self.execution_long(input) && self.long_context(context)
    }

    fn short_entry(&self, input: &StrategyInput, context: &LadderContext) -> bool {
        self.execution_short(input) && self.short_context(context)
    }

    fn execution_long(&self, input: &StrategyInput) -> bool {
        let entry = self.config.entry().execution();
        finite(input.direction).is_some_and(|value| value > entry.direction_min())
            && finite(input.momentum_force).is_some_and(|value| value > entry.momentum_min())
            && finite(input.structure_score).is_some_and(|value| value >= entry.structure_min())
            && shift_long(
                input,
                entry.structure_shift_from_max(),
                entry.structure_shift_to_min(),
            )
    }

    fn execution_short(&self, input: &StrategyInput) -> bool {
        let entry = self.config.entry().execution();
        finite(input.direction).is_some_and(|value| value < -entry.direction_min())
            && finite(input.momentum_force).is_some_and(|value| value < -entry.momentum_min())
            && finite(input.structure_score).is_some_and(|value| value <= -entry.structure_min())
            && shift_short(
                input,
                entry.structure_shift_from_max(),
                entry.structure_shift_to_min(),
            )
    }

    fn long_context(&self, context: &LadderContext) -> bool {
        context_side(context, Side::Long, &self.config)
    }

    fn short_context(&self, context: &LadderContext) -> bool {
        context_side(context, Side::Short, &self.config)
    }

    fn context_lost(&self, context: &LadderContext, side: Side) -> bool {
        !context_side(context, side, &self.config)
    }

    fn structure_lost(&self, input: &StrategyInput, side: Side) -> bool {
        match (side, input.structure_state_code) {
            (Side::Long, Some(code)) => code < 0,
            (Side::Short, Some(code)) => code > 0,
            _ => false,
        }
    }

    fn exit_reason(
        &self,
        input: &StrategyInput,
        context: &LadderContext,
        side: Side,
    ) -> Option<ExitReason> {
        let exit = self.config.exit();
        if exit.on_context_lost() && self.context_lost(context, side) {
            Some(ExitReason::ContextLost)
        } else if exit.on_structure_lost() && self.structure_lost(input, side) {
            Some(ExitReason::StructureLost)
        } else {
            None
        }
    }
}

impl Strategy for LadderContextStrategy {
    fn decide(
        &self,
        input: &StrategyInput,
        context: &LadderContext,
        position: StrategyPosition,
    ) -> Decision {
        if input.timeframe_secs != self.config.timeframe_secs() {
            return Decision::Hold;
        }

        match position.side {
            None => {
                if self.config.require_contiguous_execution() && !input.execution_contiguous {
                    return Decision::Hold;
                }
                if !context.context_complete() {
                    return Decision::Hold;
                }
                if self.long_entry(input, context) {
                    Decision::Enter(Entry {
                        bar: *input,
                        intent: self.config.risk().intent(Side::Long),
                    })
                } else if self.short_entry(input, context) {
                    Decision::Enter(Entry {
                        bar: *input,
                        intent: self.config.risk().intent(Side::Short),
                    })
                } else {
                    Decision::Hold
                }
            }
            Some(open) => match self.exit_reason(input, context, open) {
                Some(reason) => Decision::Exit(Exit {
                    side: open,
                    bar: *input,
                    reason,
                }),
                None => Decision::Hold,
            },
        }
    }
}

fn finite(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite())
}

fn shift_long(input: &StrategyInput, from_max: i8, to_min: i8) -> bool {
    match (
        input.previous_structure_state_code,
        input.structure_state_code,
    ) {
        (Some(previous), Some(current)) => previous <= from_max && current >= to_min,
        _ => false,
    }
}

fn shift_short(input: &StrategyInput, from_max: i8, to_min: i8) -> bool {
    match (
        input.previous_structure_state_code,
        input.structure_state_code,
    ) {
        (Some(previous), Some(current)) => previous >= -from_max && current <= -to_min,
        _ => false,
    }
}

fn context_side(context: &LadderContext, side: Side, config: &StrategyConfig) -> bool {
    if !context.context_complete() {
        return false;
    }
    let entry = config.entry().context();
    let direction = axis_side(&context.direction, side, entry.direction_share_min());
    if !direction {
        return false;
    }

    let opposing_momentum = match side {
        Side::Long => context.momentum.bearish_share,
        Side::Short => context.momentum.bullish_share,
    };
    if !finite(opposing_momentum).is_some_and(|share| share <= entry.momentum_bearish_share_max()) {
        return false;
    }

    if context.structure.conflict_count > entry.structure_conflict_count_max() {
        return false;
    }

    let progression_agrees = match side {
        Side::Long => context.progression.direction.agreement == AgreementCode::Bullish,
        Side::Short => context.progression.direction.agreement == AgreementCode::Bearish,
    };
    progression_agrees
        && context.progression.late_or_terminal_count
            <= entry.progression_late_or_terminal_count_max()
}

fn axis_side(axis: &AxisContext, side: Side, share_min: f64) -> bool {
    let (own, opposite) = match side {
        Side::Long => (axis.bullish_share, axis.bearish_share),
        Side::Short => (axis.bearish_share, axis.bullish_share),
    };
    match (finite(own), finite(opposite)) {
        (Some(own), Some(opposite)) => own >= share_min && own > opposite,
        _ => false,
    }
}
