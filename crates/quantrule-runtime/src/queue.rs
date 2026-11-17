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
//!
//! # Stage topology
//!
//! Diagnostic stage work travels on its own queue:
//!
//! ```text
//! exchange  quantrule.runs        (direct, durable)
//! queue     runs.backtest         run.requested, run.cancelled
//! queue     runs.evidence         run.computed
//! queue     runs.dead-letter      run.dead
//! queue     stages.worker         stage.requested, stage.cancelled
//! ```
//!
//! `stage.computed` and `stage.failed` are notifications with no required
//! consumer, exactly like `run.failed`; they name the stage row whose state is
//! authoritative. The stage queue is separate from `runs.backtest` so a long
//! diagnostic stage cannot delay an evidence run, and the backtest worker
//! registers the run consumer first. The stage consumer also declares a lower
//! RabbitMQ consumer priority (`x-priority = -1`) than the run consumer, which
//! is the default `0`.
//!
//! Every stage message carries only the stage run id. It never carries a
//! reading, a candle, an artifact, or a client path; the stage row holds the
//! input and the artifact manifest.

/// The run-request work queue.
pub const QUEUE_BACKTEST: &str = "runs.backtest";
/// The computed-run work queue.
pub const QUEUE_EVIDENCE: &str = "runs.evidence";
/// The exhausted-retry queue.
pub const QUEUE_DEAD_LETTER: &str = "runs.dead-letter";
/// The diagnostic stage work queue.
pub const QUEUE_STAGE: &str = "stages.worker";

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

/// A diagnostic stage is ready to execute.
pub const ROUTING_STAGE_REQUESTED: &str = "stage.requested";
/// A diagnostic stage recorded its artifact.
pub const ROUTING_STAGE_COMPUTED: &str = "stage.computed";
/// A diagnostic stage failed terminally.
pub const ROUTING_STAGE_FAILED: &str = "stage.failed";
/// A diagnostic stage was cancelled.
pub const ROUTING_STAGE_CANCELLED: &str = "stage.cancelled";

/// Whether a routing key belongs to the diagnostic stage surface.
///
/// Evidence and workers use this to keep the two surfaces apart: a stage key
/// never selects a run handler and a run key never selects a stage handler.
#[must_use]
pub fn is_stage_routing_key(key: &str) -> bool {
    matches!(
        key,
        ROUTING_STAGE_REQUESTED
            | ROUTING_STAGE_COMPUTED
            | ROUTING_STAGE_FAILED
            | ROUTING_STAGE_CANCELLED
    )
}

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
        let queues = [
            QUEUE_BACKTEST,
            QUEUE_EVIDENCE,
            QUEUE_DEAD_LETTER,
            QUEUE_STAGE,
        ];
        let keys = [
            ROUTING_REQUESTED,
            ROUTING_COMPUTED,
            ROUTING_FAILED,
            ROUTING_CANCELLED,
            ROUTING_DEAD,
            ROUTING_STAGE_REQUESTED,
            ROUTING_STAGE_COMPUTED,
            ROUTING_STAGE_FAILED,
            ROUTING_STAGE_CANCELLED,
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
        assert_ne!(QUEUE_STAGE, QUEUE_BACKTEST);
    }

    #[test]
    fn the_stage_surface_is_separate_from_the_run_surface() {
        for key in [
            ROUTING_STAGE_REQUESTED,
            ROUTING_STAGE_COMPUTED,
            ROUTING_STAGE_FAILED,
            ROUTING_STAGE_CANCELLED,
        ] {
            assert!(is_stage_routing_key(key), "{key}");
            assert!(key.starts_with("stage."), "{key}");
        }
        for key in [
            ROUTING_REQUESTED,
            ROUTING_COMPUTED,
            ROUTING_FAILED,
            ROUTING_CANCELLED,
            ROUTING_DEAD,
        ] {
            assert!(!is_stage_routing_key(key), "{key}");
        }
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
