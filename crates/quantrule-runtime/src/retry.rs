//! The bounded retry policy for idempotent internal HTTP calls.
//!
//! The policy is deterministic: three attempts, a 200 ms base delay, a factor
//! of four, a 5 s per-delay cap and a 15 s total deadline. There is no jitter
//! and no random source. Only a call the caller has audited as idempotent may
//! opt in; the default client path stays one attempt.
//!
//! A retry stops as soon as shutdown is requested. The classification is
//! deliberately narrow: an unreachable service, a timeout, `408`, `429`, `502`,
//! `503` and `504` retry; every other status, an invalid base, an unconfigured
//! service, an undecodable response and a draining call do not.

use std::future::Future;
use std::time::Duration;

use tokio::time::Instant;

use crate::client::{ClientError, Service, ServiceResponse};
use crate::metrics::Metrics;
use crate::shutdown::Shutdown;

/// The number of attempts, including the first.
pub const ATTEMPTS: u32 = 3;
/// The delay after the first failure.
pub const BASE_DELAY: Duration = Duration::from_millis(200);
/// The factor applied to each subsequent delay.
pub const FACTOR: u32 = 4;
/// The largest delay between two attempts.
pub const MAX_DELAY: Duration = Duration::from_secs(5);
/// The deadline for the whole policy, including every delay and attempt.
pub const TOTAL_DEADLINE: Duration = Duration::from_secs(15);

/// The retry settings one caller applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// Total attempts, including the first.
    pub attempts: u32,
    /// The delay after the first failure.
    pub base_delay: Duration,
    /// The multiplier for each subsequent delay.
    pub factor: u32,
    /// The maximum delay between two attempts.
    pub max_delay: Duration,
    /// The deadline for the whole policy.
    pub deadline: Duration,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            attempts: ATTEMPTS,
            base_delay: BASE_DELAY,
            factor: FACTOR,
            max_delay: MAX_DELAY,
            deadline: TOTAL_DEADLINE,
        }
    }
}

impl Policy {
    /// The delay after `failed_attempt`, or `None` when the budget is spent.
    ///
    /// The policy is a pure function of the attempt count, so a test asserts
    /// the schedule without sleeping.
    #[must_use]
    pub fn delay_after(&self, failed_attempt: u32) -> Option<Duration> {
        if failed_attempt == 0 || failed_attempt >= self.attempts.max(1) {
            return None;
        }
        let exponent = failed_attempt.saturating_sub(1).min(16);
        let factor = self.factor.max(1).saturating_pow(exponent);
        Some(self.base_delay.saturating_mul(factor).min(self.max_delay))
    }
}

/// Why a retried call did not produce a response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryError {
    /// The process began draining before an answer arrived.
    Shutdown,
    /// The policy ended with this last failure.
    Failed(ClientError),
}

impl std::fmt::Display for RetryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shutdown => {
                formatter.write_str("the call stopped because the process is draining")
            }
            Self::Failed(error) => std::fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for RetryError {}

/// The bounded, deterministic retry reason class for one failure.
///
/// The returned string is a bounded metric label. It never contains an upstream
/// message, a path, a run id or a response body.
#[must_use]
pub fn reason(error: &ClientError) -> Option<&'static str> {
    match error {
        ClientError::Unreachable { .. } => Some("unreachable"),
        ClientError::Timeout { .. } => Some("timeout"),
        ClientError::Status { status, .. } => status_reason(*status),
        ClientError::InvalidResponse { .. }
        | ClientError::InvalidBase { .. }
        | ClientError::Unconfigured { .. }
        | ClientError::Interrupted { .. } => None,
    }
}

/// The bounded retry reason for one HTTP status.
#[must_use]
pub fn status_reason(status: u16) -> Option<&'static str> {
    match status {
        408 => Some("status_408"),
        429 => Some("status_429"),
        502 => Some("status_502"),
        503 => Some("status_503"),
        504 => Some("status_504"),
        _ => None,
    }
}

/// Whether a failure may improve when the same idempotent request repeats.
#[must_use]
pub fn is_retryable(error: &ClientError) -> bool {
    reason(error).is_some()
}

/// Run one idempotent operation under the policy.
///
/// The closure is called once per attempt. It must be safe to repeat: the
/// client opts in only for GET reads and the two audited dataset-reference
/// writes. A claim, a model compute, a signal create, a stage create, a
/// decision, a cancel or a streaming body must never use this function.
///
/// # Errors
///
/// Returns [`RetryError::Shutdown`] when a drain interrupts the policy and
/// [`RetryError::Failed`] with the last failure when the attempts or the deadline
/// are spent.
pub async fn execute<F, Fut>(
    policy: Policy,
    service: Service,
    shutdown: Shutdown,
    metrics: Option<Metrics>,
    mut operation: F,
) -> Result<ServiceResponse, RetryError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<ServiceResponse, ClientError>>,
{
    let started = Instant::now();
    let attempts = policy.attempts.max(1);
    let mut last_error: Option<ClientError> = None;

    for failed_attempt in 1..=attempts {
        if shutdown.is_requested() {
            return Err(RetryError::Shutdown);
        }
        let remaining = policy
            .deadline
            .checked_sub(started.elapsed())
            .unwrap_or_default();
        if remaining.is_zero() {
            break;
        }

        let mut wait = shutdown.clone();
        let result = tokio::select! {
            biased;
            () = wait.wait() => return Err(RetryError::Shutdown),
            result = tokio::time::timeout(remaining, operation()) => match result {
                Ok(result) => result,
                Err(_) => Err(ClientError::Timeout { service }),
            },
        };

        let reason = match result {
            Ok(response) => match status_reason(response.status) {
                Some(reason) => {
                    last_error = Some(ClientError::Status {
                        service,
                        status: response.status,
                        message: String::new(),
                    });
                    reason
                }
                None => return Ok(response),
            },
            Err(error) => {
                let Some(reason) = reason(&error) else {
                    return Err(RetryError::Failed(error));
                };
                last_error = Some(error);
                reason
            }
        };
        let Some(delay) = policy.delay_after(failed_attempt) else {
            break;
        };
        let remaining = policy
            .deadline
            .checked_sub(started.elapsed())
            .unwrap_or_default();
        if delay >= remaining {
            break;
        }
        if let Some(metrics) = &metrics {
            metrics.http_retry(service.name(), reason);
        }
        let mut wait = shutdown.clone();
        tokio::select! {
            biased;
            () = wait.wait() => return Err(RetryError::Shutdown),
            () = tokio::time::sleep(delay) => {}
        }
    }

    Err(RetryError::Failed(
        last_error.unwrap_or(ClientError::Timeout { service }),
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    fn response() -> ServiceResponse {
        ServiceResponse::json(200, b"{}".to_vec())
    }

    fn unavailable(service: Service) -> ClientError {
        ClientError::Unreachable {
            service,
            message: "connection refused".to_owned(),
        }
    }

    #[test]
    fn the_delay_schedule_is_fixed_and_capped() {
        let policy = Policy::default();
        assert_eq!(policy.delay_after(1), Some(Duration::from_millis(200)));
        assert_eq!(policy.delay_after(2), Some(Duration::from_millis(800)));
        assert_eq!(policy.delay_after(3), None);

        let long = Policy {
            attempts: 10,
            ..Policy::default()
        };
        assert_eq!(long.delay_after(3), Some(Duration::from_millis(3_200)));
        assert_eq!(long.delay_after(4), Some(Duration::from_secs(5)));
        assert_eq!(long.delay_after(9), Some(Duration::from_secs(5)));
        assert_eq!(long.delay_after(10), None);
        assert_eq!(policy.delay_after(0), None);
    }

    #[test]
    fn the_classification_is_narrow() {
        let service = Service::Data;
        for error in [
            unavailable(service),
            ClientError::Timeout { service },
            ClientError::Status {
                service,
                status: 408,
                message: String::new(),
            },
            ClientError::Status {
                service,
                status: 429,
                message: String::new(),
            },
            ClientError::Status {
                service,
                status: 502,
                message: String::new(),
            },
            ClientError::Status {
                service,
                status: 503,
                message: String::new(),
            },
            ClientError::Status {
                service,
                status: 504,
                message: String::new(),
            },
        ] {
            assert!(is_retryable(&error), "{error}");
        }
        for error in [
            ClientError::Status {
                service,
                status: 400,
                message: String::new(),
            },
            ClientError::Status {
                service,
                status: 404,
                message: String::new(),
            },
            ClientError::Status {
                service,
                status: 409,
                message: String::new(),
            },
            ClientError::InvalidResponse {
                service,
                message: String::new(),
            },
            ClientError::InvalidBase {
                service,
                message: String::new(),
            },
            ClientError::Unconfigured { service },
            ClientError::Interrupted { service },
        ] {
            assert!(!is_retryable(&error), "{error}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_transient_failure_retries_then_succeeds() {
        let attempts = Arc::new(AtomicU32::new(0));
        let counter = Arc::clone(&attempts);
        let result = execute(
            Policy::default(),
            Service::Data,
            crate::shutdown::Shutdown::channel().1,
            None,
            move || {
                let counter = Arc::clone(&counter);
                async move {
                    if counter.fetch_add(1, Ordering::SeqCst) < 2 {
                        Err(unavailable(Service::Data))
                    } else {
                        Ok(response())
                    }
                }
            },
        )
        .await;
        assert!(result.is_ok());
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test(start_paused = true)]
    async fn a_permanent_failure_stops_after_one_attempt() {
        let attempts = Arc::new(AtomicU32::new(0));
        let counter = Arc::clone(&attempts);
        let error = execute(
            Policy::default(),
            Service::Data,
            crate::shutdown::Shutdown::channel().1,
            None,
            move || {
                let counter = Arc::clone(&counter);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err(ClientError::Status {
                        service: Service::Data,
                        status: 404,
                        message: "missing".to_owned(),
                    })
                }
            },
        )
        .await
        .expect_err("a 404 is terminal");
        assert!(matches!(
            error,
            RetryError::Failed(ClientError::Status { status: 404, .. })
        ));
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn the_total_deadline_bounds_every_delay() {
        let attempts = Arc::new(AtomicU32::new(0));
        let counter = Arc::clone(&attempts);
        let policy = Policy {
            attempts: 10,
            deadline: Duration::from_millis(300),
            ..Policy::default()
        };
        let result = execute(
            policy,
            Service::Data,
            crate::shutdown::Shutdown::channel().1,
            None,
            move || {
                let counter = Arc::clone(&counter);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err(ClientError::Timeout {
                        service: Service::Data,
                    })
                }
            },
        )
        .await;
        assert!(matches!(result, Err(RetryError::Failed(_))));
        assert!(
            attempts.load(Ordering::SeqCst) <= 2,
            "the deadline must stop the policy"
        );
    }

    #[tokio::test]
    async fn shutdown_stops_the_policy_before_a_new_attempt() {
        let attempts = Arc::new(AtomicU32::new(0));
        let counter = Arc::clone(&attempts);
        let (trigger, shutdown) = crate::shutdown::Shutdown::channel();
        trigger.trigger();
        let result = execute(
            Policy::default(),
            Service::Data,
            shutdown,
            None,
            move || {
                let counter = Arc::clone(&counter);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(response())
                }
            },
        )
        .await;
        assert_eq!(result, Err(RetryError::Shutdown));
        assert_eq!(attempts.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn shutdown_during_the_backoff_stops_cleanly() {
        let attempts = Arc::new(AtomicU32::new(0));
        let counter = Arc::clone(&attempts);
        let (trigger, shutdown) = crate::shutdown::Shutdown::channel();
        let policy = Policy {
            base_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(2),
            ..Policy::default()
        };
        let task = tokio::spawn(execute(policy, Service::Data, shutdown, None, move || {
            let counter = Arc::clone(&counter);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Err(unavailable(Service::Data))
            }
        }));
        for _ in 0..100 {
            if attempts.load(Ordering::SeqCst) == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        trigger.trigger();
        assert_eq!(task.await.expect("task"), Err(RetryError::Shutdown));
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn every_attempted_retry_increments_the_metric() {
        let metrics = Metrics::new("data", "1.5.2", None);
        let result = execute(
            Policy::default(),
            Service::Data,
            crate::shutdown::Shutdown::channel().1,
            Some(metrics.clone()),
            || async { Err(unavailable(Service::Data)) },
        )
        .await;
        assert!(matches!(result, Err(RetryError::Failed(_))));
        let text = metrics.encode();
        assert!(
            text.contains(
                "quantrule_http_retries_total{reason=\"unreachable\",service=\"data\"} 2"
            ),
            "{text}"
        );
    }
}
