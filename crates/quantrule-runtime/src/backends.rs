//! Backend construction for the server profile.
//!
//! Every service branches once, here, instead of in a route or a use case. The
//! released backends are Postgres, an S3-compatible object store, and RabbitMQ.

use std::sync::Arc;

use crate::adapters::postgres::{PgDatabase, apply_migrations_in as apply_postgres_migrations};
use crate::adapters::rabbitmq::RabbitQueue;
use crate::adapters::s3::S3Store;
use crate::adapters::{
    Backend, BoxFuture, Database, MessageHandler, ObjectStore, Queue, QueueConsumer, QueueError,
    StoreError,
};
use crate::config::Config;
use crate::error::RuntimeError;
use crate::health::HealthRegistry;
use crate::migrations::Migration;

/// Opens the service's database, applies its ledger, and registers its health.
///
/// The service names its own ledger and migration set. Returns `Ok(None)` when
/// Postgres cannot be reached: the failure is registered as a health component
/// so the service starts and reports degraded.
///
/// # Errors
///
/// Returns [`RuntimeError::Migrations`] when the ledger cannot be applied.
pub async fn open_service_database(
    config: &Config,
    ledger: &str,
    label: &str,
    postgres_migrations: &[Migration],
    health: &mut HealthRegistry,
) -> Result<Option<PgDatabase>, RuntimeError> {
    match PgDatabase::connect(&config.database_url).await {
        Ok(database) => {
            if config.migrate {
                let applied =
                    apply_postgres_migrations(database.pool(), ledger, postgres_migrations)
                        .await
                        .map_err(|error| RuntimeError::Migrations(error.to_string()))?;
                tracing::info!(applied, "{} migrations applied", label);
            }
            let ping = database.clone();
            health.register("postgres", move || {
                let database = ping.clone();
                async move { database.ping().await.map_err(|error| error.to_string()) }
            });
            Ok(Some(database))
        }
        Err(error) => {
            health.register("postgres", move || {
                let message = error.to_string();
                async move { Err(message) }
            });
            Ok(None)
        }
    }
}

/// The object store selected by the server profile.
#[derive(Debug, Clone)]
pub struct Objects {
    store: S3Store,
}

impl Objects {
    /// Build the S3-compatible object store.
    ///
    /// An unreachable endpoint is a health report, not a startup error.
    pub async fn open(config: &Config) -> Result<Self, RuntimeError> {
        Ok(Self {
            store: S3Store::new(&config.object_store_url, &config.object_store_bucket).await,
        })
    }

    /// The wire name of the selected backend, for logs and tests.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.backend().as_str()
    }

    /// Whether the selected backend answers.
    ///
    /// # Errors
    ///
    /// Returns the adapter's message when the probe fails.
    pub async fn check(&self) -> Result<(), StoreError> {
        self.store.check().await
    }
}

impl ObjectStore for Objects {
    fn backend(&self) -> Backend {
        Backend::S3
    }

    fn put(&self, key: &str, bytes: Vec<u8>) -> BoxFuture<Result<(), StoreError>> {
        self.store.put(key, bytes)
    }

    fn get(&self, key: &str) -> BoxFuture<Result<Vec<u8>, StoreError>> {
        self.store.get(key)
    }

    fn exists(&self, key: &str) -> BoxFuture<Result<bool, StoreError>> {
        self.store.exists(key)
    }

    fn delete(&self, key: &str) -> BoxFuture<Result<(), StoreError>> {
        self.store.delete(key)
    }
}

/// The control-plane queue selected by the server profile.
#[derive(Debug, Clone)]
pub struct ControlQueue {
    queue: Arc<RabbitQueue>,
}

impl ControlQueue {
    /// Build the RabbitMQ queue.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Queue`] when the broker cannot be reached.
    pub async fn open(config: &Config) -> Result<Self, RuntimeError> {
        Ok(Self {
            queue: Arc::new(
                RabbitQueue::connect(&config.queue_url)
                    .await
                    .map_err(|error| RuntimeError::Queue(error.to_string()))?,
            ),
        })
    }

    /// The wire name of the selected backend, for logs and tests.
    #[must_use]
    pub fn name(&self) -> &'static str {
        Queue::backend(self).as_str()
    }

    /// Whether the selected queue answers.
    ///
    /// # Errors
    ///
    /// Returns the adapter's message when the probe fails.
    pub async fn check(&self) -> Result<(), QueueError> {
        self.queue.check()
    }
}

impl Queue for ControlQueue {
    fn backend(&self) -> Backend {
        Backend::RabbitMq
    }

    fn publish(&self, routing_key: &str, payload: Vec<u8>) -> BoxFuture<Result<(), QueueError>> {
        self.queue.publish(routing_key, payload)
    }
}

impl QueueConsumer for ControlQueue {
    fn backend(&self) -> Backend {
        Queue::backend(self)
    }

    fn consume(
        &self,
        name: &str,
        keys: &[&str],
        handler: MessageHandler,
    ) -> BoxFuture<Result<(), QueueError>> {
        self.queue.consume(name, keys, handler)
    }
}
