//! Backend adapters.
//!
//! A service calls a trait, never a client library. The server profile ships one
//! implementation of each: Postgres, an S3-compatible store, and RabbitMQ.
//!
//! The surface is deliberately small. A trait grows when a service needs an
//! operation, not in anticipation of one.

pub mod memory;
pub mod postgres;
pub mod rabbitmq;
pub mod s3;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// A boxed future, so a trait stays usable behind `dyn`.
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// The `EnvFilter` directives that keep the driver crates at `warn`.
///
/// Each adapter owns its own directive because it is the only module allowed to
/// name its client. Logging prepends the combined value to `QR_LOG` so a global
/// `debug` cannot reveal a connection string merely by raising every target.
#[must_use]
pub fn log_driver_directives() -> String {
    format!(
        "{},{},{}",
        postgres::LOG_FILTER_DIRECTIVE,
        rabbitmq::LOG_FILTER_DIRECTIVE,
        s3::LOG_FILTER_DIRECTIVE
    )
}

/// Which implementation is behind a trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Backend {
    /// Postgres.
    Postgres,
    /// An S3-compatible object store.
    S3,
    /// A local or in-memory object store used by tests.
    Filesystem,
    /// A queue used by tests.
    InProcess,
    /// RabbitMQ.
    RabbitMq,
}

impl Backend {
    /// The wire name, used in `/health` and in logs.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::S3 => "s3",
            Self::Filesystem => "filesystem",
            Self::InProcess => "in-process",
            Self::RabbitMq => "rabbitmq",
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
///
/// The handler is shared rather than boxed so a reconnect can reuse it without
/// rebuilding the service graph.
pub type MessageHandler = Arc<dyn Fn(Message) -> BoxFuture<Result<(), QueueError>> + Send + Sync>;

/// A consumer that delivers queued messages to a handler.
pub trait QueueConsumer: Send + Sync {
    /// Which implementation this is.
    fn backend(&self) -> Backend;

    /// Consume one queue until the handler fails or the stream ends.
    ///
    /// The adapter declares and binds the queue's routing keys from the queue
    /// contract; the caller names only the generated queue. The adapter
    /// acknowledges a successful handler and negatively acknowledges a failed
    /// one with immediate requeue. A handler that wants backoff holds the
    /// delivery and retries in place.
    fn consume(&self, queue: &str, handler: MessageHandler) -> BoxFuture<Result<(), QueueError>>;

    /// Consume until a drain signal arrives.
    ///
    /// The default stops at the next message boundary by dropping the consumer
    /// future. The RabbitMQ adapter overrides it so an in-flight handler finishes
    /// and no new delivery is accepted after the signal.
    fn consume_until(
        &self,
        queue: &str,
        handler: MessageHandler,
        shutdown: crate::shutdown::Shutdown,
    ) -> BoxFuture<Result<(), QueueError>> {
        let consuming = self.consume(queue, handler);
        Box::pin(async move {
            let mut shutdown = shutdown;
            tokio::select! {
                biased;
                () = shutdown.wait() => Ok(()),
                result = consuming => result,
            }
        })
    }
}

/// A content-addressed object store.
pub trait ObjectStore: Send + Sync {
    /// Which implementation this is.
    fn backend(&self) -> Backend;

    /// Write bytes under a key.
    ///
    /// A successful write is atomic: a reader sees the previous object or the
    /// complete new one, never a partial body. The run manifest is published
    /// this way, so a killed writer cannot expose a partial artifact set.
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

/// Write content-addressed bytes only when their key is absent.
///
/// The check is not a compare-and-swap: two writers can both observe an absent
/// key and both write it. That is safe for a content-addressed key, because
/// they write the same bytes. A caller that needs a stronger guarantee must
/// serialise the key in the database, which is what the data service's object
/// guard does.
///
/// Returns `true` when this call wrote the object and `false` when it was
/// already present.
///
/// # Errors
///
/// Returns the object store's error when the existence check or the write
/// fails. A failed write leaves no object and is reported to the caller.
pub async fn put_if_absent(
    store: &dyn ObjectStore,
    key: &str,
    bytes: Vec<u8>,
) -> Result<bool, StoreError> {
    if store.exists(key).await? {
        return Ok(false);
    }
    store.put(key, bytes).await?;
    Ok(true)
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
    use super::{ObjectStore, put_if_absent};
    use crate::adapters::memory::MemoryObjects;

    #[tokio::test]
    async fn put_if_absent_keeps_the_first_content() {
        let store = MemoryObjects::default();
        assert!(
            put_if_absent(&store, "datasets/abc", b"first".to_vec())
                .await
                .expect("the first write")
        );
        assert!(
            !put_if_absent(&store, "datasets/abc", b"second".to_vec())
                .await
                .expect("the duplicate write")
        );
        assert_eq!(
            store.get("datasets/abc").await.expect("the object"),
            b"first"
        );
        assert!(
            store
                .exists("datasets/abc")
                .await
                .expect("the object exists")
        );
    }
}
