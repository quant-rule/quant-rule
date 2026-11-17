//! Backend adapters.
//!
//! A service calls a trait, never a client library. v1.0.0 shipped one
//! implementation of each: Postgres, an S3-compatible store, and RabbitMQ. The
//! embedded desktop profile implements the same traits; SQLite, the filesystem
//! store, and the in-process queue are those adapters, which is why this
//! boundary exists (ADR-2026-11-12-12).
//!
//! The surface is deliberately small. A trait grows when a service needs an
//! operation, not in anticipation of one.

pub mod filesystem;
pub mod inprocess;
pub mod memory;
pub mod postgres;
pub mod rabbitmq;
pub mod s3;
pub mod sqlite;

use std::future::Future;
use std::pin::Pin;

/// A boxed future, so a trait stays usable behind `dyn`.
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// Which implementation is behind a trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Backend {
    /// Postgres.
    Postgres,
    /// SQLite, for the embedded desktop profile.
    Sqlite,
    /// An S3-compatible object store.
    S3,
    /// Local files, for the embedded desktop profile.
    Filesystem,
    /// RabbitMQ.
    RabbitMq,
    /// An in-process queue, for the embedded desktop profile.
    InProcess,
}

impl Backend {
    /// The wire name, used in `/health` and in logs.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::Sqlite => "sqlite",
            Self::S3 => "s3",
            Self::Filesystem => "filesystem",
            Self::RabbitMq => "rabbitmq",
            Self::InProcess => "in-process",
        }
    }
}

/// A service schema.
///
/// Postgres spells a schema as a namespace; SQLite has none and spells it as a
/// table prefix. Keeping the mapping in one enum is what lets the two backends
/// share one table name: `catalog.datasets` and `catalog_datasets` are the same
/// relation, not two definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Schema {
    /// Profiles and their organisational scope.
    Gateway,
    /// Datasets, configurations, and their file bundles.
    Catalog,
    /// Runs, stages, artifacts, and submissions.
    Runs,
    /// Diagnostic stage runs, submissions, and artifacts.
    Stage,
    /// Sealed hypotheses.
    Research,
    /// The simulation research plane.
    Simulation,
    /// Durable control-plane messages, owned by the embedded queue.
    Queue,
}

impl Schema {
    /// Every schema the embedded profile creates.
    ///
    /// The order is the order the migrations are applied so a reader can walk
    /// the list; no schema has a cross-schema foreign key.
    pub const ALL: [Self; 7] = [
        Self::Gateway,
        Self::Catalog,
        Self::Runs,
        Self::Stage,
        Self::Research,
        Self::Simulation,
        Self::Queue,
    ];

    /// The Postgres schema name and the SQLite table prefix.
    #[must_use]
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Gateway => "gateway",
            Self::Catalog => "catalog",
            Self::Runs => "runs",
            Self::Stage => "stage",
            Self::Research => "research",
            Self::Simulation => "simulation",
            Self::Queue => "queue",
        }
    }

    /// The ledger name, one per schema.
    #[must_use]
    pub fn ledger(self) -> &'static str {
        self.prefix()
    }
}

/// The SQL dialect an adapter speaks.
///
/// The surface is deliberately small: a table name, a bind placeholder, and
/// the three spellings a repository cannot share verbatim. A repository grows
/// the surface when a query needs it, not in anticipation of one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dialect {
    /// Postgres.
    Postgres,
    /// SQLite, for the embedded desktop profile.
    Sqlite,
}

impl Dialect {
    /// Which backend this dialect belongs to.
    #[must_use]
    pub fn backend(self) -> Backend {
        match self {
            Self::Postgres => Backend::Postgres,
            Self::Sqlite => Backend::Sqlite,
        }
    }

    /// Spell a schema-qualified table in this dialect.
    ///
    /// Postgres keeps the schema qualifier; SQLite joins the schema and table
    /// with a double underscore, so `catalog` and `datasets` become
    /// `catalog__datasets`. The separator is two characters so a table name may
    /// contain one underscore without becoming ambiguous.
    ///
    /// # Errors
    ///
    /// Returns [`DatabaseError`] when `name` is not a lowercase SQL identifier.
    /// A table name is a code constant, so a refusal is a programming error
    /// rather than user input.
    pub fn table(self, schema: Schema, name: &str) -> Result<String, DatabaseError> {
        if !is_identifier(name) {
            return Err(DatabaseError(format!(
                "{name:?} is not a lowercase SQL identifier"
            )));
        }
        Ok(match self {
            Self::Postgres => format!("{}.{name}", schema.prefix()),
            Self::Sqlite => format!("{}__{name}", schema.prefix()),
        })
    }

    /// A one-based bind placeholder.
    #[must_use]
    pub fn placeholder(self, index: usize) -> String {
        match self {
            Self::Postgres => format!("${index}"),
            Self::Sqlite => format!("?{index}"),
        }
    }

    /// How the dialect declares a JSON document column.
    #[must_use]
    pub fn json_type(self) -> &'static str {
        match self {
            Self::Postgres => "jsonb",
            Self::Sqlite => "TEXT",
        }
    }

    /// How the dialect declares an instant column.
    #[must_use]
    pub fn timestamp_type(self) -> &'static str {
        match self {
            Self::Postgres => "timestamptz",
            Self::Sqlite => "TEXT",
        }
    }

    /// How the dialect spells "now" in a default or an assignment.
    #[must_use]
    pub fn timestamp_now(self) -> &'static str {
        match self {
            Self::Postgres => "now()",
            Self::Sqlite => "strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
        }
    }
}

/// Whether a value is a lowercase SQL identifier.
///
/// The restriction keeps an unquoted Postgres identifier and a SQLite table
/// prefix spelled the same way, and it refuses a name that could carry SQL.
fn is_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !(first.is_ascii_lowercase() || first == b'_') {
        return false;
    }
    bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

/// A database failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseError(pub String);

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DatabaseError {}

/// An object-store failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError(pub String);

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for StoreError {}

/// A queue failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueError(pub String);

impl std::fmt::Display for QueueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for QueueError {}

/// A database handle.
///
/// The surface is a liveness check at this phase. A repository trait grows here
/// when a service needs one, so the second implementation is written against the
/// operations that exist rather than a guessed SQL subset.
pub trait Database: Send + Sync {
    /// Which implementation this is.
    fn backend(&self) -> Backend;

    /// Confirm the database answers.
    fn ping(&self) -> BoxFuture<Result<(), DatabaseError>>;
}

/// One delivered message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// The routing key the broker delivered it under.
    pub routing_key: String,
    /// The body, `quantrule.queue/v1` bytes owned by the caller.
    pub payload: Vec<u8>,
    /// Whether the broker says this is a redelivery.
    pub redelivered: bool,
}

/// A queue message handler: one message to an acknowledged outcome.
pub type MessageHandler = Box<dyn Fn(Message) -> BoxFuture<Result<(), QueueError>> + Send + Sync>;

/// A consumer that delivers queued messages to a handler.
pub trait QueueConsumer: Send + Sync {
    /// Which implementation this is.
    fn backend(&self) -> Backend;

    /// Consume one queue bound to `keys` until the handler fails or the stream
    /// ends.
    ///
    /// The adapter acknowledges a successful handler and negatively acknowledges
    /// a failed one with immediate requeue. A handler that wants backoff holds
    /// the delivery and retries in place.
    fn consume(
        &self,
        queue: &str,
        keys: &[&str],
        handler: MessageHandler,
    ) -> BoxFuture<Result<(), QueueError>>;
}

/// A content-addressed object store.
pub trait ObjectStore: Send + Sync {
    /// Which implementation this is.
    fn backend(&self) -> Backend;

    /// Write bytes under a key.
    ///
    /// A successful write is atomic: a reader sees the previous object or the
    /// complete new one, never a partial body. S3 provides this; a filesystem
    /// implementation must write a temporary file and rename it. The run
    /// manifest is published this way, so a killed writer cannot expose a
    /// partial artifact set.
    fn put(&self, key: &str, bytes: Vec<u8>) -> BoxFuture<Result<(), StoreError>>;

    /// Read the bytes under a key.
    fn get(&self, key: &str) -> BoxFuture<Result<Vec<u8>, StoreError>>;

    /// Whether a key exists.
    fn exists(&self, key: &str) -> BoxFuture<Result<bool, StoreError>>;

    /// Delete the bytes under a key.
    ///
    /// Deleting a key that does not exist succeeds: retention is idempotent, and
    /// a cleanup that races a second cleanup must not fail. A run artifact is
    /// never deleted; only the caller that owns a key namespace calls this.
    fn delete(&self, key: &str) -> BoxFuture<Result<(), StoreError>>;
}

/// The control-plane queue.
pub trait Queue: Send + Sync {
    /// Which implementation this is.
    fn backend(&self) -> Backend;

    /// Publish one message under a routing key.
    fn publish(&self, routing_key: &str, payload: Vec<u8>) -> BoxFuture<Result<(), QueueError>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    type StoreMap = Arc<Mutex<std::collections::BTreeMap<String, Vec<u8>>>>;

    /// A stub store: the second implementation that proves the trait is enough.
    struct MemoryStore {
        inner: StoreMap,
    }

    impl ObjectStore for MemoryStore {
        fn backend(&self) -> Backend {
            Backend::Filesystem
        }

        fn put(&self, key: &str, bytes: Vec<u8>) -> BoxFuture<Result<(), StoreError>> {
            let inner = Arc::clone(&self.inner);
            let key = key.to_owned();
            Box::pin(async move {
                inner.lock().expect("lock").insert(key, bytes);
                Ok(())
            })
        }

        fn get(&self, key: &str) -> BoxFuture<Result<Vec<u8>, StoreError>> {
            let inner = Arc::clone(&self.inner);
            let key = key.to_owned();
            Box::pin(async move {
                inner
                    .lock()
                    .expect("lock")
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| StoreError(format!("missing {key}")))
            })
        }

        fn exists(&self, key: &str) -> BoxFuture<Result<bool, StoreError>> {
            let inner = Arc::clone(&self.inner);
            let key = key.to_owned();
            Box::pin(async move { Ok(inner.lock().expect("lock").contains_key(&key)) })
        }

        fn delete(&self, key: &str) -> BoxFuture<Result<(), StoreError>> {
            let inner = Arc::clone(&self.inner);
            let key = key.to_owned();
            Box::pin(async move {
                inner.lock().expect("lock").remove(&key);
                Ok(())
            })
        }
    }

    /// A consumer that drains a fixed message list.
    struct MemoryConsumer {
        messages: Arc<Mutex<Vec<Message>>>,
    }

    impl QueueConsumer for MemoryConsumer {
        fn backend(&self) -> Backend {
            Backend::InProcess
        }

        fn consume(
            &self,
            _queue: &str,
            keys: &[&str],
            handler: MessageHandler,
        ) -> BoxFuture<Result<(), QueueError>> {
            let messages = Arc::clone(&self.messages);
            let keys: Vec<String> = keys.iter().map(|key| (*key).to_owned()).collect();
            Box::pin(async move {
                let pending = messages
                    .lock()
                    .map_err(|_| QueueError("the consumer lock is poisoned".to_owned()))?
                    .clone();
                for message in pending {
                    if !keys.is_empty() && !keys.contains(&message.routing_key) {
                        continue;
                    }
                    handler(message).await?;
                }
                Ok(())
            })
        }
    }

    type Published = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

    /// A stub queue.
    struct MemoryQueue {
        seen: Published,
    }

    impl Queue for MemoryQueue {
        fn backend(&self) -> Backend {
            Backend::InProcess
        }

        fn publish(
            &self,
            routing_key: &str,
            payload: Vec<u8>,
        ) -> BoxFuture<Result<(), QueueError>> {
            let seen = Arc::clone(&self.seen);
            let key = routing_key.to_owned();
            Box::pin(async move {
                seen.lock().expect("lock").push((key, payload));
                Ok(())
            })
        }
    }

    /// A stub database.
    struct MemoryDatabase;

    impl Database for MemoryDatabase {
        fn backend(&self) -> Backend {
            Backend::Sqlite
        }

        fn ping(&self) -> BoxFuture<Result<(), DatabaseError>> {
            Box::pin(async { Ok(()) })
        }
    }

    #[tokio::test]
    async fn a_second_store_implementation_needs_no_use_case_change() {
        let store: Arc<dyn ObjectStore> = Arc::new(MemoryStore {
            inner: Arc::new(Mutex::new(std::collections::BTreeMap::new())),
        });
        store.put("candles/a", vec![1, 2, 3]).await.expect("put");
        assert_eq!(store.get("candles/a").await.expect("get"), vec![1, 2, 3]);
        assert!(store.exists("candles/a").await.expect("exists"));
        assert!(!store.exists("candles/b").await.expect("missing"));
        assert_eq!(store.backend(), Backend::Filesystem);
    }

    #[tokio::test]
    async fn a_consumer_implementation_needs_no_use_case_change() {
        let requested = Message {
            routing_key: "run.requested".to_owned(),
            payload: b"one".to_vec(),
            redelivered: false,
        };
        let computed = Message {
            routing_key: "run.computed".to_owned(),
            payload: b"two".to_vec(),
            redelivered: false,
        };
        let consumer: Arc<dyn QueueConsumer> = Arc::new(MemoryConsumer {
            messages: Arc::new(Mutex::new(vec![requested, computed])),
        });
        let seen: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(Vec::new()));
        let target = Arc::clone(&seen);
        let handler: MessageHandler = Box::new(move |message| {
            let target = Arc::clone(&target);
            Box::pin(async move {
                target
                    .lock()
                    .map_err(|_| QueueError("the handler lock is poisoned".to_owned()))?
                    .push(message);
                Ok(())
            })
        });
        consumer
            .consume("runs.backtest", &["run.requested"], handler)
            .await
            .expect("consume");
        let seen = seen.lock().expect("seen");
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].payload, b"one".to_vec());
    }

    #[tokio::test]
    async fn a_second_queue_implementation_needs_no_use_case_change() {
        let queue: Arc<dyn Queue> = Arc::new(MemoryQueue {
            seen: Arc::new(Mutex::new(Vec::new())),
        });
        queue
            .publish("run.requested", b"{}".to_vec())
            .await
            .expect("publish");
        assert_eq!(queue.backend(), Backend::InProcess);
    }

    #[tokio::test]
    async fn a_second_database_implementation_needs_no_use_case_change() {
        let database: Arc<dyn Database> = Arc::new(MemoryDatabase);
        database.ping().await.expect("ping");
        assert_eq!(database.backend(), Backend::Sqlite);
    }

    #[test]
    fn every_backend_has_a_name() {
        for backend in [
            Backend::Postgres,
            Backend::Sqlite,
            Backend::S3,
            Backend::Filesystem,
            Backend::RabbitMq,
            Backend::InProcess,
        ] {
            assert!(!backend.as_str().is_empty());
        }
    }

    #[test]
    fn a_schema_is_a_qualifier_in_postgres_and_a_double_prefix_in_sqlite() {
        assert_eq!(
            Dialect::Postgres
                .table(Schema::Catalog, "datasets")
                .expect("postgres"),
            "catalog.datasets"
        );
        assert_eq!(
            Dialect::Sqlite
                .table(Schema::Catalog, "datasets")
                .expect("sqlite"),
            "catalog__datasets"
        );
        assert_eq!(
            Dialect::Sqlite.table(Schema::Runs, "runs").expect("sqlite"),
            "runs__runs"
        );
        assert_eq!(
            Dialect::Sqlite
                .table(Schema::Research, "hypotheses")
                .expect("sqlite"),
            "research__hypotheses"
        );
        assert_eq!(
            Dialect::Sqlite
                .table(Schema::Simulation, "runs")
                .expect("sqlite"),
            "simulation__runs"
        );
        assert_eq!(
            Dialect::Sqlite
                .table(Schema::Queue, "messages")
                .expect("sqlite"),
            "queue__messages"
        );
        assert_eq!(
            Dialect::Sqlite
                .table(Schema::Gateway, "profiles")
                .expect("sqlite"),
            "gateway__profiles"
        );
    }

    #[test]
    fn a_table_name_cannot_escape_its_schema() {
        for name in [
            "",
            "Data",
            "data-sets",
            "datasets; DROP TABLE x",
            "catalog.datasets",
            "1data",
        ] {
            assert!(
                Dialect::Sqlite.table(Schema::Catalog, name).is_err(),
                "{name:?} was accepted"
            );
        }
    }

    #[test]
    fn the_dialect_spells_its_types_and_placeholders() {
        assert_eq!(Dialect::Postgres.placeholder(1), "$1");
        assert_eq!(Dialect::Sqlite.placeholder(1), "?1");
        assert_eq!(Dialect::Postgres.json_type(), "jsonb");
        assert_eq!(Dialect::Sqlite.json_type(), "TEXT");
        assert_eq!(Dialect::Postgres.timestamp_type(), "timestamptz");
        assert_eq!(Dialect::Sqlite.timestamp_type(), "TEXT");
        assert_eq!(
            Dialect::Postgres.timestamp_now(),
            "now()",
            "the Postgres spelling is the one the migrations use"
        );
        assert_eq!(
            Dialect::Sqlite.timestamp_now(),
            "strftime('%Y-%m-%dT%H:%M:%SZ', 'now')"
        );
    }

    #[test]
    fn a_dialect_names_its_backend() {
        assert_eq!(Dialect::Postgres.backend(), Backend::Postgres);
        assert_eq!(Dialect::Sqlite.backend(), Backend::Sqlite);
    }

    #[test]
    fn every_schema_has_a_unique_prefix_and_ledger() {
        let mut prefixes = std::collections::BTreeSet::new();
        for schema in Schema::ALL {
            assert!(
                prefixes.insert(schema.prefix()),
                "{schema:?} repeats a prefix"
            );
            assert_eq!(schema.ledger(), schema.prefix());
        }
        assert_eq!(prefixes.len(), Schema::ALL.len());
    }
}
