//! The RabbitMQ adapter.
//!
//! The only module allowed to name the `lapin` client. A message is bytes plus a
//! routing key; the envelope is `quantrule.queue/v1` and belongs to the caller.
//!
//! # Topology
//!
//! Every queue is a durable quorum queue. The run, evidence, and stage work
//! queues carry a three-delivery limit and dead-letter to
//! [`DEAD_LETTER_EXCHANGE`]; the dead-letter queue itself has no dead-letter
//! target, so an exhausted message cannot loop. Queue names come from
//! [`QueueNames`], which appends the configured generation. A changed
//! declaration therefore arrives under a new name instead of colliding with a
//! live queue.
//!
//! The adapter declares the topology when it connects, so both the producer and
//! the consumer have the same arguments. The consumer re-declares the one queue
//! it consumes with the same arguments; a mismatch is a broker error, not a
//! silent second definition.

use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, BasicQosOptions, BasicRejectOptions,
    ConfirmSelectOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::{AMQPValue, FieldTable};
use lapin::{Channel, Confirmation, Connection, ConnectionProperties, ExchangeKind};

use crate::adapters::{
    Backend, BoxFuture, Message, MessageHandler, Queue, QueueConsumer, QueueError,
};
use crate::queue::{
    QueueKind, QueueNames, ROUTING_CANCELLED, ROUTING_COMPUTED, ROUTING_DEAD, ROUTING_REQUESTED,
    ROUTING_STAGE_CANCELLED, ROUTING_STAGE_REQUESTED,
};

/// The control-plane exchange.
pub const EXCHANGE: &str = "quantrule.runs";

/// The dead-letter exchange for every queue generation.
pub const DEAD_LETTER_EXCHANGE: &str = "quantrule.runs.dlx";

/// How many failed deliveries a work message gets before it is dead-lettered.
///
/// A message may be redelivered up to this many times; the next failed
/// delivery is dead-lettered. This is the broker's safety net for a consumer
/// that keeps returning an error. The worker's own retry budget is also three
/// attempts; a run that spends it marks the run failed and publishes `run.dead`
/// before acknowledging, so this limit catches a consumer that cannot reach
/// that path.
pub const DELIVERY_LIMIT: i32 = 3;

/// The queue type every queue is declared with.
const QUEUE_TYPE: &str = "quorum";

/// How long a publish waits for the broker's confirmation.
const PUBLISH_CONFIRM_TIMEOUT: Duration = Duration::from_secs(5);

/// One queue binding in the declared topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Binding {
    exchange: &'static str,
    queue: QueueKind,
    key: &'static str,
}

/// Bindings from the control-plane exchange to a work queue or the dead-letter
/// queue.
const MAIN_BINDINGS: &[Binding] = &[
    Binding {
        exchange: EXCHANGE,
        queue: QueueKind::Backtest,
        key: ROUTING_REQUESTED,
    },
    Binding {
        exchange: EXCHANGE,
        queue: QueueKind::Backtest,
        key: ROUTING_CANCELLED,
    },
    Binding {
        exchange: EXCHANGE,
        queue: QueueKind::Evidence,
        key: ROUTING_COMPUTED,
    },
    // The worker publishes `run.dead` explicitly when its retry budget is
    // spent. The same queue also receives broker-dead-lettered work.
    Binding {
        exchange: EXCHANGE,
        queue: QueueKind::DeadLetter,
        key: ROUTING_DEAD,
    },
    Binding {
        exchange: EXCHANGE,
        queue: QueueKind::Stage,
        key: ROUTING_STAGE_REQUESTED,
    },
    Binding {
        exchange: EXCHANGE,
        queue: QueueKind::Stage,
        key: ROUTING_STAGE_CANCELLED,
    },
];

/// Bindings from the dead-letter exchange to the dead-letter queue.
///
/// The original routing key is preserved, so an exhausted `run.requested`
/// arrives as `run.requested` and an operator can replay it unchanged. The
/// `x-death` header records the source queue, the reason, and the count.
const DEAD_LETTER_BINDINGS: &[Binding] = &[
    Binding {
        exchange: DEAD_LETTER_EXCHANGE,
        queue: QueueKind::DeadLetter,
        key: ROUTING_REQUESTED,
    },
    Binding {
        exchange: DEAD_LETTER_EXCHANGE,
        queue: QueueKind::DeadLetter,
        key: ROUTING_CANCELLED,
    },
    Binding {
        exchange: DEAD_LETTER_EXCHANGE,
        queue: QueueKind::DeadLetter,
        key: ROUTING_COMPUTED,
    },
    Binding {
        exchange: DEAD_LETTER_EXCHANGE,
        queue: QueueKind::DeadLetter,
        key: ROUTING_STAGE_REQUESTED,
    },
    Binding {
        exchange: DEAD_LETTER_EXCHANGE,
        queue: QueueKind::DeadLetter,
        key: ROUTING_STAGE_CANCELLED,
    },
];

/// A RabbitMQ channel.
pub struct RabbitQueue {
    connection: Arc<Connection>,
    channel: Channel,
    names: QueueNames,
}

impl std::fmt::Debug for RabbitQueue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RabbitQueue")
            .field("exchange", &EXCHANGE)
            .field("generation", &self.names.generation())
            .finish()
    }
}

impl RabbitQueue {
    /// Connect and declare the durable exchanges and the queue generation.
    ///
    /// # Errors
    ///
    /// Returns the broker message when the connection or a declaration fails.
    pub async fn connect(url: &str, names: QueueNames) -> Result<Self, QueueError> {
        let connection = Connection::connect(url, ConnectionProperties::default())
            .await
            .map_err(|error| QueueError(error.to_string()))?;
        let channel = connection
            .create_channel()
            .await
            .map_err(|error| QueueError(error.to_string()))?;
        // The publisher channel confirms every write before `publish` returns.
        // Consumers open their own channels below, so a consumer's QoS never
        // changes the publisher's confirmation mode.
        channel
            .confirm_select(ConfirmSelectOptions::default())
            .await
            .map_err(|error| QueueError(error.to_string()))?;
        declare_topology(&channel, &names).await?;
        Ok(Self {
            connection: Arc::new(connection),
            channel,
            names,
        })
    }

    /// The queue generation this connection declared and consumes.
    #[must_use]
    pub fn names(&self) -> &QueueNames {
        &self.names
    }

    /// Whether the broker is still connected.
    ///
    /// # Errors
    ///
    /// Returns the library state when the connection is not open.
    pub fn check(&self) -> Result<(), QueueError> {
        if self.connection.status().connected() {
            Ok(())
        } else {
            Err(QueueError(format!(
                "connection {:?}",
                self.connection.status()
            )))
        }
    }
}

impl Queue for RabbitQueue {
    fn backend(&self) -> Backend {
        Backend::RabbitMq
    }

    fn publish(&self, routing_key: &str, payload: Vec<u8>) -> BoxFuture<Result<(), QueueError>> {
        let channel = self.channel.clone();
        let routing_key = routing_key.to_owned();
        Box::pin(async move {
            let confirmation = channel
                .basic_publish(
                    EXCHANGE.into(),
                    routing_key.as_str().into(),
                    BasicPublishOptions::default(),
                    &payload,
                    lapin::BasicProperties::default().with_delivery_mode(2),
                )
                .await
                .map_err(|error| QueueError(error.to_string()))?;
            match tokio::time::timeout(PUBLISH_CONFIRM_TIMEOUT, confirmation).await {
                Ok(result) => confirmed(result),
                Err(_) => Err(QueueError(
                    "the broker did not confirm the message in time".to_owned(),
                )),
            }
        })
    }
}

impl QueueConsumer for RabbitQueue {
    fn backend(&self) -> Backend {
        Backend::RabbitMq
    }

    fn consume(&self, queue: &str, handler: MessageHandler) -> BoxFuture<Result<(), QueueError>> {
        let connection = Arc::clone(&self.connection);
        let names = self.names.clone();
        let queue = queue.to_owned();
        Box::pin(async move {
            let kind = names.kind(&queue).ok_or_else(|| {
                QueueError(format!(
                    "{queue} is not part of queue generation {}",
                    names.generation()
                ))
            })?;
            // A consumer owns its channel and its prefetch slot. The run and
            // stage consumers therefore cannot hold each other's delivery, and
            // a second run consumer gets its own slot instead of sharing one.
            let channel = connection
                .create_channel()
                .await
                .map_err(|error| QueueError(error.to_string()))?;
            declare_queue(&channel, &queue, queue_arguments(kind)).await?;
            channel
                .basic_qos(1, BasicQosOptions::default())
                .await
                .map_err(|error| QueueError(error.to_string()))?;
            let mut arguments = FieldTable::default();
            if kind == QueueKind::Stage {
                // A diagnostic stage is lower priority than an evidence run.
                // The queues are already separate, so this keeps the stage
                // consumer from competing with a run consumer that drains the
                // same worker process.
                arguments.insert("x-priority".into(), AMQPValue::LongInt(-1));
            }
            let mut consumer = channel
                .basic_consume(
                    queue.as_str().into(),
                    format!("quantrule-{queue}").as_str().into(),
                    BasicConsumeOptions::default(),
                    arguments,
                )
                .await
                .map_err(|error| QueueError(error.to_string()))?;
            while let Some(delivery) = consumer.next().await {
                let delivery = delivery.map_err(|error| QueueError(error.to_string()))?;
                let message = Message {
                    routing_key: delivery.routing_key.as_str().to_owned(),
                    payload: delivery.data.clone(),
                    redelivered: delivery.redelivered,
                };
                match handler(message).await {
                    Ok(()) => {
                        delivery
                            .ack(BasicAckOptions::default())
                            .await
                            .map_err(|error| QueueError(error.to_string()))?;
                    }
                    Err(error) => {
                        // RabbitMQ 4.3 and later count only `basic.reject` (a
                        // genuine failed delivery) toward the quorum delivery
                        // limit; `basic.nack` with requeue does not. Reject is
                        // therefore what makes the limit a real poison-message
                        // guard instead of an infinite requeue loop.
                        tracing::warn!(
                            queue = %queue,
                            routing_key = %delivery.routing_key,
                            delivery_count = delivery_count(&delivery),
                            "queue handler failed; rejecting the delivery"
                        );
                        delivery
                            .reject(BasicRejectOptions { requeue: true })
                            .await
                            .map_err(|error| QueueError(error.to_string()))?;
                        return Err(error);
                    }
                }
            }
            Ok(())
        })
    }
}

/// Declare both exchanges, every queue in the generation, and every binding.
async fn declare_topology(channel: &Channel, names: &QueueNames) -> Result<(), QueueError> {
    declare_exchange(channel, EXCHANGE).await?;
    declare_exchange(channel, DEAD_LETTER_EXCHANGE).await?;
    for kind in QueueKind::ALL {
        declare_queue(channel, names.name(kind), queue_arguments(kind)).await?;
    }
    for binding in MAIN_BINDINGS.iter().chain(DEAD_LETTER_BINDINGS) {
        channel
            .queue_bind(
                names.name(binding.queue).into(),
                binding.exchange.into(),
                binding.key.into(),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(|error| QueueError(error.to_string()))?;
    }
    Ok(())
}

async fn declare_exchange(channel: &Channel, name: &str) -> Result<(), QueueError> {
    channel
        .exchange_declare(
            name.into(),
            ExchangeKind::Direct,
            ExchangeDeclareOptions {
                durable: true,
                ..ExchangeDeclareOptions::default()
            },
            FieldTable::default(),
        )
        .await
        .map_err(|error| QueueError(error.to_string()))?;
    Ok(())
}

async fn declare_queue(
    channel: &Channel,
    queue: &str,
    arguments: FieldTable,
) -> Result<(), QueueError> {
    channel
        .queue_declare(
            queue.into(),
            QueueDeclareOptions {
                durable: true,
                ..QueueDeclareOptions::default()
            },
            arguments,
        )
        .await
        .map_err(|error| QueueError(error.to_string()))?;
    Ok(())
}

/// The declaration arguments for one queue.
///
/// The dead-letter queue is quorum as well, but it deliberately has no
/// dead-letter exchange and no delivery limit: a message that reaches it is
/// kept for the operator, not requeued or dead-lettered a second time.
fn queue_arguments(kind: QueueKind) -> FieldTable {
    let mut arguments = FieldTable::default();
    arguments.insert(
        "x-queue-type".into(),
        AMQPValue::LongString(QUEUE_TYPE.into()),
    );
    if kind != QueueKind::DeadLetter {
        arguments.insert(
            "x-delivery-limit".into(),
            AMQPValue::LongInt(DELIVERY_LIMIT),
        );
        arguments.insert(
            "x-dead-letter-exchange".into(),
            AMQPValue::LongString(DEAD_LETTER_EXCHANGE.into()),
        );
    }
    arguments
}

/// The broker's delivery count for a quorum message, when it supplies one.
fn delivery_count(delivery: &lapin::message::Delivery) -> Option<i32> {
    match delivery
        .properties
        .headers()
        .as_ref()?
        .inner()
        .get("x-delivery-count")?
    {
        AMQPValue::LongInt(count) => Some(*count),
        AMQPValue::LongUInt(count) => i32::try_from(*count).ok(),
        AMQPValue::LongLongInt(count) => i32::try_from(*count).ok(),
        _ => None,
    }
}

/// Map one broker confirmation onto the queue result.
fn confirmed(result: Result<Confirmation, lapin::Error>) -> Result<(), QueueError> {
    match result {
        Ok(Confirmation::Ack(_)) => Ok(()),
        Ok(Confirmation::Nack(_)) => Err(QueueError("the broker rejected the message".to_owned())),
        Ok(Confirmation::NotRequested) => Err(QueueError(
            "the channel is not in publisher-confirm mode".to_owned(),
        )),
        Err(error) => Err(QueueError(error.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_argument<'a>(arguments: &'a FieldTable, key: &str) -> Option<&'a str> {
        match arguments.inner().get(key)? {
            AMQPValue::LongString(value) => std::str::from_utf8(value.as_bytes()).ok(),
            _ => None,
        }
    }

    fn int_argument(arguments: &FieldTable, key: &str) -> Option<i32> {
        match arguments.inner().get(key)? {
            AMQPValue::LongInt(value) => Some(*value),
            AMQPValue::LongUInt(value) => i32::try_from(*value).ok(),
            AMQPValue::LongLongInt(value) => i32::try_from(*value).ok(),
            _ => None,
        }
    }

    fn bound_keys(exchange: &str, queue: QueueKind) -> Vec<&'static str> {
        MAIN_BINDINGS
            .iter()
            .chain(DEAD_LETTER_BINDINGS)
            .filter(|binding| binding.exchange == exchange && binding.queue == queue)
            .map(|binding| binding.key)
            .collect()
    }

    #[test]
    fn an_ack_is_success() {
        assert!(confirmed(Ok(Confirmation::Ack(None))).is_ok());
    }

    #[test]
    fn a_nack_is_an_error() {
        let error = confirmed(Ok(Confirmation::Nack(None))).expect_err("a nack is an error");
        assert!(error.0.contains("rejected"), "{error}");
    }

    #[test]
    fn a_channel_without_confirms_is_an_error() {
        let error =
            confirmed(Ok(Confirmation::NotRequested)).expect_err("a missing confirm is an error");
        assert!(error.0.contains("confirm"), "{error}");
    }

    #[test]
    fn work_queues_are_quorum_with_a_delivery_limit_and_dead_letter_exchange() {
        for kind in [QueueKind::Backtest, QueueKind::Evidence, QueueKind::Stage] {
            let arguments = queue_arguments(kind);
            assert_eq!(text_argument(&arguments, "x-queue-type"), Some("quorum"));
            assert_eq!(
                int_argument(&arguments, "x-delivery-limit"),
                Some(DELIVERY_LIMIT)
            );
            assert_eq!(
                text_argument(&arguments, "x-dead-letter-exchange"),
                Some(DEAD_LETTER_EXCHANGE)
            );
        }
    }

    #[test]
    fn the_dead_letter_queue_is_quorum_and_has_no_dead_letter_target() {
        let arguments = queue_arguments(QueueKind::DeadLetter);
        assert_eq!(text_argument(&arguments, "x-queue-type"), Some("quorum"));
        assert_eq!(int_argument(&arguments, "x-delivery-limit"), None);
        assert_eq!(text_argument(&arguments, "x-dead-letter-exchange"), None);
    }

    #[test]
    fn every_work_queue_is_bound_to_its_routing_keys() {
        assert_eq!(
            bound_keys(EXCHANGE, QueueKind::Backtest),
            vec![ROUTING_REQUESTED, ROUTING_CANCELLED]
        );
        assert_eq!(
            bound_keys(EXCHANGE, QueueKind::Evidence),
            vec![ROUTING_COMPUTED]
        );
        assert_eq!(
            bound_keys(EXCHANGE, QueueKind::Stage),
            vec![ROUTING_STAGE_REQUESTED, ROUTING_STAGE_CANCELLED]
        );
    }

    #[test]
    fn the_dead_letter_queue_receives_explicit_and_exhausted_work() {
        assert_eq!(
            bound_keys(EXCHANGE, QueueKind::DeadLetter),
            vec![ROUTING_DEAD]
        );
        assert_eq!(
            bound_keys(DEAD_LETTER_EXCHANGE, QueueKind::DeadLetter),
            vec![
                ROUTING_REQUESTED,
                ROUTING_CANCELLED,
                ROUTING_COMPUTED,
                ROUTING_STAGE_REQUESTED,
                ROUTING_STAGE_CANCELLED,
            ]
        );
    }

    #[test]
    fn every_generation_binds_the_same_routing_keys_to_new_names() {
        let first = QueueNames::new(1);
        let second = QueueNames::new(2);
        for binding in MAIN_BINDINGS.iter().chain(DEAD_LETTER_BINDINGS) {
            assert!(first.kind(first.name(binding.queue)).is_some());
            assert_ne!(first.name(binding.queue), second.name(binding.queue));
        }
    }
}
