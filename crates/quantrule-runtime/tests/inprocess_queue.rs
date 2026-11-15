//! The durable in-process queue's behaviour.
//!
//! The fast tier has no broker and no network. These tests exercise the P2b
//! contract over a real SQLite file where the point is durability: publish,
//! claim, acknowledge, immediate requeue, the three-delivery backoff, the
//! `run.dead` dead letter with the original bytes, lease expiry, cancellation
//! by routing key and payload digest, two consumers on two connections, and a
//! reopen.
//!
//! The retry policy belongs to the worker (`docs/architecture/control-plane.md`
//! section 3), not to the queue. `three_deliveries_back_off_and_dead_letter_the_original_bytes`
//! applies that policy through the queue's explicit operations, which is how
//! P2c will wire it; the queue itself only records the visibility it is asked
//! for.
//!
//! A process kill is simulated by dropping the queue without an
//! acknowledgement and advancing the injected clock past the lease. That proves
//! the lease expiry and the persisted row, not the operating system's handling
//! of a killed process; the composed tier owns the service-level kill.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use quantrule_api::Digest;
use quantrule_runtime::adapters::inprocess::{InProcessQueue, Now};
use quantrule_runtime::adapters::sqlite::SqliteDatabase;
use quantrule_runtime::adapters::{Backend, MessageHandler, Queue, QueueConsumer, QueueError};
use quantrule_runtime::queue::{
    QUEUE_BACKTEST, QUEUE_EVIDENCE, ROUTING_CANCELLED, ROUTING_COMPUTED, ROUTING_DEAD,
    ROUTING_REQUESTED,
};

/// The worker's declared backoff, copied here so the harness applies the
/// contract rather than the queue inventing one.
const RETRY_DELAYS: [Duration; 2] = [Duration::from_secs(5), Duration::from_secs(25)];

/// A fixed, advanceable clock.
#[derive(Clone)]
struct TestClock {
    seconds: Arc<AtomicU64>,
}

impl TestClock {
    fn new(seconds: u64) -> Self {
        Self {
            seconds: Arc::new(AtomicU64::new(seconds)),
        }
    }

    fn now(&self) -> u64 {
        self.seconds.load(Ordering::SeqCst)
    }

    fn advance(&self, seconds: u64) {
        self.seconds.fetch_add(seconds, Ordering::SeqCst);
    }

    fn function(&self) -> Now {
        let clock = self.clone();
        Arc::new(move || clock.now())
    }
}

fn database_path(tag: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("inprocess-queue-{tag}"));
    let _ = std::fs::remove_dir_all(&directory);
    directory.join("queue.sqlite")
}

fn cleanup(path: &Path) {
    if let Some(directory) = path.parent() {
        let _ = std::fs::remove_dir_all(directory);
    }
}

async fn memory_queue(clock: &TestClock) -> InProcessQueue {
    let database = SqliteDatabase::open_in_memory().expect("the in-memory database opens");
    InProcessQueue::with_clock(database, clock.function())
        .await
        .expect("the queue opens")
}

async fn file_queue(path: &Path, clock: &TestClock) -> InProcessQueue {
    let database = SqliteDatabase::open(path).expect("the file database opens");
    InProcessQueue::with_clock(database, clock.function())
        .await
        .expect("the queue opens")
}

async fn drain(queue: InProcessQueue, seen: Arc<Mutex<Vec<Vec<u8>>>>) {
    while let Some(delivery) = queue
        .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
        .await
        .expect("claim")
    {
        seen.lock().expect("seen").push(delivery.payload.clone());
        queue.ack(&delivery).await.expect("ack");
    }
}

#[tokio::test]
async fn a_published_message_is_claimed_once_and_acked() {
    let clock = TestClock::new(1_000);
    let queue = memory_queue(&clock).await;
    assert_eq!(Queue::backend(&queue), Backend::InProcess);
    queue
        .publish(ROUTING_REQUESTED, b"run-one".to_vec())
        .await
        .expect("publish");
    assert_eq!(queue.pending_count().await.expect("count"), 1);

    let delivery = queue
        .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
        .await
        .expect("claim")
        .expect("a delivery");
    assert_eq!(delivery.routing_key, ROUTING_REQUESTED);
    assert_eq!(delivery.payload, b"run-one".to_vec());
    assert_eq!(delivery.attempts, 1);
    assert!(!delivery.redelivered());
    assert!(
        queue
            .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
            .await
            .expect("claim")
            .is_none(),
        "a leased message is not delivered twice"
    );

    queue.ack(&delivery).await.expect("ack");
    assert_eq!(queue.pending_count().await.expect("count"), 0);
    assert!(
        queue
            .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
            .await
            .expect("claim")
            .is_none()
    );
}

#[tokio::test]
async fn a_nack_redelivers_the_same_bytes() {
    let clock = TestClock::new(1_000);
    let queue = memory_queue(&clock).await;
    let payload = br#"{"run_id":"sha256:redeliver"}"#.to_vec();
    queue
        .publish(ROUTING_REQUESTED, payload.clone())
        .await
        .expect("publish");

    let first = queue
        .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
        .await
        .expect("claim")
        .expect("the first delivery");
    queue
        .nack(&first, Duration::ZERO)
        .await
        .expect("the immediate requeue");

    let second = queue
        .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
        .await
        .expect("claim")
        .expect("the redelivery");
    assert_eq!(second.payload, payload);
    assert_eq!(second.attempts, 2);
    assert!(second.redelivered());
    assert_eq!(Digest::sha256(&second.payload), Digest::sha256(&payload));
    queue.ack(&second).await.expect("ack");
}

#[tokio::test]
async fn three_deliveries_back_off_and_dead_letter_the_original_bytes() {
    let clock = TestClock::new(1_000);
    let queue = memory_queue(&clock).await;
    let payload = br#"{"run_id":"sha256:dead"}"#.to_vec();
    queue
        .publish(ROUTING_REQUESTED, payload.clone())
        .await
        .expect("publish");

    let first = queue
        .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
        .await
        .expect("claim")
        .expect("the first delivery");
    assert_eq!(first.attempts, 1);
    queue
        .nack(&first, RETRY_DELAYS[0])
        .await
        .expect("the first backoff");
    assert!(
        queue
            .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
            .await
            .expect("claim")
            .is_none(),
        "the message is not visible before 5 s"
    );
    clock.advance(4);
    assert!(
        queue
            .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
            .await
            .expect("claim")
            .is_none(),
        "the message is not visible at 4 s"
    );
    clock.advance(1);

    let second = queue
        .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
        .await
        .expect("claim")
        .expect("the second delivery");
    assert_eq!(second.attempts, 2);
    queue
        .nack(&second, RETRY_DELAYS[1])
        .await
        .expect("the second backoff");
    clock.advance(24);
    assert!(
        queue
            .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
            .await
            .expect("claim")
            .is_none(),
        "the message is not visible at 24 s"
    );
    clock.advance(1);

    let third = queue
        .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
        .await
        .expect("claim")
        .expect("the third delivery");
    assert_eq!(third.attempts, 3);
    queue
        .dead_letter(&third, "the retry budget is spent")
        .await
        .expect("dead-letter");

    assert_eq!(queue.pending_count().await.expect("count"), 0);
    let dead = queue.dead_letters().await.expect("dead letters");
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].routing_key, ROUTING_DEAD);
    assert_eq!(
        dead[0].source_routing_key.as_deref(),
        Some(ROUTING_REQUESTED)
    );
    assert_eq!(dead[0].payload, payload);
    assert_eq!(dead[0].attempts, 3);
    assert_eq!(dead[0].reason, "the retry budget is spent");
    assert!(dead[0].dead_lettered_at.ends_with('Z'));
    assert!(
        queue
            .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
            .await
            .expect("claim")
            .is_none()
    );
}

#[tokio::test]
async fn an_expired_lease_is_redelivered_and_the_old_ack_is_refused() {
    let clock = TestClock::new(1_000);
    let queue = memory_queue(&clock)
        .await
        .with_lease(Duration::from_secs(10));
    let payload = b"leased".to_vec();
    queue
        .publish(ROUTING_REQUESTED, payload.clone())
        .await
        .expect("publish");

    let first = queue
        .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
        .await
        .expect("claim")
        .expect("the first delivery");
    assert!(
        queue
            .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
            .await
            .expect("claim")
            .is_none()
    );
    clock.advance(9);
    assert!(
        queue
            .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
            .await
            .expect("claim")
            .is_none(),
        "the lease is alive at 9 s"
    );
    clock.advance(1);

    let second = queue
        .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
        .await
        .expect("claim")
        .expect("the redelivery");
    assert_eq!(second.payload, payload);
    assert_eq!(second.attempts, 2);
    assert!(second.redelivered());
    assert!(
        queue.ack(&first).await.is_err(),
        "the consumer whose lease expired cannot acknowledge it"
    );
    queue.ack(&second).await.expect("ack");
    assert_eq!(queue.pending_count().await.expect("count"), 0);
}

#[tokio::test]
async fn cancellation_matches_key_and_digest_and_survives_a_reopen() {
    let path = database_path("cancel");
    let clock = TestClock::new(1_000);
    let payload = br#"{"run_id":"sha256:cancel"}"#.to_vec();
    {
        let queue = file_queue(&path, &clock).await;
        queue
            .publish(ROUTING_REQUESTED, payload.clone())
            .await
            .expect("the request");
        queue
            .publish(ROUTING_COMPUTED, payload.clone())
            .await
            .expect("a message under another key");
        queue
            .publish(ROUTING_CANCELLED, payload.clone())
            .await
            .expect("the cancellation");
        assert_eq!(queue.pending_count().await.expect("count"), 3);

        assert_eq!(
            queue
                .cancel("run.unrelated", &payload)
                .await
                .expect("a wrong key"),
            0
        );
        assert_eq!(
            queue
                .cancel(ROUTING_REQUESTED, &payload)
                .await
                .expect("the cancellation"),
            1
        );
        assert_eq!(
            queue
                .cancel(ROUTING_REQUESTED, &payload)
                .await
                .expect("a repeated cancellation"),
            0
        );
        assert_eq!(queue.pending_count().await.expect("count"), 2);

        let computed = queue
            .claim(QUEUE_EVIDENCE, &[ROUTING_COMPUTED])
            .await
            .expect("claim")
            .expect("the computed message");
        assert_eq!(computed.routing_key, ROUTING_COMPUTED);
        assert_eq!(computed.payload, payload);
        queue.ack(&computed).await.expect("ack");

        let cancellation = queue
            .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED, ROUTING_CANCELLED])
            .await
            .expect("claim")
            .expect("the cancellation");
        assert_eq!(cancellation.routing_key, ROUTING_CANCELLED);
        assert_eq!(cancellation.payload, payload);
        assert_eq!(queue.pending_count().await.expect("count"), 1);
    }
    {
        let queue = file_queue(&path, &clock).await;
        assert_eq!(queue.pending_count().await.expect("count"), 1);
        assert!(
            queue
                .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
                .await
                .expect("claim")
                .is_none(),
            "a cancelled request never returns"
        );

        // The cancellation was leased by the first process and never acked.
        clock.advance(301);
        let cancellation = queue
            .claim(QUEUE_BACKTEST, &[ROUTING_CANCELLED])
            .await
            .expect("claim")
            .expect("the durable cancellation");
        assert_eq!(cancellation.payload, payload);
        assert_eq!(cancellation.attempts, 2);
        queue.ack(&cancellation).await.expect("ack");
    }
    cleanup(&path);
}

#[tokio::test]
async fn cancellation_leaves_a_leased_delivery_to_its_consumer() {
    let clock = TestClock::new(1_000);
    let queue = memory_queue(&clock).await;
    queue
        .publish(ROUTING_REQUESTED, b"leased".to_vec())
        .await
        .expect("publish");
    let delivery = queue
        .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
        .await
        .expect("claim")
        .expect("the delivery");

    assert_eq!(
        queue
            .cancel(ROUTING_REQUESTED, b"leased")
            .await
            .expect("cancel"),
        0,
        "a leased delivery is owned by its consumer"
    );
    assert_eq!(queue.pending_count().await.expect("count"), 1);
    queue.ack(&delivery).await.expect("ack");
    assert_eq!(queue.pending_count().await.expect("count"), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_consumers_deliver_each_message_once() {
    let path = database_path("concurrent");
    let clock = TestClock::new(1_000);
    let first = file_queue(&path, &clock).await;
    let second = file_queue(&path, &clock).await;
    let total = 32;
    for index in 0..total {
        first
            .publish(ROUTING_REQUESTED, format!("run-{index}").into_bytes())
            .await
            .expect("publish");
    }

    let seen = Arc::new(Mutex::new(Vec::new()));
    tokio::join!(
        drain(first.clone(), Arc::clone(&seen)),
        drain(second.clone(), Arc::clone(&seen)),
    );

    let mut seen = seen.lock().expect("seen").clone();
    seen.sort();
    let mut expected: Vec<Vec<u8>> = (0..total)
        .map(|index| format!("run-{index}").into_bytes())
        .collect();
    expected.sort();
    assert_eq!(seen, expected, "every message is delivered exactly once");
    assert_eq!(first.pending_count().await.expect("count"), 0);
    drop(first);
    drop(second);
    cleanup(&path);
}

#[tokio::test]
async fn a_published_message_survives_a_reopen_and_an_ack_is_durable() {
    let path = database_path("restart");
    let clock = TestClock::new(1_000);
    let payload = b"persisted".to_vec();
    {
        let queue = file_queue(&path, &clock).await;
        queue
            .publish(ROUTING_REQUESTED, payload.clone())
            .await
            .expect("publish");
    }
    {
        let queue = file_queue(&path, &clock).await;
        let delivery = queue
            .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
            .await
            .expect("claim")
            .expect("the message");
        assert_eq!(delivery.payload, payload);
        assert!(!delivery.redelivered());
        queue.ack(&delivery).await.expect("ack");
    }
    {
        let queue = file_queue(&path, &clock).await;
        assert_eq!(queue.pending_count().await.expect("count"), 0);
        assert!(
            queue
                .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
                .await
                .expect("claim")
                .is_none()
        );
    }
    cleanup(&path);
}

#[tokio::test]
async fn an_unacked_delivery_survives_a_reopen_and_is_redelivered_after_its_lease() {
    let path = database_path("unacked");
    let clock = TestClock::new(1_000);
    let payload = b"unacked".to_vec();
    {
        let queue = file_queue(&path, &clock).await;
        queue
            .publish(ROUTING_REQUESTED, payload.clone())
            .await
            .expect("publish");
        let delivery = queue
            .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
            .await
            .expect("claim")
            .expect("the first delivery");
        assert_eq!(delivery.attempts, 1);
        // A killed consumer leaves the row leased. Drop without an ack.
    }
    {
        let queue = file_queue(&path, &clock).await;
        assert!(
            queue
                .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
                .await
                .expect("claim")
                .is_none(),
            "the lease is still alive"
        );
        clock.advance(301);
        let delivery = queue
            .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
            .await
            .expect("claim")
            .expect("the redelivery");
        assert_eq!(delivery.payload, payload);
        assert_eq!(delivery.attempts, 2);
        assert!(delivery.redelivered());
        queue.ack(&delivery).await.expect("ack");
    }
    cleanup(&path);
}

#[tokio::test]
async fn the_consumer_acks_a_successful_handler() {
    let clock = TestClock::new(1_000);
    let queue = memory_queue(&clock).await;
    queue
        .publish(ROUTING_REQUESTED, b"consumed".to_vec())
        .await
        .expect("publish");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let target = Arc::clone(&seen);
    let handler: MessageHandler = Box::new(move |message| {
        let target = Arc::clone(&target);
        Box::pin(async move {
            target.lock().expect("seen").push(message);
            Ok(())
        })
    });

    let outcome = tokio::time::timeout(
        Duration::from_secs(2),
        queue.consume(QUEUE_BACKTEST, &[ROUTING_REQUESTED], handler),
    )
    .await;
    assert!(
        outcome.is_err(),
        "an idle consumer keeps polling until the service stops it"
    );
    {
        let seen = seen.lock().expect("seen");
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].payload, b"consumed".to_vec());
        assert!(!seen[0].redelivered);
    }
    assert_eq!(queue.pending_count().await.expect("count"), 0);
}

#[tokio::test]
async fn the_consumer_requeues_immediately_after_a_handler_failure() {
    let clock = TestClock::new(1_000);
    let queue = memory_queue(&clock).await;
    queue
        .publish(ROUTING_REQUESTED, b"fails".to_vec())
        .await
        .expect("publish");
    let handler: MessageHandler =
        Box::new(|_| Box::pin(async { Err(QueueError("the handler failed".to_owned())) }));

    let outcome = tokio::time::timeout(
        Duration::from_secs(2),
        queue.consume(QUEUE_BACKTEST, &[ROUTING_REQUESTED], handler),
    )
    .await
    .expect("the consumer returns after a handler failure");
    let error = outcome.expect_err("the handler error is returned");
    assert!(error.0.contains("the handler failed"), "{error}");

    let delivery = queue
        .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED])
        .await
        .expect("claim")
        .expect("the immediate requeue");
    assert_eq!(delivery.payload, b"fails".to_vec());
    assert_eq!(delivery.attempts, 2);
    assert!(delivery.redelivered());
    queue.ack(&delivery).await.expect("ack");
}

#[tokio::test]
async fn the_queue_is_usable_behind_the_publish_and_consume_traits() {
    let clock = TestClock::new(1_000);
    let queue = memory_queue(&clock).await;
    let publisher: Arc<dyn Queue> = Arc::new(queue.clone());
    assert_eq!(publisher.backend(), Backend::InProcess);
    publisher
        .publish(ROUTING_REQUESTED, b"trait".to_vec())
        .await
        .expect("publish");

    let seen = Arc::new(Mutex::new(Vec::new()));
    let target = Arc::clone(&seen);
    let handler: MessageHandler = Box::new(move |message| {
        let target = Arc::clone(&target);
        Box::pin(async move {
            target.lock().expect("seen").push(message);
            Ok(())
        })
    });
    let consumer: Arc<dyn QueueConsumer> = Arc::new(queue);
    let outcome = tokio::time::timeout(
        Duration::from_secs(2),
        consumer.consume(QUEUE_BACKTEST, &[ROUTING_REQUESTED], handler),
    )
    .await;
    assert!(outcome.is_err());
    assert_eq!(seen.lock().expect("seen").len(), 1);
}

#[tokio::test]
async fn a_published_run_dead_is_stored_with_its_bytes() {
    let clock = TestClock::new(1_000);
    let queue = memory_queue(&clock).await;
    let payload = br#"{"run_id":"sha256:dead"}"#.to_vec();
    queue
        .publish(ROUTING_DEAD, payload.clone())
        .await
        .expect("publish");

    assert_eq!(queue.pending_count().await.expect("count"), 0);
    let dead = queue.dead_letters().await.expect("dead letters");
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].routing_key, ROUTING_DEAD);
    assert_eq!(dead[0].source_routing_key, None);
    assert_eq!(dead[0].payload, payload);
    assert_eq!(dead[0].attempts, 0);
    assert_eq!(dead[0].reason, "published");
}

#[tokio::test]
async fn a_claim_reads_only_the_keys_the_queue_is_bound_to() {
    let clock = TestClock::new(1_000);
    let queue = memory_queue(&clock).await;
    queue
        .publish(ROUTING_REQUESTED, b"requested".to_vec())
        .await
        .expect("publish requested");
    queue
        .publish(ROUTING_COMPUTED, b"computed".to_vec())
        .await
        .expect("publish computed");

    let computed = queue
        .claim(QUEUE_EVIDENCE, &[ROUTING_COMPUTED])
        .await
        .expect("claim")
        .expect("the computed message");
    assert_eq!(computed.payload, b"computed".to_vec());
    queue.ack(&computed).await.expect("ack");

    let requested = queue
        .claim(QUEUE_BACKTEST, &[ROUTING_REQUESTED, ROUTING_CANCELLED])
        .await
        .expect("claim")
        .expect("the requested message");
    assert_eq!(requested.payload, b"requested".to_vec());
    queue.ack(&requested).await.expect("ack");
}

#[tokio::test]
async fn a_claim_with_no_bound_keys_delivers_nothing() {
    let clock = TestClock::new(1_000);
    let queue = memory_queue(&clock).await;
    queue
        .publish(ROUTING_REQUESTED, b"kept".to_vec())
        .await
        .expect("publish");
    assert!(
        queue
            .claim(QUEUE_BACKTEST, &[])
            .await
            .expect("claim")
            .is_none()
    );
    assert_eq!(queue.pending_count().await.expect("count"), 1);
}

#[tokio::test]
async fn the_file_database_runs_in_wal_with_a_busy_timeout() {
    let path = database_path("wal");
    let clock = TestClock::new(1_000);
    let database = SqliteDatabase::open(&path).expect("the file database opens");
    let queue = InProcessQueue::with_clock(database.clone(), clock.function())
        .await
        .expect("the queue opens");
    let (mode, busy_timeout): (String, i64) = database
        .call(|connection| -> rusqlite::Result<(String, i64)> {
            let mode =
                connection.query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))?;
            let timeout =
                connection.query_row("PRAGMA busy_timeout", [], |row| row.get::<_, i64>(0))?;
            Ok((mode, timeout))
        })
        .await
        .expect("the pragmas");
    assert_eq!(mode.to_ascii_lowercase(), "wal");
    assert!(
        busy_timeout >= 5_000,
        "the busy timeout is {busy_timeout} ms"
    );
    drop(queue);
    drop(database);
    cleanup(&path);
}

#[tokio::test]
async fn an_empty_routing_key_is_refused() {
    let clock = TestClock::new(1_000);
    let queue = memory_queue(&clock).await;
    assert!(queue.publish("", b"x".to_vec()).await.is_err());
    assert_eq!(queue.pending_count().await.expect("count"), 0);
}

#[test]
fn the_fixture_clock_advances_without_sleeping() {
    let clock = TestClock::new(10);
    assert_eq!(clock.now(), 10);
    clock.advance(5);
    assert_eq!(clock.now(), 15);
}
