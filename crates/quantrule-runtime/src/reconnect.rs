//! The queue consumer reconnect loop.
//!
//! A consumer that fails must not end the process on the first error: a broker
//! restart, a channel error or a rejected delivery is transient. The loop
//! reconnects in-process with a fixed capped backoff and only returns an error
//! after the budget is spent, so the container restarts deliberately.
//!
//! Shutdown is not a failure. If the drain signal arrives before an attempt or
//! during a backoff, the loop returns `Ok(())` and the caller can finish its own
//! drain. The schedule is 1, 2, 4, 8, 16 and 30 seconds; there is no jitter.

use std::sync::Arc;
use std::time::Duration;

use crate::adapters::{MessageHandler, QueueConsumer};
use crate::metrics::Metrics;
use crate::shutdown::Shutdown;

/// The wait after each failed consumer attempt, in order.
pub const BACKOFF: [Duration; 6] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
    Duration::from_secs(8),
    Duration::from_secs(16),
    Duration::from_secs(30),
];

/// The wait before the reconnect after `failed_attempt`, or `None` when the
/// budget is spent.
#[must_use]
pub fn backoff(failed_attempt: usize) -> Option<Duration> {
    BACKOFF.get(failed_attempt).copied()
}

/// Why a consumer stopped reconnecting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectError(pub String);

impl std::fmt::Display for ReconnectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ReconnectError {}

/// Consume a queue until shutdown or an exhausted reconnect budget.
///
/// `kind` is a fixed consumer label such as `run`, `stage` or `evidence`; it is
/// never the generated queue name. A shutdown is a clean `Ok(())`.
///
/// # Errors
///
/// Returns [`ReconnectError`] after the sixth backoff and one more failed
/// attempt, so the container restarts instead of looping forever.
pub async fn consume(
    consumer: Arc<dyn QueueConsumer>,
    queue: String,
    handler: MessageHandler,
    shutdown: Shutdown,
    service: &'static str,
    kind: &'static str,
    metrics: Option<Metrics>,
) -> Result<(), ReconnectError> {
    let mut failed_attempt = 0usize;
    loop {
        if shutdown.is_requested() {
            return Ok(());
        }
        let mut wait = shutdown.clone();
        let result = tokio::select! {
            biased;
            () = wait.wait() => return Ok(()),
            result = consumer.consume_until(&queue, handler.clone(), shutdown.clone()) => result,
        };
        if shutdown.is_requested() {
            return Ok(());
        }
        let reason = match result {
            Ok(()) => "the consumer ended".to_owned(),
            Err(error) => error.to_string(),
        };
        let Some(pause) = backoff(failed_attempt) else {
            tracing::error!(
                service,
                consumer = kind,
                attempts = failed_attempt.saturating_add(1),
                %reason,
                "the queue consumer exhausted its reconnect budget"
            );
            return Err(ReconnectError(format!(
                "the {kind} consumer for {service} failed after {} attempts: {reason}",
                failed_attempt.saturating_add(1)
            )));
        };
        if let Some(metrics) = &metrics {
            metrics.consumer_reconnect(service, kind);
        }
        tracing::warn!(
            service,
            consumer = kind,
            attempt = failed_attempt.saturating_add(1),
            delay_ms = pause.as_millis(),
            %reason,
            queue = %queue,
            "the queue consumer will reconnect"
        );
        let mut wait = shutdown.clone();
        tokio::select! {
            biased;
            () = wait.wait() => return Ok(()),
            () = tokio::time::sleep(pause) => {}
        }
        failed_attempt = failed_attempt.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::adapters::{Backend, BoxFuture, Message, QueueError};

    #[derive(Clone, Default)]
    struct FlakyConsumer {
        failures: Arc<AtomicUsize>,
        starts: Arc<AtomicUsize>,
    }

    impl FlakyConsumer {
        fn failing(failures: usize) -> Self {
            Self {
                failures: Arc::new(AtomicUsize::new(failures)),
                starts: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    impl QueueConsumer for FlakyConsumer {
        fn backend(&self) -> Backend {
            Backend::InProcess
        }

        fn consume(
            &self,
            _queue: &str,
            _handler: MessageHandler,
        ) -> BoxFuture<Result<(), QueueError>> {
            let started = self.starts.fetch_add(1, Ordering::SeqCst) + 1;
            let failures = self.failures.load(Ordering::SeqCst);
            Box::pin(async move {
                if started <= failures {
                    Err(QueueError("the broker channel closed".to_owned()))
                } else {
                    std::future::pending::<()>().await;
                    Ok(())
                }
            })
        }
    }

    fn handler() -> MessageHandler {
        Arc::new(|_message: Message| Box::pin(async { Ok(()) }))
    }

    #[test]
    fn the_backoff_schedule_is_fixed_and_then_exhausted() {
        assert_eq!(backoff(0), Some(Duration::from_secs(1)));
        assert_eq!(backoff(1), Some(Duration::from_secs(2)));
        assert_eq!(backoff(2), Some(Duration::from_secs(4)));
        assert_eq!(backoff(3), Some(Duration::from_secs(8)));
        assert_eq!(backoff(4), Some(Duration::from_secs(16)));
        assert_eq!(backoff(5), Some(Duration::from_secs(30)));
        assert_eq!(backoff(6), None);
    }

    #[tokio::test(start_paused = true)]
    async fn a_consumer_fails_twice_then_reconnects() {
        let consumer = Arc::new(FlakyConsumer::failing(2));
        let (trigger, shutdown) = Shutdown::channel();
        let task = tokio::spawn(consume(
            Arc::clone(&consumer) as Arc<dyn QueueConsumer>,
            "runs.backtest.g1".to_owned(),
            handler(),
            shutdown,
            "backtest",
            "run",
            None,
        ));
        for _ in 0..100 {
            if consumer.starts.load(Ordering::SeqCst) >= 3 {
                break;
            }
            tokio::time::advance(Duration::from_secs(1)).await;
        }
        assert_eq!(consumer.starts.load(Ordering::SeqCst), 3);
        trigger.trigger();
        assert!(task.await.expect("task").is_ok());
    }

    #[tokio::test]
    async fn a_shutdown_before_the_first_attempt_consumes_nothing() {
        let consumer = Arc::new(FlakyConsumer::failing(10));
        let (trigger, shutdown) = Shutdown::channel();
        trigger.trigger();
        assert!(
            consume(
                Arc::clone(&consumer) as Arc<dyn QueueConsumer>,
                "runs.backtest.g1".to_owned(),
                handler(),
                shutdown,
                "backtest",
                "run",
                None,
            )
            .await
            .is_ok()
        );
        assert_eq!(consumer.starts.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn a_shutdown_during_the_backoff_is_clean() {
        let consumer = Arc::new(FlakyConsumer::failing(10));
        let (trigger, shutdown) = Shutdown::channel();
        let task = tokio::spawn(consume(
            Arc::clone(&consumer) as Arc<dyn QueueConsumer>,
            "runs.backtest.g1".to_owned(),
            handler(),
            shutdown,
            "backtest",
            "run",
            None,
        ));
        for _ in 0..100 {
            if consumer.starts.load(Ordering::SeqCst) == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        trigger.trigger();
        assert!(task.await.expect("task").is_ok());
        assert_eq!(consumer.starts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn an_exhausted_budget_returns_an_error() {
        let consumer = Arc::new(FlakyConsumer::failing(100));
        let (_, shutdown) = Shutdown::channel();
        let error = consume(
            Arc::clone(&consumer) as Arc<dyn QueueConsumer>,
            "runs.backtest.g1".to_owned(),
            handler(),
            shutdown,
            "backtest",
            "run",
            None,
        )
        .await
        .expect_err("an exhausted budget must return an error");
        assert!(error.0.contains("failed after 7 attempts"), "{error}");
        assert_eq!(consumer.starts.load(Ordering::SeqCst), 7);
    }

    #[tokio::test(start_paused = true)]
    async fn each_reconnect_increments_the_metric() {
        let metrics = Metrics::new("backtest", "1.5.2", None);
        let consumer = Arc::new(FlakyConsumer::failing(100));
        let (_, shutdown) = Shutdown::channel();
        let _ = consume(
            Arc::clone(&consumer) as Arc<dyn QueueConsumer>,
            "runs.backtest.g1".to_owned(),
            handler(),
            shutdown,
            "backtest",
            "run",
            Some(metrics.clone()),
        )
        .await;
        let text = metrics.encode();
        assert!(
            text.contains(
                "quantrule_consumer_reconnects_total{consumer=\"run\",service=\"backtest\"} 6"
            ),
            "{text}"
        );
    }
}
