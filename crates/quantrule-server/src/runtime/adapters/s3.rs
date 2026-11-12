//! The S3-compatible object store adapter.
//!
//! Content addressing is the caller's rule: the key is the digest exposed by
//! `quantrule-api`. This adapter only moves bytes.

use aws_sdk_s3::Client;
use aws_sdk_s3::config::{BehaviorVersion, Region};
use aws_sdk_s3::primitives::ByteStream;

use crate::runtime::adapters::{Backend, BoxFuture, ObjectStore, StoreError};

/// An S3-compatible bucket.
#[derive(Debug, Clone)]
pub struct S3Store {
    client: Client,
    bucket: String,
}

impl S3Store {
    /// Build a client for a local S3-compatible endpoint.
    ///
    /// Credentials come from the standard AWS environment, which the compose
    /// stack sets. Path-style addressing is forced because the endpoint is not
    /// `amazonaws.com`.
    pub async fn new(endpoint: &str, bucket: impl Into<String>) -> Self {
        let shared = aws_config::defaults(BehaviorVersion::latest())
            .endpoint_url(endpoint)
            .region(Region::new("us-east-1"))
            .load()
            .await;
        let config = aws_sdk_s3::config::Builder::from(&shared)
            .force_path_style(true)
            .build();
        Self {
            client: Client::from_conf(config),
            bucket: bucket.into(),
        }
    }

    /// Whether the bucket answers.
    ///
    /// # Errors
    ///
    /// Returns the service message when the call fails.
    pub async fn check(&self) -> Result<(), StoreError> {
        self.client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .map(|_| ())
            .map_err(|error| StoreError(error.to_string()))
    }
}

impl ObjectStore for S3Store {
    fn backend(&self) -> Backend {
        Backend::S3
    }

    fn put(&self, key: &str, bytes: Vec<u8>) -> BoxFuture<Result<(), StoreError>> {
        let client = self.client.clone();
        let bucket = self.bucket.clone();
        let key = key.to_owned();
        Box::pin(async move {
            client
                .put_object()
                .bucket(bucket)
                .key(key)
                .body(ByteStream::from(bytes))
                .send()
                .await
                .map(|_| ())
                .map_err(|error| StoreError(error.to_string()))
        })
    }

    fn get(&self, key: &str) -> BoxFuture<Result<Vec<u8>, StoreError>> {
        let client = self.client.clone();
        let bucket = self.bucket.clone();
        let key = key.to_owned();
        Box::pin(async move {
            let response = client
                .get_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|error| StoreError(error.to_string()))?;
            let body = response
                .body
                .collect()
                .await
                .map_err(|error| StoreError(error.to_string()))?;
            Ok(body.into_bytes().to_vec())
        })
    }

    fn exists(&self, key: &str) -> BoxFuture<Result<bool, StoreError>> {
        let client = self.client.clone();
        let bucket = self.bucket.clone();
        let key = key.to_owned();
        Box::pin(async move {
            match client.head_object().bucket(bucket).key(key).send().await {
                Ok(_) => Ok(true),
                Err(error) => {
                    let missing = error.as_service_error().is_some_and(
                        aws_sdk_s3::operation::head_object::HeadObjectError::is_not_found,
                    );
                    if missing {
                        Ok(false)
                    } else {
                        Err(StoreError(error.to_string()))
                    }
                }
            }
        })
    }
}
