//! The strategy rule contract.
//!
//! A strategy reads one closed execution bar and the position execution
//! actually holds, then returns a causal [`Decision`]. It holds no position
//! state between calls. It never creates an order, a fill, a position, or a P&L
//! record.
//!
//! An entry carries a [`TradeIntent`]: the side and the risk specification.
//! Execution resolves the intent. The strategy never chooses a fill price, fee,
//! or slippage.
//!
//! Two generic, event-driven engines implement the current strategy layer: the
//! bounded [`EventCompositeStrategy`] and the two-sided
//! [`CompressionBracketStrategy`].

mod bracket;
mod config;
mod decision;
mod event;
mod event_input;
mod input;
pub mod registry;

pub use crate::intent::{
    EntryStyle, RiskPolicy, RiskPolicyError, StopSpec, StrategyPosition, TargetSpec, TradeIntent,
};
pub use crate::side::Side;
pub use bracket::{CompressionBracketConfig, CompressionBracketInput, CompressionBracketStrategy};
pub use config::StrategyConfigError;
pub use decision::{BracketEntry, Decision, Entry, Exit, ExitReason, Strategy};
pub use event::{
    Condition, EventCompositeConfig, EventCompositeStrategy, MAX_CONDITIONS, StateField,
};
pub use event_input::{EventStrategyInput, EventTimeframeView, EventView, context_window_bars};
pub use input::StrategyInput;
pub use registry::{EventIdentity, EventRole};
