//! The strategy rule contract.
//!
//! A strategy reads one closed execution bar, the as-of higher-timeframe
//! context, and the position execution actually holds, then returns a causal
//! [`Decision`]. It holds no position state between calls. It never creates an
//! order, a fill, a position, or a P&L record.
//!
//! An entry carries a [`TradeIntent`]: the side and the risk specification.
//! Execution resolves the intent. The strategy never chooses a fill price, fee,
//! or slippage.
//!
//! The unified [`LadderContextStrategy`] selects the active ladder, weights the
//! higher-timeframe context, and applies the execution and context entry
//! conditions. The rule is defined in
//! `docs/config/2026-10-30/strategy-v3.md`.

mod config;
mod decision;
mod input;
mod ladder;
#[cfg(test)]
mod tests;
mod walk;

pub use crate::context::{
    AgreementCode, AxisContext, ContextTimeframe, LadderConfig, LadderContext, LadderWeights,
    LevelContext, LevelView, ProgressionContext, StructureContext,
};
pub use crate::intent::{
    EntryStyle, RiskPolicy, RiskPolicyError, StopSpec, StrategyPosition, TargetSpec, TradeIntent,
};
pub use crate::side::Side;
pub use config::{
    ContextEntryConfig, DEFAULT_CONFLICT_COUNT_MAX, DEFAULT_DIRECTION_MIN,
    DEFAULT_DIRECTION_SHARE_MIN, DEFAULT_LATE_OR_TERMINAL_COUNT_MAX,
    DEFAULT_MOMENTUM_BEARISH_SHARE_MAX, DEFAULT_MOMENTUM_MIN, DEFAULT_STRUCTURE_MIN, EntryConfig,
    ExecutionEntryConfig, ExitConfig, LevelRetestConfig, StrategyConfig, StrategyConfigError,
    StrategyMode, ThreeTimeframeConfig,
};
pub use decision::{Decision, Entry, Exit, ExitReason, Strategy};
pub use input::StrategyInput;
pub use ladder::LadderContextStrategy;
pub use walk::StrategyWalk;
