//! The shell's bounded retry policy for idempotent reads.
//!
//! The shell is one process with no queue: a transient gateway refusal or an
//! unreachable service is retried in place, and a mutation is not. The policy is
//! deterministic — three attempts, a 200 ms base delay, a factor of four, a 5 s
//! per-delay cap and a 15 s total deadline — with no jitter and no random
//! source. The vocabulary mirrors the runtime policy without depending on it,
//! because the shell depends only on the transport contract.

use std::time::Duration;

/// Total attempts, including the first.
pub(crate) const ATTEMPTS: u32 = 3;
/// The delay after the first failure.
pub(crate) const BASE_DELAY: Duration = Duration::from_millis(200);
/// The factor applied to each subsequent delay.
pub(crate) const FACTOR: u32 = 4;
/// The largest delay between two attempts.
pub(crate) const MAX_DELAY: Duration = Duration::from_secs(5);
/// The deadline for the whole policy, including every delay and attempt.
pub(crate) const TOTAL_DEADLINE: Duration = Duration::from_secs(15);

/// The delay after `failed_attempt`, or `None` when the budget is spent.
///
/// The policy is a pure function of the attempt count, so a test asserts the
/// schedule without sleeping.
pub(crate) fn delay_after(failed_attempt: u32) -> Option<Duration> {
    if failed_attempt == 0 || failed_attempt >= ATTEMPTS {
        return None;
    }
    let exponent = failed_attempt.saturating_sub(1);
    let factor = FACTOR.saturating_pow(exponent);
    Some(BASE_DELAY.saturating_mul(factor).min(MAX_DELAY))
}

/// Whether a status may improve when the same GET repeats.
///
/// The set is deliberately narrow. A `401`, `403`, `404`, every other `4xx`, and
/// any `2xx` are terminal: repeating them cannot change the answer.
pub(crate) fn is_retryable_status(status: u16) -> bool {
    matches!(status, 408 | 429 | 502 | 503 | 504)
}

#[cfg(test)]
mod tests {
    use super::{delay_after, is_retryable_status};
    use std::time::Duration;

    #[test]
    fn the_delay_schedule_is_fixed_and_capped() {
        assert_eq!(delay_after(1), Some(Duration::from_millis(200)));
        assert_eq!(delay_after(2), Some(Duration::from_millis(800)));
        assert_eq!(delay_after(3), None);
        assert_eq!(delay_after(0), None);
        assert_eq!(delay_after(99), None);
    }

    #[test]
    fn only_the_transient_statuses_retry() {
        for status in [408, 429, 502, 503, 504] {
            assert!(is_retryable_status(status), "{status}");
        }
        for status in [200, 204, 301, 400, 401, 403, 404, 409, 500, 501] {
            assert!(!is_retryable_status(status), "{status}");
        }
    }
}
