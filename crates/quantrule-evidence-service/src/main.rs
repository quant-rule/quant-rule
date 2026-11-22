//! The evidence service binary.
//!
//! It connects to Postgres, the object store, the broker, and the data service.
//! It applies only the `runs`/`research` migrations, serves the public run,
//! hypothesis, research, and operations surface plus the internal lifecycle,
//! and consumes `run.computed` to verify the hash chain and finalize the run.
//! It also publishes the run-notification outbox and retries the
//! dataset-reference sweeps, so a crash after a commit cannot lose a run or
//! leave a reference that blocks a dataset. Artifact bytes live in S3 and the
//! control-plane queue is RabbitMQ.

#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use quantrule_runtime::adapters::{
    Message, MessageHandler, ObjectStore, Queue, QueueConsumer, QueueError,
};
use quantrule_runtime::backends::{
    ControlQueue, Objects, migrate_owner_database, open_service_database,
};
use quantrule_runtime::client::{ClientError, DataClient, HttpServiceClient, Service};
use quantrule_runtime::config::{ConfigError, migrate_only_requested};
use quantrule_runtime::queue::QueueKind;
use quantrule_runtime::{Config, HealthRegistry, RuntimeError, logging};
use quantrule_service_api::INTERNAL_API_VERSION;

use quantrule_evidence_service::evidence::{EvidenceServices, finalize};
use quantrule_evidence_service::outbox::RunOutbox;
use quantrule_evidence_service::postgres::{PgHypothesisStore, PgRunStore};
use quantrule_evidence_service::releases::ReferenceReleaseLog;
use quantrule_evidence_service::research::{HypothesisStore, ImprovementStore};
use quantrule_evidence_service::runs::RunStore;
use quantrule_evidence_service::stage::{PgStageStore, StageStore, retention};
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

    if migrate_only_requested() {
        return migrate_owner_database(
            &config,
            "runs",
            "run and research",
            &quantrule_evidence_service::migrations::available(),
        )
        .await;
    }

    let mut health = HealthRegistry::named("evidence");
    let opened = open_service_database(&config, &mut health).await?;
    let pool = opened.as_ref().map(|database| database.pool().clone());
    // One store serves the run lifecycle, the notification outbox, and the
    // release tombstones. They are three traits over one database, so a run
    // write and its intent cannot diverge.
    let run_store: Option<Arc<PgRunStore>> = pool
        .as_ref()
        .map(|pool| Arc::new(PgRunStore::new(pool.clone())));
    let runs: Option<RunHandle> = run_store
        .as_ref()
        .map(|store| Arc::clone(store) as Arc<dyn RunStore>);
    let outbox: Option<Arc<dyn RunOutbox>> = run_store
        .as_ref()
        .map(|store| Arc::clone(store) as Arc<dyn RunOutbox>);
    let releases: Option<Arc<dyn ReferenceReleaseLog>> = run_store
        .as_ref()
        .map(|store| Arc::clone(store) as Arc<dyn ReferenceReleaseLog>);
    let stages: Option<StageHandle> = pool
        .as_ref()
        .map(|pool| Arc::new(PgStageStore::new(pool.clone())) as Arc<dyn StageStore>);
    let hypotheses: Option<HypothesisHandle> = pool
        .as_ref()
        .map(|pool| Arc::new(PgHypothesisStore::new(pool.clone())) as Arc<dyn HypothesisStore>);
    let improvements: Option<ImprovementHandle> = pool.as_ref().map(|pool| {
        Arc::new(quantrule_evidence_service::research::PgImprovementStore::new(pool.clone()))
            as Arc<dyn ImprovementStore>
    });

    let backend = Objects::open(&config).await?;
    let objects: Arc<dyn ObjectStore> = Arc::new(backend.clone());
    health.register("object-store", move || {
        let backend = backend.clone();
        async move { backend.check().await.map_err(|error| error.to_string()) }
    });

    let queue = Arc::new(ControlQueue::open(&config).await?);
    let evidence_queue = queue.names().name(QueueKind::Evidence).to_owned();
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
            outbox,
            releases,
            stages: stages.as_ref().map(Arc::clone),
            hypotheses,
            improvements,
            objects: Some(Arc::clone(&objects)),
            queue: Some(publish),
            data: Some(data),
        },
    ));

    // The publisher reconciles the live runs with the outbox, then drains it.
    // Its first pass is a startup pass, so a restarted process republishes
    // every unfinished run into an empty or rebuilt queue.
    if let (Some(outbox), Some(queue)) = (
        state.outbox.as_ref().map(Arc::clone),
        state.queue.as_ref().map(Arc::clone),
    ) {
        quantrule_evidence_service::outbox::spawn_publisher(
            outbox,
            queue,
            quantrule_evidence_service::outbox::PUBLISH_INTERVAL,
        );
    }

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

    // The release sweep retries every tombstone the erase paths wrote. A
    // release is idempotent on the data side; a `404` means the dataset is
    // already gone, so the tombstone is confirmed rather than retried forever.
    if let (Some(releases), Some(data)) = (
        state.releases.as_ref().map(Arc::clone),
        state.data.as_ref().map(Arc::clone),
    ) {
        tokio::spawn(async move {
            loop {
                match quantrule_evidence_service::reconcile::release_all(
                    releases.as_ref(),
                    data.as_ref(),
                )
                .await
                {
                    Ok(0) => {}
                    Ok(count) => tracing::info!(count, "dataset references released"),
                    Err(error) => {
                        tracing::warn!(%error, "dataset-reference release will retry");
                    }
                }
                tokio::time::sleep(Duration::from_secs(30)).await;
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
    let server = quantrule_runtime::serve_config(&config, router(state));
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
            .consume(&evidence_queue, handler)
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
