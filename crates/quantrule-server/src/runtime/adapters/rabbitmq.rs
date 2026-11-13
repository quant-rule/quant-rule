//! The RabbitMQ adapter.
//!
//! The only module allowed to name the `lapin` client. A message is bytes plus a
//! routing key; the envelope is `quantrule.queue/v1` and belongs to the caller.

use std::sync::Arc;

use futures_util::StreamExt;
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicPublishOptions, BasicQosOptions,
    ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{Channel, Connection, ConnectionProperties, ExchangeKind};

use crate::runtime::adapters::{
    Backend, BoxFuture, MessageHandler, Queue, QueueConsumer, QueueError,
};

/// The control-plane exchange.
pub const EXCHANGE: &str = "quantrule.runs";

/// A RabbitMQ channel.
pub struct RabbitQueue {
    connection: Arc<Connection>,
    channel: Channel,
}

impl std::fmt::Debug for RabbitQueue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RabbitQueue")
            .field("exchange", &EXCHANGE)
            .finish()
    }
}

impl RabbitQueue {
    /// Connect and declare the durable exchange.
    ///
    /// # Errors
    ///
    /// Returns the broker message when the connection or the declaration fails.
    pub async fn connect(url: &str) -> Result<Self, QueueError> {
        let connection = Connection::connect(url, ConnectionProperties::default())
            .await
            .map_err(|error| QueueError(error.to_string()))?;
        let channel = connection
            .create_channel()
            .await
            .map_err(|error| QueueError(error.to_string()))?;
        channel
            .exchange_declare(
                EXCHANGE.into(),
                ExchangeKind::Direct,
                ExchangeDeclareOptions {
                    durable: true,
                    ..ExchangeDeclareOptions::default()
                },
                FieldTable::default(),
            )
            .await
            .map_err(|error| QueueError(error.to_string()))?;
        Ok(Self {
            connection: Arc::new(connection),
            channel,
        })
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
            channel
                .basic_publish(
                    EXCHANGE.into(),
                    routing_key.as_str().into(),
                    BasicPublishOptions::default(),
                    &payload,
                    lapin::BasicProperties::default().with_delivery_mode(2),
                )
                .await
                .map(|_| ())
                .map_err(|error| QueueError(error.to_string()))
        })
    }
}

impl QueueConsumer for RabbitQueue {
    fn backend(&self) -> Backend {
        Backend::RabbitMq
    }

    fn consume(&self, queue: &str, handler: MessageHandler) -> BoxFuture<Result<(), QueueError>> {
        let channel = self.channel.clone();
        let queue = queue.to_owned();
        Box::pin(async move {
            channel
                .queue_declare(
                    queue.as_str().into(),
                    QueueDeclareOptions {
                        durable: true,
                        ..QueueDeclareOptions::default()
                    },
                    FieldTable::default(),
                )
                .await
                .map_err(|error| QueueError(error.to_string()))?;
            channel
                .queue_bind(
                    queue.as_str().into(),
                    EXCHANGE.into(),
                    queue.as_str().into(),
                    QueueBindOptions::default(),
                    FieldTable::default(),
                )
                .await
                .map_err(|error| QueueError(error.to_string()))?;
            channel
                .basic_qos(1, BasicQosOptions::default())
                .await
                .map_err(|error| QueueError(error.to_string()))?;
            let mut consumer = channel
                .basic_consume(
                    queue.as_str().into(),
                    format!("quantrule-{queue}").as_str().into(),
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await
                .map_err(|error| QueueError(error.to_string()))?;
            while let Some(delivery) = consumer.next().await {
                let delivery = delivery.map_err(|error| QueueError(error.to_string()))?;
                let payload = delivery.data.clone();
                match handler(payload).await {
                    Ok(()) => {
                        delivery
                            .ack(BasicAckOptions::default())
                            .await
                            .map_err(|error| QueueError(error.to_string()))?;
                    }
                    Err(error) => {
                        delivery
                            .nack(BasicNackOptions {
                                requeue: true,
                                ..BasicNackOptions::default()
                            })
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
