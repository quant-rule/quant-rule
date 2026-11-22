//! Fenced finalisation for the memory run store.
//!
//! The leader row is locked before the run state, the same order the Postgres
//! store takes. Acquisition takes only the leader lock, so a stale leader
//! cannot finish a run after a new term owns the row.

use std::time::SystemTime;

use quantrule_api::{ArtifactRef, Digest, RunState, StageProgress};

use super::{MemoryRuns, RunRecord, RunStoreError};

impl MemoryRuns {
    /// Finish a computed run under a live leader term.
    pub(crate) fn finalize_succeeded(
        &self,
        run_id: String,
        term: u64,
        leader_term: u64,
        digest: Digest,
        artifacts: Vec<ArtifactRef>,
        stages: Vec<StageProgress>,
    ) -> Result<RunRecord, RunStoreError> {
        // The leader row is locked first, then the run state. Acquisition takes
        // only the leader lock, so this order cannot deadlock and the fence and
        // the write are one critical section.
        let leader_row = self.leader.lock_row();
        let mut state = self
            .inner
            .lock()
            .map_err(|_| RunStoreError("the run lock is poisoned".to_owned()))?;
        let Some(record) = state.runs.get_mut(&run_id) else {
            return Err(RunStoreError(format!("run {run_id} is unknown")));
        };
        // Only a running run can finish, and only the attempt that holds the
        // run term and the live leader may finish it. The Postgres store
        // enforces all three with its predicates; the memory store agrees.
        if record.status != RunState::Running || record.lease_term != term {
            return Ok(record.clone());
        }
        if leader_row.live_term(SystemTime::now()) != Some(leader_term) {
            return Ok(record.clone());
        }
        record.status = RunState::Succeeded;
        record.digest = Some(digest);
        record.stages = stages;
        record.artifacts = artifacts;
        record.error = None;
        record.lease_holder = None;
        record.lease_expires_at = None;
        let record = record.clone();
        state.leases.remove(&run_id);
        state.clear_notifications(&run_id);
        Ok(record)
    }

    /// Fail a computed run under a live leader term.
    pub(crate) fn finalize_failed(
        &self,
        run_id: String,
        term: u64,
        leader_term: u64,
        error: String,
        stages: Vec<StageProgress>,
    ) -> Result<RunRecord, RunStoreError> {
        let leader_row = self.leader.lock_row();
        let mut state = self
            .inner
            .lock()
            .map_err(|_| RunStoreError("the run lock is poisoned".to_owned()))?;
        let Some(record) = state.runs.get_mut(&run_id) else {
            return Err(RunStoreError(format!("run {run_id} is unknown")));
        };
        if record.status != RunState::Running || record.lease_term != term {
            return Ok(record.clone());
        }
        if leader_row.live_term(SystemTime::now()) != Some(leader_term) {
            return Ok(record.clone());
        }
        record.status = RunState::Failed;
        record.error = Some(error);
        record.stages = stages;
        record.lease_holder = None;
        record.lease_expires_at = None;
        let record = record.clone();
        state.leases.remove(&run_id);
        state.clear_notifications(&run_id);
        Ok(record)
    }
}
