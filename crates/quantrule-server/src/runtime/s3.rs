//! The object store.
//!
//! Every artifact and candle series lives under the SHA-256 of its bytes. The
//! client is built once; the check is a real request, because a reachable socket
//! is not a working bucket.

use aws_sdk_s3::Client;
use aws_sdk_s3::config::{BehaviorVersion, Region};

/// Build a client for an S3-compatible endpoint.
///
/// Credentials come from the standard AWS environment, which the compose stack
/// sets. Path-style addressing is forced because the endpoint is a local server,
/// not `amazonaws.com`.
pub async fn client(endpoint: &str) -> Client {
    let shared = aws_config::defaults(BehaviorVersion::latest())
        .endpoint_url(endpoint)
        .region(Region::new("us-east-1"))
        .load()
        .await;
    let config = aws_sdk_s3::config::Builder::from(&shared)
        .force_path_style(true)
        .build();
    Client::from_conf(config)
}

/// Ask the store to list buckets.
///
/// # Errors
///
/// Returns the service message when the call fails.
pub async fn check(client: &Client) -> Result<(), String> {
    client
        .list_buckets()
        .send()
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}
