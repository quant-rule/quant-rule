//! Verify a computed run and finish it.

use std::sync::Arc;

use quantrule_api::{RunState, StageName, StageStatus};

use crate::evidence::{EvidenceError, artifacts};
use crate::runs::RunStore;
use crate::runtime::adapters::ObjectStore;

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

/// Verify one computed run and mark it succeeded.
///
/// # Errors
///
/// Returns [`EvidenceError::Store`] when a store is unavailable. A verification
/// failure is recorded on the run and returned as [`FinalizeOutcome::Failed`].
pub async fn finalize(
    run_id: String,
    services: &EvidenceServices,
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
        return fail(services, &run_id, "the run has no artifacts", &record).await;
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
                &format!("the {} stage did not finish", stage.name.as_str()),
                &record,
            )
            .await;
        }
    }
    let digest = match artifacts::verify(&services.objects, &run_id, &record.artifacts).await {
        Ok(digest) => digest,
        Err(error) => return fail(services, &run_id, &error.to_string(), &record).await,
    };
    if record.digest.as_ref() != Some(&digest) {
        return fail(
            services,
            &run_id,
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
        .complete(run_id, digest, record.artifacts, stages)
        .await
        .map_err(|error| EvidenceError::Store(error.to_string()))?;
    if finished.status == RunState::Succeeded {
        Ok(FinalizeOutcome::Succeeded)
    } else {
        Ok(FinalizeOutcome::Skipped)
    }
}

async fn fail(
    services: &EvidenceServices,
    run_id: &str,
    message: &str,
    record: &crate::runs::RunRecord,
) -> Result<FinalizeOutcome, EvidenceError> {
    let mut stages = record.stages.clone();
    for stage in &mut stages {
        if stage.name == StageName::Evidence {
            stage.state = StageStatus::Failed;
        }
    }
    services
        .runs
        .fail(run_id.to_owned(), message.to_owned(), stages)
        .await
        .map_err(|error| EvidenceError::Store(error.to_string()))?;
    Ok(FinalizeOutcome::Failed)
}
