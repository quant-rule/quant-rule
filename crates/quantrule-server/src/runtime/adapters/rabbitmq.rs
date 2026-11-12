//! The RabbitMQ adapter.
//!
//! The only module allowed to name the `lapin` client. A message is bytes plus a
//! routing key; the envelope is `quantrule.queue/v1` and belongs to the caller.

use std::sync::Arc;

use lapin::options::{BasicPublishOptions, ExchangeDeclareOptions};
use lapin::types::FieldTable;
use lapin::{Channel, Connection, ConnectionProperties, ExchangeKind};

use crate::runtime::adapters::{Backend, BoxFuture, Queue, QueueError};

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
