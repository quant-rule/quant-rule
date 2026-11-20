//! Shared in-memory simulation services and fixture bundles for the tests.
//!
//! The memory repository mirrors the Postgres semantics the routes rely on:
//! profile scoping, the decision lock, deterministic oracle replacement, and
//! the status transitions. The object store is the same in-memory store the
//! other service tests use.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use quantrule_api::{Digest, MarkOrigin, PROFILE_HEADER, ProfileId, SimulationStatus};
use quantrule_runtime::adapters::BoxFuture;
use quantrule_runtime::{Config, HealthRegistry};
use quantrule_simulation::{CheckRecord, Selection};
use quantrule_simulation_service::bundle::SIMULATION_BUNDLE_SCHEMA;
use quantrule_simulation_service::records::{
    DecisionRecord, MarkRecord, NewSimulation, RunRecord, SimulationSummary, action_text,
    origin_text, side_text,
};
use quantrule_simulation_service::store::{
    ArchiveOutcome, EraseOutcome, SimulationStore, StoreError as RepositoryError,
};
use quantrule_simulation_service::{SimulationServices, SimulationState, router};
use serde_json::json;
use tower::ServiceExt;

/// The in-memory object store from `quantrule_runtime`.
// Each integration target compiles this module separately, and not every
// target exercises the object store.
#[allow(unused_imports)]
pub use quantrule_runtime::adapters::memory::MemoryObjects;

#[derive(Default)]
struct Inner {
    runs: BTreeMap<(String, String), RunRecord>,
    marks: BTreeMap<(String, String, String), MarkRecord>,
    archived: BTreeMap<(String, String), String>,
}

/// An in-memory simulation repository with the same contract as Postgres.
#[derive(Clone, Default)]
pub struct MemoryStore {
    inner: Arc<Mutex<Inner>>,
}

impl MemoryStore {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().expect("store lock")
    }

    fn recount(&self, inner: &mut Inner, profile: &str, sim_id: &str) {
        let marks: Vec<MarkRecord> = inner
            .marks
            .iter()
            .filter(|((profile_id, sid, _), _)| profile_id == profile && sid == sim_id)
            .map(|(_, mark)| mark.clone())
            .collect();
        let human: Vec<&MarkRecord> = marks
            .iter()
            .filter(|mark| mark.origin == MarkOrigin::Human)
            .collect();
        let oracle = marks
            .iter()
            .filter(|mark| mark.origin == MarkOrigin::Oracle)
            .count() as u64;
        let mut charts: Vec<u64> = human.iter().map(|mark| mark.chart).collect();
        charts.sort_unstable();
        charts.dedup();
        let Some(run) = inner.runs.get_mut(&(profile.to_owned(), sim_id.to_owned())) else {
            return;
        };
        run.human_marks = human.len() as u64;
        run.oracle_marks = oracle;
        run.marked_windows = charts.len() as u64;
    }
}

fn not_found() -> RepositoryError {
    RepositoryError::NotFound
}

impl SimulationStore for MemoryStore {
    fn create(
        &self,
        profile: ProfileId,
        run: NewSimulation,
    ) -> BoxFuture<Result<RunRecord, RepositoryError>> {
        let store = self.clone();
        Box::pin(async move {
            let mut inner = store.lock();
            let key = (profile.as_str().to_owned(), run.sim_id.clone());
            if let Some(existing) = inner.runs.get(&key) {
                return Ok(existing.clone());
            }
            let record = RunRecord {
                sim_id: run.sim_id,
                premise: run.premise,
                bundle: run.bundle,
                inputs: run.inputs,
                inputs_digest: run.inputs_digest,
                seed: run.seed,
                criteria: run.criteria,
                window_spec: run.window_spec,
                oracle_spec: run.oracle_spec,
                check_config: run.check_config,
                selection: None,
                status: SimulationStatus::Draft,
                check_result: None,
                decision: None,
                created_at: run.created_at,
                human_marks: 0,
                oracle_marks: 0,
                marked_windows: 0,
            };
            inner.runs.insert(key, record.clone());
            Ok(record)
        })
    }

    fn list(
        &self,
        profile: ProfileId,
        status: Option<SimulationStatus>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<SimulationSummary>, RepositoryError>> {
        let store = self.clone();
        Box::pin(async move {
            let inner = store.lock();
            let mut rows: Vec<(&(String, String), &RunRecord)> = inner
                .runs
                .iter()
                .filter(|((profile_id, _), _)| profile_id == profile.as_str())
                .filter(|((profile_id, sim_id), _)| {
                    !inner
                        .archived
                        .contains_key(&(profile_id.clone(), sim_id.clone()))
                })
                .filter(|(_, run)| status.is_none_or(|status| run.status == status))
                .collect();
            rows.sort_by(|(_, left), (_, right)| {
                right
                    .created_at
                    .cmp(&left.created_at)
                    .then_with(|| left.sim_id.cmp(&right.sim_id))
            });
            Ok(rows
                .into_iter()
                .take(limit)
                .map(|(key, run)| {
                    let mut summary = run.summary();
                    summary.archived_at = inner.archived.get(key).cloned();
                    summary
                })
                .collect())
        })
    }

    fn list_including_archived(
        &self,
        profile: ProfileId,
        status: Option<SimulationStatus>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<SimulationSummary>, RepositoryError>> {
        let store = self.clone();
        Box::pin(async move {
            let inner = store.lock();
            let mut rows: Vec<(&(String, String), &RunRecord)> = inner
                .runs
                .iter()
                .filter(|((profile_id, _), _)| profile_id == profile.as_str())
                .filter(|(_, run)| status.is_none_or(|status| run.status == status))
                .collect();
            rows.sort_by(|(_, left), (_, right)| {
                right
                    .created_at
                    .cmp(&left.created_at)
                    .then_with(|| left.sim_id.cmp(&right.sim_id))
            });
            Ok(rows
                .into_iter()
                .take(limit)
                .map(|(key, run)| {
                    let mut summary = run.summary();
                    summary.archived_at = inner.archived.get(key).cloned();
                    summary
                })
                .collect())
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        sim_id: String,
    ) -> BoxFuture<Result<Option<RunRecord>, RepositoryError>> {
        let store = self.clone();
        Box::pin(async move {
            Ok(store
                .lock()
                .runs
                .get(&(profile.as_str().to_owned(), sim_id))
                .cloned())
        })
    }

    fn archive(
        &self,
        profile: ProfileId,
        sim_id: String,
        archived_at: String,
    ) -> BoxFuture<Result<ArchiveOutcome, RepositoryError>> {
        let store = self.clone();
        Box::pin(async move {
            let mut inner = store.lock();
            let key = (profile.as_str().to_owned(), sim_id);
            let Some(run) = inner.runs.get(&key).cloned() else {
                return Ok(ArchiveOutcome::NotFound);
            };
            if inner.archived.contains_key(&key) {
                return Ok(ArchiveOutcome::AlreadyArchived);
            }
            inner.archived.insert(key, archived_at.clone());
            let mut summary = run.summary();
            summary.archived_at = Some(archived_at);
            Ok(ArchiveOutcome::Archived(summary))
        })
    }

    fn erase(
        &self,
        profile: ProfileId,
        sim_id: String,
    ) -> BoxFuture<Result<EraseOutcome, RepositoryError>> {
        let store = self.clone();
        Box::pin(async move {
            let mut inner = store.lock();
            let key = (profile.as_str().to_owned(), sim_id.clone());
            let Some(run) = inner.runs.get(&key) else {
                return Ok(EraseOutcome::NotFound);
            };
            if !inner.archived.contains_key(&key) {
                return Ok(EraseOutcome::NotArchived);
            }
            if matches!(
                run.status,
                SimulationStatus::Checked | SimulationStatus::Decided
            ) {
                return Ok(EraseOutcome::Protected(run.status));
            }
            inner.runs.remove(&key);
            inner.archived.remove(&key);
            inner
                .marks
                .retain(|(profile_id, sid, _), _| profile_id != &key.0 || sid != &sim_id);
            Ok(EraseOutcome::Erased(sim_id))
        })
    }

    fn save_windows(
        &self,
        profile: ProfileId,
        sim_id: String,
        selection: Selection,
    ) -> BoxFuture<Result<RunRecord, RepositoryError>> {
        let store = self.clone();
        Box::pin(async move {
            let mut inner = store.lock();
            let key = (profile.as_str().to_owned(), sim_id.clone());
            let Some(run) = inner.runs.get_mut(&key) else {
                return Err(not_found());
            };
            if run.status == SimulationStatus::Decided {
                return Err(RepositoryError::Locked);
            }
            run.selection = Some(selection);
            if run.status == SimulationStatus::Draft {
                run.status = SimulationStatus::Marking;
            }
            Ok(run.clone())
        })
    }

    fn put_mark(
        &self,
        profile: ProfileId,
        sim_id: String,
        mark: MarkRecord,
    ) -> BoxFuture<Result<MarkRecord, RepositoryError>> {
        let store = self.clone();
        Box::pin(async move {
            let mut inner = store.lock();
            let key = (profile.as_str().to_owned(), sim_id.clone());
            let Some(existing) = inner.runs.get(&key) else {
                return Err(not_found());
            };
            if existing.status == SimulationStatus::Decided {
                return Err(RepositoryError::Locked);
            }
            inner.marks.retain(|(profile_id, sid, _), existing| {
                profile_id != profile.as_str()
                    || sid != &sim_id
                    || existing.origin != mark.origin
                    || existing.chart != mark.chart
                    || existing.bar != mark.bar
                    || existing.action != mark.action
                    || existing.side != mark.side
            });
            inner.marks.insert(
                (
                    profile.as_str().to_owned(),
                    sim_id.clone(),
                    mark.mark_id.clone(),
                ),
                mark.clone(),
            );
            if let Some(run) = inner.runs.get_mut(&key)
                && run.status == SimulationStatus::Draft
            {
                run.status = SimulationStatus::Marking;
            }
            store.recount(&mut inner, profile.as_str(), &sim_id);
            Ok(mark)
        })
    }

    fn delete_mark(
        &self,
        profile: ProfileId,
        sim_id: String,
        mark_id: String,
    ) -> BoxFuture<Result<bool, RepositoryError>> {
        let store = self.clone();
        Box::pin(async move {
            let mut inner = store.lock();
            let key = (profile.as_str().to_owned(), sim_id.clone());
            let Some(existing) = inner.runs.get(&key) else {
                return Err(not_found());
            };
            if existing.status == SimulationStatus::Decided {
                return Err(RepositoryError::Locked);
            }
            let removed = inner
                .marks
                .remove(&(profile.as_str().to_owned(), sim_id.clone(), mark_id))
                .is_some();
            store.recount(&mut inner, profile.as_str(), &sim_id);
            Ok(removed)
        })
    }

    fn list_marks(
        &self,
        profile: ProfileId,
        sim_id: String,
        origin: Option<MarkOrigin>,
    ) -> BoxFuture<Result<Vec<MarkRecord>, RepositoryError>> {
        let store = self.clone();
        Box::pin(async move {
            let inner = store.lock();
            if !inner
                .runs
                .contains_key(&(profile.as_str().to_owned(), sim_id.clone()))
            {
                return Err(not_found());
            }
            let mut marks: Vec<MarkRecord> = inner
                .marks
                .iter()
                .filter(|((profile_id, sid, _), mark)| {
                    profile_id == profile.as_str()
                        && sid == &sim_id
                        && origin.is_none_or(|o| mark.origin == o)
                })
                .map(|(_, mark)| mark.clone())
                .collect();
            marks.sort_by(|left, right| {
                (
                    left.chart,
                    left.bar,
                    action_text(left.action),
                    side_text(left.side),
                    origin_text(left.origin),
                )
                    .cmp(&(
                        right.chart,
                        right.bar,
                        action_text(right.action),
                        side_text(right.side),
                        origin_text(right.origin),
                    ))
            });
            Ok(marks)
        })
    }

    fn replace_oracle(
        &self,
        profile: ProfileId,
        sim_id: String,
        marks: Vec<MarkRecord>,
    ) -> BoxFuture<Result<Vec<MarkRecord>, RepositoryError>> {
        let store = self.clone();
        Box::pin(async move {
            let mut inner = store.lock();
            let key = (profile.as_str().to_owned(), sim_id.clone());
            let Some(existing) = inner.runs.get(&key) else {
                return Err(not_found());
            };
            if existing.status == SimulationStatus::Decided {
                return Err(RepositoryError::Locked);
            }
            inner.marks.retain(|(profile_id, sid, _), mark| {
                profile_id != profile.as_str()
                    || sid != &sim_id
                    || mark.origin != MarkOrigin::Oracle
            });
            for mark in &marks {
                inner.marks.insert(
                    (
                        profile.as_str().to_owned(),
                        sim_id.clone(),
                        mark.mark_id.clone(),
                    ),
                    mark.clone(),
                );
            }
            if let Some(run) = inner.runs.get_mut(&key)
                && run.status != SimulationStatus::Checked
            {
                run.status = SimulationStatus::Reviewed;
            }
            store.recount(&mut inner, profile.as_str(), &sim_id);
            Ok(marks)
        })
    }

    fn record_check(
        &self,
        profile: ProfileId,
        sim_id: String,
        check: CheckRecord,
    ) -> BoxFuture<Result<(), RepositoryError>> {
        let store = self.clone();
        Box::pin(async move {
            let mut inner = store.lock();
            let key = (profile.as_str().to_owned(), sim_id);
            let Some(run) = inner.runs.get_mut(&key) else {
                return Err(not_found());
            };
            run.check_result = Some(check);
            if run.status != SimulationStatus::Decided {
                run.status = SimulationStatus::Checked;
            }
            Ok(())
        })
    }

    fn decide(
        &self,
        profile: ProfileId,
        sim_id: String,
        decision: DecisionRecord,
    ) -> BoxFuture<Result<DecisionRecord, RepositoryError>> {
        let store = self.clone();
        Box::pin(async move {
            let mut inner = store.lock();
            let key = (profile.as_str().to_owned(), sim_id);
            let Some(run) = inner.runs.get_mut(&key) else {
                return Err(not_found());
            };
            if let Some(existing) = &run.decision {
                if existing.decision == decision.decision && existing.note == decision.note {
                    return Ok(existing.clone());
                }
                return Err(RepositoryError::Locked);
            }
            run.decision = Some(decision.clone());
            run.status = SimulationStatus::Decided;
            Ok(decision)
        })
    }
}

/// Build a service state over one store and object store.
pub fn state(store: Arc<MemoryStore>, objects: Arc<MemoryObjects>) -> Arc<SimulationState> {
    Arc::new(SimulationState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        SimulationServices {
            store: Some(store),
            objects: Some(objects),
        },
    ))
}

/// Send one request through the service router.
pub async fn request(
    state: Arc<SimulationState>,
    method: &str,
    path: &str,
    profile: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, Vec<u8>) {
    let mut builder = Request::builder().method(method).uri(path);
    if !profile.is_empty() {
        builder = builder.header(PROFILE_HEADER, profile);
    }
    let body = match body {
        Some(value) => {
            builder = builder.header("content-type", "application/json");
            Body::from(value.to_string())
        }
        None => Body::empty(),
    };
    let response = router(state)
        .oneshot(builder.body(body).expect("request"))
        .await
        .expect("response");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .expect("body")
        .to_vec();
    (status, bytes)
}

/// Decode a JSON response body.
pub fn json(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(bytes).expect("json response")
}

/// One recorded fixture bundle with its digest and key.
pub struct Fixture {
    /// The wire digest.
    pub digest: Digest,
}

/// Store a deterministic fixture bundle with `events` event bars.
///
/// Every bar has a 1.0 ATR and a 3.5-point range, so the oracle has a good long
/// path and a bad short path to label.
pub async fn store_bundle(objects: &MemoryObjects, events: usize) -> Fixture {
    let bars: Vec<serde_json::Value> = (0..600)
        .map(|index| {
            let close = 100.0 + ((index % 7) as f64) * 0.1;
            json!({
                "bar": index,
                "timestamp": 1_514_764_800_i64 + i64::from(index) * 900,
                "open": close,
                "high": close + 3.0,
                "low": close - 0.5,
                "close": close,
                "volume": if index % 11 == 0 { 0.0 } else { 10.0 + index as f64 },
                "atr": 1.0,
                "ema_fast": close - 0.2,
                "ema_mid": close,
                "ema_slow": close + 0.2,
                "rsi": 50.0 + ((index % 5) as f64),
                "rvol": 1.0 + ((index % 3) as f64) * 0.1,
                "adx": 20.0 + ((index % 4) as f64),
                "ema_stack": 1,
                "rsi_zone": 0,
                "atr_ratio": 1.0,
                "close_position": 0.7,
                "upper_wick": 0.2,
                "lower_wick": 0.1,
                "body_ratio": 0.7
            })
        })
        .collect();
    let event_rows: Vec<serde_json::Value> = (0..events)
        .map(|event| {
            let bar = 100 + event * 10;
            json!({
                "event_index": event,
                "code": 149,
                "type_code": 1,
                "subtype_code": 1,
                "source_code": 1,
                "occurred_bar": bar,
                "available_bar": bar,
                "direction": 1,
                "reference_code": 149,
                "primary": 100.0,
                "reference": 99.0,
                "completeness": 0.99
            })
        })
        .collect();
    let levels: Vec<serde_json::Value> = (0..6)
        .map(|level| {
            let bar = 60 + level * 40;
            json!({
                "reference_code": 1000 + level,
                "scale_code": 1,
                "kind_code": if level % 2 == 0 { -1 } else { 1 },
                "price": 99.0 + level as f64,
                "occurred_bar": bar,
                "available_bar": bar,
                "distance_atr": 0.5
            })
        })
        .collect();
    let body = serde_json::to_vec(&json!({
        "schema": SIMULATION_BUNDLE_SCHEMA,
        "timeframe_secs": 900,
        "bars": bars,
        "events": event_rows,
        "levels": levels,
        "catalog": {"149": "level.fib_test"},
        "provenance": {"data_version": "fixture-btcusdt-15m"}
    }))
    .expect("bundle json");
    let digest = Digest::sha256(&body);
    let key = quantrule_simulation_service::bundle::bundle_key(&digest);
    objects.store(&key, body).await.expect("store the bundle");
    Fixture { digest }
}
