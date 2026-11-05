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
//! Two generic, event-driven engines remain for S3.x: the bounded
//! [`EventCompositeStrategy`] and the two-sided [`CompressionBracketStrategy`].
//! The axis-ladder strategy left with the four-axis cutover.

mod bracket;
mod config;
mod decision;
mod event;
mod event_input;
mod input;

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
pub use event_input::{EventStrategyInput, EventTimeframeView, EventView};
pub use input::StrategyInput;
