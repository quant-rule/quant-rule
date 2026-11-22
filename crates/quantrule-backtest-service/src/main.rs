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

use quantrule_api::StageKind;
use quantrule_backtest_service::backtest::{ProcessOutcome, WorkerServices, process};
use quantrule_backtest_service::stage::{
    ModelStageExecutor, PortfolioStageExecutor, StageDispatch, StageProcessOutcome,
    StageWorkerServices, StrategyStageExecutor, process_stage,
};
use quantrule_runtime::adapters::{
    Message, MessageHandler, ObjectStore, Queue, QueueConsumer, QueueError,
};
use quantrule_runtime::backends::{ControlQueue, Objects};
use quantrule_runtime::client::{
    ClientError, DataClient, EvidenceClient, HttpServiceClient, ModelClient, Service, StageClient,
};
use quantrule_runtime::config::ConfigError;
use quantrule_runtime::queue::{
    QueueKind, ROUTING_CANCELLED, ROUTING_REQUESTED, ROUTING_STAGE_CANCELLED,
    ROUTING_STAGE_REQUESTED,
};
use quantrule_runtime::{Config, RuntimeError, logging};
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;

    let objects: Arc<dyn ObjectStore> = Arc::new(Objects::open(&config).await?);
    let queue = Arc::new(ControlQueue::open(&config).await?);
    let names = queue.names().clone();

    let data: Arc<dyn DataClient> = Arc::new(
        HttpServiceClient::from_env(Service::Data)
            .map_err(|error| service_config_error(Service::Data, error))?,
    );
    let model: Arc<dyn ModelClient> = Arc::new(
        HttpServiceClient::from_env(Service::Model)
            .map_err(|error| service_config_error(Service::Model, error))?,
    );
    let evidence: Arc<dyn EvidenceClient> = Arc::new(
        HttpServiceClient::from_env(Service::Evidence)
            .map_err(|error| service_config_error(Service::Evidence, error))?,
    );
    let stage_evidence: Arc<dyn StageClient> = Arc::new(
        HttpServiceClient::from_env(Service::Evidence)
            .map_err(|error| service_config_error(Service::Evidence, error))?,
    );
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

    let mut consumers = tokio::task::JoinSet::new();
    for _ in 0..config.backtest_concurrency {
        let queue = Arc::clone(&queue);
        let handler = make_run_handler(Arc::clone(&services));
        let queue_name = names.name(QueueKind::Backtest).to_owned();
        consumers.spawn(async move { queue.consume(&queue_name, handler).await });
    }
    let stage_queue = Arc::clone(&queue);
    let stage_handler = make_stage_handler(Arc::clone(&stage_services), Arc::clone(&dispatch));
    let stage_name = names.name(QueueKind::Stage).to_owned();
    consumers.spawn(async move { stage_queue.consume(&stage_name, stage_handler).await });

    tracing::info!(
        runs = config.backtest_concurrency,
        queue = names.name(QueueKind::Backtest),
        stage_queue = names.name(QueueKind::Stage),
        "backtest worker consuming the generated run and stage queues"
    );
    tokio::select! {
        _ = quantrule_runtime::shutdown::signal() => Ok(()),
        result = consumers.join_next() => match result {
            Some(Ok(result)) => result.map_err(|error| RuntimeError::Queue(error.to_string())),
            Some(Err(error)) => Err(RuntimeError::Queue(format!(
                "a consumer task failed: {error}"
            ))),
            None => Ok(()),
        },
    }
}

fn make_run_handler(services: Arc<WorkerServices>) -> MessageHandler {
    Box::new(move |message: Message| {
        let services = Arc::clone(&services);
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
                        tracing::warn!(run_id = %run_id, ?delay, term, "retrying the run");
                        tokio::time::sleep(delay).await;
                        continuation = Some(term);
                    }
                    Ok(outcome) => {
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
) -> MessageHandler {
    Box::new(move |message: Message| {
        let services = Arc::clone(&services);
        let dispatch = Arc::clone(&dispatch);
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
            loop {
                match process_stage(stage_run_id.clone(), redelivered, &services, &dispatch).await {
                    Ok(StageProcessOutcome::Retry { delay }) => {
                        tracing::warn!(stage_run_id = %stage_run_id, ?delay, "retrying the stage");
                        tokio::time::sleep(delay).await;
                        redelivered = true;
                    }
                    Ok(outcome) => {
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
