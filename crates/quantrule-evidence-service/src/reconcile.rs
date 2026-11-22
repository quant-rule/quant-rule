//! Dataset-reference reconciliation.
//!
//! A run or research record registers its dataset reference in the same use
//! case that writes the record. Rows written before the registry existed have
//! no reference, so the service reconciles them once at startup. Registration is
//! idempotent, so a restart or a repeated sweep cannot double-count.
//!
//! The other direction is the release: the consumer delete commits with a
//! tombstone in the evidence database, then the data-service call is attempted.
//! A failed or interrupted call is retried from the tombstone, so an erase
//! cannot leave a reference that blocks the dataset forever.
//!
//! The sweep reads only evidence-owned rows and calls the data service's
//! internal endpoint. It never reads the catalog schema.

use std::sync::Arc;

use quantrule_api::DatasetReferenceKind;
use quantrule_runtime::client::{ClientError, DataClient};

use crate::releases::ReferenceReleaseLog;
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

/// Retry every dataset-reference release the evidence tombstone still holds.
///
/// A `404` means the dataset row is already gone, so nothing can be blocked;
/// the tombstone is confirmed. Any other failure leaves the tombstone pending
/// and stops the pass, because the data service is the shared dependency.
///
/// # Errors
///
/// Returns a human-readable store or transport failure. A later sweep retries.
pub async fn release_all(
    releases: &dyn ReferenceReleaseLog,
    data: &dyn DataClient,
) -> Result<usize, String> {
    let mut released = 0;
    for release in releases
        .pending_releases()
        .await
        .map_err(|error| format!("the release scan failed: {error}"))?
    {
        match data
            .release_dataset_reference(
                release.profile.clone(),
                release.data_version.clone(),
                release.reference_id.clone(),
            )
            .await
        {
            Ok(_) => {}
            Err(error) if dataset_gone(&error) => {}
            Err(error) => {
                let message = error.to_string();
                releases
                    .recorded(&release, Some(&message))
                    .await
                    .map_err(|error| format!("the release record failed: {error}"))?;
                return Err(format!(
                    "the dataset reference {} was not released: {message}",
                    release.reference_id
                ));
            }
        }
        releases
            .recorded(&release, None)
            .await
            .map_err(|error| format!("the release record failed: {error}"))?;
        released += 1;
    }
    Ok(released)
}

/// Whether the dataset is already gone, which means no release is needed.
///
/// The data service answers `404` when the catalog row no longer exists. That
/// is the state a release wants, so it confirms the tombstone; treating it as
/// a retry would keep a dead reference pending forever.
fn dataset_gone(error: &ClientError) -> bool {
    matches!(error, ClientError::Status { status: 404, .. })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::{Digest, ProfileId, RunId};
    use quantrule_runtime::client::{MemoryDataClient, Service};
    use quantrule_service_api::NewRun;

    use crate::research::MemoryImprovementStore;
    use crate::runs::{DatasetReferenceRelease, MemoryRuns};

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

    #[tokio::test]
    async fn an_erased_records_tombstone_releases_its_dataset_reference() {
        let owner = profile("default");
        let data_version = Digest::sha256(b"release-dataset");
        let runs = MemoryRuns::default();
        let data = MemoryDataClient::default();
        data.register_dataset_reference(
            owner.clone(),
            data_version.clone(),
            "run:release".to_owned(),
            DatasetReferenceKind::Run,
        )
        .await
        .expect("reference");
        runs.record(DatasetReferenceRelease {
            profile: owner.clone(),
            data_version: data_version.clone(),
            reference_id: "run:release".to_owned(),
            kind: DatasetReferenceKind::Run,
        })
        .await
        .expect("tombstone");

        let released = release_all(&runs, &data).await.expect("release");
        assert_eq!(released, 1);
        assert!(runs.pending_releases().await.expect("releases").is_empty());
        assert!(
            data.reference_ids(&owner, &data_version.to_string())
                .is_empty(),
            "the tombstone did not release the reference"
        );

        // A second pass is quiet.
        assert_eq!(release_all(&runs, &data).await.expect("second"), 0);
    }

    #[test]
    fn a_missing_dataset_confirms_the_release() {
        let missing = ClientError::Status {
            service: Service::Data,
            status: 404,
            message: "dataset is unknown".to_owned(),
        };
        assert!(dataset_gone(&missing));
        let unavailable = ClientError::Status {
            service: Service::Data,
            status: 503,
            message: "try later".to_owned(),
        };
        assert!(!dataset_gone(&unavailable));
    }
}
