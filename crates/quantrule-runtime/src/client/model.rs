//! The model client seam.
//!
//! The backtest worker reaches the model only through this module. The worker
//! asks for a content-addressed bundle reference and never receives a reading
//! series; the concrete transport is the parent's `HttpServiceClient`, and
//! `MemoryModel` is the fake a worker test uses without a socket.

use std::collections::BTreeMap;

use quantrule_api::{Digest, ProfileId};
use quantrule_service_api::ModelBundleManifest;

use crate::adapters::BoxFuture;
use crate::client::{
    ClientError, HttpServiceClient, Service, ServiceClient, ServiceRequest, upstream_message,
};

/// The model bundle the backtest worker consumes.
///
/// A successful response names every artifact with its byte count and SHA-256.
/// The worker materializes the referenced objects from the object store and
/// never recomputes the model.
pub trait ModelClient: Send + Sync {
    /// Compute or reuse one model bundle for a profile's dataset and config.
    fn bundle(
        &self,
        profile: ProfileId,
        data_version: Digest,
        config_id: String,
    ) -> BoxFuture<Result<ModelBundleManifest, ClientError>>;
}

/// The body of one internal model-bundle request.
#[derive(serde::Serialize)]
struct BundleRequest<'a> {
    data_version: &'a Digest,
    config_id: &'a str,
}

/// Build the internal model-bundle request the service expects.
fn bundle_request(
    profile: &ProfileId,
    data_version: &Digest,
    config_id: &str,
) -> Result<ServiceRequest, ClientError> {
    let request = BundleRequest {
        data_version,
        config_id,
    };
    let body = serde_json::to_vec(&request).map_err(|error| ClientError::InvalidResponse {
        service: Service::Model,
        message: error.to_string(),
    })?;
    Ok(ServiceRequest::post("/internal/v1/model/bundle", body)
        .with_header(quantrule_api::PROFILE_HEADER, profile.as_str()))
}

impl ModelClient for HttpServiceClient {
    fn bundle(
        &self,
        profile: ProfileId,
        data_version: Digest,
        config_id: String,
    ) -> BoxFuture<Result<ModelBundleManifest, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            let request = bundle_request(&profile, &data_version, &config_id)?;
            let response = <Self as ServiceClient>::send(&client, Service::Model, request).await?;
            match response.status {
                200 => serde_json::from_slice(&response.body).map_err(|error| {
                    ClientError::InvalidResponse {
                        service: Service::Model,
                        message: error.to_string(),
                    }
                }),
                status => Err(ClientError::Status {
                    service: Service::Model,
                    status,
                    message: upstream_message(&response.body),
                }),
            }
        })
    }
}

type Bundles =
    std::sync::Arc<std::sync::Mutex<BTreeMap<(String, String, String), ModelBundleManifest>>>;

/// An in-memory model service, for worker tests.
#[derive(Debug, Clone, Default)]
pub struct MemoryModel {
    bundles: Bundles,
    calls: std::sync::Arc<std::sync::Mutex<usize>>,
}

impl MemoryModel {
    /// An empty model service.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one bundle manifest for a profile.
    pub fn add_bundle(&self, profile: &ProfileId, manifest: ModelBundleManifest) {
        let key = (
            profile.as_str().to_owned(),
            manifest.data_version.to_string(),
            manifest.config_id.clone(),
        );
        self.bundles
            .lock()
            .expect("the bundle lock")
            .insert(key, manifest);
    }

    /// How many bundle requests this fake has answered.
    #[must_use]
    pub fn calls(&self) -> usize {
        *self.calls.lock().expect("the call lock")
    }
}

impl ModelClient for MemoryModel {
    fn bundle(
        &self,
        profile: ProfileId,
        data_version: Digest,
        config_id: String,
    ) -> BoxFuture<Result<ModelBundleManifest, ClientError>> {
        let bundles = std::sync::Arc::clone(&self.bundles);
        let calls = std::sync::Arc::clone(&self.calls);
        Box::pin(async move {
            *calls.lock().expect("the call lock") += 1;
            bundles
                .lock()
                .map_err(|_| ClientError::InvalidResponse {
                    service: Service::Model,
                    message: "the model lock is poisoned".to_owned(),
                })?
                .get(&(
                    profile.as_str().to_owned(),
                    data_version.to_string(),
                    config_id,
                ))
                .cloned()
                .ok_or(ClientError::Status {
                    service: Service::Model,
                    status: 404,
                    message: "the model bundle is unknown".to_owned(),
                })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::ArtifactRef;
    use quantrule_service_api::MODEL_BUNDLE_SCHEMA;

    fn manifest(data_version: &Digest) -> ModelBundleManifest {
        ModelBundleManifest {
            schema: MODEL_BUNDLE_SCHEMA.to_owned(),
            data_version: data_version.clone(),
            config_id: "cfg_1".to_owned(),
            config_hash: Digest::sha256(b"config"),
            model_version: "M1.0".to_owned(),
            ladder: vec!["1m".to_owned()],
            manifest_key: "model/bundle/manifest.json".to_owned(),
            artifacts: vec![ArtifactRef {
                name: "readings.csv".to_owned(),
                bytes: 1,
                sha256: Digest::sha256(b"x"),
            }],
        }
    }

    #[test]
    fn the_bundle_request_names_the_internal_route_and_both_references() {
        let profile = ProfileId::parse("default").expect("profile");
        let version = Digest::sha256(b"data");
        let request =
            bundle_request(&profile, &version, "cfg_1").expect("the bundle request builds");
        assert_eq!(request.method, crate::client::Method::Post);
        assert_eq!(request.path, "/internal/v1/model/bundle");
        assert_eq!(
            request
                .headers
                .get(quantrule_api::PROFILE_HEADER)
                .map(String::as_str),
            Some("default")
        );
        let body: serde_json::Value = serde_json::from_slice(&request.body).expect("body");
        assert_eq!(body["data_version"], version.to_string());
        assert_eq!(body["config_id"], "cfg_1");
    }

    #[tokio::test]
    async fn a_registered_bundle_is_returned_for_its_profile() {
        let model = MemoryModel::new();
        let profile = ProfileId::parse("default").expect("profile");
        let version = Digest::sha256(b"data");
        model.add_bundle(&profile, manifest(&version));
        let found = model
            .bundle(profile.clone(), version.clone(), "cfg_1".to_owned())
            .await
            .expect("bundle");
        assert_eq!(found, manifest(&version));
        let missing = model
            .bundle(profile, version, "cfg_2".to_owned())
            .await
            .expect_err("unknown config");
        assert_eq!(
            missing,
            ClientError::Status {
                service: Service::Model,
                status: 404,
                message: "the model bundle is unknown".to_owned(),
            }
        );
    }
}
