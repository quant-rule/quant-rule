//! Graceful shutdown.
//!
//! A signal starts a drain: the server stops accepting, in-flight requests
//! finish, and the process exits. A dropped connection is not a shutdown.

use tokio::sync::watch;

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
    pub async fn wait(&mut self) {
        if *self.receiver.borrow() {
            return;
        }
        let _ = self.receiver.changed().await;
    }

    /// Whether shutdown has already been requested.
    #[must_use]
    pub fn is_requested(&self) -> bool {
        *self.receiver.borrow()
    }
}

/// The handle that requests shutdown.
#[derive(Debug)]
pub struct ShutdownTrigger {
    sender: watch::Sender<bool>,
}

impl ShutdownTrigger {
    /// Request shutdown. Calling it twice is harmless.
    pub fn trigger(&self) {
        let _ = self.sender.send(true);
    }
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
}
