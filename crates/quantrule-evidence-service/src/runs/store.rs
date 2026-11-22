//! Run storage behind one trait.

use quantrule_api::{
    ArtifactRef, DatasetReferenceKind, Digest, ProfileId, RunId, RunState, RunSummary,
    StageProgress,
};

use quantrule_runtime::adapters::BoxFuture;

pub use quantrule_service_api::{MAX_ATTEMPTS, NewRun, RunRecord};
pub use quantrule_service_api::{
    RunCancelOutcome as CancelOutcome, RunClaimOutcome as ClaimOutcome,
};

/// Why a run write failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStoreError(pub String);

impl std::fmt::Display for RunStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for RunStoreError {}

/// What a submission did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitOutcome {
    /// The run was created and queued.
    Created(RunRecord),
    /// The identical run already existed; this profile now points at it.
    Existing(RunRecord),
}

impl SubmitOutcome {
    /// The record either way.
    #[must_use]
    pub fn record(&self) -> &RunRecord {
        match self {
            Self::Created(record) | Self::Existing(record) => record,
        }
    }

    /// Whether the run was created.
    #[must_use]
    pub fn created(&self) -> bool {
        matches!(self, Self::Created(_))
    }
}

/// One profile's reference to a run's dataset, for reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunDatasetReference {
    /// The profile that sees the run.
    pub profile: ProfileId,
    /// The run identity.
    pub run_id: RunId,
    /// The dataset the run consumes.
    pub data_version: Digest,
}

/// What an archive did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunArchiveOutcome {
    /// The profile's view is archived.
    Archived(RunSummary),
    /// The profile does not see the run.
    Missing,
    /// The profile already archived it.
    AlreadyArchived,
}

/// The receipt for one erased run submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunEraseReceipt {
    /// The run identity.
    pub run_id: RunId,
    /// The dataset reference to release.
    pub data_version: Digest,
    /// Artifact bytes whose blobs became unreferenced.
    pub bytes: u64,
    /// Blob hashes with no remaining run reference.
    pub blobs: Vec<Digest>,
    /// Whether the shared run row itself was removed.
    pub removed_run: bool,
}

/// What an erase did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEraseOutcome {
    /// The submission and possibly the run row were removed.
    Erased(RunEraseReceipt),
    /// The profile does not see the run.
    Missing,
    /// The profile has not archived the run.
    NotArchived,
    /// The run is evidence or still active.
    Protected(RunState),
}

/// The queue notification a live run still needs.
///
/// The run row is authoritative; this record says which message must be
/// published for a run that has not reached a terminal state. The routing key
/// is fixed by the kind, so two writers cannot invent two names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunNotification {
    /// The run the message names.
    pub run_id: RunId,
    /// Which message the run needs.
    pub kind: NotificationKind,
}

/// The two run notifications the evidence outbox publishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    /// A queued run needs a worker.
    Requested,
    /// A computed run needs evidence.
    Computed,
}

impl NotificationKind {
    /// The queue routing key for this notification.
    #[must_use]
    pub fn routing_key(self) -> &'static str {
        match self {
            Self::Requested => "run.requested",
            Self::Computed => "run.computed",
        }
    }

    /// Parse a stored routing key.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "run.requested" => Some(Self::Requested),
            "run.computed" => Some(Self::Computed),
            _ => None,
        }
    }
}

/// One dataset reference a consumer has promised to release.
///
/// A release is recorded before the consumer row is removed, commits with it,
/// and survives the removal. The reconciler retries the data-service call until
/// the release is confirmed; the row is the tombstone that makes the retry
/// idempotent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetReferenceRelease {
    /// The profile that owns the reference.
    pub profile: ProfileId,
    /// The dataset the reference named.
    pub data_version: Digest,
    /// The consumer identity, such as `run:<id>`.
    pub reference_id: String,
    /// Which consumer kind held the reference.
    pub kind: DatasetReferenceKind,
}

/// What a lease heartbeat did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeartbeatOutcome {
    /// The lease is renewed and the record is current.
    Renewed(Box<RunRecord>),
    /// The term or holder no longer owns the lease.
    Stale,
    /// The run does not exist.
    Missing,
}

/// Where runs live.
pub trait RunStore: Send + Sync {
    /// Submit one run for a profile.
    fn submit(
        &self,
        profile: ProfileId,
        run: NewRun,
    ) -> BoxFuture<Result<SubmitOutcome, RunStoreError>>;

    /// List a profile's runs, newest first.
    fn list(
        &self,
        profile: ProfileId,
        status: Option<RunState>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<RunSummary>, RunStoreError>>;

    /// List a profile's runs including archived submissions, newest first.
    fn list_including_archived(
        &self,
        profile: ProfileId,
        status: Option<RunState>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<RunSummary>, RunStoreError>>;

    /// Read one run within a profile.
    fn get(
        &self,
        profile: ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<Option<RunRecord>, RunStoreError>>;

    /// Archive one profile's view of a run.
    fn archive(
        &self,
        profile: ProfileId,
        run_id: String,
        archived_at: String,
    ) -> BoxFuture<Result<RunArchiveOutcome, RunStoreError>>;

    /// Erase one archived submission and, when it is the last one, the run.
    fn erase(
        &self,
        profile: ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<RunEraseOutcome, RunStoreError>>;

    /// Every run a profile sees, for dataset-reference reconciliation.
    fn submitted_datasets(&self) -> BoxFuture<Result<Vec<RunDatasetReference>, RunStoreError>>;

    /// Cancel a run within a profile.
    fn cancel(
        &self,
        profile: ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<CancelOutcome, RunStoreError>>;

    /// Read one run without a profile, for the worker.
    fn get_any(&self, run_id: String) -> BoxFuture<Result<Option<RunRecord>, RunStoreError>>;

    /// Claim a delivery for a worker.
    ///
    /// A queued run is claimable. A running run is claimable only when its
    /// lease is absent or expired, or when `continuation` names the same
    /// holder's own live term. A live lease is never stolen.
    fn claim(
        &self,
        run_id: String,
        holder: String,
        continuation: Option<u64>,
    ) -> BoxFuture<Result<ClaimOutcome, RunStoreError>>;

    /// Renew the lease for one attempt.
    fn heartbeat(
        &self,
        run_id: String,
        holder: String,
        term: u64,
    ) -> BoxFuture<Result<HeartbeatOutcome, RunStoreError>>;

    /// Record the computed digest and artifacts without finishing the run.
    fn computed(
        &self,
        run_id: String,
        term: u64,
        digest: Digest,
        artifacts: Vec<ArtifactRef>,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, RunStoreError>>;

    /// Mark a run succeeded with its artifacts and final digest.
    fn complete(
        &self,
        run_id: String,
        term: u64,
        digest: Digest,
        artifacts: Vec<ArtifactRef>,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, RunStoreError>>;

    /// Mark a run failed with its stages.
    fn fail(
        &self,
        run_id: String,
        term: u64,
        error: String,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, RunStoreError>>;
}
