//! The RabbitMQ adapter.
//!
//! The only module allowed to name the `lapin` client. A message is bytes plus a
//! routing key; the envelope is `quantrule.queue/v1` and belongs to the caller.

use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicPublishOptions, BasicQosOptions,
    ConfirmSelectOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::{AMQPValue, FieldTable};
use lapin::{Channel, Confirmation, Connection, ConnectionProperties, ExchangeKind};

use crate::adapters::{
    Backend, BoxFuture, Message, MessageHandler, Queue, QueueConsumer, QueueError,
};
use crate::queue::{
    QUEUE_BACKTEST, QUEUE_DEAD_LETTER, QUEUE_EVIDENCE, QUEUE_STAGE, ROUTING_CANCELLED,
    ROUTING_COMPUTED, ROUTING_DEAD, ROUTING_REQUESTED, ROUTING_STAGE_CANCELLED,
    ROUTING_STAGE_REQUESTED,
};

/// The control-plane exchange.
pub const EXCHANGE: &str = "quantrule.runs";

/// How long a publish waits for the broker's confirmation.
const PUBLISH_CONFIRM_TIMEOUT: Duration = Duration::from_secs(5);

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
        // The publisher channel confirms every write before `publish` returns.
        // Consumers open their own channels below, so a consumer's QoS never
        // changes the publisher's confirmation mode.
        channel
            .confirm_select(ConfirmSelectOptions::default())
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
        declare_queue(
            &channel,
            QUEUE_BACKTEST,
            &[ROUTING_REQUESTED, ROUTING_CANCELLED],
        )
        .await?;
        declare_queue(&channel, QUEUE_EVIDENCE, &[ROUTING_COMPUTED]).await?;
        declare_queue(&channel, QUEUE_DEAD_LETTER, &[ROUTING_DEAD]).await?;
        declare_queue(
            &channel,
            QUEUE_STAGE,
            &[ROUTING_STAGE_REQUESTED, ROUTING_STAGE_CANCELLED],
        )
        .await?;
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

    fn consume(
        &self,
        queue: &str,
        keys: &[&str],
        handler: MessageHandler,
    ) -> BoxFuture<Result<(), QueueError>> {
        let connection = Arc::clone(&self.connection);
        let queue = queue.to_owned();
        let keys: Vec<String> = keys.iter().map(|key| (*key).to_owned()).collect();
        Box::pin(async move {
            // A consumer owns its channel and its prefetch slot. The run and
            // stage consumers therefore cannot hold each other's delivery, and
            // a second run consumer gets its own slot instead of sharing one.
            let channel = connection
                .create_channel()
                .await
                .map_err(|error| QueueError(error.to_string()))?;
            declare_queue(&channel, &queue, &keys).await?;
            channel
                .basic_qos(1, BasicQosOptions::default())
                .await
                .map_err(|error| QueueError(error.to_string()))?;
            let mut arguments = FieldTable::default();
            if queue == QUEUE_STAGE {
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

async fn declare_queue<S: AsRef<str>>(
    channel: &Channel,
    queue: &str,
    keys: &[S],
) -> Result<(), QueueError> {
    channel
        .queue_declare(
            queue.into(),
            QueueDeclareOptions {
                durable: true,
                ..QueueDeclareOptions::default()
            },
            FieldTable::default(),
        )
        .await
        .map_err(|error| QueueError(error.to_string()))?;
    for key in keys {
        channel
            .queue_bind(
                queue.into(),
                EXCHANGE.into(),
                key.as_ref().into(),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(|error| QueueError(error.to_string()))?;
    }
    Ok(())
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
}
