//! Backend adapters.
//!
//! A service calls a trait, never a client library. v1.0.0 ships one
//! implementation of each: Postgres, an S3-compatible store, and RabbitMQ. The
//! embedded desktop profile implements the same traits, which is why this
//! boundary exists (ADR-2026-11-12-12).
//!
//! The surface is deliberately small. A trait grows when a service needs an
//! operation, not in anticipation of one.

pub mod postgres;
pub mod rabbitmq;
pub mod s3;

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
}
