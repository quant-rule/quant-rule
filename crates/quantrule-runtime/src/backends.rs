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
use crate::config::{Config, ConfigError};
use crate::error::RuntimeError;
use crate::health::HealthRegistry;
use crate::migrations::Migration;
use crate::queue::QueueNames;

/// Applies the service ledger once with the database owner role.
///
/// This is the body of `--migrate-only`: the process opens the owner URL,
/// applies the ledger, logs the result, and returns so the binary exits before
/// it creates the application pool or binds HTTP. The owner credential is
/// therefore present only in the one-shot migration job, never in the
/// long-running service environment.
///
/// # Errors
///
/// Returns [`RuntimeError::Config`] when the owner URL is absent,
/// [`RuntimeError::Database`] when the owner connection fails, and
/// [`RuntimeError::Migrations`] when the ledger cannot be applied.
pub async fn migrate_owner_database(
    config: &Config,
    ledger: &str,
    label: &str,
    postgres_migrations: &[Migration],
) -> Result<(), RuntimeError> {
    let owner_url = config
        .database_owner_url
        .as_deref()
        .ok_or(ConfigError::Missing {
            name: "QR_DATABASE_OWNER_URL",
        })?;
    let owner = PgDatabase::connect(owner_url)
        .await
        .map_err(|error| RuntimeError::Database(error.to_string()))?;
    let applied = apply_postgres_migrations(owner.pool(), ledger, postgres_migrations)
        .await
        .map_err(|error| RuntimeError::Migrations(error.to_string()))?;
    tracing::info!(applied, "{} migrations applied by the migration job", label);
    Ok(())
}

/// Opens the service's application database and registers its health.
///
/// The long-running service owns no migration path: `QR_MIGRATE` must be `0`
/// and `QR_DATABASE_OWNER_URL` must be absent. A service that receives either
/// fails closed instead of holding DDL or owner credentials. Returns `Ok(None)`
/// when Postgres cannot be reached: the failure is registered as a health
/// component so the service starts and reports degraded.
///
/// # Errors
///
/// Returns [`RuntimeError::Config`] when migration is enabled or an owner URL is
/// present.
pub async fn open_service_database(
    config: &Config,
    health: &mut HealthRegistry,
) -> Result<Option<PgDatabase>, RuntimeError> {
    if config.database_owner_url.is_some() {
        return Err(RuntimeError::Config(ConfigError::Invalid {
            name: "QR_DATABASE_OWNER_URL",
            value: "<set>".to_owned(),
            reason: "belongs to the --migrate-only job, not the long-running service",
        }));
    }
    if config.migrate {
        return Err(RuntimeError::Config(ConfigError::Invalid {
            name: "QR_MIGRATE",
            value: "1".to_owned(),
            reason: "the long-running service must not migrate; run the migration job",
        }));
    }

    match PgDatabase::connect(&config.database_url).await {
        Ok(database) => {
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
                RabbitQueue::connect(&config.queue_url, config.queue_names())
                    .await
                    .map_err(|error| RuntimeError::Queue(error.to_string()))?,
            ),
        })
    }

    /// The queue generation this connection declared and consumes.
    #[must_use]
    pub fn names(&self) -> &QueueNames {
        self.queue.names()
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

    fn consume(&self, name: &str, handler: MessageHandler) -> BoxFuture<Result<(), QueueError>> {
        self.queue.consume(name, handler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_long_running_service_refuses_an_owner_url() {
        let config = Config {
            migrate: false,
            database_owner_url: Some(
                "postgres://qr_gateway_owner:secret@postgres:5432/qr_gateway".to_owned(),
            ),
            ..Config::default()
        };
        let mut health = HealthRegistry::new();
        let error = open_service_database(&config, &mut health)
            .await
            .expect_err("an owner URL must be refused");
        assert!(error.to_string().contains("QR_DATABASE_OWNER_URL"));
    }

    #[tokio::test]
    async fn a_long_running_service_refuses_qr_migrate() {
        let config = Config {
            migrate: true,
            ..Config::default()
        };
        let mut health = HealthRegistry::new();
        let error = open_service_database(&config, &mut health)
            .await
            .expect_err("QR_MIGRATE=1 must be refused");
        assert!(error.to_string().contains("QR_MIGRATE"));
    }
}
