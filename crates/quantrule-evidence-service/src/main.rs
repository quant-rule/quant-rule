//! The evidence service binary.
//!
//! It connects to Postgres, the object store, the broker, and the data service.
//! It applies only the `runs`/`research` migrations, serves the public run and
//! hypothesis surface plus the internal lifecycle, and consumes `run.computed`
//! to verify the hash chain and finalize the run.

#![forbid(unsafe_code)]

use std::sync::Arc;

use quantrule_runtime::adapters::postgres::{PgDatabase, apply_migrations_in};
use quantrule_runtime::adapters::rabbitmq::RabbitQueue;
use quantrule_runtime::adapters::s3::S3Store;
use quantrule_runtime::adapters::{
    Database, Message, MessageHandler, ObjectStore, Queue, QueueConsumer, QueueError,
};
use quantrule_runtime::client::{ClientError, DataClient, HttpServiceClient, Service};
use quantrule_runtime::config::ConfigError;
use quantrule_runtime::queue::{QUEUE_EVIDENCE, ROUTING_COMPUTED};
use quantrule_runtime::{Config, HealthRegistry, RuntimeError, logging};
use quantrule_service_api::INTERNAL_API_VERSION;

use quantrule_evidence_service::evidence::{EvidenceServices, finalize};
use quantrule_evidence_service::postgres::{PgHypothesisStore, PgRunStore};
use quantrule_evidence_service::runs::RunStore;
use quantrule_evidence_service::{EvidenceState, router};

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;

    let mut health = HealthRegistry::new();
    let mut runs: Option<Arc<dyn RunStore>> = None;
    let mut hypotheses: Option<Arc<dyn quantrule_evidence_service::research::HypothesisStore>> =
        None;

    match PgDatabase::connect(&config.database_url).await {
        Ok(database) => {
            if config.migrate {
                let applied = apply_migrations_in(
                    database.pool(),
                    "runs",
                    &quantrule_evidence_service::migrations::available(),
                )
                .await
                .map_err(|error| RuntimeError::Migrations(error.to_string()))?;
                tracing::info!(applied, "run and research migrations applied");
            }
            runs = Some(Arc::new(PgRunStore::new(database.pool().clone())));
            hypotheses = Some(Arc::new(PgHypothesisStore::new(database.pool().clone())));
            health.register("postgres", move || {
                let database = database.clone();
                async move { database.ping().await.map_err(|error| error.to_string()) }
            })
        }
        Err(error) => health.register("postgres", move || {
            let message = error.to_string();
            async move { Err(message) }
        }),
    }

    let s3 = S3Store::new(&config.object_store_url, &config.object_store_bucket).await;
    let objects: Arc<dyn ObjectStore> = Arc::new(s3.clone());
    health.register("object-store", move || {
        let store = s3.clone();
        async move { store.check().await.map_err(|error| error.to_string()) }
    });

    let queue = Arc::new(
        RabbitQueue::connect(&config.queue_url)
            .await
            .map_err(|error| RuntimeError::Queue(error.to_string()))?,
    );
    health.register("queue", {
        let queue = Arc::clone(&queue);
        move || {
            let queue = Arc::clone(&queue);
            async move { queue.check().map_err(|error| error.to_string()) }
        }
    });

    let data: Arc<dyn DataClient> =
        Arc::new(HttpServiceClient::from_env(Service::Data).map_err(data_config_error)?);

    let state = Arc::new(EvidenceState::with_services(
        &config,
        health,
        quantrule_evidence_service::EvidenceServices {
            runs: runs.as_ref().map(Arc::clone),
            hypotheses,
            objects: Some(Arc::clone(&objects)),
            queue: Some(Arc::clone(&queue) as Arc<dyn Queue>),
            data: Some(data),
        },
    ));

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
