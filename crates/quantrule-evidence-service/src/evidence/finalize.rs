//! Verify a computed run and finish it.

use std::sync::Arc;

use quantrule_api::{RunState, StageName, StageStatus};

use crate::evidence::{EvidenceError, artifacts};
use crate::runs::RunStore;
use quantrule_runtime::adapters::ObjectStore;

/// The stores evidence needs.
pub struct EvidenceServices {
    /// The run store.
    pub runs: Arc<dyn RunStore>,
    /// The object store.
    pub objects: Arc<dyn ObjectStore>,
}

/// What one evidence delivery did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalizeOutcome {
    /// The run finished successfully.
    Succeeded,
    /// Evidence rejected the run; its failure is recorded.
    Failed,
    /// There was nothing to finalize.
    Skipped,
}

/// Verify one computed run and mark it succeeded under a leader term.
///
/// `leader_term` is the evidence leader lease term the caller holds. The store
/// refuses the terminal write when that term is stale, so only one replica can
/// finalise the run.
///
/// # Errors
///
/// Returns [`EvidenceError::Store`] when a store is unavailable. A verification
/// failure is recorded on the run and returned as [`FinalizeOutcome::Failed`].
/// Returns [`EvidenceError::LeaderFenced`] when the leader term is no longer
/// live; the delivery must be retried by the next leader.
pub async fn finalize(
    run_id: String,
    services: &EvidenceServices,
    leader_term: u64,
) -> Result<FinalizeOutcome, EvidenceError> {
    let Some(record) = services
        .runs
        .get_any(run_id.clone())
        .await
        .map_err(|error| EvidenceError::Store(error.to_string()))?
    else {
        return Ok(FinalizeOutcome::Skipped);
    };
    if record.status != RunState::Running {
        return Ok(FinalizeOutcome::Skipped);
    }
    if record.artifacts.is_empty() {
        return fail(
            services,
            &run_id,
            leader_term,
            "the run has no artifacts",
            &record,
        )
        .await;
    }
    for stage in &record.stages {
        if matches!(
            stage.name,
            StageName::Data | StageName::Model | StageName::Backtest
        ) && stage.state != StageStatus::Done
        {
            return fail(
                services,
                &run_id,
                leader_term,
                &format!("the {} stage did not finish", stage.name.as_str()),
                &record,
            )
            .await;
        }
    }
    let digest = match artifacts::verify(&services.objects, &run_id, &record.artifacts).await {
        Ok(digest) => digest,
        Err(error) => {
            return fail(services, &run_id, leader_term, &error.to_string(), &record).await;
        }
    };
    if record.digest.as_ref() != Some(&digest) {
        return fail(
            services,
            &run_id,
            leader_term,
            "the stored digest does not match the artifacts",
            &record,
        )
        .await;
    }
    let mut stages = record.stages.clone();
    for stage in &mut stages {
        if stage.name == StageName::Evidence {
            stage.state = StageStatus::Done;
        }
    }
    let finished = services
        .runs
        .complete(
            run_id,
            record.lease_term,
            leader_term,
            digest,
            record.artifacts,
            stages,
        )
        .await
        .map_err(|error| EvidenceError::Store(error.to_string()))?;
    if finished.status == RunState::Succeeded {
        Ok(FinalizeOutcome::Succeeded)
    } else if finished.status == RunState::Running && finished.lease_term == record.lease_term {
        // The run attempt is still the same one, so the refusal came from the
        // leader fence, not from a newer worker attempt.
        Err(EvidenceError::LeaderFenced)
    } else {
        Ok(FinalizeOutcome::Skipped)
    }
}

async fn fail(
    services: &EvidenceServices,
    run_id: &str,
    leader_term: u64,
    message: &str,
    record: &crate::runs::RunRecord,
) -> Result<FinalizeOutcome, EvidenceError> {
    let mut stages = record.stages.clone();
    for stage in &mut stages {
        if stage.name == StageName::Evidence {
            stage.state = StageStatus::Failed;
        }
    }
    let finished = services
        .runs
        .fail_finalization(
            run_id.to_owned(),
            record.lease_term,
            leader_term,
            message.to_owned(),
            stages,
        )
        .await
        .map_err(|error| EvidenceError::Store(error.to_string()))?;
    if finished.status == RunState::Failed {
        Ok(FinalizeOutcome::Failed)
    } else if finished.status == RunState::Running && finished.lease_term == record.lease_term {
        Err(EvidenceError::LeaderFenced)
    } else {
        Ok(FinalizeOutcome::Skipped)
    }
}
