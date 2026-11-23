//! Graceful shutdown and drain.
//!
//! A signal starts a drain: the server stops accepting, in-flight requests
//! finish, and the process exits. A dropped connection is not a shutdown. The
//! [`Drain`] couples the signal to the readiness gauges, so `/ready` flips at
//! the same instant the process stops accepting new work.

use std::time::Duration;

use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::metrics::Metrics;

/// A shutdown broadcaster.
///
/// Cloning a receiver gives every task the same signal. The sender is dropped
/// when the process decides to stop.
#[derive(Debug, Clone)]
pub struct Shutdown {
    receiver: watch::Receiver<bool>,
}

impl Shutdown {
    /// Build a shutdown handle and its trigger.
    #[must_use]
    pub fn channel() -> (ShutdownTrigger, Self) {
        let (sender, receiver) = watch::channel(false);
        (ShutdownTrigger { sender }, Self { receiver })
    }

    /// Wait until shutdown is requested.
    ///
    /// A dropped trigger without a `true` value is not a signal: only the value
    /// decides. That keeps a test or a helper that discards the trigger from
    /// looking like a drain.
    pub async fn wait(&mut self) {
        loop {
            if *self.receiver.borrow_and_update() {
                return;
            }
            if self.receiver.changed().await.is_err() {
                std::future::pending::<()>().await;
            }
        }
    }

    /// Whether shutdown has already been requested.
    #[must_use]
    pub fn is_requested(&self) -> bool {
        *self.receiver.borrow()
    }
}

/// The handle that requests shutdown.
#[derive(Debug, Clone)]
pub struct ShutdownTrigger {
    sender: watch::Sender<bool>,
}

impl ShutdownTrigger {
    /// Request shutdown. Calling it twice is harmless.
    pub fn trigger(&self) {
        let _ = self.sender.send(true);
    }
}

/// The drain state of one process.
///
/// The handle owns the deadline the server or worker uses and the metrics
/// registry whose readiness gauges it flips. `/ready` reads the same
/// [`Shutdown`] channel, so the HTTP answer and the gauges cannot disagree.
#[derive(Debug, Clone)]
pub struct Drain {
    shutdown: Shutdown,
    metrics: Option<Metrics>,
    timeout: Duration,
}

impl Drain {
    /// Build a drain handle and mark the process ready.
    #[must_use]
    pub fn new(shutdown: Shutdown, metrics: Option<Metrics>, timeout: Duration) -> Self {
        if let Some(metrics) = &metrics {
            metrics.set_ready(true);
            metrics.set_draining(false);
        }
        Self {
            shutdown,
            metrics,
            timeout,
        }
    }

    /// Whether a drain signal has arrived.
    #[must_use]
    pub fn is_draining(&self) -> bool {
        self.shutdown.is_requested()
    }

    /// The deadline for in-flight work after the signal.
    #[must_use]
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// A clone of the shutdown channel, for the server and worker loops.
    #[must_use]
    pub fn shutdown(&self) -> Shutdown {
        self.shutdown.clone()
    }

    /// Wait for the signal and flip the readiness gauges.
    pub async fn wait(&mut self) {
        self.shutdown.wait().await;
        if let Some(metrics) = &self.metrics {
            metrics.set_ready(false);
            metrics.set_draining(true);
        }
    }

    /// Mark the process ready and spawn a task that flips the gauges on a
    /// signal.
    ///
    /// The caller keeps the returned task or drops it with the process; it only
    /// observes the same shutdown channel the server uses.
    #[must_use]
    pub fn spawn_watch(&self) -> JoinHandle<()> {
        let mut drain = self.clone();
        tokio::spawn(async move { drain.wait().await })
    }
}

/// Trigger a shutdown from `SIGTERM` or `Ctrl-C`.
///
/// The task owns a clone of the trigger, so both the HTTP server and the worker
/// loops observe the same signal.
#[must_use]
pub fn watch_signal(trigger: ShutdownTrigger) -> JoinHandle<()> {
    tokio::spawn(async move {
        signal().await;
        trigger.trigger();
    })
}

/// Wait for a termination signal.
///
/// On Unix that is `SIGTERM` or `SIGINT`; elsewhere it is `Ctrl-C`.
pub async fn signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut terminate = match signal(SignalKind::terminate()) {
            Ok(stream) => stream,
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
                return;
            }
        };
        tokio::select! {
            _ = terminate.recv() => {}
            _ = tokio::signal::ctrl_c() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_trigger_wakes_every_receiver() {
        let (trigger, first) = Shutdown::channel();
        let mut second = first.clone();
        assert!(!first.is_requested());
        trigger.trigger();
        first.clone().wait().await;
        second.wait().await;
        assert!(second.is_requested());
    }

    #[tokio::test]
    async fn wait_returns_immediately_after_a_trigger() {
        let (trigger, mut shutdown) = Shutdown::channel();
        trigger.trigger();
        shutdown.wait().await;
        assert!(shutdown.is_requested());
    }

    #[tokio::test]
    async fn a_drain_flips_readiness_and_draining_together() {
        let (trigger, shutdown) = Shutdown::channel();
        let metrics = Metrics::new("data", "1.5.2", None);
        let mut drain = Drain::new(shutdown, Some(metrics.clone()), Duration::from_secs(20));
        assert!(metrics.encode().contains("quantrule_ready 1"));
        assert!(metrics.encode().contains("quantrule_draining 0"));
        trigger.trigger();
        drain.wait().await;
        assert!(drain.is_draining());
        let text = metrics.encode();
        assert!(text.contains("quantrule_ready 0"), "{text}");
        assert!(text.contains("quantrule_draining 1"), "{text}");
    }

    #[tokio::test]
    async fn the_watch_task_flips_the_gauges_before_the_server_returns() {
        let (trigger, shutdown) = Shutdown::channel();
        let metrics = Metrics::new("gateway", "1.5.2", None);
        let drain = Drain::new(shutdown, Some(metrics.clone()), Duration::from_secs(20));
        let watch = drain.spawn_watch();
        assert!(metrics.encode().contains("quantrule_ready 1"));
        trigger.trigger();
        watch.await.expect("watch task");
        let text = metrics.encode();
        assert!(text.contains("quantrule_ready 0"), "{text}");
        assert!(text.contains("quantrule_draining 1"), "{text}");
    }
}
