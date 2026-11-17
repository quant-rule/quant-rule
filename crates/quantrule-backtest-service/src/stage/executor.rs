//! The stage executor seam.
//!
//! A stage worker owns the control-plane mechanics; an executor owns the engine
//! call. The seam exists so #852–#854 can add the model, strategy, and portfolio
//! executors without changing the worker, and so a worker test uses a
//! deterministic fake with no engine, dataset, or clock.
//!
//! An executor reads one [`StageRecord`] and returns file bytes. It never writes
//! an object, records a stage row, publishes a message, or names a run. The
//! worker computes the artifact identity, writes the CAS keys, and records the
//! result.

use std::sync::Arc;

use quantrule_api::{StageKind, StageState};
use quantrule_runtime::adapters::BoxFuture;
use quantrule_service_api::StageRecord;

/// Why an executor could not produce a stage artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageExecutorError(pub String);

impl std::fmt::Display for StageExecutorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for StageExecutorError {}

/// One file in a stage output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageFile {
    /// The relative artifact name, such as `results.json`.
    pub name: String,
    /// The file bytes.
    pub bytes: Vec<u8>,
}

/// The files one executor produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageOutput {
    /// The artifact files, in any order; the worker sorts them by name.
    pub files: Vec<StageFile>,
}

impl StageOutput {
    /// Build an output from one file.
    #[must_use]
    pub fn one(name: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            files: vec![StageFile {
                name: name.into(),
                bytes: bytes.into(),
            }],
        }
    }
}

/// Compute one stage artifact from its durable record.
///
/// The executor is pure with respect to the control plane: it reads the record
/// and returns bytes. It must be deterministic for the same record, because the
/// artifact identity is content-addressed.
pub trait StageExecutor: Send + Sync {
    /// Execute one stage.
    fn execute(&self, record: &StageRecord) -> BoxFuture<Result<StageOutput, StageExecutorError>>;
}

/// The executor per stage kind.
///
/// A kind with no executor fails the stage with a clear message instead of
/// silently succeeding. The real model, strategy, and portfolio executors are
/// installed here by #852–#854; until then a production worker has none.
#[derive(Clone, Default)]
pub struct StageDispatch {
    model: Option<Arc<dyn StageExecutor>>,
    strategy: Option<Arc<dyn StageExecutor>>,
    portfolio: Option<Arc<dyn StageExecutor>>,
}

impl std::fmt::Debug for StageDispatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StageDispatch")
            .field("model", &self.model.is_some())
            .field("strategy", &self.strategy.is_some())
            .field("portfolio", &self.portfolio.is_some())
            .finish()
    }
}

impl StageDispatch {
    /// An empty dispatch.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Install the executor for one kind.
    #[must_use]
    pub fn with(mut self, kind: StageKind, executor: Arc<dyn StageExecutor>) -> Self {
        match kind {
            StageKind::Model => self.model = Some(executor),
            StageKind::Strategy => self.strategy = Some(executor),
            StageKind::Portfolio => self.portfolio = Some(executor),
        }
        self
    }

    /// Install one executor for every kind, which a test uses.
    #[must_use]
    pub fn uniform(executor: Arc<dyn StageExecutor>) -> Self {
        Self {
            model: Some(Arc::clone(&executor)),
            strategy: Some(Arc::clone(&executor)),
            portfolio: Some(executor),
        }
    }

    /// The executor for one kind, or `None` when it is not installed.
    #[must_use]
    pub fn executor(&self, kind: StageKind) -> Option<&Arc<dyn StageExecutor>> {
        match kind {
            StageKind::Model => self.model.as_ref(),
            StageKind::Strategy => self.strategy.as_ref(),
            StageKind::Portfolio => self.portfolio.as_ref(),
        }
    }
}

/// Whether a record is safe to execute.
#[must_use]
pub fn is_executable(record: &StageRecord) -> bool {
    record.state == StageState::Running
}
