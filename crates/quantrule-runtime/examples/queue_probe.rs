//! Lab-only queue probe: publish one acknowledged message and read it back.
//!
//! This is not a product command and it never runs in a service. The HA lab
//! uses it to prove that a quorum-queue message is confirmed before a broker
//! loss and is still readable afterwards through the HAProxy `rabbitmq` alias.
//! The probe queue (`qr.lab.queue-probe` by default) is durable and empty after
//! a successful consume; no queue, user, or vhost is deleted.
//!
//! Usage:
//!
//! ```text
//! queue_probe publish <queue> <payload> <url-file> [ca-file]
//! queue_probe consume <queue> <payload> <url-file> [ca-file]
//! queue_probe drain <queue> "" <url-file> [ca-file]
//! ```
//!
//! The URL file is the mode-600 `amqps://` secret; the value is read from the
//! file and never printed. The optional CA file is the private bundle.

use std::error::Error;
use std::path::Path;

use lapin::options::{
    BasicAckOptions, BasicGetOptions, BasicPublishOptions, ConfirmSelectOptions,
    QueueDeclareOptions,
};
use lapin::tcp::OwnedTLSConfig;
use lapin::types::{AMQPValue, FieldTable};
use lapin::{Confirmation, Connection, DefaultConnectionBuilder};

const PROBE_QUEUE: &str = "qr.lab.queue-probe";
const PROBE_PREFIX: &str = "quantrule-lab-queue-probe";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args().skip(1);
    let action = arguments.next().ok_or(
        "usage: queue_probe publish|consume|drain <queue> <payload|empty> <url-file> [ca-file]",
    )?;
    let queue = arguments.next().unwrap_or_else(|| PROBE_QUEUE.to_owned());
    let payload = arguments.next().unwrap_or_default();
    let url_file = arguments
        .next()
        .ok_or("the queue URL file path is required")?;
    let ca = arguments.next();
    if arguments.next().is_some() {
        return Err(
            "usage: queue_probe publish|consume|drain <queue> <payload|empty> <url-file> [ca-file]"
                .into(),
        );
    }
    if matches!(action.as_str(), "publish" | "consume") && payload.is_empty() {
        return Err(format!("{action} needs a payload").into());
    }
    let url = std::fs::read_to_string(&url_file)
        .map_err(|error| format!("cannot read {url_file}: {error}"))?
        .trim()
        .to_owned();
    if !url.starts_with("amqps://") {
        return Err("the queue URL must use amqps://".into());
    }
    let connection = connect(&url, ca.as_deref()).await?;
    let channel = connection.create_channel().await?;
    let mut arguments_table = FieldTable::default();
    arguments_table.insert(
        "x-queue-type".into(),
        AMQPValue::LongString("quorum".into()),
    );
    channel
        .queue_declare(
            queue.as_str().into(),
            QueueDeclareOptions {
                durable: true,
                ..QueueDeclareOptions::default()
            },
            arguments_table,
        )
        .await?;
    match action.as_str() {
        "publish" => {
            channel
                .confirm_select(ConfirmSelectOptions::default())
                .await?;
            let confirmation = channel
                .basic_publish(
                    "".into(),
                    queue.as_str().into(),
                    BasicPublishOptions::default(),
                    payload.as_bytes(),
                    lapin::BasicProperties::default().with_delivery_mode(2),
                )
                .await?;
            match confirmation.await? {
                Confirmation::Ack(_) | Confirmation::NotRequested => {}
                Confirmation::Nack(_) => return Err("the broker rejected the probe message".into()),
            }
            println!(
                "queue_probe: published {queue} ({} bytes, confirmed)",
                payload.len()
            );
        }
        "consume" => {
            let delivery = channel
                .basic_get(queue.as_str().into(), BasicGetOptions { no_ack: false })
                .await?;
            let Some(delivery) = delivery else {
                return Err(format!("the probe message is missing from {queue}").into());
            };
            if delivery.data != payload.as_bytes() {
                return Err(format!("the probe message in {queue} has different bytes").into());
            }
            delivery.ack(BasicAckOptions::default()).await?;
            println!(
                "queue_probe: consumed {queue} ({} bytes, acked)",
                payload.len()
            );
        }
        "drain" => {
            let mut total = 0_u64;
            loop {
                let delivery = channel
                    .basic_get(queue.as_str().into(), BasicGetOptions { no_ack: false })
                    .await?;
                let Some(delivery) = delivery else {
                    break;
                };
                let text = String::from_utf8_lossy(&delivery.data);
                if !text.starts_with(PROBE_PREFIX) {
                    return Err(format!(
                        "{queue} holds a message that is not a lab probe; refusing to ack it"
                    )
                    .into());
                }
                delivery.ack(BasicAckOptions::default()).await?;
                total += 1;
            }
            println!("queue_probe: drained {queue} ({total} probe message(s))");
        }
        other => {
            return Err(
                format!("unknown action {other}; expected publish, consume, or drain").into(),
            );
        }
    }
    connection.close(200, "probe done".into()).await?;
    Ok(())
}

async fn connect(url: &str, ca: Option<&str>) -> Result<Connection, Box<dyn Error>> {
    let builder = DefaultConnectionBuilder::new()?.with_uri_str(url.to_owned());
    let builder = match ca {
        Some(path) => {
            let pem = std::fs::read_to_string(Path::new(path))
                .map_err(|error| format!("cannot read {path}: {error}"))?;
            builder.with_tls_config(OwnedTLSConfig {
                identity: None,
                cert_chain: Some(pem),
            })
        }
        None => builder,
    };
    Ok(builder.connect().await?)
}
