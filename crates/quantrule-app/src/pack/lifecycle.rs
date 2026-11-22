//! The pack lifecycle.
//!
//! Lifecycle state is metadata outside the content hash. A pack starts as a
//! `draft`, becomes `validated`, may be frozen by the human decision recorded
//! elsewhere, and is finally `retired`. Freeze never rewrites the content: the
//! transition mutates the state field only, and the canonical bytes and pack ID
//! stay fixed for the life of the value.

use std::fmt;

/// One pack lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackState {
    /// Editable and validatable; not usable in a run.
    Draft,
    /// Schema-valid and ready for the freeze decision.
    Validated,
    /// Immutable and usable in new runs.
    Frozen,
    /// Readable for existing runs only; refused for new runs.
    Retired,
}

impl PackState {
    /// The stable wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Validated => "validated",
            Self::Frozen => "frozen",
            Self::Retired => "retired",
        }
    }

    /// Whether the lifecycle permits a move from this state to `next`.
    ///
    /// `validated -> draft` is the rejection path. A frozen pack cannot return
    /// to an earlier state, and a retired pack is terminal.
    #[must_use]
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::Validated)
                | (Self::Validated, Self::Draft)
                | (Self::Validated, Self::Frozen)
                | (Self::Frozen, Self::Retired)
        )
    }

    /// Whether a new run may reference a pack in this state.
    ///
    /// Only a frozen pack is usable for a new run. A retired pack remains
    /// readable for runs that already cite it.
    #[must_use]
    pub const fn allows_new_runs(self) -> bool {
        matches!(self, Self::Frozen)
    }
}

impl fmt::Display for PackState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

/// A lifecycle transition is not allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackStateError {
    from: PackState,
    to: PackState,
}

impl PackStateError {
    pub(crate) const fn new(from: PackState, to: PackState) -> Self {
        Self { from, to }
    }

    /// The state the pack was in.
    #[must_use]
    pub const fn from(&self) -> PackState {
        self.from
    }

    /// The state the caller requested.
    #[must_use]
    pub const fn to(&self) -> PackState {
        self.to
    }
}

impl fmt::Display for PackStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "a pack cannot move from {} to {}",
            self.from, self.to
        )
    }
}

impl std::error::Error for PackStateError {}

#[cfg(test)]
mod tests {
    use super::{PackState, PackStateError};

    #[test]
    fn every_documented_transition_is_reachable() {
        assert!(PackState::Draft.can_transition_to(PackState::Validated));
        assert!(PackState::Validated.can_transition_to(PackState::Draft));
        assert!(PackState::Validated.can_transition_to(PackState::Frozen));
        assert!(PackState::Frozen.can_transition_to(PackState::Retired));
    }

    #[test]
    fn freeze_is_not_reversible() {
        for next in [PackState::Draft, PackState::Validated, PackState::Frozen] {
            assert!(!PackState::Frozen.can_transition_to(next));
        }
    }

    #[test]
    fn retired_is_terminal() {
        for next in [
            PackState::Draft,
            PackState::Validated,
            PackState::Frozen,
            PackState::Retired,
        ] {
            assert!(!PackState::Retired.can_transition_to(next));
        }
    }

    #[test]
    fn only_a_frozen_pack_is_usable_for_a_new_run() {
        assert!(!PackState::Draft.allows_new_runs());
        assert!(!PackState::Validated.allows_new_runs());
        assert!(PackState::Frozen.allows_new_runs());
        assert!(!PackState::Retired.allows_new_runs());
    }

    #[test]
    fn a_transition_error_names_both_states() {
        let error = PackStateError::new(PackState::Frozen, PackState::Validated);
        assert_eq!(error.from(), PackState::Frozen);
        assert_eq!(error.to(), PackState::Validated);
        assert_eq!(
            error.to_string(),
            "a pack cannot move from frozen to validated"
        );
    }
}
