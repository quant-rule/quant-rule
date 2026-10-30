//! The trade-side vocabulary shared by strategy, execution, and portfolio.
//!
//! The side describes a position or a decision, not an order. Execution maps it
//! to an [`crate::execution::OrderSide`] at the fill boundary. It lives in its
//! own module so execution and portfolio do not depend on the strategy module.

/// The direction of a position or a decision.
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// A long position.
    Long = 1,
    /// A short position.
    Short = -1,
}

impl Side {
    /// The contract's numeric code: `+1` for long and `-1` for short.
    #[must_use]
    pub const fn code(self) -> i8 {
        self as i8
    }

    /// The opposite side.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Long => Self::Short,
            Self::Short => Self::Long,
        }
    }
}
