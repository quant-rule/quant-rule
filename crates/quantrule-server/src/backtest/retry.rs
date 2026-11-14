//! The bounded retry policy for one run delivery.
//!
//! The policy is a pure function of the attempt count so the worker and its
//! tests agree without sleeping. The handler applies the delay.

use std::time::Duration;

use crate::runs::MAX_ATTEMPTS;

/// The waits after the first and second failed attempts.
pub const RETRY_DELAYS: [Duration; 2] = [Duration::from_secs(5), Duration::from_secs(25)];

/// What a failed attempt should do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDecision {
    /// Hold the delivery, wait, and retry it.
    Retry(Duration),
    /// The retry budget is spent.
    DeadLetter,
}

/// Decide from the number of attempts already claimed.
#[must_use]
pub fn decide(attempts: u32) -> RetryDecision {
    if attempts >= MAX_ATTEMPTS {
        return RetryDecision::DeadLetter;
    }
    let index = usize::try_from(attempts.saturating_sub(1))
        .unwrap_or(0)
        .min(RETRY_DELAYS.len() - 1);
    RetryDecision::Retry(RETRY_DELAYS[index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_two_failures_back_off_and_the_third_is_terminal() {
        assert_eq!(decide(1), RetryDecision::Retry(Duration::from_secs(5)));
        assert_eq!(decide(2), RetryDecision::Retry(Duration::from_secs(25)));
        assert_eq!(decide(3), RetryDecision::DeadLetter);
        assert_eq!(decide(4), RetryDecision::DeadLetter);
    }
}
