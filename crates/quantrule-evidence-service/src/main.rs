//! The evidence service binary.
//!
//! It connects to Postgres, the object store, the broker, and the data service.
//! It applies only the `runs`/`research` migrations, serves the public run,
//! hypothesis, research, and operations surface plus the internal lifecycle,
//! and consumes `run.computed` only while it holds the database leader lease.
//! It also publishes the run-notification outbox and retries the
//! dataset-reference sweeps, so a crash after a commit cannot lose a run or
//! leave a reference that blocks a dataset. Artifact bytes live in S3 and the
//! control-plane queue is RabbitMQ.

#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use quantrule_runtime::adapters::{ObjectStore, Queue, QueueConsumer};
use quantrule_runtime::backends::{
    ControlQueue, Objects, migrate_owner_database, open_service_database,
};
use quantrule_runtime::client::{ClientError, DataClient, HttpServiceClient, Service};
use quantrule_runtime::config::{ConfigError, migrate_only_requested};
use quantrule_runtime::metrics::{self, Metrics};
use quantrule_runtime::queue::QueueKind;
use quantrule_runtime::request_id::RequestIds;
use quantrule_runtime::{
    Config, Drain, HealthRegistry, RuntimeError, Shutdown, logging, observability, watch_signal,
};
use quantrule_service_api::INTERNAL_API_VERSION;

use quantrule_evidence_service::evidence::EvidenceServices;
use quantrule_evidence_service::leader::{
    LEADER_LEASE_SECONDS, LeaderConsumer, LeaderLease, Leadership, release_leadership,
    spawn_leader_election,
};
use quantrule_evidence_service::lineage::{PgLineage, RunLineageStore};
use quantrule_evidence_service::outbox::RunOutbox;
use quantrule_evidence_service::postgres::{PgHypothesisStore, PgRunStore};
use quantrule_evidence_service::postgres_leader::PgLeaderLease;
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
/// The run lineage store a route handler reads.
type LineageHandle = Arc<dyn RunLineageStore>;

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("evidence", config.image_digest.as_ref())?;

    if migrate_only_requested() {
        return migrate_owner_database(
            &config,
            "runs",
            "run and research",
            &quantrule_evidence_service::migrations::available(),
        )
        .await;
    }

    let (trigger, shutdown) = Shutdown::channel();
    let metrics = Metrics::new(
        "evidence",
        env!("CARGO_PKG_VERSION"),
        config.image_digest.as_ref().map(ToString::to_string),
    );
    let drain = Drain::new(
        shutdown.clone(),
        Some(metrics.clone()),
        config.drain_timeout_http,
    );
    metrics::start(&config, &metrics).await?;
    let mut health = HealthRegistry::named("evidence")
        .with_metrics(metrics.clone())
        .with_shutdown(shutdown.clone());
    let opened = open_service_database(&config, &mut health).await?;
    let pool = opened.as_ref().map(|database| database.pool().clone());
    // One store serves the run lifecycle, the notification outbox, and the
    // release tombstones. They are three traits over one database, so a run
    // write and its intent cannot diverge.
    let run_store: Option<Arc<PgRunStore>> = pool
        .as_ref()
        .map(|pool| Arc::new(PgRunStore::new(pool.clone())));
    let leader_lease: Option<Arc<dyn LeaderLease>> = pool.as_ref().map(|pool| {
        Arc::new(PgLeaderLease::with_lease_seconds(
            pool.clone(),
            LEADER_LEASE_SECONDS,
        )) as Arc<dyn LeaderLease>
    });
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
    let lineage: Option<LineageHandle> = pool
        .as_ref()
        .map(|pool| Arc::new(PgLineage::new(pool.clone())) as Arc<dyn RunLineageStore>);

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

    let data: Arc<dyn DataClient> = Arc::new(
        HttpServiceClient::from_env(Service::Data)
            .map_err(data_config_error)?
            .with_retry(metrics.clone(), shutdown.clone()),
    );

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
            lineage,
            objects: Some(Arc::clone(&objects)),
            queue: Some(publish),
            data: Some(data),
        },
    ));

    // Background work observes the same drain channel as the HTTP server. Each
    // loop stops at its next pass boundary; the caller waits for them up to
    // `QR_DRAIN_TIMEOUT_WORK` and aborts what remains.
    let mut background = Vec::new();

    // The publisher reconciles the live runs with the outbox, then drains it.
    // Its first pass is a startup pass, so a restarted process republishes
    // every unfinished run into an empty or rebuilt queue.
    if let (Some(outbox), Some(queue)) = (
        state.outbox.as_ref().map(Arc::clone),
        state.queue.as_ref().map(Arc::clone),
    ) {
        background.push(quantrule_evidence_service::outbox::spawn_publisher(
            outbox,
            queue,
            quantrule_evidence_service::outbox::PUBLISH_INTERVAL,
            shutdown.clone(),
            Some(metrics.clone()),
        ));
    }

    // Rows written before the dataset-reference registry existed are registered
    // once. The sweep is idempotent and retries until the data service answers;
    // a missing reference blocks a dataset erase, which is the safe direction.
    if let (Some(runs), Some(improvements), Some(data)) = (
        reconcile_runs,
        reconcile_improvements,
        state.data.as_ref().map(Arc::clone),
    ) {
        background.push(quantrule_evidence_service::reconcile::spawn_register_all(
            runs,
            improvements,
            data,
            Duration::from_secs(30),
            shutdown.clone(),
        ));
    }

    // The release sweep retries every tombstone the erase paths wrote. A
    // release is idempotent on the data side; a `404` means the dataset is
    // already gone, so the tombstone is confirmed rather than retried forever.
    if let (Some(releases), Some(data)) = (
        state.releases.as_ref().map(Arc::clone),
        state.data.as_ref().map(Arc::clone),
    ) {
        background.push(quantrule_evidence_service::reconcile::spawn_release_all(
            releases,
            data,
            Duration::from_secs(30),
            shutdown.clone(),
        ));
    }

    // Retention sweeps at startup and hourly. It names only the stage schema
    // and the stage object namespace; a run, a research record, and a run
    // artifact key are untouched. The outcome is recorded on the shared
    // operations status, success or failure, and the service still starts
    // because the health probes say what is down.
    if let Some(stages) = stages.as_ref() {
        background.push(tokio::spawn(retention::sweep(
            Arc::clone(stages),
            Arc::clone(&objects),
            Arc::clone(&state.operations),
            Arc::clone(&state.stage_gate),
            config.stage_retention_days,
            retention::SWEEP_INTERVAL,
            shutdown.clone(),
        )));
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

    // The lease loop starts before the HTTP server. A standby consumes nothing
    // until it acquires the lease; reads are served either way.
    let holder = leader_holder();
    let leadership = leader_lease.clone().map(|lease| {
        let (sender, receiver) = tokio::sync::watch::channel(Leadership::Standby);
        background.push(spawn_leader_election(
            lease,
            holder.clone(),
            sender,
            shutdown.clone(),
        ));
        receiver
    });

    // A takeover is a recovery point: re-arm every unfinished run when this
    // replica acquires the lease, even though it may not have restarted.
    if let (Some(outbox), Some(publish), Some(receiver)) = (
        state.outbox.as_ref().map(Arc::clone),
        state.queue.as_ref().map(Arc::clone),
        leadership.as_ref().cloned(),
    ) {
        background.push(quantrule_evidence_service::outbox::spawn_leader_reconcile(
            receiver,
            outbox,
            publish,
            shutdown.clone(),
        ));
    }

    // The leader and queue gauges are observations, never finalisation inputs.
    if let Some(receiver) = leadership.as_ref().cloned() {
        background.push(watch_leadership(
            receiver,
            metrics.clone(),
            shutdown.clone(),
        ));
    }
    if let Some(store) = run_store.as_ref() {
        background.push(watch_missing_digest(
            Arc::clone(store),
            metrics.clone(),
            shutdown.clone(),
        ));
    }
    background.push(watch_queue_depth(
        Arc::clone(&queue),
        evidence_queue.clone(),
        metrics.clone(),
        shutdown.clone(),
    ));

    std::mem::drop(watch_signal(trigger.clone()));
    tracing::info!(
        bind = %config.bind,
        contract = INTERNAL_API_VERSION,
        "starting evidence service"
    );
    let server = quantrule_runtime::serve_config(
        &config,
        observability::instrument(
            router(state),
            metrics.clone(),
            RequestIds::from_env("evidence"),
        ),
        &drain,
    );
    let consumer = {
        let metrics = metrics.clone();
        let shutdown = shutdown.clone();
        let leadership = leadership.clone();
        async move {
            let (Some(evidence), Some(leadership)) = (evidence, leadership) else {
                let mut wait = shutdown.clone();
                wait.wait().await;
                return Ok::<(), String>(());
            };
            let consumer: Arc<dyn QueueConsumer> = queue;
            LeaderConsumer::new(consumer, evidence_queue, evidence)
                .with_metrics(metrics)
                .run(leadership, shutdown)
                .await
                .map_err(|error| error.to_string())?;
            Ok(())
        }
    };

    let mut server = Box::pin(server);
    let mut consumer = Box::pin(consumer);
    let mut server_done = false;
    let mut consumer_done = false;
    let mut failure = None;
    tokio::select! {
        result = server.as_mut() => {
            server_done = true;
            if let Err(error) = result {
                failure = Some(error);
            }
        }
        result = consumer.as_mut() => {
            consumer_done = true;
            if let Err(error) = result {
                failure = Some(RuntimeError::Queue(error));
            }
        }
    }

    // A consumer or server failure is also a drain: stop the other side and the
    // background work before the process exits. A signal already triggered the
    // same path.
    trigger.trigger();
    let deadline = tokio::time::Instant::now() + config.drain_timeout_work;
    let mut deadline_exceeded = false;
    if !consumer_done
        && tokio::time::timeout_at(deadline, consumer.as_mut())
            .await
            .is_err()
    {
        deadline_exceeded = true;
    }
    let stop = async {
        for handle in &mut background {
            let _ = handle.await;
        }
    };
    if tokio::time::timeout_at(deadline, stop).await.is_err() {
        deadline_exceeded = true;
        for handle in &background {
            handle.abort();
        }
    }
    if deadline_exceeded {
        tracing::warn!("drain.deadline_exceeded");
    }
    if let (Some(lease), Some(receiver)) = (leader_lease.as_ref(), leadership.as_ref())
        && let Err(error) = release_leadership(lease.as_ref(), &holder, receiver).await
    {
        tracing::warn!(%error, "the evidence leader lease was not released");
    }
    metrics.set_leader(false);
    metrics.set_leader_term(0);
    if !server_done {
        let _ = server.as_mut().await;
    }
    failure.map_or(Ok(()), Err)
}

/// Follow the leadership watch and update the internal gauges.
fn watch_leadership(
    mut leadership: tokio::sync::watch::Receiver<Leadership>,
    metrics: Metrics,
    shutdown: Shutdown,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match *leadership.borrow_and_update() {
                Leadership::Leader(term) => {
                    metrics.set_leader(true);
                    metrics.set_leader_term(term);
                }
                Leadership::Standby => {
                    metrics.set_leader(false);
                    metrics.set_leader_term(0);
                }
            }
            let mut drain = shutdown.clone();
            tokio::select! {
                biased;
                () = drain.wait() => return,
                changed = leadership.changed() => {
                    if changed.is_err() {
                        return;
                    }
                }
            }
        }
    })
}

/// Poll the succeeded-run missing-digest count for the partial-publish gauge.
fn watch_missing_digest(
    store: Arc<PgRunStore>,
    metrics: Metrics,
    shutdown: Shutdown,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match store.missing_digest_count().await {
                Ok(missing) => metrics.set_runs_missing_digest(missing),
                Err(error) => {
                    tracing::debug!(%error, "the missing-digest count is unavailable")
                }
            }
            let mut drain = shutdown.clone();
            tokio::select! {
                biased;
                () = drain.wait() => return,
                () = tokio::time::sleep(Duration::from_secs(30)) => {}
            }
        }
    })
}

/// Poll the declared evidence queue depth for the internal gauge.
fn watch_queue_depth(
    queue: Arc<ControlQueue>,
    name: String,
    metrics: Metrics,
    shutdown: Shutdown,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match queue.depth(&name).await {
                Ok(depth) => metrics.set_queue_depth("evidence", depth),
                Err(error) => tracing::debug!(%error, "the evidence queue depth is unavailable"),
            }
            let mut drain = shutdown.clone();
            tokio::select! {
                biased;
                () = drain.wait() => return,
                () = tokio::time::sleep(Duration::from_secs(10)) => {}
            }
        }
    })
}

/// The identity this replica records in the leader lease.
///
/// `QR_LEADER_ID` names the replica explicitly. The hostname and process id
/// keep the default unique per container, and the term, not the holder, is the
/// fence.
fn leader_holder() -> String {
    std::env::var("QR_LEADER_ID")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            std::env::var("HOSTNAME")
                .ok()
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        })
        .map_or_else(
            || format!("evidence-{}", std::process::id()),
            |host| format!("evidence-{host}-{}", std::process::id()),
        )
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
