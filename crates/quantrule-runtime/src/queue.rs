//! The control-plane queue contract.
//!
//! The routing keys and queue names are shared by the gateway, both workers,
//! and the adapter. They live here, not in the RabbitMQ module, so a service
//! never names a client library just to publish a message.

/// The run-request work queue.
pub const QUEUE_BACKTEST: &str = "runs.backtest";
/// The computed-run work queue.
pub const QUEUE_EVIDENCE: &str = "runs.evidence";
/// The exhausted-retry queue.
pub const QUEUE_DEAD_LETTER: &str = "runs.dead-letter";

/// A run is ready to execute.
pub const ROUTING_REQUESTED: &str = "run.requested";
/// A run has its artifact manifest and digest.
pub const ROUTING_COMPUTED: &str = "run.computed";
/// A run failed terminally.
pub const ROUTING_FAILED: &str = "run.failed";
/// A run was cancelled.
pub const ROUTING_CANCELLED: &str = "run.cancelled";
/// A run spent its retry budget.
pub const ROUTING_DEAD: &str = "run.dead";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_queue_and_routing_key_has_one_name() {
        let queues = [QUEUE_BACKTEST, QUEUE_EVIDENCE, QUEUE_DEAD_LETTER];
        let keys = [
            ROUTING_REQUESTED,
            ROUTING_COMPUTED,
            ROUTING_FAILED,
            ROUTING_CANCELLED,
            ROUTING_DEAD,
        ];
        for (index, name) in queues.iter().enumerate() {
            assert!(!name.is_empty());
            assert!(!queues[..index].contains(name));
        }
        for (index, name) in keys.iter().enumerate() {
            assert!(!name.is_empty());
            assert!(!keys[..index].contains(name));
        }
        assert_ne!(QUEUE_BACKTEST, QUEUE_EVIDENCE);
        assert_ne!(QUEUE_EVIDENCE, QUEUE_DEAD_LETTER);
    }
}
