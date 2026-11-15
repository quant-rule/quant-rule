//! The control-plane queue contract.
//!
//! The routing keys and queue names are shared by the gateway, both workers,
//! and the adapter. They live here, not in the RabbitMQ module, so a service
//! never names a client library just to publish a message.
//!
//! The module also names the two records a durable queue exchanges with its
//! consumer: a [`Delivery`] is one claim and the lease that owns it, and a
//! [`DeadLetter`] is an exhausted message kept for the operator. The retry
//! policy is deliberately not here. The worker counts the run's attempts and
//! holds the delivery for 5 s and then 25 s
//! ([`docs/architecture/control-plane.md`](../../../docs/architecture/control-plane.md)
//! section 3); the queue records the visibility the caller asks for. A queue
//! that invented a second policy would diverge from the RabbitMQ profile.

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

/// One claimed message and the lease that owns it.
///
/// The bytes are the producer's, unchanged. A redelivery carries the same
/// payload, so a consumer can hash it and compare digests; only `attempts` and
/// the redelivery flag differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    /// The stored message identity, stable across redeliveries.
    pub id: i64,
    /// The routing key the producer published under.
    pub routing_key: String,
    /// The exact bytes the producer published.
    pub payload: Vec<u8>,
    /// How many times this message has been delivered, including this one.
    pub attempts: u32,
    /// The opaque lease token that must accompany an acknowledgement.
    pub lease_owner: String,
}

impl Delivery {
    /// Whether this is a redelivery, which is true after the first attempt.
    #[must_use]
    pub fn redelivered(&self) -> bool {
        self.attempts > 1
    }
}

/// An exhausted message kept for the operator.
///
/// A dead letter carries the original bytes and the routing key it arrived
/// under, so an operator can replay the same work. The queue never rewrites the
/// body; the routing key it is filed under is [`ROUTING_DEAD`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadLetter {
    /// The dead-letter row identity.
    pub id: i64,
    /// The routing key the message was dead-lettered under.
    pub routing_key: String,
    /// The routing key the message was originally published under, when it
    /// arrived through a delivery rather than a direct publish.
    pub source_routing_key: Option<String>,
    /// The original bytes, unchanged.
    pub payload: Vec<u8>,
    /// How many deliveries the message had before it was dead-lettered.
    pub attempts: u32,
    /// Why the message was dead-lettered.
    pub reason: String,
    /// When the dead-letter row was written, RFC 3339 UTC.
    pub dead_lettered_at: String,
}

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

    #[test]
    fn a_delivery_is_a_redelivery_after_the_first_attempt() {
        let first = Delivery {
            id: 1,
            routing_key: ROUTING_REQUESTED.to_owned(),
            payload: b"run".to_vec(),
            attempts: 1,
            lease_owner: "consumer-1".to_owned(),
        };
        assert!(!first.redelivered());
        let second = Delivery {
            attempts: 2,
            ..first
        };
        assert!(second.redelivered());
    }
}
