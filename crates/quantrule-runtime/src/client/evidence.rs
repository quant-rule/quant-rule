//! The evidence client seam.
//!
//! The backtest worker reaches run records only through this module. The
//! concrete transport is the parent's `HttpServiceClient`; `MemoryEvidence` is
//! the fake a worker test uses without a database or a socket.

use std::collections::BTreeMap;

use quantrule_api::{ArtifactRef, Digest, ProfileId, RunId, RunState, StageProgress};
use quantrule_service_api::{
    RunCancelOutcome, RunClaimOutcome, RunClaimRequest, RunComputed, RunFailed, RunRecord,
};

use crate::adapters::BoxFuture;
use crate::client::{ClientError, HttpServiceClient, Service};

/// The evidence lifecycle the backtest worker calls.
///
/// The trait is the worker's only run-store seam: P5 moves the row to
/// `qr-evidence`, and a worker hop is a request rather than a local query. A
/// fake implements it so the worker is tested without a socket.
pub trait EvidenceClient: Send + Sync {
    /// Claim one delivery.
    fn claim(
        &self,
        run_id: String,
        redelivered: bool,
    ) -> BoxFuture<Result<RunClaimOutcome, ClientError>>;

    /// Read a run without a profile, for the worker's cancellation check.
    fn get_run(&self, run_id: String) -> BoxFuture<Result<Option<RunRecord>, ClientError>>;

    /// Record the computed artifact set and digest.
    fn computed(
        &self,
        run_id: String,
        digest: Digest,
        artifacts: Vec<ArtifactRef>,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, ClientError>>;

    /// Record a terminal failure.
    fn failed(
        &self,
        run_id: String,
        error: String,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, ClientError>>;
}

impl EvidenceClient for HttpServiceClient {
    fn claim(
        &self,
        run_id: String,
        redelivered: bool,
    ) -> BoxFuture<Result<RunClaimOutcome, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            let id =
                RunId(
                    Digest::parse(&run_id).map_err(|error| ClientError::InvalidResponse {
                        service: Service::Evidence,
                        message: format!("the run id is invalid: {error}"),
                    })?,
                );
            let request = RunClaimRequest {
                run_id: id,
                redelivered,
            };
            client
                .post_json(
                    Service::Evidence,
                    format!("/internal/v1/runs/{run_id}/claim"),
                    &request,
                )
                .await
        })
    }

    fn get_run(&self, run_id: String) -> BoxFuture<Result<Option<RunRecord>, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            client
                .get_json(
                    Service::Evidence,
                    None,
                    format!("/internal/v1/runs/{run_id}"),
                )
                .await
        })
    }

    fn computed(
        &self,
        run_id: String,
        digest: Digest,
        artifacts: Vec<ArtifactRef>,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            let id =
                RunId(
                    Digest::parse(&run_id).map_err(|error| ClientError::InvalidResponse {
                        service: Service::Evidence,
                        message: format!("the run id is invalid: {error}"),
                    })?,
                );
            let computed = RunComputed {
                run_id: id,
                digest,
                artifacts,
                stages,
            };
            client
                .post_json(
                    Service::Evidence,
                    format!("/internal/v1/runs/{run_id}/computed"),
                    &computed,
                )
                .await
        })
    }

    fn failed(
        &self,
        run_id: String,
        error: String,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            let id =
                RunId(
                    Digest::parse(&run_id).map_err(|inner| ClientError::InvalidResponse {
                        service: Service::Evidence,
                        message: format!("the run id is invalid: {inner}"),
                    })?,
                );
            let failed = RunFailed {
                run_id: id,
                error,
                stages,
            };
            client
                .post_json(
                    Service::Evidence,
                    format!("/internal/v1/runs/{run_id}/failed"),
                    &failed,
                )
                .await
        })
    }
}

type EvidenceState = (
    BTreeMap<String, RunRecord>,
    std::collections::BTreeSet<(String, String)>,
);

/// An in-memory evidence lifecycle for worker tests.
///
/// It mirrors the run state machine the evidence store enforces: a claim starts
/// a queued run, a redelivery reclaims a running run until the budget is spent,
/// and only a running run can be computed or failed. The fake exists so the
/// worker is tested without a database or a socket.
#[derive(Debug, Clone, Default)]
pub struct MemoryEvidence {
    inner: std::sync::Arc<std::sync::Mutex<EvidenceState>>,
}

impl MemoryEvidence {
    /// Submit one run for a profile, as evidence does after validation.
    pub fn submit(
        &self,
        profile: &ProfileId,
        run: quantrule_service_api::NewRun,
    ) -> BoxFuture<Result<RunRecord, ClientError>> {
        let inner = std::sync::Arc::clone(&self.inner);
        let profile = profile.as_str().to_owned();
        Box::pin(async move {
            let mut state = inner.lock().map_err(|_| ClientError::InvalidResponse {
                service: Service::Evidence,
                message: "the evidence lock is poisoned".to_owned(),
            })?;
            let key = run.run_id.to_string();
            let record = state
                .0
                .get(&key)
                .cloned()
                .unwrap_or_else(|| RunRecord::queued(&run));
            state.0.entry(key.clone()).or_insert_with(|| record.clone());
            state.1.insert((profile, key));
            Ok(record)
        })
    }

    /// Cancel a run within a profile.
    pub fn cancel(
        &self,
        profile: &ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<RunCancelOutcome, ClientError>> {
        let inner = std::sync::Arc::clone(&self.inner);
        let profile = profile.as_str().to_owned();
        Box::pin(async move {
            let mut state = inner.lock().map_err(|_| ClientError::InvalidResponse {
                service: Service::Evidence,
                message: "the evidence lock is poisoned".to_owned(),
            })?;
            if !state.1.contains(&(profile, run_id.clone())) {
                return Ok(RunCancelOutcome::Missing);
            }
            let Some(record) = state.0.get_mut(&run_id) else {
                return Ok(RunCancelOutcome::Missing);
            };
            if !record.status.is_cancellable() {
                return Ok(RunCancelOutcome::Conflict(record.clone()));
            }
            record.status = RunState::Cancelled;
            Ok(RunCancelOutcome::Cancelled(record.clone()))
        })
    }

    /// Read one run without a profile.
    pub fn get_any(&self, run_id: String) -> BoxFuture<Result<Option<RunRecord>, ClientError>> {
        let inner = std::sync::Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner.lock().map_err(|_| ClientError::InvalidResponse {
                service: Service::Evidence,
                message: "the evidence lock is poisoned".to_owned(),
            })?;
            Ok(state.0.get(&run_id).cloned())
        })
    }
}

impl EvidenceClient for MemoryEvidence {
    fn claim(
        &self,
        run_id: String,
        redelivered: bool,
    ) -> BoxFuture<Result<RunClaimOutcome, ClientError>> {
        let inner = std::sync::Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner.lock().map_err(|_| ClientError::InvalidResponse {
                service: Service::Evidence,
                message: "the evidence lock is poisoned".to_owned(),
            })?;
            let Some(record) = state.0.get_mut(&run_id) else {
                return Ok(RunClaimOutcome::Missing);
            };
            if record.status == RunState::Queued {
                record.status = RunState::Running;
                record.attempts = 1;
                return Ok(RunClaimOutcome::Claimed(record.clone()));
            }
            if record.status != RunState::Running {
                return Ok(RunClaimOutcome::Terminal(record.clone()));
            }
            if !redelivered || record.digest.is_some() {
                return Ok(RunClaimOutcome::Running(record.clone()));
            }
            if record.attempts >= quantrule_service_api::MAX_ATTEMPTS {
                return Ok(RunClaimOutcome::Exhausted(record.clone()));
            }
            record.attempts += 1;
            Ok(RunClaimOutcome::Claimed(record.clone()))
        })
    }

    fn get_run(&self, run_id: String) -> BoxFuture<Result<Option<RunRecord>, ClientError>> {
        self.get_any(run_id)
    }

    fn computed(
        &self,
        run_id: String,
        digest: Digest,
        artifacts: Vec<ArtifactRef>,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, ClientError>> {
        let inner = std::sync::Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner.lock().map_err(|_| ClientError::InvalidResponse {
                service: Service::Evidence,
                message: "the evidence lock is poisoned".to_owned(),
            })?;
            let Some(record) = state.0.get_mut(&run_id) else {
                return Err(ClientError::Status {
                    service: Service::Evidence,
                    status: 404,
                    message: format!("run {run_id} is unknown"),
                });
            };
            if record.status == RunState::Running {
                record.digest = Some(digest);
                record.stages = stages;
                record.artifacts = artifacts;
            }
            Ok(record.clone())
        })
    }

    fn failed(
        &self,
        run_id: String,
        error: String,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, ClientError>> {
        let inner = std::sync::Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner.lock().map_err(|_| ClientError::InvalidResponse {
                service: Service::Evidence,
                message: "the evidence lock is poisoned".to_owned(),
            })?;
            let Some(record) = state.0.get_mut(&run_id) else {
                return Err(ClientError::Status {
                    service: Service::Evidence,
                    status: 404,
                    message: format!("run {run_id} is unknown"),
                });
            };
            if record.status == RunState::Running {
                record.status = RunState::Failed;
                record.error = Some(error);
                record.stages = stages;
            }
            Ok(record.clone())
        })
    }
}
