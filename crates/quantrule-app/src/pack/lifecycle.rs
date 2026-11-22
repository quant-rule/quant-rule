//! The pack lifecycle.
//!
//! Lifecycle state is metadata outside the content hash. A pack starts as a
//! `draft`, becomes `validated`, may be frozen by the human decision recorded
//! elsewhere, and is finally `retired`. Freeze never rewrites the content: the
//! transition mutates the state field only, and the canonical bytes and pack ID
//! stay fixed for the life of the value.
//!
//! The state vocabulary itself is the shared `quantrule.api/v1` contract, so
//! the registry and the parser cannot drift into two spellings. This module
//! owns only the transition guard an in-process value enforces.

use std::fmt;

pub use quantrule_api::PackState;

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
