//! The evidence service binary.
//!
//! It connects to Postgres, the object store, the broker, and the data service.
//! It applies only the `runs`/`research` migrations, serves the public run,
//! hypothesis, research, and operations surface plus the internal lifecycle,
//! and consumes `run.computed` to verify the hash chain and finalize the run. With `QR_PROFILE=desktop` the
//! run and hypothesis stores sit on the embedded SQLite file under
//! `QR_DATA_DIR`, artifact bytes live in the filesystem CAS under
//! `<QR_DATA_DIR>/objects`, and the control-plane queue is the durable queue on
//! that same SQLite file, so `qr-backtest` sees the `run.computed` publish.
//! Postgres, S3, and RabbitMQ remain the default.

#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use quantrule_runtime::adapters::{
    Message, MessageHandler, ObjectStore, Queue, QueueConsumer, QueueError,
};
use quantrule_runtime::backends::{ControlQueue, Objects, ServiceDatabase, open_service_database};
use quantrule_runtime::client::{ClientError, DataClient, HttpServiceClient, Service};
use quantrule_runtime::config::ConfigError;
use quantrule_runtime::queue::{QUEUE_EVIDENCE, ROUTING_COMPUTED};
use quantrule_runtime::{Config, HealthRegistry, RuntimeError, logging};
use quantrule_service_api::INTERNAL_API_VERSION;

use quantrule_evidence_service::evidence::{EvidenceServices, finalize};
use quantrule_evidence_service::postgres::{PgHypothesisStore, PgRunStore};
use quantrule_evidence_service::research::ImprovementStore;
use quantrule_evidence_service::runs::RunStore;
use quantrule_evidence_service::sqlite::{SqliteHypothesisStore, SqliteRunStore};
use quantrule_evidence_service::stage::{PgStageStore, SqliteStageStore, StageStore, retention};
use quantrule_evidence_service::{EvidenceState, router};

/// The run store a route handler reads.
type RunHandle = Arc<dyn RunStore>;
/// The stage store a route handler reads.
type StageHandle = Arc<dyn StageStore>;
/// The hypothesis store a route handler reads.
type HypothesisHandle = Arc<dyn quantrule_evidence_service::research::HypothesisStore>;
/// The research-ledger store a route handler reads.
type ImprovementHandle = Arc<dyn ImprovementStore>;

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;

    let mut health = HealthRegistry::named("evidence");
    let opened = open_service_database(
        &config,
        "runs",
        "run and research",
        &quantrule_evidence_service::migrations::sqlite(),
        &quantrule_evidence_service::migrations::available(),
        &mut health,
    )
    .await?;
    // The queue shares the handle the desktop branch opens, so the queue and the
    // run store use one connection on one file in this process.
    let sqlite = match &opened {
        Some(ServiceDatabase::Sqlite(database)) => Some(database.clone()),
        Some(ServiceDatabase::Postgres(_)) | None => None,
    };
    let (runs, stages, hypotheses, improvements): (
        Option<RunHandle>,
        Option<StageHandle>,
        Option<HypothesisHandle>,
        Option<ImprovementHandle>,
    ) = match opened {
        Some(ServiceDatabase::Sqlite(database)) => (
            Some(Arc::new(SqliteRunStore::new(database.clone()))),
            Some(Arc::new(SqliteStageStore::new(database.clone()))),
            Some(Arc::new(SqliteHypothesisStore::new(database.clone()))),
            Some(Arc::new(
                quantrule_evidence_service::research::SqliteImprovementStore::new(database),
            )),
        ),
        Some(ServiceDatabase::Postgres(database)) => (
            Some(Arc::new(PgRunStore::new(database.pool().clone()))),
            Some(Arc::new(PgStageStore::new(database.pool().clone()))),
            Some(Arc::new(PgHypothesisStore::new(database.pool().clone()))),
            Some(Arc::new(
                quantrule_evidence_service::research::PgImprovementStore::new(
                    database.pool().clone(),
                ),
            )),
        ),
        None => (None, None, None, None),
    };

    let backend = Objects::open(&config).await?;
    let objects: Arc<dyn ObjectStore> = Arc::new(backend.clone());
    health.register("object-store", move || {
        let backend = backend.clone();
        async move { backend.check().await.map_err(|error| error.to_string()) }
    });

    let queue = Arc::new(ControlQueue::open(&config, sqlite.as_ref()).await?);
    health.register("queue", {
        let queue = Arc::clone(&queue);
        move || {
            let queue = Arc::clone(&queue);
            async move { queue.check().await.map_err(|error| error.to_string()) }
        }
    });

    let data: Arc<dyn DataClient> =
        Arc::new(HttpServiceClient::from_env(Service::Data).map_err(data_config_error)?);

    let publish: Arc<dyn Queue> = queue.clone();
    let reconcile_runs = runs.as_ref().map(Arc::clone);
    let reconcile_improvements = improvements.as_ref().map(Arc::clone);
    let state = Arc::new(EvidenceState::with_services(
        &config,
        health,
        quantrule_evidence_service::EvidenceServices {
            runs: runs.as_ref().map(Arc::clone),
            stages: stages.as_ref().map(Arc::clone),
            hypotheses,
            improvements,
            objects: Some(Arc::clone(&objects)),
            queue: Some(publish),
            data: Some(data),
        },
    ));

    // Rows written before the dataset-reference registry existed are registered
    // once. The sweep is idempotent and retries until the data service answers;
    // a missing reference blocks a dataset erase, which is the safe direction.
    if let (Some(runs), Some(improvements), Some(data)) = (
        reconcile_runs,
        reconcile_improvements,
        state.data.as_ref().map(Arc::clone),
    ) {
        tokio::spawn(async move {
            loop {
                match quantrule_evidence_service::reconcile::register_all(
                    Arc::clone(&runs),
                    Arc::clone(&improvements),
                    Arc::clone(&data),
                )
                .await
                {
                    Ok(count) => {
                        tracing::info!(count, "dataset references reconciled");
                        break;
                    }
                    Err(error) => {
                        tracing::warn!(%error, "dataset-reference reconciliation will retry");
                        tokio::time::sleep(Duration::from_secs(30)).await;
                    }
                }
            }
        });
    }

    // Retention sweeps at startup and hourly. It names only the stage schema
    // and the stage object namespace; a run, a research record, and a run
    // artifact key are untouched. The outcome is recorded on the shared
    // operations status, success or failure, and the service still starts
    // because the health probes say what is down.
    if let Some(stages) = stages.as_ref() {
        tokio::spawn(retention::sweep(
            Arc::clone(stages),
            Arc::clone(&objects),
            Arc::clone(&state.operations),
            Arc::clone(&state.stage_gate),
            config.stage_retention_days,
            retention::SWEEP_INTERVAL,
        ));
    }

    // A database that is merely unreachable is a health state, not a startup
    // failure: the service stays up so the operator can see what is down. The
    // finalizer waits instead of consuming when there is no store.
    let evidence = runs.map(|runs| {
        Arc::new(EvidenceServices {
            runs,
            objects: Arc::clone(&objects),
        })
    });

    tracing::info!(
        bind = %config.bind,
        contract = INTERNAL_API_VERSION,
        "starting evidence service"
    );
    let server = quantrule_runtime::serve(&config.bind, router(state));
    let consumer = async move {
        let Some(evidence) = evidence else {
            return std::future::pending::<Result<(), RuntimeError>>().await;
        };
        let handler: MessageHandler = Box::new(move |message: Message| {
            let evidence = Arc::clone(&evidence);
            Box::pin(async move {
                let run_id = serde_json::from_slice::<serde_json::Value>(&message.payload)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("run_id")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_owned)
                    });
                let Some(run_id) = run_id else {
                    tracing::error!("run.computed carries no run_id; acknowledging");
                    return Ok(());
                };
                match finalize(run_id.clone(), &evidence).await {
                    Ok(outcome) => {
                        tracing::info!(run_id = %run_id, outcome = ?outcome, "run finalized");
                        Ok(())
                    }
                    Err(error) => {
                        tracing::error!(run_id = %run_id, error = %error, "run finalization failed");
                        Err(QueueError(error.to_string()))
                    }
                }
            })
        });
        queue
            .consume(QUEUE_EVIDENCE, &[ROUTING_COMPUTED], handler)
            .await
            .map_err(|error| RuntimeError::Queue(error.to_string()))
    };
    tokio::select! {
        result = server => result,
        result = consumer => result,
    }
}

/// Name the missing or invalid `QR_DATA_URL` at startup.
fn data_config_error(error: ClientError) -> RuntimeError {
    let name = Service::Data.url_variable();
    RuntimeError::Config(match error {
        ClientError::Unconfigured { .. } => ConfigError::Missing { name },
        _ => ConfigError::Invalid {
            name,
            value: "<invalid>".to_owned(),
            reason: "must be an absolute HTTP URL",
        },
    })
}
