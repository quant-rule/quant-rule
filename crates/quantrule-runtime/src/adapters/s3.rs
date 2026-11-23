//! The S3-compatible object store adapter.
//!
//! Content addressing is the caller's rule: the key is the digest exposed by
//! `quantrule-api`. This adapter only moves bytes.

use aws_sdk_s3::Client;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;

use crate::adapters::{Backend, BoxFuture, ObjectStore, StoreError};

/// The log filter directives for these drivers.
pub const LOG_FILTER_DIRECTIVE: &str =
    "aws_sdk_s3=warn,aws_smithy_runtime=warn,aws_smithy_runtime_api=warn,aws_smithy_types=warn";

/// An S3-compatible bucket.
#[derive(Clone)]
pub struct S3Store {
    client: Client,
    bucket: String,
}

impl std::fmt::Debug for S3Store {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("S3Store")
            .field("bucket", &self.bucket)
            .finish_non_exhaustive()
    }
}

impl S3Store {
    /// Build a client for a local S3-compatible endpoint.
    ///
    /// The two credentials are the values the runtime resolved from their
    /// `_FILE` variables. The AWS default chain is deliberately not consulted:
    /// it would be a second, unaudited secret reader with its own environment
    /// variables, config files and instance metadata. Path-style addressing is
    /// forced because the endpoint is not `amazonaws.com`.
    pub(crate) fn new(
        endpoint: &str,
        bucket: impl Into<String>,
        access_key: impl Into<String>,
        secret_key: impl Into<String>,
    ) -> Self {
        let credentials =
            Credentials::new(access_key, secret_key, None, None, "quantrule-secret-file");
        let config = aws_sdk_s3::config::Builder::new()
            .behavior_version(BehaviorVersion::latest())
            .endpoint_url(endpoint)
            .region(Region::new("us-east-1"))
            .credentials_provider(credentials)
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

    fn delete(&self, key: &str) -> BoxFuture<Result<(), StoreError>> {
        let client = self.client.clone();
        let bucket = self.bucket.clone();
        let key = key.to_owned();
        Box::pin(async move {
            // S3 delete is idempotent: deleting an absent key succeeds.
            client
                .delete_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map(|_| ())
                .map_err(|error| StoreError(error.to_string()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::S3Store;

    #[test]
    fn explicit_credentials_and_the_bucket_are_accepted() {
        let store = S3Store::new(
            "http://minio:9000",
            "quantrule",
            "quantrule",
            "sentinel-secret",
        );
        assert_eq!(store.bucket, "quantrule");
        assert!(
            !format!("{store:?}").contains("sentinel-secret"),
            "the secret leaked through Debug"
        );
    }

    #[test]
    fn the_aws_default_chain_is_not_a_credential_source() {
        // The source is the contract: a second reader would make the AWS
        // environment, shared config and instance metadata an unaudited path.
        let source = include_str!("s3.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(!production.contains("aws_config::defaults("));
        assert!(!production.contains("AWS_ACCESS_KEY_ID"));
        assert!(!production.contains("AWS_SECRET_ACCESS_KEY"));
    }
}
