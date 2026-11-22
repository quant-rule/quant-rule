//! Opt-in integration test for the quorum queue topology.
//!
//! The fast gate cannot reach a broker, so this test is ignored by default.
//! Run it against a disposable RabbitMQ 4 broker:
//!
//! ```bash
//! docker run --rm -d --name qr-rabbitmq-test -p 127.0.0.1:5673:5672 \
//!   -e RABBITMQ_DEFAULT_USER=quantrule \
//!   -e RABBITMQ_DEFAULT_PASS=quantrule \
//!   rabbitmq:4-management
//! QR_RABBITMQ_TEST_URL=amqp://quantrule:quantrule@127.0.0.1:5673/%2f \
//!   cargo test -p quantrule-runtime --test rabbitmq_quorum -- --ignored --nocapture
//! ```
//!
//! The test uses a unique queue generation, so it does not touch a running
//! stack's generation 1 queues. It publishes with confirms, consumes with a
//! manual ack, proves a failing handler is limited and lands in the dead-letter
//! queue, and removes its own queues.
//!
//! `QR_RABBITMQ_TEST_RECONNECT_URL` optionally names a second broker address
//! for the consumer. `QR_RABBITMQ_TEST_PAUSE_MS` pauses after the publish so a
//! three-node cluster test can stop a broker before the consumer reconnects.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use lapin::options::{BasicAckOptions, BasicGetOptions, QueueDeleteOptions};
use lapin::types::AMQPValue;
use lapin::{Connection, ConnectionProperties};
use quantrule_runtime::adapters::rabbitmq::{DELIVERY_LIMIT, RabbitQueue};
use quantrule_runtime::adapters::{Message, MessageHandler, Queue, QueueConsumer, QueueError};
use quantrule_runtime::queue::{QueueKind, QueueNames, ROUTING_REQUESTED};

#[tokio::test]
#[ignore = "requires QR_RABBITMQ_TEST_URL and a disposable RabbitMQ 4 broker"]
async fn acknowledged_quorum_messages_survive_and_exhausted_deliveries_reach_the_dlq() {
    let url = std::env::var("QR_RABBITMQ_TEST_URL")
        .expect("set QR_RABBITMQ_TEST_URL to a disposable RabbitMQ 4 broker");
    let reconnect = std::env::var("QR_RABBITMQ_TEST_RECONNECT_URL").unwrap_or_else(|_| url.clone());
    let generation = std::env::var("QR_RABBITMQ_TEST_GENERATION")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or_else(|| std::process::id().max(2));
    let names = QueueNames::new(generation);

    let result = run(&url, &reconnect, &names).await;
    if std::env::var("QR_RABBITMQ_TEST_KEEP").is_err() {
        cleanup(&reconnect, &names).await;
    }
    if let Err(error) = result {
        panic!("{error}");
    }
}

async fn run(publish_url: &str, consume_url: &str, names: &QueueNames) -> Result<(), String> {
    let publisher = RabbitQueue::connect(publish_url, names.clone())
        .await
        .map_err(|error| format!("publisher connect: {error}"))?;
    let payload = br#"{"run_id":"sha256:quorum-integration"}"#.to_vec();
    publisher
        .publish(ROUTING_REQUESTED, payload.clone())
        .await
        .map_err(|error| format!("publish: {error}"))?;

    let pause_ms = std::env::var("QR_RABBITMQ_TEST_PAUSE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    if pause_ms > 0 {
        println!(
            "published {}; pausing {pause_ms} ms for a broker-loss step",
            names.name(QueueKind::Backtest)
        );
        tokio::time::sleep(Duration::from_millis(pause_ms)).await;
    }

    // A fresh connection after the publish is the reconnect a failover forces
    // on the client. It re-declares the same generation, which must be
    // idempotent.
    let consumer_queue = RabbitQueue::connect(consume_url, names.clone())
        .await
        .map_err(|error| format!("consumer connect: {error}"))?;

    let captured = Arc::new(Mutex::new(None));
    let handler = capture_handler(Arc::clone(&captured));
    let mut consumer =
        tokio::spawn(consumer_queue.consume(names.name(QueueKind::Backtest), handler));
    let message = tokio::select! {
        result = &mut consumer => {
            return Err(format!(
                "the consumer ended before the acknowledged message arrived: {result:?}"
            ));
        }
        message = wait_for_message(&captured, Duration::from_secs(15)) => message?,
    };
    assert_eq!(message.routing_key, ROUTING_REQUESTED);
    assert_eq!(message.payload, payload);
    assert!(
        !message.redelivered,
        "the first delivery is not a redelivery"
    );
    // The adapter acks after the handler returns. Give the ack a moment before
    // the consumer task is aborted; the poison phase then sees an empty queue.
    tokio::time::sleep(Duration::from_millis(250)).await;
    consumer.abort();
    let _ = consumer.await;

    let poison = br#"{"run_id":"sha256:quorum-poison"}"#.to_vec();
    consumer_queue
        .publish(ROUTING_REQUESTED, poison.clone())
        .await
        .map_err(|error| format!("poison publish: {error}"))?;
    for attempt in 1..=DELIVERY_LIMIT + 1 {
        let outcome = tokio::time::timeout(
            Duration::from_secs(10),
            consumer_queue.consume(names.name(QueueKind::Backtest), failing_handler()),
        )
        .await
        .map_err(|_| format!("delivery {attempt} of the poison message never arrived"))?;
        match outcome {
            Err(_) => {}
            Ok(()) => return Err(format!("delivery {attempt} ended without a handler error")),
        }
    }

    let connection = Connection::connect(consume_url, ConnectionProperties::default())
        .await
        .map_err(|error| format!("dead-letter connect: {error}"))?;
    let channel = connection
        .create_channel()
        .await
        .map_err(|error| format!("dead-letter channel: {error}"))?;
    let mut dead_letter = None;
    for _ in 0..100 {
        let message = channel
            .basic_get(
                names.name(QueueKind::DeadLetter).into(),
                BasicGetOptions { no_ack: false },
            )
            .await
            .map_err(|error| format!("dead-letter get: {error}"))?;
        if let Some(message) = message {
            channel
                .basic_ack(message.delivery.delivery_tag, BasicAckOptions::default())
                .await
                .map_err(|error| format!("dead-letter ack: {error}"))?;
            dead_letter = Some(message.delivery);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let dead_letter = dead_letter.ok_or_else(|| {
        format!(
            "the poison message did not reach {}",
            names.name(QueueKind::DeadLetter)
        )
    })?;
    assert_eq!(dead_letter.routing_key.as_str(), ROUTING_REQUESTED);
    assert_eq!(dead_letter.data, poison);
    assert!(
        died_on_delivery_limit(&dead_letter),
        "the dead letter has no x-death entry with reason delivery_limit: {:?}",
        dead_letter.properties.headers()
    );
    Ok(())
}

fn capture_handler(captured: Arc<Mutex<Option<Message>>>) -> MessageHandler {
    Arc::new(move |message: Message| {
        *captured.lock().expect("the capture lock is poisoned") = Some(message);
        Box::pin(async { Ok(()) })
    })
}

fn failing_handler() -> MessageHandler {
    Arc::new(|_message: Message| Box::pin(async { Err(QueueError("poison".to_owned())) }))
}

async fn wait_for_message(
    captured: &Arc<Mutex<Option<Message>>>,
    timeout: Duration,
) -> Result<Message, String> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if let Some(message) = captured
            .lock()
            .expect("the capture lock is poisoned")
            .take()
        {
            return Ok(message);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("the acknowledged message never arrived".to_owned());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn died_on_delivery_limit(delivery: &lapin::message::Delivery) -> bool {
    let Some(headers) = delivery.properties.headers().as_ref() else {
        return false;
    };
    let Some(AMQPValue::FieldArray(deaths)) = headers.inner().get("x-death") else {
        return false;
    };
    deaths.as_slice().iter().any(|entry| {
        let AMQPValue::FieldTable(table) = entry else {
            return false;
        };
        matches!(
            table.inner().get("reason"),
            Some(AMQPValue::LongString(reason)) if reason.as_bytes() == b"delivery_limit"
        )
    })
}

async fn cleanup(url: &str, names: &QueueNames) {
    let Ok(connection) = Connection::connect(url, ConnectionProperties::default()).await else {
        return;
    };
    for kind in QueueKind::ALL {
        if let Ok(channel) = connection.create_channel().await {
            let _ = channel
                .queue_delete(names.name(kind).into(), QueueDeleteOptions::default())
                .await;
        }
    }
    // The exchanges are shared by every generation, so the test leaves them in
    // place; a disposable broker discards them with the container.
    let _ = connection.close(200, "test cleanup".into()).await;
}
