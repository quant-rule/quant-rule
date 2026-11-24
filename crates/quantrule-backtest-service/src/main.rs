//! The backtest worker binary.
//!
//! It consumes the generated `runs.backtest.<generation>` queue
//! (`run.requested` and `run.cancelled`), loads the
//! dataset and config from `qr-data`, obtains the model bundle from `qr-model`,
//! runs strategy, execution, and portfolio from that bundle, writes the
//! artifacts to CAS, records the computed digest through `qr-evidence`, and
//! publishes `run.computed` or `run.failed`. Evidence finalizes the run. A
//! retryable failure holds the delivery, waits, and retries in place. It serves
//! no HTTP and owns no database schema.
//!
//! The same process also consumes the generated `stages.worker.<generation>`
//! queue (`stage.requested` and `stage.cancelled`) through the diagnostic stage
//! worker. The run consumer is registered first, so a stage cannot delay an
//! evidence run. The model, strategy, and portfolio executors are wired here; a
//! stage never writes a run row, a canonical digest, or an evidence record.
//!
//! The worker uses RabbitMQ for the control-plane queue and S3 for artifact
//! bytes.

#![forbid(unsafe_code)]

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use quantrule_api::{HealthReport, StageKind};
use quantrule_backtest_service::backtest::{ProcessOutcome, WorkerServices, process};
use quantrule_backtest_service::stage::{
    ModelStageExecutor, PortfolioStageExecutor, StageDispatch, StageProcessOutcome,
    StageWorkerServices, StrategyStageExecutor, process_stage,
};
use quantrule_runtime::adapters::{Message, MessageHandler, ObjectStore, Queue, QueueError};
use quantrule_runtime::backends::{ControlQueue, Objects};
use quantrule_runtime::client::{
    ClientError, DataClient, EvidenceClient, HttpServiceClient, ModelClient, Service, StageClient,
};
use quantrule_runtime::config::ConfigError;
use quantrule_runtime::metrics::{self, Metrics};
use quantrule_runtime::queue::{
    QueueKind, ROUTING_CANCELLED, ROUTING_REQUESTED, ROUTING_STAGE_CANCELLED,
    ROUTING_STAGE_REQUESTED,
};
use quantrule_runtime::{
    Config, Drain, HealthRegistry, RuntimeError, Shutdown, logging, reconnect, watch_signal,
};
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("backtest", config.image_digest.as_ref())?;

    // The worker serves no public HTTP API. The internal listener carries
    // `/metrics`, `/health` and `/ready`, and it is disabled unless
    // QR_METRICS_BIND is set.
    let (trigger, mut shutdown) = Shutdown::channel();
    let metrics = Metrics::new(
        "backtest",
        env!("CARGO_PKG_VERSION"),
        config.image_digest.as_ref().map(ToString::to_string),
    );
    let drain = Drain::new(
        shutdown.clone(),
        Some(metrics.clone()),
        config.drain_timeout_http,
    );
    std::mem::drop(drain.spawn_watch());
    metrics.set_workers(u64::from(config.backtest_concurrency));

    let objects: Arc<dyn ObjectStore> = Arc::new(Objects::open(&config).await?);
    let queue = Arc::new(ControlQueue::open(&config).await?);
    let names = queue.names().clone();

    let data_http = HttpServiceClient::from_env(Service::Data)
        .map_err(|error| service_config_error(Service::Data, error))?
        .with_retry(metrics.clone(), shutdown.clone());
    let model_http = HttpServiceClient::from_env(Service::Model)
        .map_err(|error| service_config_error(Service::Model, error))?
        .with_retry(metrics.clone(), shutdown.clone());
    let evidence_http = HttpServiceClient::from_env(Service::Evidence)
        .map_err(|error| service_config_error(Service::Evidence, error))?
        .with_retry(metrics.clone(), shutdown.clone());

    let mut health = HealthRegistry::named("backtest")
        .with_metrics(metrics.clone())
        .with_shutdown(shutdown.clone());
    health.register("data", {
        let client = data_http.clone();
        move || {
            let client = client.clone();
            async move { probe_ready(&client, Service::Data).await }
        }
    });
    health.register("model", {
        let client = model_http.clone();
        move || {
            let client = client.clone();
            async move { probe_ready(&client, Service::Model).await }
        }
    });
    health.register("evidence", {
        let client = evidence_http.clone();
        move || {
            let client = client.clone();
            async move { probe_ready(&client, Service::Evidence).await }
        }
    });
    let internal = metrics::router(metrics.clone()).merge(
        axum::Router::new()
            .route("/health", get(internal_health))
            .route("/ready", get(internal_ready))
            .with_state(InternalHealth {
                health: health.clone(),
                digest: config.image_digest.clone(),
            }),
    );
    metrics::start_router(&config, internal).await?;

    let data: Arc<dyn DataClient> = Arc::new(data_http);
    let model: Arc<dyn ModelClient> = Arc::new(model_http);
    let evidence: Arc<dyn EvidenceClient> = Arc::new(evidence_http.clone());
    let stage_evidence: Arc<dyn StageClient> = Arc::new(evidence_http);
    let publish: Arc<dyn Queue> = queue.clone();
    let stage_publish: Arc<dyn Queue> = queue.clone();
    let stage_objects: Arc<dyn ObjectStore> = Arc::clone(&objects);
    let stage_model: Arc<dyn ModelClient> = Arc::clone(&model);
    let stage_data: Arc<dyn DataClient> = Arc::clone(&data);
    let stage_strategy_evidence: Arc<dyn StageClient> = Arc::clone(&stage_evidence);
    let stage_portfolio_evidence: Arc<dyn StageClient> = Arc::clone(&stage_evidence);
    let stage_portfolio_data: Arc<dyn DataClient> = Arc::clone(&data);
    let stage_portfolio_objects: Arc<dyn ObjectStore> = Arc::clone(&stage_objects);
    let holder = worker_holder();
    tracing::info!(%holder, "backtest worker identity");
    let services = Arc::new(WorkerServices {
        evidence,
        data,
        model,
        stages: Arc::clone(&stage_evidence),
        objects,
        queue: publish,
        holder,
    });
    let stage_services = Arc::new(StageWorkerServices {
        evidence: stage_evidence,
        objects: Arc::clone(&stage_objects),
        queue: stage_publish,
        holder: worker_holder(),
    });
    let dispatch = Arc::new(
        StageDispatch::new()
            .with(
                StageKind::Model,
                Arc::new(ModelStageExecutor::new(
                    stage_model,
                    Arc::clone(&stage_objects),
                )),
            )
            .with(
                StageKind::Strategy,
                Arc::new(StrategyStageExecutor::new(
                    stage_strategy_evidence,
                    stage_data,
                    Arc::clone(&stage_objects),
                )),
            )
            .with(
                StageKind::Portfolio,
                Arc::new(PortfolioStageExecutor::new(
                    stage_portfolio_evidence,
                    stage_portfolio_data,
                    stage_portfolio_objects,
                )),
            ),
    );

    let run_queue = names.name(QueueKind::Backtest).to_owned();
    let stage_name = names.name(QueueKind::Stage).to_owned();
    let mut consumers = tokio::task::JoinSet::new();
    for _ in 0..config.backtest_concurrency {
        let queue = Arc::clone(&queue);
        let handler = make_run_handler(Arc::clone(&services), metrics.clone());
        let queue_name = run_queue.clone();
        let shutdown = shutdown.clone();
        let metrics = metrics.clone();
        consumers.spawn(async move {
            reconnect::consume(
                queue,
                queue_name,
                handler,
                shutdown,
                "backtest",
                "run",
                Some(metrics),
            )
            .await
        });
    }
    let stage_queue = Arc::clone(&queue);
    let stage_handler = make_stage_handler(
        Arc::clone(&stage_services),
        Arc::clone(&dispatch),
        metrics.clone(),
    );
    let shutdown_for_stage = shutdown.clone();
    let metrics_for_stage = metrics.clone();
    let stage_queue_name = stage_name.clone();
    consumers.spawn(async move {
        reconnect::consume(
            stage_queue,
            stage_queue_name,
            stage_handler,
            shutdown_for_stage,
            "backtest",
            "stage",
            Some(metrics_for_stage),
        )
        .await
    });
    let depth = watch_queue_depth(
        Arc::clone(&queue),
        run_queue.clone(),
        stage_name.clone(),
        metrics.clone(),
        shutdown.clone(),
    );

    tracing::info!(
        runs = config.backtest_concurrency,
        queue = %run_queue,
        stage_queue = %stage_name,
        "backtest worker consuming the generated run and stage queues"
    );
    std::mem::drop(watch_signal(trigger.clone()));
    let mut failure = None;
    tokio::select! {
        _ = shutdown.wait() => {}
        result = consumers.join_next() => {
            failure = match result {
                Some(Ok(Ok(()))) => None,
                Some(Ok(Err(error))) => Some(RuntimeError::Queue(error.to_string())),
                Some(Err(error)) => Some(RuntimeError::Queue(format!(
                    "a consumer task failed: {error}"
                ))),
                None => None,
            };
            trigger.trigger();
        }
    }

    // Stop claiming at the signal and let the in-flight run or stage finish up
    // to the worker deadline. At the deadline the tasks are dropped: the run
    // keeps its lease and is recovered by lease expiry, never marked failed.
    if quantrule_backtest_service::drain::drain_consumers(&mut consumers, config.drain_timeout_work)
        .await
        == quantrule_backtest_service::drain::DrainOutcome::DeadlineExceeded
    {
        tracing::warn!("drain.deadline_exceeded");
    }
    depth.abort();
    failure.map_or(Ok(()), Err)
}

/// What the internal `/health` and `/ready` routes need.
#[derive(Clone)]
struct InternalHealth {
    health: HealthRegistry,
    digest: Option<quantrule_api::Digest>,
}

async fn internal_health(State(state): State<InternalHealth>) -> Json<HealthReport> {
    Json(state.health.report(state.digest.clone()).await)
}

async fn internal_ready(State(state): State<InternalHealth>) -> (StatusCode, Json<HealthReport>) {
    let digest = state.digest.clone();
    let (ready, report) = state.health.readiness(digest).await;
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(report))
}

/// A worker dependency is available only when its own readiness probe agrees.
async fn probe_ready(client: &HttpServiceClient, service: Service) -> Result<(), String> {
    match client.ready(service).await {
        Ok((200, report)) if report.state.is_available() => Ok(()),
        Ok((status, report)) => Err(format!(
            "{} answered {status} with state {:?}",
            service.name(),
            report.state
        )),
        Err(error) => Err(error.to_string()),
    }
}

/// Poll the run and stage queue depths for the internal gauges.
fn watch_queue_depth(
    queue: Arc<ControlQueue>,
    run: String,
    stage: String,
    metrics: Metrics,
    shutdown: Shutdown,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            for (label, name) in [("run", &run), ("stage", &stage)] {
                match queue.depth(name).await {
                    Ok(depth) => metrics.set_queue_depth(label, depth),
                    Err(error) => {
                        tracing::debug!(%error, queue = %name, "the queue depth is unavailable")
                    }
                }
            }
            let mut drain = shutdown.clone();
            tokio::select! {
                biased;
                () = drain.wait() => return,
                () = tokio::time::sleep(std::time::Duration::from_secs(10)) => {}
            }
        }
    })
}

fn make_run_handler(services: Arc<WorkerServices>, metrics: Metrics) -> MessageHandler {
    Arc::new(move |message: Message| {
        let services = Arc::clone(&services);
        let metrics = metrics.clone();
        Box::pin(async move {
            let Some(run_id) = run_id(&message.payload) else {
                tracing::error!(
                    routing_key = %message.routing_key,
                    "a control message carries no run_id; acknowledging"
                );
                return Ok(());
            };
            if message.routing_key == ROUTING_CANCELLED {
                // Evidence already wrote the flag. The message is an
                // observation; the worker checks the flag at its own boundaries.
                tracing::info!(run_id = %run_id, "cancellation observed");
                return Ok(());
            }
            if message.routing_key != ROUTING_REQUESTED {
                tracing::warn!(
                    routing_key = %message.routing_key,
                    "the backtest queue received an unexpected key; acknowledging"
                );
                return Ok(());
            }
            let mut continuation = None;
            loop {
                match process(run_id.clone(), continuation, &services).await {
                    Ok(ProcessOutcome::Retry { delay, term }) => {
                        metrics.record_run("retry");
                        tracing::warn!(run_id = %run_id, ?delay, term, "retrying the run");
                        tokio::time::sleep(delay).await;
                        continuation = Some(term);
                    }
                    Ok(outcome) => {
                        metrics.record_run(run_outcome(outcome));
                        tracing::info!(run_id = %run_id, outcome = ?outcome, "run processed");
                        return Ok(());
                    }
                    Err(error) => {
                        tracing::error!(run_id = %run_id, error = %error, "run processing failed");
                        return Err(QueueError(error.to_string()));
                    }
                }
            }
        })
    })
}

fn make_stage_handler(
    services: Arc<StageWorkerServices>,
    dispatch: Arc<StageDispatch>,
    metrics: Metrics,
) -> MessageHandler {
    Arc::new(move |message: Message| {
        let services = Arc::clone(&services);
        let dispatch = Arc::clone(&dispatch);
        let metrics = metrics.clone();
        Box::pin(async move {
            let Some(stage_run_id) = id(&message.payload, "stage_run_id") else {
                tracing::error!(
                    routing_key = %message.routing_key,
                    "a stage control message carries no stage_run_id; acknowledging"
                );
                return Ok(());
            };
            if message.routing_key == ROUTING_STAGE_CANCELLED {
                // Evidence already wrote the flag. The message is an
                // observation; the worker checks the row at its boundaries.
                tracing::info!(stage_run_id = %stage_run_id, "stage cancellation observed");
                return Ok(());
            }
            if message.routing_key != ROUTING_STAGE_REQUESTED {
                tracing::warn!(
                    routing_key = %message.routing_key,
                    "the stage queue received an unexpected key; acknowledging"
                );
                return Ok(());
            }
            let mut redelivered = message.redelivered;
            let mut continuation = None;
            loop {
                match process_stage(
                    stage_run_id.clone(),
                    continuation,
                    redelivered,
                    &services,
                    &dispatch,
                )
                .await
                {
                    Ok(StageProcessOutcome::Retry { delay, term }) => {
                        metrics.record_stage("retry");
                        tracing::warn!(stage_run_id = %stage_run_id, ?delay, "retrying the stage");
                        tokio::time::sleep(delay).await;
                        redelivered = true;
                        continuation = Some(term);
                    }
                    Ok(outcome) => {
                        metrics.record_stage(stage_outcome(outcome));
                        tracing::info!(stage_run_id = %stage_run_id, outcome = ?outcome, "stage processed");
                        return Ok(());
                    }
                    Err(error) => {
                        tracing::error!(stage_run_id = %stage_run_id, error = %error, "stage processing failed");
                        return Err(QueueError(error.to_string()));
                    }
                }
            }
        })
    })
}

fn run_outcome(outcome: ProcessOutcome) -> &'static str {
    match outcome {
        ProcessOutcome::Computed => "computed",
        ProcessOutcome::Failed => "failed",
        ProcessOutcome::Retry { .. } => "retry",
        ProcessOutcome::Exhausted => "exhausted",
        ProcessOutcome::Skipped => "skipped",
    }
}

fn stage_outcome(outcome: StageProcessOutcome) -> &'static str {
    match outcome {
        StageProcessOutcome::Computed => "computed",
        StageProcessOutcome::Failed => "failed",
        StageProcessOutcome::Retry { .. } => "retry",
        StageProcessOutcome::Exhausted => "exhausted",
        StageProcessOutcome::Skipped => "skipped",
    }
}

fn run_id(payload: &[u8]) -> Option<String> {
    id(payload, "run_id")
}

fn id(payload: &[u8], field: &str) -> Option<String> {
    serde_json::from_slice::<Value>(payload)
        .ok()
        .and_then(|value| value.get(field).and_then(Value::as_str).map(str::to_owned))
}

/// The worker identity recorded in a run lease.
///
/// `QR_WORKER_ID` names the process explicitly. The fallback keeps the field
/// populated for a single-host stack; the term, not the holder, is the fence.
fn worker_holder() -> String {
    std::env::var("QR_WORKER_ID")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("backtest-{}", std::process::id()))
}

/// Name the missing or invalid `QR_*_URL` at startup.
fn service_config_error(service: Service, error: ClientError) -> RuntimeError {
    let name = service.url_variable();
    RuntimeError::Config(match error {
        ClientError::Unconfigured { .. } => ConfigError::Missing { name },
        _ => ConfigError::Invalid {
            name,
            value: "<invalid>".to_owned(),
            reason: "must be an absolute HTTP URL",
        },
    })
}
