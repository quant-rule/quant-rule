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
use crate::intent::{EntryStyle, StopSpec, StrategyPosition, TargetSpec, TradeIntent};
use crate::side::Side;

use super::config::{LevelRetestConfig, StrategyConfig, StrategyMode};
use super::decision::{Decision, Entry, Exit, ExitReason, Strategy};
use super::input::StrategyInput;

/// The unified ladder-context rule.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::context::{
///     AgreementCode, AgreementContext, AxisContext, LadderContext, LevelContext,
///     ProgressionContext, StructureContext,
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
///     levels: LevelContext::unavailable(),
///     agreement: AgreementContext::unavailable(),
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
        self.execution_long_values(input)
            && shift_long(
                input,
                self.config.entry().execution().structure_shift_from_max(),
                self.config.entry().execution().structure_shift_to_min(),
            )
    }

    fn execution_short(&self, input: &StrategyInput) -> bool {
        self.execution_short_values(input)
            && shift_short(
                input,
                self.config.entry().execution().structure_shift_from_max(),
                self.config.entry().execution().structure_shift_to_min(),
            )
    }

    fn execution_long_values(&self, input: &StrategyInput) -> bool {
        let entry = self.config.entry().execution();
        finite(input.direction).is_some_and(|value| value > entry.direction_min())
            && finite(input.momentum_force).is_some_and(|value| value > entry.momentum_min())
            && finite(input.structure_score).is_some_and(|value| value >= entry.structure_min())
    }

    fn execution_short_values(&self, input: &StrategyInput) -> bool {
        let entry = self.config.entry().execution();
        finite(input.direction).is_some_and(|value| value < -entry.direction_min())
            && finite(input.momentum_force).is_some_and(|value| value < -entry.momentum_min())
            && finite(input.structure_score).is_some_and(|value| value <= -entry.structure_min())
    }

    fn long_context(&self, context: &LadderContext) -> bool {
        context_side(context, Side::Long, &self.config) == Some(true)
    }

    fn short_context(&self, context: &LadderContext) -> bool {
        context_side(context, Side::Short, &self.config) == Some(true)
    }

    fn context_lost(&self, context: &LadderContext, side: Side) -> bool {
        context_side(context, side, &self.config) == Some(false)
    }

    fn structure_lost(&self, input: &StrategyInput, side: Side) -> bool {
        match (side, input.structure_state_code) {
            (Side::Long, Some(code)) => code < 0,
            (Side::Short, Some(code)) => code > 0,
            _ => false,
        }
    }

    fn level_lost(&self, context: &LadderContext, side: Side) -> bool {
        match side {
            Side::Long => context.levels.support_broken || context.levels.broken_support.is_some(),
            Side::Short => {
                context.levels.resistance_broken || context.levels.broken_resistance.is_some()
            }
        }
    }

    fn level_entry(&self, input: &StrategyInput, context: &LadderContext) -> Option<TradeIntent> {
        let level = self.config.level()?;
        if self.long_context(context) && self.execution_long_values(input) {
            return self.level_intent(context, Side::Long, level);
        }
        if self.short_context(context) && self.execution_short_values(input) {
            return self.level_intent(context, Side::Short, level);
        }
        None
    }

    fn level_intent(
        &self,
        context: &LadderContext,
        side: Side,
        level: LevelRetestConfig,
    ) -> Option<TradeIntent> {
        let retested = match side {
            Side::Long => context.levels.retest_support,
            Side::Short => context.levels.retest_resistance,
        };
        if !retested {
            return None;
        }
        self.bracket_intent(context, side, level)
    }

    fn bracket_intent(
        &self,
        context: &LadderContext,
        side: Side,
        level: LevelRetestConfig,
    ) -> Option<TradeIntent> {
        let levels = &context.levels;
        let reference = levels.reference_price?;
        let support = levels.support?;
        let resistance = levels.resistance?;
        let distance = match side {
            Side::Long => levels.support_distance_fraction?,
            Side::Short => levels.resistance_distance_fraction?,
        };
        if !distance.is_finite() || distance > level.max_distance_fraction() {
            return None;
        }
        let (stop, target) = match side {
            Side::Long => (
                support.price * (1.0 - level.stop_buffer_fraction()),
                resistance.price,
            ),
            Side::Short => (
                resistance.price * (1.0 + level.stop_buffer_fraction()),
                support.price,
            ),
        };
        let (risk, reward) = match side {
            Side::Long => (reference - stop, target - reference),
            Side::Short => (stop - reference, reference - target),
        };
        if !risk.is_finite() || risk <= 0.0 || !reward.is_finite() || reward <= 0.0 {
            return None;
        }
        if reward / risk < level.min_reward_to_risk() {
            return None;
        }
        let intent = TradeIntent {
            side,
            entry: EntryStyle::MarketNextOpen,
            stop: StopSpec::Price { price: stop },
            target: TargetSpec::Price { price: target },
            valid_until: None,
            max_slippage_bps: None,
        };
        if !intent.stop.is_valid() || !intent.target.is_valid() {
            return None;
        }
        Some(intent)
    }

    fn trend_level_intent(
        &self,
        context: &LadderContext,
        side: Side,
        level: LevelRetestConfig,
    ) -> Option<TradeIntent> {
        let retested = match side {
            Side::Long => context.levels.retest_support,
            Side::Short => context.levels.retest_resistance,
        };
        if !retested {
            return None;
        }
        self.trend_bracket_intent(context, side, level)
    }

    fn trend_bracket_intent(
        &self,
        context: &LadderContext,
        side: Side,
        level: LevelRetestConfig,
    ) -> Option<TradeIntent> {
        let levels = &context.levels;
        let reference = levels.reference_price?;
        let (stop_level, stop_distance) = match side {
            Side::Long => (levels.support?, levels.support_distance_fraction?),
            Side::Short => (levels.resistance?, levels.resistance_distance_fraction?),
        };
        if !stop_distance.is_finite() || stop_distance > level.max_distance_fraction() {
            return None;
        }
        let stop = match side {
            Side::Long => stop_level.price * (1.0 - level.stop_buffer_fraction()),
            Side::Short => stop_level.price * (1.0 + level.stop_buffer_fraction()),
        };
        let risk = match side {
            Side::Long => reference - stop,
            Side::Short => stop - reference,
        };
        if !risk.is_finite() || risk <= 0.0 {
            return None;
        }
        let target = match side {
            Side::Long => levels
                .resistance
                .map(|level| TargetSpec::Price { price: level.price }),
            Side::Short => levels
                .support
                .map(|level| TargetSpec::Price { price: level.price }),
        }
        .unwrap_or(TargetSpec::RewardToRisk {
            multiple: level.min_reward_to_risk(),
        });
        if let TargetSpec::Price { price } = target {
            let reward = match side {
                Side::Long => price - reference,
                Side::Short => reference - price,
            };
            if !reward.is_finite() || reward <= 0.0 || reward / risk < level.min_reward_to_risk() {
                return None;
            }
        }
        let intent = TradeIntent {
            side,
            entry: EntryStyle::MarketNextOpen,
            stop: StopSpec::Price { price: stop },
            target,
            valid_until: None,
            max_slippage_bps: None,
        };
        if !intent.stop.is_valid() || !intent.target.is_valid() {
            return None;
        }
        Some(intent)
    }

    fn three_timeframe_entry(
        &self,
        _input: &StrategyInput,
        context: &LadderContext,
    ) -> Option<TradeIntent> {
        let three = self.config.three_timeframe()?;
        let level = three.level();
        if context.agreement.long
            && context.agreement.long_bars >= three.min_agreement_bars()
            && let Some(intent) = self.trend_level_intent(context, Side::Long, level)
        {
            return Some(intent);
        }
        if context.agreement.short
            && context.agreement.short_bars >= three.min_agreement_bars()
            && let Some(intent) = self.trend_level_intent(context, Side::Short, level)
        {
            return Some(intent);
        }
        None
    }

    fn level_fade_entry(
        &self,
        _input: &StrategyInput,
        context: &LadderContext,
    ) -> Option<TradeIntent> {
        let level = self.config.level()?;
        // A failed support test is a long; a failed resistance test is a short.
        // The rule deliberately does not use the directional context: the edge
        // hypothesis is the rejection itself, not trend agreement.
        if context.levels.retest_support {
            return self.bracket_intent(context, Side::Long, level);
        }
        if context.levels.retest_resistance {
            return self.bracket_intent(context, Side::Short, level);
        }
        None
    }

    fn breakout_entry(
        &self,
        input: &StrategyInput,
        context: &LadderContext,
    ) -> Option<TradeIntent> {
        let breakout = self.config.level()?;
        if self.long_context(context) && self.execution_long_values(input) {
            return self.breakout_intent(context, Side::Long, breakout);
        }
        if self.short_context(context) && self.execution_short_values(input) {
            return self.breakout_intent(context, Side::Short, breakout);
        }
        None
    }

    fn breakout_intent(
        &self,
        context: &LadderContext,
        side: Side,
        breakout: LevelRetestConfig,
    ) -> Option<TradeIntent> {
        let levels = &context.levels;
        let reference = levels.reference_price?;
        if !reference.is_finite() || reference <= 0.0 {
            return None;
        }
        let broken = match side {
            Side::Long => levels.broken_resistance?,
            Side::Short => levels.broken_support?,
        };
        let distance = match side {
            Side::Long => (reference - broken.price) / reference,
            Side::Short => (broken.price - reference) / reference,
        };
        if !distance.is_finite()
            || distance < breakout.stop_buffer_fraction()
            || distance > breakout.max_distance_fraction()
        {
            return None;
        }
        let stop = match side {
            Side::Long => broken.price * (1.0 - breakout.stop_buffer_fraction()),
            Side::Short => broken.price * (1.0 + breakout.stop_buffer_fraction()),
        };
        let risk = match side {
            Side::Long => reference - stop,
            Side::Short => stop - reference,
        };
        if !risk.is_finite() || risk <= 0.0 {
            return None;
        }
        let target = match side {
            Side::Long => levels
                .resistance
                .map(|level| TargetSpec::Price { price: level.price }),
            Side::Short => levels
                .support
                .map(|level| TargetSpec::Price { price: level.price }),
        }
        .unwrap_or(TargetSpec::RewardToRisk {
            multiple: breakout.min_reward_to_risk(),
        });
        if let TargetSpec::Price { price } = target {
            let reward = match side {
                Side::Long => price - reference,
                Side::Short => reference - price,
            };
            if !reward.is_finite() || reward <= 0.0 || reward / risk < breakout.min_reward_to_risk()
            {
                return None;
            }
        }
        let intent = TradeIntent {
            side,
            entry: EntryStyle::MarketNextOpen,
            stop: StopSpec::Price { price: stop },
            target,
            valid_until: None,
            max_slippage_bps: None,
        };
        if !intent.stop.is_valid() || !intent.target.is_valid() {
            return None;
        }
        Some(intent)
    }

    fn level_exit_reason(
        &self,
        input: &StrategyInput,
        context: &LadderContext,
        side: Side,
    ) -> Option<ExitReason> {
        let exit = self.config.exit();
        if exit.on_context_lost() && self.context_lost(context, side) {
            Some(ExitReason::ContextLost)
        } else if exit.on_level_lost() && self.level_lost(context, side) {
            Some(ExitReason::LevelLost)
        } else if exit.on_structure_lost() && self.structure_lost(input, side) {
            Some(ExitReason::StructureLost)
        } else {
            None
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
        if context.execution_timeframe_secs != input.timeframe_secs
            || context.execution_bar_index != input.bar_index
            || context.execution_open_time != input.open_time
        {
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
                match self.config.mode() {
                    StrategyMode::LadderContext => {
                        let Some(risk) = self.config.risk() else {
                            return Decision::Hold;
                        };
                        if self.long_entry(input, context) {
                            Decision::Enter(Entry {
                                bar: *input,
                                intent: risk.intent(Side::Long),
                            })
                        } else if self.short_entry(input, context) {
                            Decision::Enter(Entry {
                                bar: *input,
                                intent: risk.intent(Side::Short),
                            })
                        } else {
                            Decision::Hold
                        }
                    }
                    StrategyMode::LevelRetest(_) => match self.level_entry(input, context) {
                        Some(intent) => Decision::Enter(Entry {
                            bar: *input,
                            intent,
                        }),
                        None => Decision::Hold,
                    },
                    StrategyMode::ThreeTimeframe(_) => {
                        match self.three_timeframe_entry(input, context) {
                            Some(intent) => Decision::Enter(Entry {
                                bar: *input,
                                intent,
                            }),
                            None => Decision::Hold,
                        }
                    }
                    StrategyMode::Breakout(_) => match self.breakout_entry(input, context) {
                        Some(intent) => Decision::Enter(Entry {
                            bar: *input,
                            intent,
                        }),
                        None => Decision::Hold,
                    },
                    StrategyMode::LevelFade(_) => match self.level_fade_entry(input, context) {
                        Some(intent) => Decision::Enter(Entry {
                            bar: *input,
                            intent,
                        }),
                        None => Decision::Hold,
                    },
                }
            }
            Some(open) => {
                let reason = match self.config.mode() {
                    StrategyMode::LadderContext => self.exit_reason(input, context, open),
                    StrategyMode::LevelRetest(_)
                    | StrategyMode::ThreeTimeframe(_)
                    | StrategyMode::Breakout(_)
                    | StrategyMode::LevelFade(_) => self.level_exit_reason(input, context, open),
                };
                match reason {
                    Some(reason) => Decision::Exit(Exit {
                        side: open,
                        bar: *input,
                        reason,
                    }),
                    None => Decision::Hold,
                }
            }
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

fn context_side(context: &LadderContext, side: Side, config: &StrategyConfig) -> Option<bool> {
    if !context.context_complete() {
        return None;
    }
    let entry = config.entry().context();
    if !axis_side(&context.direction, side, entry.direction_share_min())? {
        return Some(false);
    }

    let opposing_momentum = match side {
        Side::Long => context.momentum.bearish_share,
        Side::Short => context.momentum.bullish_share,
    };
    let opposing_momentum = finite(opposing_momentum)?;
    if opposing_momentum > entry.momentum_bearish_share_max() {
        return Some(false);
    }

    if context.structure.conflict_count > entry.structure_conflict_count_max() {
        return Some(false);
    }

    // An unavailable reading has no measured direction to disagree with. Only
    // a measured non-agreement may trigger context_lost.
    if context.progression.direction.agreement == AgreementCode::Unavailable {
        return None;
    }
    let progression_agrees = match side {
        Side::Long => context.progression.direction.agreement == AgreementCode::Bullish,
        Side::Short => context.progression.direction.agreement == AgreementCode::Bearish,
    };
    Some(
        progression_agrees
            && context.progression.late_or_terminal_count
                <= entry.progression_late_or_terminal_count_max(),
    )
}

fn axis_side(axis: &AxisContext, side: Side, share_min: f64) -> Option<bool> {
    let (own, opposite) = match side {
        Side::Long => (axis.bullish_share, axis.bearish_share),
        Side::Short => (axis.bearish_share, axis.bullish_share),
    };
    match (finite(own), finite(opposite)) {
        (Some(own), Some(opposite)) => Some(own >= share_min && own > opposite),
        _ => None,
    }
}
