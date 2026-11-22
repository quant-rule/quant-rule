//! The evidence client seam.
//!
//! The backtest worker reaches run records only through this module. The
//! concrete transport is the parent's `HttpServiceClient`; `MemoryEvidence` is
//! the fake a worker test uses without a database or a socket.

use std::collections::BTreeMap;

use quantrule_api::{ArtifactRef, Digest, ProfileId, RunId, RunState, StageProgress};
use quantrule_service_api::{
    RUN_LEASE_TTL_SECONDS, RunCancelOutcome, RunClaimOutcome, RunClaimRequest, RunComputed,
    RunFailed, RunHeartbeat, RunRecord,
};

use crate::adapters::BoxFuture;
use crate::client::{ClientError, HttpServiceClient, Service};

/// The evidence lifecycle the backtest worker calls.
///
/// The trait is the worker's only run-store seam: P5 moves the row to
/// `qr-evidence`, and a worker hop is a request rather than a local query. A
/// fake implements it so the worker is tested without a socket.
pub trait EvidenceClient: Send + Sync {
    /// Claim one delivery for a worker and record a new lease term.
    ///
    /// `continuation` is the term of the worker's own previous attempt when it
    /// is retrying; it lets the same holder continue a live lease without
    /// letting a different worker steal one.
    fn claim(
        &self,
        run_id: String,
        holder: String,
        continuation: Option<u64>,
    ) -> BoxFuture<Result<RunClaimOutcome, ClientError>>;

    /// Renew the lease for one attempt.
    fn heartbeat(
        &self,
        run_id: String,
        holder: String,
        term: u64,
    ) -> BoxFuture<Result<RunRecord, ClientError>>;

    /// Read a run without a profile, for the worker's cancellation check.
    fn get_run(&self, run_id: String) -> BoxFuture<Result<Option<RunRecord>, ClientError>>;

    /// Record the computed artifact set and digest.
    fn computed(
        &self,
        run_id: String,
        term: u64,
        digest: Digest,
        artifacts: Vec<ArtifactRef>,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, ClientError>>;

    /// Record a terminal failure.
    fn failed(
        &self,
        run_id: String,
        term: u64,
        error: String,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, ClientError>>;
}

impl EvidenceClient for HttpServiceClient {
    fn claim(
        &self,
        run_id: String,
        holder: String,
        continuation: Option<u64>,
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
                redelivered: false,
                holder,
                continuation,
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

    fn heartbeat(
        &self,
        run_id: String,
        holder: String,
        term: u64,
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
            let request = RunHeartbeat {
                run_id: id,
                holder,
                term,
            };
            client
                .post_json(
                    Service::Evidence,
                    format!("/internal/v1/runs/{run_id}/heartbeat"),
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
        term: u64,
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
                term,
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
        term: u64,
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
                term,
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

#[derive(Debug, Clone)]
struct FakeLease {
    holder: String,
    term: u64,
    expires_at: std::time::SystemTime,
}

#[derive(Debug, Default)]
struct EvidenceState {
    runs: BTreeMap<String, RunRecord>,
    submissions: std::collections::BTreeSet<(String, String)>,
    leases: BTreeMap<String, FakeLease>,
}

impl EvidenceState {
    fn start_lease(&mut self, run_id: &str, holder: &str) -> bool {
        let Some(record) = self.runs.get_mut(run_id) else {
            return false;
        };
        if record.attempts >= quantrule_service_api::MAX_ATTEMPTS {
            return false;
        }
        record.status = RunState::Running;
        record.attempts += 1;
        record.lease_term += 1;
        record.lease_holder = Some(holder.to_owned());
        let expires_at =
            std::time::SystemTime::now() + std::time::Duration::from_secs(RUN_LEASE_TTL_SECONDS);
        record.lease_expires_at = crate::clock::rfc3339_at(expires_at).ok();
        self.leases.insert(
            run_id.to_owned(),
            FakeLease {
                holder: holder.to_owned(),
                term: record.lease_term,
                expires_at,
            },
        );
        true
    }

    fn clear_lease(&mut self, run_id: &str) {
        self.leases.remove(run_id);
        if let Some(record) = self.runs.get_mut(run_id) {
            record.lease_holder = None;
            record.lease_expires_at = None;
        }
    }

    fn lease_is_live(&self, run_id: &str) -> Option<&FakeLease> {
        self.leases
            .get(run_id)
            .filter(|lease| lease.expires_at > std::time::SystemTime::now())
    }
}

/// An in-memory evidence lifecycle for worker tests.
///
/// It mirrors the run state machine the evidence store enforces: a queued run
/// is claimable, a running run is claimable only when its lease is absent,
/// expired, or continued by the same holder, and a claim records a new term.
/// The fake exists so the worker is tested without a database or a socket.
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
                .runs
                .get(&key)
                .cloned()
                .unwrap_or_else(|| RunRecord::queued(&run));
            state
                .runs
                .entry(key.clone())
                .or_insert_with(|| record.clone());
            state.submissions.insert((profile, key));
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
            if !state.submissions.contains(&(profile, run_id.clone())) {
                return Ok(RunCancelOutcome::Missing);
            }
            let Some(record) = state.runs.get_mut(&run_id) else {
                return Ok(RunCancelOutcome::Missing);
            };
            if !record.status.is_cancellable() {
                return Ok(RunCancelOutcome::Conflict(record.clone()));
            }
            record.status = RunState::Cancelled;
            // Cancellation is the fence: a stale worker's term no longer
            // matches, so it cannot complete the run.
            record.lease_term += 1;
            let record = record.clone();
            state.clear_lease(&run_id);
            Ok(RunCancelOutcome::Cancelled(record))
        })
    }

    /// Expire the current lease, as a killed worker would after the TTL.
    ///
    /// The term remains on the record; only the time-based lease is removed,
    /// so the next claim records a new term.
    pub fn expire_lease(&self, run_id: &str) {
        let mut state = self.inner.lock().expect("the evidence lock");
        state.leases.remove(run_id);
        if let Some(record) = state.runs.get_mut(run_id) {
            record.lease_expires_at = None;
        }
    }

    /// Read one run without a profile.
    pub fn get_any(&self, run_id: String) -> BoxFuture<Result<Option<RunRecord>, ClientError>> {
        let inner = std::sync::Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner.lock().map_err(|_| ClientError::InvalidResponse {
                service: Service::Evidence,
                message: "the evidence lock is poisoned".to_owned(),
            })?;
            Ok(state.runs.get(&run_id).cloned())
        })
    }
}

impl EvidenceClient for MemoryEvidence {
    fn claim(
        &self,
        run_id: String,
        holder: String,
        continuation: Option<u64>,
    ) -> BoxFuture<Result<RunClaimOutcome, ClientError>> {
        let inner = std::sync::Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner.lock().map_err(|_| ClientError::InvalidResponse {
                service: Service::Evidence,
                message: "the evidence lock is poisoned".to_owned(),
            })?;
            let Some(record) = state.runs.get(&run_id).cloned() else {
                return Ok(RunClaimOutcome::Missing);
            };
            match record.status {
                RunState::Queued => {
                    state.start_lease(&run_id, &holder);
                    let record = state.runs.get(&run_id).cloned().expect("the run");
                    Ok(RunClaimOutcome::Claimed(record))
                }
                RunState::Running => {
                    if record.digest.is_some() {
                        return Ok(RunClaimOutcome::Running(record));
                    }
                    let same_attempt = state.lease_is_live(&run_id).is_some_and(|lease| {
                        continuation == Some(lease.term) && lease.holder == holder
                    });
                    let live = state.lease_is_live(&run_id).is_some();
                    if live && !same_attempt {
                        return Ok(RunClaimOutcome::Running(record));
                    }
                    if record.attempts >= quantrule_service_api::MAX_ATTEMPTS {
                        return Ok(RunClaimOutcome::Exhausted(record));
                    }
                    state.start_lease(&run_id, &holder);
                    let record = state.runs.get(&run_id).cloned().expect("the run");
                    Ok(RunClaimOutcome::Claimed(record))
                }
                _ => Ok(RunClaimOutcome::Terminal(record)),
            }
        })
    }

    fn heartbeat(
        &self,
        run_id: String,
        holder: String,
        term: u64,
    ) -> BoxFuture<Result<RunRecord, ClientError>> {
        let inner = std::sync::Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner.lock().map_err(|_| ClientError::InvalidResponse {
                service: Service::Evidence,
                message: "the evidence lock is poisoned".to_owned(),
            })?;
            if !state.runs.contains_key(&run_id) {
                return Err(ClientError::Status {
                    service: Service::Evidence,
                    status: 404,
                    message: format!("run {run_id} is unknown"),
                });
            }
            let Some(lease) = state.lease_is_live(&run_id).cloned() else {
                return Err(ClientError::Status {
                    service: Service::Evidence,
                    status: 409,
                    message: "the run lease is stale".to_owned(),
                });
            };
            if lease.holder != holder || lease.term != term {
                return Err(ClientError::Status {
                    service: Service::Evidence,
                    status: 409,
                    message: "the run lease is stale".to_owned(),
                });
            }
            let expires_at = std::time::SystemTime::now()
                + std::time::Duration::from_secs(RUN_LEASE_TTL_SECONDS);
            state.leases.insert(
                run_id.clone(),
                FakeLease {
                    holder,
                    term,
                    expires_at,
                },
            );
            let Some(record) = state.runs.get_mut(&run_id) else {
                return Err(ClientError::Status {
                    service: Service::Evidence,
                    status: 404,
                    message: format!("run {run_id} is unknown"),
                });
            };
            record.lease_expires_at = crate::clock::rfc3339_at(expires_at).ok();
            Ok(record.clone())
        })
    }

    fn get_run(&self, run_id: String) -> BoxFuture<Result<Option<RunRecord>, ClientError>> {
        self.get_any(run_id)
    }

    fn computed(
        &self,
        run_id: String,
        term: u64,
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
            let Some(record) = state.runs.get_mut(&run_id) else {
                return Err(ClientError::Status {
                    service: Service::Evidence,
                    status: 404,
                    message: format!("run {run_id} is unknown"),
                });
            };
            if record.status == RunState::Running && record.lease_term == term {
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
        term: u64,
        error: String,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, ClientError>> {
        let inner = std::sync::Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner.lock().map_err(|_| ClientError::InvalidResponse {
                service: Service::Evidence,
                message: "the evidence lock is poisoned".to_owned(),
            })?;
            let Some(record) = state.runs.get_mut(&run_id) else {
                return Err(ClientError::Status {
                    service: Service::Evidence,
                    status: 404,
                    message: format!("run {run_id} is unknown"),
                });
            };
            if record.status == RunState::Running && record.lease_term == term {
                record.status = RunState::Failed;
                record.error = Some(error);
                record.stages = stages;
                let record = record.clone();
                state.clear_lease(&run_id);
                return Ok(record);
            }
            Ok(record.clone())
        })
    }
}
