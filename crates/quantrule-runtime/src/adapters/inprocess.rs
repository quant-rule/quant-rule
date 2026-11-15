//! The durable in-process queue.
//!
//! The embedded desktop profile has no RabbitMQ. This module implements the
//! [`Queue`] and [`QueueConsumer`] traits over two SQLite tables in the shared
//! `quantrule.sqlite` file, so a published run survives a service restart and a
//! killed consumer's delivery is redelivered rather than lost (ADR-2026-11-12-12,
//! plan D6).
//!
//! `queue__messages` holds a message until it is acknowledged. A publish commits
//! before it returns. A claim takes a lease and increments the delivery count in
//! one immediate transaction. A handler success deletes the row; a handler
//! failure makes it immediately ready, matching the RabbitMQ adapter's
//! `nack(requeue = true)`. A killed consumer leaves a lease that expires, after
//! which the next consumer receives the same bytes with `redelivered` set.
//!
//! `queue__dead_letter` holds an exhausted message. The retry policy stays where
//! it is in the server profile: the worker counts the run's attempts, holds the
//! delivery for 5 s and then 25 s, and the third failure marks the run failed
//! and publishes `run.dead` (`docs/architecture/control-plane.md` section 3).
//! The queue stores the visibility the caller asks for and moves a delivery to
//! the dead-letter table when the caller says the budget is spent. It does not
//! invent a second policy, so the two profiles agree. Publishing `run.dead`
//! writes the dead-letter table directly, because no consumer drains that queue.
//!
//! Cancellation matches a routing key and payload digest. A ready message is
//! deleted in one transaction, and a leased message is left to its consumer
//! because the run row is authoritative and the worker checks it at its
//! boundaries. A repeated cancel is a no-op. Publishing `run.cancelled` remains
//! a separate durable message, exactly as the gateway publishes it today.
//!
//! The file is opened by [`SqliteDatabase`], which sets WAL, a five-second busy
//! timeout, foreign keys, and `synchronous=FULL`. This adapter never opens a
//! second connection and never changes those settings. `rusqlite` is
//! synchronous, so every statement runs through [`SqliteDatabase::call`] on the
//! blocking pool. The clock is injected: production passes the system clock and
//! a test passes a fixed one, so a lease or a backoff test does not sleep. The
//! idle poll interval is real time, because it only decides how soon a consumer
//! looks again.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use quantrule_api::sha256_hex;
use rusqlite::types::Value;
use rusqlite::{OptionalExtension, TransactionBehavior, params, params_from_iter};

use crate::adapters::sqlite::SqliteDatabase;
use crate::adapters::{
    Backend, BoxFuture, DatabaseError, Message, MessageHandler, Queue, QueueConsumer, QueueError,
};
use crate::clock;
use crate::queue::{DeadLetter, Delivery, ROUTING_DEAD};

/// The pending-message table.
const MESSAGES_TABLE: &str = "queue__messages";
/// The exhausted-message table.
const DEAD_LETTER_TABLE: &str = "queue__dead_letter";
/// How long a claim owns a message before another consumer may reclaim it.
pub const DEFAULT_LEASE: Duration = Duration::from_secs(300);
/// How often an idle consumer looks for a message.
const POLL_INTERVAL: Duration = Duration::from_millis(50);
/// A counter that keeps two lease tokens in one process distinct.
static NEXT_LEASE: AtomicU64 = AtomicU64::new(1);
/// A row of the dead-letter table, in column order.
type DeadLetterRow = (i64, String, Option<String>, Vec<u8>, i64, String, String);

/// The clock a queue reads, in Unix seconds.
///
/// It is injected so a test advances a lease or a backoff without sleeping.
pub type Now = Arc<dyn Fn() -> u64 + Send + Sync>;

/// A durable queue over the shared SQLite file.
///
/// The handle is cheap to clone: clones share the same database handle, clock,
/// and lease duration. [`InProcessQueue::open`] creates the two tables
/// idempotently, so a second service process opens the same file safely.
#[derive(Clone)]
pub struct InProcessQueue {
    database: SqliteDatabase,
    now: Now,
    lease: Duration,
}

impl std::fmt::Debug for InProcessQueue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InProcessQueue")
            .field("database", &self.database)
            .field("lease", &self.lease)
            .finish_non_exhaustive()
    }
}

impl InProcessQueue {
    /// Open the queue over an existing SQLite handle.
    ///
    /// The two queue tables are created idempotently in one immediate
    /// transaction. The queue is runtime infrastructure, not a service schema,
    /// so it owns its DDL instead of taking a service ledger; a second open in
    /// the same process or another one is a no-op.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError`] when SQLite refuses the DDL.
    pub async fn open(database: SqliteDatabase) -> Result<Self, QueueError> {
        Self::with_clock(database, Arc::new(system_now)).await
    }

    /// Open the queue with an injected clock.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError`] when SQLite refuses the DDL.
    pub async fn with_clock(database: SqliteDatabase, now: Now) -> Result<Self, QueueError> {
        let queue = Self {
            database,
            now,
            lease: DEFAULT_LEASE,
        };
        queue.ensure_schema().await?;
        Ok(queue)
    }

    /// Use a different lease duration.
    ///
    /// The default is [`DEFAULT_LEASE`]. A test uses a short lease to prove
    /// recovery without sleeping; production keeps the default so a running
    /// handler is not reclaimed before it can acknowledge. A handler that runs
    /// longer than the lease can be delivered twice; the default is deliberately
    /// long, and P2c may add a renewal if a fixture run approaches it.
    #[must_use]
    pub fn with_lease(mut self, lease: Duration) -> Self {
        self.lease = lease;
        self
    }

    /// Claim the oldest ready message matching one of `keys`.
    ///
    /// The claim takes a lease and increments `attempts` in one immediate
    /// transaction, so two consumers never hold the same message. A message
    /// whose lease has expired is ready again.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError`] when SQLite refuses the transaction.
    pub async fn claim(&self, queue: &str, keys: &[&str]) -> Result<Option<Delivery>, QueueError> {
        if keys.is_empty() {
            return Ok(None);
        }
        let now_seconds = (self.now)();
        let now = clock::rfc3339(now_seconds);
        let lease_expires_at = clock::rfc3339(now_seconds.saturating_add(self.lease.as_secs()));
        let owner = lease_owner(queue);
        let placeholders = (0..keys.len())
            .map(|index| format!("?{}", index + 2))
            .collect::<Vec<_>>()
            .join(", ");
        let select = format!(
            "SELECT id, routing_key, payload, attempts
               FROM {MESSAGES_TABLE}
              WHERE visible_at <= ?1
                AND (lease_owner IS NULL OR lease_expires_at <= ?1)
                AND routing_key IN ({placeholders})
              ORDER BY visible_at, id
              LIMIT 1"
        );
        let update = format!(
            "UPDATE {MESSAGES_TABLE}
                SET attempts = attempts + 1,
                    lease_owner = ?1,
                    lease_expires_at = ?2,
                    updated_at = ?3
              WHERE id = ?4
                AND (lease_owner IS NULL OR lease_expires_at <= ?3)"
        );
        let mut values: Vec<Value> = Vec::with_capacity(keys.len() + 1);
        values.push(Value::Text(now.clone()));
        for key in keys {
            values.push(Value::Text((*key).to_owned()));
        }
        self.database
            .call(move |connection| -> Result<Option<Delivery>, QueueError> {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(sql_error)?;
                let row: Option<(i64, String, Vec<u8>, i64)> = {
                    let mut statement = transaction.prepare(&select).map_err(sql_error)?;
                    statement
                        .query_row(params_from_iter(values), |row| {
                            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                        })
                        .optional()
                        .map_err(sql_error)?
                };
                let Some((id, routing_key, payload, attempts)) = row else {
                    transaction.commit().map_err(sql_error)?;
                    return Ok(None);
                };
                let attempts = u32::try_from(attempts).map_err(|_| {
                    QueueError(format!("stored attempts {attempts} exceed the u32 range"))
                })?;
                let changed = transaction
                    .execute(&update, params![owner, lease_expires_at, now, id])
                    .map_err(sql_error)?;
                if changed != 1 {
                    return Err(QueueError(format!(
                        "the message {id} was claimed by another consumer"
                    )));
                }
                transaction.commit().map_err(sql_error)?;
                Ok(Some(Delivery {
                    id,
                    routing_key,
                    payload,
                    attempts: attempts.saturating_add(1),
                    lease_owner: owner,
                }))
            })
            .await
            .map_err(database_error)
    }

    /// Acknowledge a delivery and delete it.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError`] when the delivery is no longer held by this
    /// consumer, which happens after a lease expiry, a cancellation, or a
    /// dead-letter move.
    pub async fn ack(&self, delivery: &Delivery) -> Result<(), QueueError> {
        let id = delivery.id;
        let owner = delivery.lease_owner.clone();
        let sql = format!("DELETE FROM {MESSAGES_TABLE} WHERE id = ?1 AND lease_owner = ?2");
        let changed = self
            .database
            .call(move |connection| -> Result<usize, QueueError> {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(sql_error)?;
                let changed = transaction
                    .execute(&sql, params![id, owner])
                    .map_err(sql_error)?;
                transaction.commit().map_err(sql_error)?;
                Ok(changed)
            })
            .await
            .map_err(database_error)?;
        if changed == 1 {
            Ok(())
        } else {
            Err(QueueError(format!(
                "delivery {id} is no longer held by its consumer"
            )))
        }
    }

    /// Return a delivery to the queue after `delay`.
    ///
    /// The caller owns the backoff policy. The worker holds the delivery for
    /// 5 s after the first failure and 25 s after the second; the queue only
    /// records the resulting visibility and keeps the attempt count.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError`] when the delivery is no longer held by this
    /// consumer.
    pub async fn nack(&self, delivery: &Delivery, delay: Duration) -> Result<(), QueueError> {
        let id = delivery.id;
        let owner = delivery.lease_owner.clone();
        let now_seconds = (self.now)();
        let now = clock::rfc3339(now_seconds);
        let visible_at = clock::rfc3339(now_seconds.saturating_add(delay.as_secs()));
        let sql = format!(
            "UPDATE {MESSAGES_TABLE}
                SET lease_owner = NULL,
                    lease_expires_at = NULL,
                    visible_at = ?1,
                    updated_at = ?2
              WHERE id = ?3 AND lease_owner = ?4"
        );
        let changed = self
            .database
            .call(move |connection| -> Result<usize, QueueError> {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(sql_error)?;
                let changed = transaction
                    .execute(&sql, params![visible_at, now, id, owner])
                    .map_err(sql_error)?;
                transaction.commit().map_err(sql_error)?;
                Ok(changed)
            })
            .await
            .map_err(database_error)?;
        if changed == 1 {
            Ok(())
        } else {
            Err(QueueError(format!(
                "delivery {id} is no longer held by its consumer"
            )))
        }
    }

    /// Move a delivery to the dead-letter table with its original bytes.
    ///
    /// The move inserts the dead letter and deletes the pending row in one
    /// transaction, so a crash cannot leave the message in both places or in
    /// neither. The caller invokes this when the retry budget is spent.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError`] when the delivery is no longer held by this
    /// consumer or SQLite refuses the transaction.
    pub async fn dead_letter(&self, delivery: &Delivery, reason: &str) -> Result<(), QueueError> {
        let id = delivery.id;
        let owner = delivery.lease_owner.clone();
        let reason = reason.to_owned();
        let dead_lettered_at = self.now_text();
        let select = format!(
            "SELECT routing_key, payload, payload_digest, attempts, created_at
               FROM {MESSAGES_TABLE}
              WHERE id = ?1 AND lease_owner = ?2"
        );
        let insert = format!(
            "INSERT INTO {DEAD_LETTER_TABLE}
                 (routing_key, source_routing_key, payload, payload_digest,
                  attempts, reason, created_at, dead_lettered_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
        );
        let delete = format!("DELETE FROM {MESSAGES_TABLE} WHERE id = ?1 AND lease_owner = ?2");
        self.database
            .call(move |connection| -> Result<(), QueueError> {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(sql_error)?;
                let row: Option<(String, Vec<u8>, String, i64, String)> = {
                    let mut statement = transaction.prepare(&select).map_err(sql_error)?;
                    statement
                        .query_row(params![id, owner], |row| {
                            Ok((
                                row.get(0)?,
                                row.get(1)?,
                                row.get(2)?,
                                row.get(3)?,
                                row.get(4)?,
                            ))
                        })
                        .optional()
                        .map_err(sql_error)?
                };
                let Some((source_routing_key, payload, payload_digest, attempts, created_at)) = row
                else {
                    return Err(QueueError(format!(
                        "delivery {id} is no longer held by its consumer"
                    )));
                };
                let attempts = u32::try_from(attempts).map_err(|_| {
                    QueueError(format!("stored attempts {attempts} exceed the u32 range"))
                })?;
                transaction
                    .execute(
                        &insert,
                        params![
                            ROUTING_DEAD,
                            source_routing_key,
                            payload,
                            payload_digest,
                            i64::from(attempts),
                            reason,
                            created_at,
                            dead_lettered_at
                        ],
                    )
                    .map_err(sql_error)?;
                let deleted = transaction
                    .execute(&delete, params![id, owner])
                    .map_err(sql_error)?;
                if deleted != 1 {
                    return Err(QueueError(format!(
                        "delivery {id} is no longer held by its consumer"
                    )));
                }
                transaction.commit().map_err(sql_error)?;
                Ok(())
            })
            .await
            .map_err(database_error)
    }

    /// Remove a ready message identified by its routing key and payload digest.
    ///
    /// Cancellation is durable: the row is deleted in one transaction, so a
    /// reopen does not resurrect it. A leased delivery is left to its consumer
    /// because the run row is authoritative and the worker re-checks it; the
    /// consumer's acknowledgement still succeeds. A repeated cancel is a no-op
    /// and returns 0.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError`] when SQLite refuses the transaction.
    pub async fn cancel(&self, routing_key: &str, payload: &[u8]) -> Result<usize, QueueError> {
        let routing_key = routing_key.to_owned();
        let payload_digest = sha256_hex(payload);
        let now = self.now_text();
        let sql = format!(
            "DELETE FROM {MESSAGES_TABLE}
              WHERE routing_key = ?1
                AND payload_digest = ?2
                AND (lease_owner IS NULL OR lease_expires_at <= ?3)"
        );
        self.database
            .call(move |connection| -> Result<usize, QueueError> {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(sql_error)?;
                let removed = transaction
                    .execute(&sql, params![routing_key, payload_digest, now])
                    .map_err(sql_error)?;
                transaction.commit().map_err(sql_error)?;
                Ok(removed)
            })
            .await
            .map_err(database_error)
    }

    /// How many messages are stored, ready or leased.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError`] when SQLite refuses the query.
    pub async fn pending_count(&self) -> Result<u64, QueueError> {
        let sql = format!("SELECT COUNT(*) FROM {MESSAGES_TABLE}");
        let count: i64 = self
            .database
            .call(move |connection| connection.query_row(&sql, [], |row| row.get(0)))
            .await
            .map_err(database_error)?;
        u64::try_from(count)
            .map_err(|_| QueueError(format!("the pending count {count} is negative")))
    }

    /// Every dead letter, oldest first.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError`] when SQLite refuses the query.
    pub async fn dead_letters(&self) -> Result<Vec<DeadLetter>, QueueError> {
        let sql = format!(
            "SELECT id, routing_key, source_routing_key, payload, attempts, reason,
                    dead_lettered_at
               FROM {DEAD_LETTER_TABLE}
              ORDER BY id"
        );
        let rows: Vec<DeadLetterRow> = self
            .database
            .call(move |connection| {
                let mut statement = connection.prepare(&sql)?;
                let rows = statement.query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                })?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await
            .map_err(database_error)?;
        rows.into_iter()
            .map(
                |(id, routing_key, source_routing_key, payload, attempts, reason, at)| {
                    let attempts = u32::try_from(attempts).map_err(|_| {
                        QueueError(format!("stored attempts {attempts} exceed the u32 range"))
                    })?;
                    Ok(DeadLetter {
                        id,
                        routing_key,
                        source_routing_key,
                        payload,
                        attempts,
                        reason,
                        dead_lettered_at: at,
                    })
                },
            )
            .collect()
    }

    /// The current time as RFC 3339 UTC.
    fn now_text(&self) -> String {
        clock::rfc3339((self.now)())
    }

    /// Insert the queue's two tables when they are absent.
    async fn ensure_schema(&self) -> Result<(), QueueError> {
        let ddl = format!(
            "CREATE TABLE IF NOT EXISTS {MESSAGES_TABLE} (
                 id               INTEGER PRIMARY KEY AUTOINCREMENT,
                 routing_key      TEXT NOT NULL,
                 payload          BLOB NOT NULL,
                 payload_digest   TEXT NOT NULL,
                 attempts         INTEGER NOT NULL DEFAULT 0,
                 lease_owner      TEXT,
                 lease_expires_at TEXT,
                 visible_at       TEXT NOT NULL,
                 created_at       TEXT NOT NULL,
                 updated_at       TEXT NOT NULL,
                 CHECK (attempts >= 0),
                 CHECK ((lease_owner IS NULL) = (lease_expires_at IS NULL))
             ) STRICT;
             CREATE INDEX IF NOT EXISTS {MESSAGES_TABLE}_ready
                 ON {MESSAGES_TABLE} (visible_at, id);
             CREATE INDEX IF NOT EXISTS {MESSAGES_TABLE}_cancel
                 ON {MESSAGES_TABLE} (routing_key, payload_digest);
             CREATE TABLE IF NOT EXISTS {DEAD_LETTER_TABLE} (
                 id                 INTEGER PRIMARY KEY AUTOINCREMENT,
                 routing_key        TEXT NOT NULL,
                 source_routing_key TEXT,
                 payload            BLOB NOT NULL,
                 payload_digest     TEXT NOT NULL,
                 attempts           INTEGER NOT NULL DEFAULT 0,
                 reason             TEXT NOT NULL,
                 created_at         TEXT NOT NULL,
                 dead_lettered_at   TEXT NOT NULL,
                 CHECK (attempts >= 0)
             ) STRICT;"
        );
        self.database
            .call(move |connection| -> Result<(), QueueError> {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(sql_error)?;
                transaction.execute_batch(&ddl).map_err(sql_error)?;
                transaction.commit().map_err(sql_error)?;
                Ok(())
            })
            .await
            .map_err(database_error)
    }

    /// Store a work message. `run.dead` is routed to the dead-letter table.
    async fn store(&self, routing_key: String, payload: Vec<u8>) -> Result<(), QueueError> {
        if routing_key == ROUTING_DEAD {
            let created_at = self.now_text();
            self.store_dead_letter(
                routing_key,
                None,
                payload,
                0,
                "published".to_owned(),
                created_at,
            )
            .await
        } else {
            self.store_message(routing_key, payload).await
        }
    }

    /// Insert one pending row.
    async fn store_message(&self, routing_key: String, payload: Vec<u8>) -> Result<(), QueueError> {
        let payload_digest = sha256_hex(&payload);
        let now = self.now_text();
        let sql = format!(
            "INSERT INTO {MESSAGES_TABLE}
                 (routing_key, payload, payload_digest, visible_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4, ?4)"
        );
        self.database
            .call(move |connection| -> Result<(), QueueError> {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(sql_error)?;
                transaction
                    .execute(&sql, params![routing_key, payload, payload_digest, now])
                    .map_err(sql_error)?;
                transaction.commit().map_err(sql_error)?;
                Ok(())
            })
            .await
            .map_err(database_error)
    }

    /// Insert one dead-letter row.
    async fn store_dead_letter(
        &self,
        routing_key: String,
        source_routing_key: Option<String>,
        payload: Vec<u8>,
        attempts: u32,
        reason: String,
        created_at: String,
    ) -> Result<(), QueueError> {
        let payload_digest = sha256_hex(&payload);
        let dead_lettered_at = self.now_text();
        let sql = format!(
            "INSERT INTO {DEAD_LETTER_TABLE}
                 (routing_key, source_routing_key, payload, payload_digest,
                  attempts, reason, created_at, dead_lettered_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
        );
        self.database
            .call(move |connection| -> Result<(), QueueError> {
                let transaction = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(sql_error)?;
                transaction
                    .execute(
                        &sql,
                        params![
                            routing_key,
                            source_routing_key,
                            payload,
                            payload_digest,
                            i64::from(attempts),
                            reason,
                            created_at,
                            dead_lettered_at
                        ],
                    )
                    .map_err(sql_error)?;
                transaction.commit().map_err(sql_error)?;
                Ok(())
            })
            .await
            .map_err(database_error)
    }
}

impl Queue for InProcessQueue {
    fn backend(&self) -> Backend {
        Backend::InProcess
    }

    fn publish(&self, routing_key: &str, payload: Vec<u8>) -> BoxFuture<Result<(), QueueError>> {
        if routing_key.is_empty() {
            return Box::pin(async { Err(QueueError("a routing key cannot be empty".to_owned())) });
        }
        let queue = self.clone();
        let routing_key = routing_key.to_owned();
        Box::pin(async move { queue.store(routing_key, payload).await })
    }
}

impl QueueConsumer for InProcessQueue {
    fn backend(&self) -> Backend {
        Backend::InProcess
    }

    fn consume(
        &self,
        queue: &str,
        keys: &[&str],
        handler: MessageHandler,
    ) -> BoxFuture<Result<(), QueueError>> {
        let this = self.clone();
        let queue_name = queue.to_owned();
        let keys: Vec<String> = keys.iter().map(|key| (*key).to_owned()).collect();
        Box::pin(async move {
            let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
            loop {
                match this.claim(&queue_name, &keys).await? {
                    None => tokio::time::sleep(POLL_INTERVAL).await,
                    Some(delivery) => {
                        let message = Message {
                            routing_key: delivery.routing_key.clone(),
                            payload: delivery.payload.clone(),
                            redelivered: delivery.redelivered(),
                        };
                        match handler(message).await {
                            Ok(()) => this.ack(&delivery).await?,
                            Err(error) => {
                                // RabbitMQ's nack requeues immediately, and the
                                // worker holds its own delivery for backoff. A
                                // second policy here would diverge from it.
                                return match this.nack(&delivery, Duration::ZERO).await {
                                    Ok(()) => Err(error),
                                    Err(nack) => Err(QueueError(format!(
                                        "{error}; the nack also failed: {nack}"
                                    ))),
                                };
                            }
                        }
                    }
                }
            }
        })
    }
}

/// A lease token that is unique within this process and names the queue.
fn lease_owner(queue: &str) -> String {
    let counter = NEXT_LEASE.fetch_add(1, Ordering::Relaxed);
    format!("{queue}-{}-{counter}", std::process::id())
}

/// The wall clock, in Unix seconds.
fn system_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn sql_error(error: rusqlite::Error) -> QueueError {
    QueueError(error.to_string())
}

fn database_error(error: DatabaseError) -> QueueError {
    QueueError(error.0)
}

#[cfg(test)]
mod tests {
    use super::{DEAD_LETTER_TABLE, InProcessQueue, MESSAGES_TABLE};
    use crate::adapters::{Dialect, Schema};

    #[test]
    fn the_tables_use_the_queue_prefix() {
        assert_eq!(
            MESSAGES_TABLE,
            Dialect::Sqlite
                .table(Schema::Queue, "messages")
                .expect("the messages table")
        );
        assert_eq!(
            DEAD_LETTER_TABLE,
            Dialect::Sqlite
                .table(Schema::Queue, "dead_letter")
                .expect("the dead-letter table")
        );
        assert!(!MESSAGES_TABLE.contains("queue_messages"));
        assert!(!DEAD_LETTER_TABLE.contains("queue.dead_letter"));
    }

    #[test]
    fn the_queue_is_send_and_sync() {
        fn assert<T: Send + Sync>() {}
        assert::<InProcessQueue>();
    }
}
