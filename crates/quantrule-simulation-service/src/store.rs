//! The simulation repository seam.
//!
//! The routes call this trait; Postgres is one implementation and the tests use
//! another. Every method takes the profile the gateway injected, so no query can
//! cross a profile boundary. A mutation that can race a decision locks the run
//! row and checks the status inside the same transaction.

use quantrule_api::{MarkOrigin, ProfileId, SimulationStatus};
use quantrule_runtime::adapters::BoxFuture;
use quantrule_simulation::{CheckRecord, Selection};

use crate::records::{DecisionRecord, MarkRecord, NewSimulation, RunRecord, SimulationSummary};

/// Why a store call did not complete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// The run is unknown in this profile.
    NotFound,
    /// Marking is closed after a decision.
    Locked,
    /// The requested change conflicts with the stored state.
    Conflict(String),
    /// The stored row violates the contract.
    Invalid(String),
    /// The database refused or is unreachable.
    Backend(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("the simulation run is unknown"),
            Self::Locked => formatter.write_str("the simulation run is decided"),
            Self::Conflict(message) => formatter.write_str(message),
            Self::Invalid(message) => formatter.write_str(message),
            Self::Backend(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for StoreError {}

/// What an archive did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveOutcome {
    /// The run is now archived.
    Archived(SimulationSummary),
    /// The profile holds no such run.
    NotFound,
    /// The run was already archived.
    AlreadyArchived,
}

/// What an erase did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EraseOutcome {
    /// The run and its marks were removed.
    Erased(String),
    /// The profile holds no such run.
    NotFound,
    /// The run has not been archived.
    NotArchived,
    /// The run is checked or decided and is evidence.
    Protected(SimulationStatus),
}

/// The simulation repository.
pub trait SimulationStore: Send + Sync {
    /// Insert a run, or return the stored row for the same server-assigned id.
    fn create(
        &self,
        profile: ProfileId,
        run: NewSimulation,
    ) -> BoxFuture<Result<RunRecord, StoreError>>;

    /// List this profile's runs.
    fn list(
        &self,
        profile: ProfileId,
        status: Option<SimulationStatus>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<SimulationSummary>, StoreError>>;

    /// List this profile's active and archived runs.
    fn list_including_archived(
        &self,
        profile: ProfileId,
        status: Option<SimulationStatus>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<SimulationSummary>, StoreError>>;

    /// Read one run in this profile.
    fn get(
        &self,
        profile: ProfileId,
        sim_id: String,
    ) -> BoxFuture<Result<Option<RunRecord>, StoreError>>;

    /// Archive one run. The row and its marks remain.
    fn archive(
        &self,
        profile: ProfileId,
        sim_id: String,
        archived_at: String,
    ) -> BoxFuture<Result<ArchiveOutcome, StoreError>>;

    /// Erase one archived run and its marks when its state allows it.
    fn erase(
        &self,
        profile: ProfileId,
        sim_id: String,
    ) -> BoxFuture<Result<EraseOutcome, StoreError>>;

    /// Store the deterministic selection and advance `draft` to `marking`.
    fn save_windows(
        &self,
        profile: ProfileId,
        sim_id: String,
        selection: Selection,
    ) -> BoxFuture<Result<RunRecord, StoreError>>;

    /// Insert or replace one mark by its identity.
    ///
    /// A duplicate identity keeps the stored mark id and creation time; an
    /// exact retry therefore leaves the row unchanged. A decided run refuses
    /// with [`StoreError::Locked`].
    fn put_mark(
        &self,
        profile: ProfileId,
        sim_id: String,
        mark: MarkRecord,
    ) -> BoxFuture<Result<MarkRecord, StoreError>>;

    /// Delete one mark; `false` means it was absent.
    fn delete_mark(
        &self,
        profile: ProfileId,
        sim_id: String,
        mark_id: String,
    ) -> BoxFuture<Result<bool, StoreError>>;

    /// Read this profile's marks, optionally only one origin.
    fn list_marks(
        &self,
        profile: ProfileId,
        sim_id: String,
        origin: Option<MarkOrigin>,
    ) -> BoxFuture<Result<Vec<MarkRecord>, StoreError>>;

    /// Replace every oracle mark and advance to `reviewed`.
    fn replace_oracle(
        &self,
        profile: ProfileId,
        sim_id: String,
        marks: Vec<MarkRecord>,
    ) -> BoxFuture<Result<Vec<MarkRecord>, StoreError>>;

    /// Record the check and advance to `checked`.
    fn record_check(
        &self,
        profile: ProfileId,
        sim_id: String,
        check: CheckRecord,
    ) -> BoxFuture<Result<(), StoreError>>;

    /// Record the terminal decision and lock marking.
    ///
    /// Exactly one decision wins. A later decision returns
    /// [`StoreError::Locked`], even when it carries the same body, so a client
    /// reads the stored run rather than treating a duplicate as a new success.
    fn decide(
        &self,
        profile: ProfileId,
        sim_id: String,
        decision: DecisionRecord,
    ) -> BoxFuture<Result<DecisionRecord, StoreError>>;
}
