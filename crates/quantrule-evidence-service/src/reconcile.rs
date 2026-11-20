//! Dataset-reference reconciliation.
//!
//! A run or research record registers its dataset reference in the same use
//! case that writes the record. Rows written before the registry existed have
//! no reference, so the service reconciles them once at startup. Registration is
//! idempotent, so a restart or a repeated sweep cannot double-count.
//!
//! The sweep reads only evidence-owned rows and calls the data service's
//! internal endpoint. It never reads the catalog schema.

use std::sync::Arc;

use quantrule_api::DatasetReferenceKind;
use quantrule_runtime::client::DataClient;

use crate::research::ImprovementStore;
use crate::runs::RunStore;

/// Register every current run and research dataset reference.
///
/// # Errors
///
/// Returns a human-readable store or transport failure. A later sweep retries;
/// a missing reference blocks a dataset erase, which is the safe direction.
pub async fn register_all(
    runs: Arc<dyn RunStore>,
    improvements: Arc<dyn ImprovementStore>,
    data: Arc<dyn DataClient>,
) -> Result<usize, String> {
    let mut registered = 0;
    for row in runs
        .submitted_datasets()
        .await
        .map_err(|error| format!("the run reference scan failed: {error}"))?
    {
        data.register_dataset_reference(
            row.profile,
            row.data_version,
            format!("run:{}", row.run_id),
            DatasetReferenceKind::Run,
        )
        .await
        .map_err(|error| format!("the run reference registration failed: {error}"))?;
        registered += 1;
    }
    for row in improvements
        .submitted_datasets()
        .await
        .map_err(|error| format!("the research reference scan failed: {error}"))?
    {
        data.register_dataset_reference(
            row.profile,
            row.data_version,
            format!("research:{}", row.improvement_id),
            DatasetReferenceKind::Research,
        )
        .await
        .map_err(|error| format!("the research reference registration failed: {error}"))?;
        registered += 1;
    }
    Ok(registered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::{Digest, ProfileId, RunId};
    use quantrule_runtime::client::MemoryDataClient;
    use quantrule_service_api::NewRun;

    use crate::research::MemoryImprovementStore;
    use crate::runs::MemoryRuns;

    fn profile(name: &str) -> ProfileId {
        ProfileId::parse(name).expect("profile")
    }

    #[tokio::test]
    async fn every_current_run_is_registered_once() {
        let runs = Arc::new(MemoryRuns::default());
        let data = Arc::new(MemoryDataClient::default());
        let owner = profile("default");
        let data_version = Digest::sha256(b"reconcile-dataset");
        let submitted = runs
            .submit(
                owner.clone(),
                NewRun {
                    run_id: RunId(Digest::sha256(b"reconcile-run")),
                    config_id: "cfg".to_owned(),
                    data_version: data_version.clone(),
                    config_hash: Digest::sha256(b"config"),
                    model_version: "M1.0".to_owned(),
                    image_digest: Digest::sha256(b"image"),
                    model_artifact_id: None,
                    owner: "default".to_owned(),
                    submitted_at: "2026-11-20T00:00:00Z".to_owned(),
                },
            )
            .await
            .expect("submit");
        let run_id = submitted.record().run_id.clone();
        let improvements: Arc<dyn ImprovementStore> = Arc::new(MemoryImprovementStore::default());
        let registered = register_all(
            Arc::clone(&runs) as Arc<dyn RunStore>,
            improvements,
            Arc::clone(&data) as Arc<dyn DataClient>,
        )
        .await
        .expect("reconcile");
        assert_eq!(registered, 1);
        assert_eq!(
            data.reference_ids(&owner, &data_version.to_string()),
            vec![format!("run:{run_id}")]
        );

        // A second sweep is idempotent.
        let registered = register_all(
            runs as Arc<dyn RunStore>,
            Arc::new(MemoryImprovementStore::default()),
            data as Arc<dyn DataClient>,
        )
        .await
        .expect("reconcile again");
        assert_eq!(registered, 1);
    }
}
