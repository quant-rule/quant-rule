//! The diagnostic stage lifecycle seam.
//!
//! The backtest stage worker reaches the evidence stage row only through this
//! module. The concrete transport is the parent's `HttpServiceClient`; the
//! in-memory fake mirrors the same state machine so a worker test needs no
//! database or socket.
//!
//! A stage has no canonical digest and no run id. The trait carries a
//! [`StageArtifact`] and a stage record, never a reading, a candle, or a client
//! path.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use quantrule_api::{Digest, ProfileId, StageArtifact, StageRunId, StageState};
use quantrule_service_api::{
    NewStageRun, StageClaim, StageClaimOutcome, StageComputed, StageFailed, StageRecord,
};

use crate::adapters::BoxFuture;
use crate::client::{ClientError, HttpServiceClient, Service};

/// The evidence stage lifecycle the backtest worker calls.
pub trait StageClient: Send + Sync {
    /// Claim one delivery.
    fn claim_stage(
        &self,
        stage_run_id: String,
        redelivered: bool,
    ) -> BoxFuture<Result<StageClaimOutcome, ClientError>>;

    /// Read a stage without a profile, for the worker's cancellation check.
    fn get_stage(
        &self,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageRecord>, ClientError>>;

    /// Record the content-addressed artifact and finish the stage.
    fn stage_computed(
        &self,
        stage_run_id: String,
        artifact: StageArtifact,
    ) -> BoxFuture<Result<StageRecord, ClientError>>;

    /// Record a terminal failure.
    fn stage_failed(
        &self,
        stage_run_id: String,
        error: String,
    ) -> BoxFuture<Result<StageRecord, ClientError>>;
}

fn stage_id(stage_run_id: &str) -> Result<StageRunId, ClientError> {
    Digest::parse(stage_run_id)
        .map(StageRunId)
        .map_err(|error| ClientError::InvalidResponse {
            service: Service::Evidence,
            message: format!("the stage run id is invalid: {error}"),
        })
}

impl StageClient for HttpServiceClient {
    fn claim_stage(
        &self,
        stage_run_id: String,
        redelivered: bool,
    ) -> BoxFuture<Result<StageClaimOutcome, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            let request = StageClaim {
                stage_run_id: stage_id(&stage_run_id)?,
                redelivered,
            };
            client
                .post_json(
                    Service::Evidence,
                    format!("/internal/v1/stages/{stage_run_id}/claim"),
                    &request,
                )
                .await
        })
    }

    fn get_stage(
        &self,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageRecord>, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            client
                .get_json(
                    Service::Evidence,
                    None,
                    format!("/internal/v1/stages/{stage_run_id}"),
                )
                .await
        })
    }

    fn stage_computed(
        &self,
        stage_run_id: String,
        artifact: StageArtifact,
    ) -> BoxFuture<Result<StageRecord, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            let computed = StageComputed {
                stage_run_id: stage_id(&stage_run_id)?,
                artifact,
            };
            client
                .post_json(
                    Service::Evidence,
                    format!("/internal/v1/stages/{stage_run_id}/computed"),
                    &computed,
                )
                .await
        })
    }

    fn stage_failed(
        &self,
        stage_run_id: String,
        error: String,
    ) -> BoxFuture<Result<StageRecord, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            let failed = StageFailed {
                stage_run_id: stage_id(&stage_run_id)?,
                error,
            };
            client
                .post_json(
                    Service::Evidence,
                    format!("/internal/v1/stages/{stage_run_id}/failed"),
                    &failed,
                )
                .await
        })
    }
}

type StageStateMap = (BTreeMap<String, StageRecord>, BTreeSet<(String, String)>);

/// An in-memory stage lifecycle for worker tests.
///
/// It mirrors the evidence store: a claim starts a queued stage, a redelivery
/// reclaims an uncomputed running stage up to the shared retry budget, and only
/// a running stage can succeed or fail. A cancelled stage is terminal.
#[derive(Debug, Clone, Default)]
pub struct MemoryStageClient {
    inner: Arc<Mutex<StageStateMap>>,
}

fn lock(
    inner: &Arc<Mutex<StageStateMap>>,
) -> Result<std::sync::MutexGuard<'_, StageStateMap>, ClientError> {
    inner.lock().map_err(|_| ClientError::InvalidResponse {
        service: Service::Evidence,
        message: "the stage lock is poisoned".to_owned(),
    })
}

impl MemoryStageClient {
    /// Submit one stage for a profile, as evidence does after validation.
    pub fn submit(
        &self,
        profile: &ProfileId,
        run: NewStageRun,
    ) -> BoxFuture<Result<StageRecord, ClientError>> {
        let inner = Arc::clone(&self.inner);
        let profile = profile.as_str().to_owned();
        Box::pin(async move {
            let mut state = lock(&inner)?;
            let key = run.stage_run_id.to_string();
            let record = state
                .0
                .get(&key)
                .cloned()
                .unwrap_or_else(|| StageRecord::queued(&run));
            state.0.entry(key.clone()).or_insert_with(|| record.clone());
            state.1.insert((profile, key));
            Ok(record)
        })
    }

    /// Cancel a queued or running stage within a profile.
    pub fn cancel(
        &self,
        profile: &ProfileId,
        stage_run_id: String,
    ) -> BoxFuture<Result<bool, ClientError>> {
        let inner = Arc::clone(&self.inner);
        let profile = profile.as_str().to_owned();
        Box::pin(async move {
            let mut state = lock(&inner)?;
            if !state.1.contains(&(profile, stage_run_id.clone())) {
                return Ok(false);
            }
            let Some(record) = state.0.get_mut(&stage_run_id) else {
                return Ok(false);
            };
            if !record.state.is_cancellable() {
                return Ok(false);
            }
            record.state = StageState::Cancelled;
            Ok(true)
        })
    }

    /// Read one stage without a profile.
    pub fn get_any(
        &self,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageRecord>, ClientError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move { Ok(lock(&inner)?.0.get(&stage_run_id).cloned()) })
    }
}

impl StageClient for MemoryStageClient {
    fn claim_stage(
        &self,
        stage_run_id: String,
        redelivered: bool,
    ) -> BoxFuture<Result<StageClaimOutcome, ClientError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = lock(&inner)?;
            let Some(record) = state.0.get_mut(&stage_run_id) else {
                return Ok(StageClaimOutcome::Missing);
            };
            if record.state == StageState::Queued {
                record.state = StageState::Running;
                record.attempts = 1;
                return Ok(StageClaimOutcome::Claimed(record.clone()));
            }
            if record.state != StageState::Running {
                return Ok(StageClaimOutcome::Terminal(record.clone()));
            }
            if !redelivered {
                return Ok(StageClaimOutcome::Running(record.clone()));
            }
            if record.attempts >= quantrule_service_api::MAX_ATTEMPTS {
                return Ok(StageClaimOutcome::Exhausted(record.clone()));
            }
            record.attempts += 1;
            Ok(StageClaimOutcome::Claimed(record.clone()))
        })
    }

    fn get_stage(
        &self,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageRecord>, ClientError>> {
        self.get_any(stage_run_id)
    }

    fn stage_computed(
        &self,
        stage_run_id: String,
        artifact: StageArtifact,
    ) -> BoxFuture<Result<StageRecord, ClientError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            if !artifact.is_consistent() {
                return Err(ClientError::InvalidResponse {
                    service: Service::Evidence,
                    message: "the stage artifact id does not match its files".to_owned(),
                });
            }
            let mut state = lock(&inner)?;
            let Some(record) = state.0.get_mut(&stage_run_id) else {
                return Err(ClientError::Status {
                    service: Service::Evidence,
                    status: 404,
                    message: format!("stage {stage_run_id} is unknown"),
                });
            };
            if record.state == StageState::Running {
                record.state = StageState::Succeeded;
                record.artifact = Some(artifact);
                record.error = None;
            }
            Ok(record.clone())
        })
    }

    fn stage_failed(
        &self,
        stage_run_id: String,
        error: String,
    ) -> BoxFuture<Result<StageRecord, ClientError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = lock(&inner)?;
            let Some(record) = state.0.get_mut(&stage_run_id) else {
                return Err(ClientError::Status {
                    service: Service::Evidence,
                    status: 404,
                    message: format!("stage {stage_run_id} is unknown"),
                });
            };
            if record.state == StageState::Running {
                record.state = StageState::Failed;
                record.error = Some(error);
            }
            Ok(record.clone())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::{ArtifactRef, StageInput, StageKind, StageProvenance};

    fn profile() -> ProfileId {
        ProfileId::parse("default").expect("profile")
    }

    fn run() -> NewStageRun {
        let data_version = Digest::sha256(b"data");
        NewStageRun::new(
            StageKind::Model,
            "cfg-1".to_owned(),
            StageInput::DataVersion { data_version },
            StageProvenance {
                data_version: Digest::sha256(b"data"),
                config_hash: Digest::sha256(b"config"),
                model_version: "M1.0".to_owned(),
                image_digest: Digest::sha256(b"image"),
            },
            "default".to_owned(),
            "2026-11-17T00:00:00Z".to_owned(),
        )
    }

    fn artifact() -> StageArtifact {
        StageArtifact::from_files(vec![ArtifactRef {
            name: "summary.txt".to_owned(),
            bytes: 2,
            sha256: Digest::sha256(b"ok"),
        }])
        .expect("artifact")
    }

    #[tokio::test]
    async fn the_fake_lifecycle_matches_the_store() {
        let client = MemoryStageClient::default();
        let run = run();
        let id = run.stage_run_id.to_string();
        let record = client.submit(&profile(), run).await.expect("submit");
        assert_eq!(record.state, StageState::Queued);
        let StageClaimOutcome::Claimed(record) =
            client.claim_stage(id.clone(), false).await.expect("claim")
        else {
            panic!("the queued stage was not claimed");
        };
        assert_eq!(record.state, StageState::Running);
        let record = client
            .stage_computed(id.clone(), artifact())
            .await
            .expect("computed");
        assert_eq!(record.state, StageState::Succeeded);
        assert_eq!(record.artifact, Some(artifact()));
        assert!(matches!(
            client.claim_stage(id, false).await.expect("terminal"),
            StageClaimOutcome::Terminal(_)
        ));
    }

    #[tokio::test]
    async fn a_cancel_is_profile_scoped_and_terminal() {
        let client = MemoryStageClient::default();
        let run = run();
        let id = run.stage_run_id.to_string();
        client.submit(&profile(), run).await.expect("submit");
        assert!(
            !client
                .cancel(&profile(), "missing".to_owned())
                .await
                .expect("missing")
        );
        assert!(client.cancel(&profile(), id.clone()).await.expect("cancel"));
        assert!(!client.cancel(&profile(), id.clone()).await.expect("again"));
        let record = client.get_stage(id).await.expect("get").expect("record");
        assert_eq!(record.state, StageState::Cancelled);
    }
}
