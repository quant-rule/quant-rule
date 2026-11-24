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

use quantrule_api::{Digest, ProfileId, StageArtifact, StageKind, StageRunId, StageState};
use quantrule_service_api::{
    NewStageRun, StageClaim, StageClaimOutcome, StageComputed, StageFailed, StageHeartbeat,
    StageHeartbeatOutcome, StageRecord,
};

use crate::adapters::BoxFuture;
use crate::client::{ClientError, HttpServiceClient, Service};

/// The evidence stage lifecycle the backtest worker calls.
pub trait StageClient: Send + Sync {
    /// Claim one delivery.
    fn claim_stage(
        &self,
        stage_run_id: String,
        holder: String,
        continuation: Option<u64>,
        redelivered: bool,
    ) -> BoxFuture<Result<StageClaimOutcome, ClientError>>;

    /// Renew the current stage lease.
    fn heartbeat_stage(
        &self,
        stage_run_id: String,
        holder: String,
        term: u64,
    ) -> BoxFuture<Result<StageHeartbeatOutcome, ClientError>>;

    /// Read a stage without a profile, for the worker's cancellation check.
    fn get_stage(
        &self,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageRecord>, ClientError>>;

    /// Read the succeeded stage of `kind` that a profile's artifact names.
    fn get_stage_by_artifact(
        &self,
        profile: ProfileId,
        kind: StageKind,
        artifact_id: Digest,
    ) -> BoxFuture<Result<Option<StageRecord>, ClientError>>;

    /// Record the content-addressed artifact and finish the stage.
    fn stage_computed(
        &self,
        stage_run_id: String,
        lease_term: u64,
        artifact: StageArtifact,
    ) -> BoxFuture<Result<StageRecord, ClientError>>;

    /// Record a terminal failure.
    fn stage_failed(
        &self,
        stage_run_id: String,
        lease_term: u64,
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
        holder: String,
        continuation: Option<u64>,
        redelivered: bool,
    ) -> BoxFuture<Result<StageClaimOutcome, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            let request = StageClaim {
                stage_run_id: stage_id(&stage_run_id)?,
                redelivered,
                holder,
                continuation,
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

    fn heartbeat_stage(
        &self,
        stage_run_id: String,
        holder: String,
        term: u64,
    ) -> BoxFuture<Result<StageHeartbeatOutcome, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            let request = StageHeartbeat {
                stage_run_id: stage_id(&stage_run_id)?,
                holder,
                term,
            };
            client
                .post_json(
                    Service::Evidence,
                    format!("/internal/v1/stages/{stage_run_id}/heartbeat"),
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

    fn get_stage_by_artifact(
        &self,
        profile: ProfileId,
        kind: StageKind,
        artifact_id: Digest,
    ) -> BoxFuture<Result<Option<StageRecord>, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            client
                .get_json(
                    Service::Evidence,
                    Some(&profile),
                    format!(
                        "/internal/v1/stages/by-artifact/{}/{}",
                        kind.as_str(),
                        artifact_id
                    ),
                )
                .await
        })
    }

    fn stage_computed(
        &self,
        stage_run_id: String,
        lease_term: u64,
        artifact: StageArtifact,
    ) -> BoxFuture<Result<StageRecord, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            let computed = StageComputed {
                stage_run_id: stage_id(&stage_run_id)?,
                lease_term,
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
        lease_term: u64,
        error: String,
    ) -> BoxFuture<Result<StageRecord, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            let failed = StageFailed {
                stage_run_id: stage_id(&stage_run_id)?,
                lease_term,
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

type StageStateMap = (
    BTreeMap<String, StageRecord>,
    BTreeSet<(String, String)>,
    BTreeMap<String, (String, u64)>,
);

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
            state.2.remove(&stage_run_id);
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
        holder: String,
        continuation: Option<u64>,
        _redelivered: bool,
    ) -> BoxFuture<Result<StageClaimOutcome, ClientError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = lock(&inner)?;
            let Some(record) = state.0.get(&stage_run_id) else {
                return Ok(StageClaimOutcome::Missing);
            };
            if record.state == StageState::Queued {
                let record =
                    state
                        .0
                        .get_mut(&stage_run_id)
                        .ok_or_else(|| ClientError::InvalidResponse {
                            service: Service::Evidence,
                            message: "the stage vanished".to_owned(),
                        })?;
                record.state = StageState::Running;
                record.attempts = 1;
                record.lease_term = record.lease_term.saturating_add(1);
                let term = record.lease_term;
                let record = record.clone();
                state.2.insert(stage_run_id, (holder, term));
                return Ok(StageClaimOutcome::Claimed(record));
            }
            if record.state != StageState::Running {
                return Ok(StageClaimOutcome::Terminal(record.clone()));
            }
            let own_continuation = continuation.is_some_and(|term| {
                state
                    .2
                    .get(&stage_run_id)
                    .is_some_and(|(current, current_term)| {
                        *current_term == term && current == &holder
                    })
            });
            if !own_continuation {
                return Ok(StageClaimOutcome::Running(record.clone()));
            }
            if record.attempts >= quantrule_service_api::MAX_ATTEMPTS {
                return Ok(StageClaimOutcome::Exhausted(record.clone()));
            }
            let record =
                state
                    .0
                    .get_mut(&stage_run_id)
                    .ok_or_else(|| ClientError::InvalidResponse {
                        service: Service::Evidence,
                        message: "the stage vanished".to_owned(),
                    })?;
            record.attempts = record.attempts.saturating_add(1);
            record.lease_term = record.lease_term.saturating_add(1);
            let term = record.lease_term;
            let record = record.clone();
            state.2.insert(stage_run_id, (holder, term));
            Ok(StageClaimOutcome::Claimed(record))
        })
    }

    fn heartbeat_stage(
        &self,
        stage_run_id: String,
        holder: String,
        term: u64,
    ) -> BoxFuture<Result<StageHeartbeatOutcome, ClientError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = lock(&inner)?;
            let Some(record) = state.0.get(&stage_run_id) else {
                return Ok(StageHeartbeatOutcome::Missing);
            };
            let live = record.state == StageState::Running
                && state
                    .2
                    .get(&stage_run_id)
                    .is_some_and(|(current, current_term)| {
                        current == &holder && *current_term == term
                    });
            if live {
                Ok(StageHeartbeatOutcome::Renewed(Box::new(record.clone())))
            } else {
                Ok(StageHeartbeatOutcome::Stale)
            }
        })
    }

    fn get_stage(
        &self,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageRecord>, ClientError>> {
        self.get_any(stage_run_id)
    }

    fn get_stage_by_artifact(
        &self,
        profile: ProfileId,
        kind: StageKind,
        artifact_id: Digest,
    ) -> BoxFuture<Result<Option<StageRecord>, ClientError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = lock(&inner)?;
            Ok(state
                .0
                .values()
                .find(|record| {
                    record.state == StageState::Succeeded
                        && record.kind == kind
                        && record
                            .artifact
                            .as_ref()
                            .is_some_and(|artifact| artifact.artifact_id == artifact_id)
                        && state.1.contains(&(
                            profile.as_str().to_owned(),
                            record.stage_run_id.to_string(),
                        ))
                })
                .cloned())
        })
    }

    fn stage_computed(
        &self,
        stage_run_id: String,
        lease_term: u64,
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
            let Some(current) = state.0.get(&stage_run_id) else {
                return Err(ClientError::Status {
                    service: Service::Evidence,
                    status: 404,
                    message: format!("stage {stage_run_id} is unknown"),
                });
            };
            if current.state == StageState::Running && current.lease_term == lease_term {
                let record =
                    state
                        .0
                        .get_mut(&stage_run_id)
                        .ok_or_else(|| ClientError::InvalidResponse {
                            service: Service::Evidence,
                            message: "the stage vanished".to_owned(),
                        })?;
                record.state = StageState::Succeeded;
                record.artifact = Some(artifact);
                record.error = None;
                state.2.remove(&stage_run_id);
            }
            state
                .0
                .get(&stage_run_id)
                .cloned()
                .ok_or_else(|| ClientError::InvalidResponse {
                    service: Service::Evidence,
                    message: "the stage vanished".to_owned(),
                })
        })
    }

    fn stage_failed(
        &self,
        stage_run_id: String,
        lease_term: u64,
        error: String,
    ) -> BoxFuture<Result<StageRecord, ClientError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = lock(&inner)?;
            let Some(current) = state.0.get(&stage_run_id) else {
                return Err(ClientError::Status {
                    service: Service::Evidence,
                    status: 404,
                    message: format!("stage {stage_run_id} is unknown"),
                });
            };
            if current.state == StageState::Running && current.lease_term == lease_term {
                let record =
                    state
                        .0
                        .get_mut(&stage_run_id)
                        .ok_or_else(|| ClientError::InvalidResponse {
                            service: Service::Evidence,
                            message: "the stage vanished".to_owned(),
                        })?;
                record.state = StageState::Failed;
                record.error = Some(error);
                state.2.remove(&stage_run_id);
            }
            state
                .0
                .get(&stage_run_id)
                .cloned()
                .ok_or_else(|| ClientError::InvalidResponse {
                    service: Service::Evidence,
                    message: "the stage vanished".to_owned(),
                })
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
        let StageClaimOutcome::Claimed(record) = client
            .claim_stage(id.clone(), "worker".to_owned(), None, false)
            .await
            .expect("claim")
        else {
            panic!("the queued stage was not claimed");
        };
        assert_eq!(record.state, StageState::Running);
        let record = client
            .stage_computed(id.clone(), record.lease_term, artifact())
            .await
            .expect("computed");
        assert_eq!(record.state, StageState::Succeeded);
        assert_eq!(record.artifact, Some(artifact()));
        assert!(matches!(
            client
                .claim_stage(id, "worker".to_owned(), None, false)
                .await
                .expect("terminal"),
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
