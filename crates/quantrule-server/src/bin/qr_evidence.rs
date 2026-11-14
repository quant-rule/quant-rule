//! The evidence binary.
//!
//! It consumes `run.computed`, verifies the run's hash chain and canonical
//! digest, and finalizes the run. It serves no HTTP.

use std::sync::Arc;

use quantrule_server::evidence::{EvidenceServices, finalize};
use quantrule_server::runs::RunStore;
use quantrule_server::runtime::adapters::postgres::PgDatabase;
use quantrule_server::runtime::adapters::postgres::PgRunStore;
use quantrule_server::runtime::adapters::postgres::apply_migrations;
use quantrule_server::runtime::adapters::rabbitmq::RabbitQueue;
use quantrule_server::runtime::adapters::s3::S3Store;
use quantrule_server::runtime::adapters::{
    Message, MessageHandler, ObjectStore, QueueConsumer, QueueError,
};
use quantrule_server::runtime::queue::{QUEUE_EVIDENCE, ROUTING_COMPUTED};
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

    let services = Arc::new(EvidenceServices {
        runs: Arc::new(PgRunStore::new(database.pool().clone())) as Arc<dyn RunStore>,
        objects: Arc::new(objects) as Arc<dyn ObjectStore>,
    });

    let handler: MessageHandler = Box::new(move |message: Message| {
        let services = Arc::clone(&services);
        Box::pin(async move {
            let run_id = serde_json::from_slice::<Value>(&message.payload)
                .ok()
                .and_then(|value| {
                    value
                        .get("run_id")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                });
            let Some(run_id) = run_id else {
                tracing::error!("run.computed carries no run_id; acknowledging");
                return Ok(());
            };
            match finalize(run_id.clone(), &services).await {
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

    tracing::info!("evidence consuming run.computed");
    tokio::select! {
        result = queue.consume(QUEUE_EVIDENCE, &[ROUTING_COMPUTED], handler) => {
            result.map_err(|error| RuntimeError::Queue(error.to_string()))
        }
        _ = shutdown::signal() => Ok(()),
    }
}
