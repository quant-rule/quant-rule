//! The control-plane queue contract.
//!
//! The routing keys and queue base names are shared by the gateway, both
//! workers, and the adapter. They live here, not in the RabbitMQ module, so a
//! service never names a client library just to publish a message.
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
//! # Queue topology
//!
//! Diagnostic stage work travels on its own queue. Every queue name carries the
//! configured [`QueueNames::generation`], so a topology change can declare a
//! new generation beside the live one instead of redeclaring a name with new
//! arguments:
//!
//! ```text
//! exchange  quantrule.runs           (direct, durable)
//! exchange  quantrule.runs.dlx       (direct, durable)
//! queue     runs.backtest.g1         run.requested, run.cancelled
//! queue     runs.evidence.g1         run.computed
//! queue     runs.dead-letter.g1      run.dead and dead-lettered work
//! queue     stages.worker.g1         stage.requested, stage.cancelled
//! ```
//!
//! The adapter declares the run, stage, and evidence queues as quorum queues
//! with `x-delivery-limit=3` and `x-dead-letter-exchange=quantrule.runs.dlx`.
//! A message that exhausts the limit keeps its original routing key and lands
//! in the dead-letter queue with an `x-death` header. The dead-letter queue is
//! quorum and has no dead-letter target of its own, so a poison message cannot
//! loop. The operator retires a drained generation deliberately; the service
//! never deletes a queue.
//!
//! `stage.computed` and `stage.failed` are notifications with no required
//! consumer, exactly like `run.failed`; they name the stage row whose state is
//! authoritative. The stage queue is separate from the run queue so a long
//! diagnostic stage cannot delay an evidence run, and the backtest worker
//! registers the run consumer first. The stage consumer also declares a lower
//! RabbitMQ consumer priority (`x-priority = -1`) than the run consumer, which
//! is the default `0`.
//!
//! Every stage message carries only the stage run id. It never carries a
//! reading, a candle, an artifact, or a client path; the stage row holds the
//! input and the artifact manifest. A run message carries only the run id; the
//! run row holds the inputs.

/// The run-request queue base name.
pub const QUEUE_BACKTEST_BASE: &str = "runs.backtest";
/// The computed-run queue base name.
pub const QUEUE_EVIDENCE_BASE: &str = "runs.evidence";
/// The exhausted-retry queue base name.
pub const QUEUE_DEAD_LETTER_BASE: &str = "runs.dead-letter";
/// The diagnostic stage queue base name.
pub const QUEUE_STAGE_BASE: &str = "stages.worker";

/// The generation used when `QR_QUEUE_GENERATION` is unset.
///
/// Generation 1 is the first quorum topology. The pre-generation classic queues
/// (`runs.backtest`, `runs.evidence`, `runs.dead-letter`, `stages.worker`) are
/// never declared by this version; an operator drains or retires them after the
/// upgrade.
pub const DEFAULT_QUEUE_GENERATION: u32 = 1;

/// The separator between a queue base name and its generation.
pub const QUEUE_GENERATION_SEPARATOR: &str = ".g";

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

/// One queue in the control-plane topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueueKind {
    /// A run-request work queue.
    Backtest,
    /// A computed-run work queue.
    Evidence,
    /// The exhausted-retry queue.
    DeadLetter,
    /// A diagnostic stage work queue.
    Stage,
}

impl QueueKind {
    /// Every queue in declaration order.
    pub const ALL: [Self; 4] = [
        Self::Backtest,
        Self::Evidence,
        Self::DeadLetter,
        Self::Stage,
    ];

    /// The generation-independent base name.
    #[must_use]
    pub fn base(self) -> &'static str {
        match self {
            Self::Backtest => QUEUE_BACKTEST_BASE,
            Self::Evidence => QUEUE_EVIDENCE_BASE,
            Self::DeadLetter => QUEUE_DEAD_LETTER_BASE,
            Self::Stage => QUEUE_STAGE_BASE,
        }
    }
}

/// The queue names of one generation.
///
/// The generation is appended to every base name. Two generations therefore
/// have disjoint queue names, which is what lets a changed declaration (a
/// different queue type, delivery limit, or dead-letter target) be deployed
/// without redeclaring a live queue. The exchange names stay stable: only the
/// queues move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueNames {
    generation: u32,
    backtest: String,
    evidence: String,
    dead_letter: String,
    stage: String,
}

impl QueueNames {
    /// Build the names for one generation.
    ///
    /// The configuration rejects generation zero; this constructor does not
    /// panic on it, because a name is still well formed and a library value
    /// must not abort.
    #[must_use]
    pub fn new(generation: u32) -> Self {
        Self {
            generation,
            backtest: generated(QueueKind::Backtest, generation),
            evidence: generated(QueueKind::Evidence, generation),
            dead_letter: generated(QueueKind::DeadLetter, generation),
            stage: generated(QueueKind::Stage, generation),
        }
    }

    /// The generation suffix.
    #[must_use]
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// The declared name of one queue.
    #[must_use]
    pub fn name(&self, kind: QueueKind) -> &str {
        match kind {
            QueueKind::Backtest => &self.backtest,
            QueueKind::Evidence => &self.evidence,
            QueueKind::DeadLetter => &self.dead_letter,
            QueueKind::Stage => &self.stage,
        }
    }

    /// The queue this declared name belongs to, or `None` when the name is not
    /// part of this generation.
    ///
    /// The consumer uses this to refuse a base name or another generation's
    /// name instead of silently creating a queue outside the topology.
    #[must_use]
    pub fn kind(&self, name: &str) -> Option<QueueKind> {
        QueueKind::ALL
            .into_iter()
            .find(|kind| self.name(*kind) == name)
    }
}

fn generated(kind: QueueKind, generation: u32) -> String {
    format!(
        "{}{}{}",
        kind.base(),
        QUEUE_GENERATION_SEPARATOR,
        generation
    )
}

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
/// A dead letter carries the original bytes. A broker-dead-lettered work
/// message keeps its original routing key and gains the broker's `x-death`
/// header; the worker's explicit retry-budget publish uses [`ROUTING_DEAD`].
/// The queue never rewrites the body.
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
            QUEUE_BACKTEST_BASE,
            QUEUE_EVIDENCE_BASE,
            QUEUE_DEAD_LETTER_BASE,
            QUEUE_STAGE_BASE,
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
        assert_ne!(QUEUE_BACKTEST_BASE, QUEUE_EVIDENCE_BASE);
        assert_ne!(QUEUE_EVIDENCE_BASE, QUEUE_DEAD_LETTER_BASE);
        assert_ne!(QUEUE_STAGE_BASE, QUEUE_BACKTEST_BASE);
    }

    #[test]
    fn every_queue_name_carries_the_generation() {
        let names = QueueNames::new(1);
        assert_eq!(names.generation(), 1);
        assert_eq!(names.name(QueueKind::Backtest), "runs.backtest.g1");
        assert_eq!(names.name(QueueKind::Evidence), "runs.evidence.g1");
        assert_eq!(names.name(QueueKind::DeadLetter), "runs.dead-letter.g1");
        assert_eq!(names.name(QueueKind::Stage), "stages.worker.g1");
        for kind in QueueKind::ALL {
            assert!(names.name(kind).ends_with(".g1"), "{}", names.name(kind));
            assert_eq!(names.kind(names.name(kind)), Some(kind));
        }
    }

    #[test]
    fn a_new_generation_uses_new_names_and_rejects_the_old_ones() {
        let first = QueueNames::new(1);
        let second = QueueNames::new(2);
        for kind in QueueKind::ALL {
            assert_ne!(first.name(kind), second.name(kind));
            assert_eq!(second.kind(first.name(kind)), None);
            assert_eq!(first.kind(kind.base()), None);
        }
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
