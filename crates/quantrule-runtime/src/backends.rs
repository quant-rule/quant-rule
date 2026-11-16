//! Backend construction for the two deployment profiles.
//!
//! Every service branches once, here, instead of in a route or a use case. The
//! server profile keeps the released backends: Postgres (chosen by the service,
//! because only it knows which migrations to apply), an S3-compatible object
//! store, and RabbitMQ. The desktop profile selects the embedded backends: one
//! SQLite file at `<QR_DATA_DIR>/quantrule.sqlite` (shared by every process for
//! state and for the queue), a content-addressed directory at
//! `<QR_DATA_DIR>/objects`, and the durable in-process queue over that same
//! SQLite file.
//!
//! The selection is additive: [`Objects`] and [`ControlQueue`] implement the
//! same [`ObjectStore`], [`Queue`], and [`QueueConsumer`] traits the services
//! already call, so no route, use case, or engine reads the profile. The
//! in-process queue is never a per-process queue: it is a handle on the shared
//! file, which is what makes a message published by `qr-evidence` visible to
//! `qr-backtest`.
//!
//! The server profile is the default. A desktop-only path refuses to start
//! without `QR_DATA_DIR`, exactly as the SQLite services do, so a half-selected
//! backend set is a configuration error rather than a silent fallback.

use std::path::PathBuf;
use std::sync::Arc;

use crate::adapters::filesystem::FilesystemStore;
use crate::adapters::inprocess::InProcessQueue;
use crate::adapters::postgres::{PgDatabase, apply_migrations_in as apply_postgres_migrations};
use crate::adapters::rabbitmq::RabbitQueue;
use crate::adapters::s3::S3Store;
use crate::adapters::sqlite::{
    DATABASE_FILE, SqliteDatabase, apply_migrations_in as apply_sqlite_migrations,
};
use crate::adapters::{
    Backend, BoxFuture, Database, MessageHandler, ObjectStore, Queue, QueueConsumer, QueueError,
    StoreError,
};
use crate::config::{Config, ConfigError, DeploymentProfile};
use crate::error::RuntimeError;
use crate::health::HealthRegistry;
use crate::migrations::Migration;

/// The object-store directory under `QR_DATA_DIR` on the desktop profile.
pub const OBJECTS_DIR: &str = "objects";

/// Where the desktop object store is rooted.
///
/// The path is derived from the configured data directory and nothing else, so
/// every service resolves the same CAS root without an environment variable of
/// its own.
///
/// # Errors
///
/// Returns [`ConfigError::Missing`] when the desktop profile carries no
/// `QR_DATA_DIR`; the configuration reader already refuses that, so this guard
/// keeps the constructor honest rather than defaulting a path.
pub fn objects_path(config: &Config) -> Result<PathBuf, ConfigError> {
    Ok(data_dir(config)?.join(OBJECTS_DIR))
}

/// The shared desktop SQLite file.
///
/// # Errors
///
/// Returns [`ConfigError::Missing`] when the desktop profile carries no
/// `QR_DATA_DIR`.
pub fn database_path(config: &Config) -> Result<PathBuf, ConfigError> {
    Ok(data_dir(config)?.join(DATABASE_FILE))
}

/// Open the shared SQLite file for the desktop profile, or `None` on the
/// server profile.
///
/// A database-owning service opens the file in its own declared profile branch,
/// because only that service knows which ledger to apply. The backtest worker
/// owns no schema but needs the queue handle, so it calls this once and passes
/// the result to [`ControlQueue::open`]. Either way, the queue and the
/// repositories use one connection per process on one file.
///
/// # Errors
///
/// Returns [`RuntimeError::Database`] when SQLite cannot open the file, and
/// [`RuntimeError::Config`] when the desktop profile has no data directory.
pub fn open_database(config: &Config) -> Result<Option<SqliteDatabase>, RuntimeError> {
    match config.profile {
        DeploymentProfile::Server => Ok(None),
        DeploymentProfile::Desktop => {
            let path = database_path(config)?;
            SqliteDatabase::open(&path)
                .map(Some)
                .map_err(|error| RuntimeError::Database(error.to_string()))
        }
    }
}

/// The database a service owns for its declared schema.
#[derive(Clone)]
pub enum ServiceDatabase {
    /// The embedded SQLite file shared by the desktop processes.
    Sqlite(SqliteDatabase),
    /// Postgres, on the server profile.
    Postgres(PgDatabase),
}

/// Opens the service's database, applies its ledger, and registers its health.
///
/// This is the one implementation of the profile branch every schema-owning
/// service needs. The service still names its own ledger and migration sets;
/// what it no longer repeats is the open, migrate, and health sequence.
///
/// `label` is the startup log's noun. It usually equals `ledger`, but the
/// evidence service applies its `runs` ledger for both its run and research
/// stores, so it passes "run and research".
///
/// Returns `Ok(None)` when the server profile cannot reach Postgres: the
/// failure is registered as a health component so the service starts and
/// reports degraded, exactly as it did before this helper existed.
///
/// # Errors
///
/// Returns [`RuntimeError::Database`] when SQLite cannot open the file, and
/// [`RuntimeError::Migrations`] when either backend cannot apply its ledger.
pub async fn open_service_database(
    config: &Config,
    ledger: &str,
    label: &str,
    sqlite_migrations: &[Migration],
    postgres_migrations: &[Migration],
    health: &mut HealthRegistry,
) -> Result<Option<ServiceDatabase>, RuntimeError> {
    match config.profile {
        DeploymentProfile::Desktop => {
            let path = database_path(config)?;
            let database = SqliteDatabase::open(&path)
                .map_err(|error| RuntimeError::Database(error.to_string()))?;
            if config.migrate {
                let applied = apply_sqlite_migrations(&database, ledger, sqlite_migrations)
                    .await
                    .map_err(|error| RuntimeError::Migrations(error.to_string()))?;
                tracing::info!(applied, "{} SQLite migrations applied", label);
            }
            let ping = database.clone();
            health.register("sqlite", move || {
                let database = ping.clone();
                async move { database.ping().await.map_err(|error| error.to_string()) }
            });
            Ok(Some(ServiceDatabase::Sqlite(database)))
        }
        DeploymentProfile::Server => match PgDatabase::connect(&config.database_url).await {
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
                Ok(Some(ServiceDatabase::Postgres(database)))
            }
            Err(error) => {
                health.register("postgres", move || {
                    let message = error.to_string();
                    async move { Err(message) }
                });
                Ok(None)
            }
        },
    }
}

/// The object store selected by `QR_PROFILE`.
///
/// The enum owns the concrete adapter so the health check can reach the
/// adapter's own probe; the trait object the routes receive is built from it.
#[derive(Debug, Clone)]
pub enum Objects {
    /// An S3-compatible bucket.
    S3(S3Store),
    /// A content-addressed directory.
    Filesystem(FilesystemStore),
}

impl Objects {
    /// Build the object store for the configured profile.
    ///
    /// The server constructor never fails: an unreachable endpoint is a health
    /// report, not a startup error. The desktop constructor resolves the CAS
    /// root and creates it on the first write or probe.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Config`] when the desktop profile has no
    /// `QR_DATA_DIR`.
    pub async fn open(config: &Config) -> Result<Self, RuntimeError> {
        match config.profile {
            DeploymentProfile::Server => Ok(Self::S3(
                S3Store::new(&config.object_store_url, &config.object_store_bucket).await,
            )),
            DeploymentProfile::Desktop => Ok(Self::Filesystem(FilesystemStore::new(objects_path(
                config,
            )?))),
        }
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
        match self {
            Self::S3(store) => store.check().await,
            Self::Filesystem(store) => store.check().await,
        }
    }
}

impl ObjectStore for Objects {
    fn backend(&self) -> Backend {
        match self {
            Self::S3(_) => Backend::S3,
            Self::Filesystem(_) => Backend::Filesystem,
        }
    }

    fn put(&self, key: &str, bytes: Vec<u8>) -> BoxFuture<Result<(), StoreError>> {
        match self {
            Self::S3(store) => store.put(key, bytes),
            Self::Filesystem(store) => store.put(key, bytes),
        }
    }

    fn get(&self, key: &str) -> BoxFuture<Result<Vec<u8>, StoreError>> {
        match self {
            Self::S3(store) => store.get(key),
            Self::Filesystem(store) => store.get(key),
        }
    }

    fn exists(&self, key: &str) -> BoxFuture<Result<bool, StoreError>> {
        match self {
            Self::S3(store) => store.exists(key),
            Self::Filesystem(store) => store.exists(key),
        }
    }
}

/// The control-plane queue selected by `QR_PROFILE`.
///
/// Both variants are handles on one queue, not two policies. On the desktop
/// profile every process opens the same SQLite file, so a publish in evidence
/// is a row the backtest worker claims; the queue is never recreated per
/// process.
#[derive(Debug, Clone)]
pub enum ControlQueue {
    /// RabbitMQ.
    RabbitMq(Arc<RabbitQueue>),
    /// The durable queue over the shared SQLite file.
    InProcess(InProcessQueue),
}

impl ControlQueue {
    /// Build the queue for the configured profile.
    ///
    /// The desktop profile requires the shared SQLite handle; passing one that
    /// is already open is deliberate, because a second connection in the same
    /// process would queue behind a different mutex and add no isolation.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Queue`] when the broker or the SQLite DDL fails,
    /// and [`RuntimeError::Database`] when the desktop profile has no handle.
    pub async fn open(
        config: &Config,
        database: Option<&SqliteDatabase>,
    ) -> Result<Self, RuntimeError> {
        match config.profile {
            DeploymentProfile::Server => Ok(Self::RabbitMq(Arc::new(
                RabbitQueue::connect(&config.queue_url)
                    .await
                    .map_err(|error| RuntimeError::Queue(error.to_string()))?,
            ))),
            DeploymentProfile::Desktop => {
                let database = database.ok_or_else(|| {
                    RuntimeError::Database(
                        "the desktop queue needs the shared SQLite handle".to_owned(),
                    )
                })?;
                let queue = InProcessQueue::open(database.clone())
                    .await
                    .map_err(|error| RuntimeError::Queue(error.to_string()))?;
                Ok(Self::InProcess(queue))
            }
        }
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
        match self {
            Self::RabbitMq(queue) => queue.check(),
            Self::InProcess(queue) => queue.check().await,
        }
    }
}

impl Queue for ControlQueue {
    fn backend(&self) -> Backend {
        match self {
            Self::RabbitMq(_) => Backend::RabbitMq,
            Self::InProcess(_) => Backend::InProcess,
        }
    }

    fn publish(&self, routing_key: &str, payload: Vec<u8>) -> BoxFuture<Result<(), QueueError>> {
        match self {
            Self::RabbitMq(queue) => queue.publish(routing_key, payload),
            Self::InProcess(queue) => queue.publish(routing_key, payload),
        }
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
        match self {
            Self::RabbitMq(queue) => queue.consume(name, keys, handler),
            Self::InProcess(queue) => queue.consume(name, keys, handler),
        }
    }
}

/// The configured desktop data directory.
fn data_dir(config: &Config) -> Result<PathBuf, ConfigError> {
    config.data_dir.clone().ok_or(ConfigError::Missing {
        name: "QR_DATA_DIR",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DeploymentProfile;
    use std::collections::BTreeMap;

    fn desktop(dir: &str) -> Config {
        Config::from_map(&BTreeMap::from([
            ("QR_PROFILE".to_owned(), "desktop".to_owned()),
            ("QR_DATA_DIR".to_owned(), dir.to_owned()),
        ]))
        .expect("desktop config")
    }

    #[test]
    fn the_desktop_object_store_is_the_objects_directory_under_the_data_dir() {
        let config = desktop("/srv/quantrule");
        assert_eq!(
            objects_path(&config).expect("objects path"),
            PathBuf::from("/srv/quantrule/objects")
        );
        assert_eq!(
            database_path(&config).expect("database path"),
            PathBuf::from("/srv/quantrule/quantrule.sqlite")
        );
    }

    #[test]
    fn the_server_profile_opens_no_embedded_database() {
        let config = Config::from_map(&BTreeMap::new()).expect("defaults");
        assert_eq!(config.profile, DeploymentProfile::Server);
        assert!(
            open_database(&config).expect("server").is_none(),
            "the server profile keeps Postgres"
        );
    }

    #[test]
    fn the_desktop_profile_opens_the_shared_database_file() {
        let directory = std::env::temp_dir().join(format!(
            "quantrule-backends-{}-database",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        let config = desktop(&directory.display().to_string());
        let database = open_database(&config)
            .expect("desktop database")
            .expect("a handle");
        assert!(database.path().is_some_and(|path| path.is_file()));
        std::fs::remove_dir_all(&directory).expect("cleanup");
    }

    #[tokio::test]
    async fn the_desktop_service_database_applies_its_ledger_and_registers_health() {
        let directory = std::env::temp_dir().join(format!(
            "quantrule-backends-{}-service-database",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        let config = desktop(&directory.display().to_string());
        let mut health = HealthRegistry::new();

        let opened = open_service_database(&config, "catalog", "catalog", &[], &[], &mut health)
            .await
            .expect("the service database opens")
            .expect("the desktop profile opens SQLite");

        assert!(matches!(opened, ServiceDatabase::Sqlite(_)));
        assert_eq!(health.names(), vec!["sqlite"]);
        let report = health.report(None).await;
        assert!(matches!(
            report.services[0].state,
            quantrule_api::ServiceState::Healthy
        ));
        std::fs::remove_dir_all(&directory).expect("cleanup");
    }

    #[tokio::test]
    async fn a_desktop_queue_opens_over_the_shared_database() {
        let directory =
            std::env::temp_dir().join(format!("quantrule-backends-{}-queue", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let config = desktop(&directory.display().to_string());
        let database = open_database(&config)
            .expect("desktop database")
            .expect("a handle");
        let queue = ControlQueue::open(&config, Some(&database))
            .await
            .expect("desktop queue");
        assert_eq!(queue.name(), "in-process");
        queue
            .publish("run.requested", b"run".to_vec())
            .await
            .expect("publish");
        queue
            .check()
            .await
            .expect("the in-process probe reads the queue tables");
        std::fs::remove_dir_all(&directory).expect("cleanup");
    }

    #[tokio::test]
    async fn a_desktop_queue_without_the_shared_handle_is_refused() {
        let config = desktop("/srv/quantrule");
        let error = ControlQueue::open(&config, None)
            .await
            .expect_err("no handle");
        assert!(
            error.to_string().contains("shared SQLite handle"),
            "{error}"
        );
    }

    #[tokio::test]
    async fn two_desktop_queue_handles_share_the_file() {
        use crate::queue::{QUEUE_BACKTEST, ROUTING_REQUESTED};

        let directory =
            std::env::temp_dir().join(format!("quantrule-backends-{}-shared", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let config = desktop(&directory.display().to_string());
        let first_database = open_database(&config)
            .expect("first database")
            .expect("a handle");
        let second_database = open_database(&config)
            .expect("second database")
            .expect("a handle");
        let first = ControlQueue::open(&config, Some(&first_database))
            .await
            .expect("first queue");
        let second = ControlQueue::open(&config, Some(&second_database))
            .await
            .expect("second queue");
        first
            .publish(ROUTING_REQUESTED, b"run".to_vec())
            .await
            .expect("publish");

        let ControlQueue::InProcess(queue) = second else {
            panic!("the desktop profile selects the in-process queue");
        };
        let delivery = queue
            .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
            .await
            .expect("claim")
            .expect("the message published by the first handle");
        assert_eq!(delivery.payload, b"run");
        queue.ack(&delivery).await.expect("ack");
        std::fs::remove_dir_all(&directory).expect("cleanup");
    }

    #[tokio::test]
    async fn the_desktop_profile_selects_the_filesystem_store() {
        let config = desktop("/srv/quantrule");
        let objects = Objects::open(&config).await.expect("objects");
        assert_eq!(objects.name(), "filesystem");
        assert_eq!(objects.backend(), Backend::Filesystem);
    }

    #[tokio::test]
    async fn the_filesystem_store_delegates_through_the_selection() {
        let directory =
            std::env::temp_dir().join(format!("quantrule-backends-{}-objects", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let config = desktop(&directory.display().to_string());
        let objects = Objects::open(&config).await.expect("objects");
        let key = format!("artifacts/{}", quantrule_api::sha256_hex(b"body"));
        objects.put(&key, b"body".to_vec()).await.expect("put");
        assert_eq!(objects.get(&key).await.expect("get"), b"body");
        assert!(objects.exists(&key).await.expect("exists"));
        objects.check().await.expect("the CAS root is writable");
        std::fs::remove_dir_all(&directory).expect("cleanup");
    }

    #[test]
    fn the_filesystem_objects_report_their_backend() {
        let objects = Objects::Filesystem(FilesystemStore::new("/srv/quantrule/objects"));
        assert_eq!(objects.name(), "filesystem");
        assert_eq!(objects.backend(), Backend::Filesystem);
    }
}
