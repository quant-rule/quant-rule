//! The backtest worker binary.
//!
//! It consumes `runs.backtest` (`run.requested` and `run.cancelled`), claims the
//! run, executes it, writes the artifacts to CAS, records the computed digest,
//! and publishes `run.computed` or `run.failed`. Evidence finalizes the run. A
//! retryable failure holds the delivery, waits, and retries in place. It serves
//! no HTTP.

use std::sync::Arc;

use quantrule_server::backtest::{ProcessOutcome, WorkerServices, process};
use quantrule_server::configs::ConfigCatalog;
use quantrule_server::data::DatasetCatalog;
use quantrule_server::runs::RunStore;
use quantrule_server::runtime::adapters::postgres::PgConfigStore;
use quantrule_server::runtime::adapters::postgres::PgDatabase;
use quantrule_server::runtime::adapters::postgres::PgDatasetCatalog;
use quantrule_server::runtime::adapters::postgres::PgRunStore;
use quantrule_server::runtime::adapters::postgres::apply_migrations;
use quantrule_server::runtime::adapters::rabbitmq::RabbitQueue;
use quantrule_server::runtime::adapters::s3::S3Store;
use quantrule_server::runtime::adapters::{
    Message, MessageHandler, ObjectStore, Queue, QueueConsumer, QueueError,
};
use quantrule_server::runtime::queue::{QUEUE_BACKTEST, ROUTING_CANCELLED, ROUTING_REQUESTED};
use quantrule_server::runtime::shutdown;
use quantrule_server::runtime::{Config, RuntimeError, logging};
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;

    let database = PgDatabase::connect(&config.database_url)
        .await
        .map_err(|error| RuntimeError::Database(error.to_string()))?;
    if config.migrate {
        apply_migrations(database.pool())
            .await
            .map_err(|error| RuntimeError::Migrations(error.to_string()))?;
    }
    let objects = S3Store::new(&config.object_store_url, &config.object_store_bucket).await;
    let queue = Arc::new(
        RabbitQueue::connect(&config.queue_url)
            .await
            .map_err(|error| RuntimeError::Queue(error.to_string()))?,
    );

    let services = Arc::new(WorkerServices {
        runs: Arc::new(PgRunStore::new(database.pool().clone())) as Arc<dyn RunStore>,
        catalog: Arc::new(PgDatasetCatalog::new(database.pool().clone()))
            as Arc<dyn DatasetCatalog>,
        configs: Arc::new(PgConfigStore::new(database.pool().clone())) as Arc<dyn ConfigCatalog>,
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
