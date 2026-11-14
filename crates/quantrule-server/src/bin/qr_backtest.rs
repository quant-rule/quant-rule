//! The backtest worker binary.
//!
//! It consumes `runs.backtest` (`run.requested` and `run.cancelled`), claims the
//! run, executes it, writes the artifacts to CAS, records the computed digest,
//! and publishes `run.computed` or `run.failed`. Evidence finalizes the run. A
//! retryable failure holds the delivery, waits, and retries in place. It serves
//! no HTTP.

use std::sync::Arc;

use quantrule_runtime::adapters::postgres::{PgDatabase, apply_migrations_in};
use quantrule_runtime::adapters::rabbitmq::RabbitQueue;
use quantrule_runtime::adapters::s3::S3Store;
use quantrule_runtime::adapters::{
    Message, MessageHandler, ObjectStore, Queue, QueueConsumer, QueueError,
};
use quantrule_runtime::client::{ClientError, DataClient, HttpServiceClient, Service};
use quantrule_runtime::config::ConfigError;
use quantrule_runtime::queue::{QUEUE_BACKTEST, ROUTING_CANCELLED, ROUTING_REQUESTED};
use quantrule_runtime::{Config, RuntimeError, logging, shutdown};
use quantrule_server::adapters::postgres::PgRunStore;
use quantrule_server::backtest::{ProcessOutcome, WorkerServices, process};
use quantrule_server::runs::RunStore;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;

    let database = PgDatabase::connect(&config.database_url)
        .await
        .map_err(|error| RuntimeError::Database(error.to_string()))?;
    if config.migrate {
        apply_migrations_in(
            database.pool(),
            "runs",
            &quantrule_server::migrations::service(),
        )
        .await
        .map_err(|error| RuntimeError::Migrations(error.to_string()))?;
    }
    let objects = S3Store::new(&config.object_store_url, &config.object_store_bucket).await;
    let queue = Arc::new(
        RabbitQueue::connect(&config.queue_url)
            .await
            .map_err(|error| RuntimeError::Queue(error.to_string()))?,
    );

    let data: Arc<dyn DataClient> =
        Arc::new(HttpServiceClient::from_env(Service::Data).map_err(|error| {
            RuntimeError::Config(match error {
                ClientError::Unconfigured { .. } => ConfigError::Missing {
                    name: "QR_DATA_URL",
                },
                _ => ConfigError::Invalid {
                    name: "QR_DATA_URL",
                    value: "<invalid>".to_owned(),
                    reason: "must be an absolute HTTP URL",
                },
            })
        })?);
    let services = Arc::new(WorkerServices {
        runs: Arc::new(PgRunStore::new(database.pool().clone())) as Arc<dyn RunStore>,
        data,
        objects: Arc::new(objects) as Arc<dyn ObjectStore>,
        queue: Arc::clone(&queue) as Arc<dyn Queue>,
    });

    let handler: MessageHandler = Box::new(move |message: Message| {
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
                // The gateway already wrote the flag. The message is an
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
            let mut redelivered = message.redelivered;
            loop {
                match process(run_id.clone(), redelivered, &services).await {
                    Ok(ProcessOutcome::Retry { delay }) => {
                        tracing::warn!(run_id = %run_id, ?delay, "retrying the run");
                        tokio::time::sleep(delay).await;
                        redelivered = true;
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
    });

    tracing::info!("backtest worker consuming {QUEUE_BACKTEST}");
    tokio::select! {
        result = queue.consume(
            QUEUE_BACKTEST,
            &[ROUTING_REQUESTED, ROUTING_CANCELLED],
            handler,
        ) => {
            result.map_err(|error| RuntimeError::Queue(error.to_string()))
        }
        _ = shutdown::signal() => Ok(()),
    }
}

fn run_id(payload: &[u8]) -> Option<String> {
    serde_json::from_slice::<Value>(payload)
        .ok()
        .and_then(|value| {
            value
                .get("run_id")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
}
