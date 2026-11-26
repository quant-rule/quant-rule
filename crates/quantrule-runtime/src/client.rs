//! The internal service client contract.
//!
//! A service reaches another only through this seam: a named service, a path on
//! the `quantrule.internal/v1` surface, and an opaque body. The contract is
//! bytes-only on purpose. Typed payloads belong to the service that owns the
//! endpoint, and a concrete HTTP transport lands with the first proxied route;
//! P1 freezes the seam so the gateway and the workers share one vocabulary.
//!
//! Timeouts, retries, and the 503 mapping are transport policy, not contract.
//! They are applied by the concrete client, never by a caller.

use quantrule_api::{
    DatasetReferenceCount, DatasetReferenceKind, DatasetReferenceRequest, Digest, ProfileId,
    ReleaseDatasetReferenceRequest,
};
use quantrule_service_api::{ConfigBundle, DatasetObject};
use std::collections::BTreeMap;

use crate::adapters::BoxFuture;

mod artifacts;
mod counts;
mod evidence;
mod http;
mod memory;
mod model;
mod packs;
mod stage;

pub use artifacts::{ArtifactClient, MemoryArtifacts};
pub use counts::{COUNT_PAGE, MemoryCounts, RunCounter, SimulationCounter};
pub use evidence::{EvidenceClient, MemoryEvidence};
pub(crate) use http::upstream_message;
pub use http::{HttpServiceClient, REQUEST_TIMEOUT};
pub use memory::MemoryClient;
pub use model::{MemoryModel, ModelClient};
pub use packs::{MemoryPacks, PackClient};
pub use stage::{MemoryStageClient, StageClient};

/// A service reachable on the internal network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Service {
    /// Ingestion, the catalog, and the dataset ladder.
    Data,
    /// Model compute and model bundles.
    Model,
    /// Run records, finalization, and research memory.
    Evidence,
    /// Chart-first research.
    Simulation,
    /// The backtest worker's health endpoint; it exposes no other route.
    Backtest,
    /// The stateless signal-mode strategy compute.
    Strategy,
}

impl Service {
    /// The service name used in a health report and in an error.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Data => "data",
            Self::Model => "model",
            Self::Evidence => "evidence",
            Self::Simulation => "simulation",
            Self::Backtest => "backtest",
            Self::Strategy => "strategy",
        }
    }

    /// The environment variable that names its base URL.
    #[must_use]
    pub fn url_variable(self) -> &'static str {
        match self {
            Self::Data => "QR_DATA_URL",
            Self::Model => "QR_MODEL_URL",
            Self::Evidence => "QR_EVIDENCE_URL",
            Self::Simulation => "QR_SIMULATION_URL",
            Self::Backtest => "QR_BACKTEST_HEALTH_URL",
            Self::Strategy => "QR_STRATEGY_URL",
        }
    }
}

/// The HTTP method of an internal call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// A read.
    Get,
    /// A write or a submission.
    Post,
}

impl Method {
    /// The wire token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

/// One request on the internal surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRequest {
    /// The method.
    pub method: Method,
    /// The path, beginning with `/`.
    pub path: String,
    /// Request headers the transport may forward. The profile header travels
    /// here; a service never invents one.
    pub headers: BTreeMap<String, String>,
    /// The body; empty for a read.
    pub body: Vec<u8>,
}

impl ServiceRequest {
    /// A read.
    #[must_use]
    pub fn get(path: impl Into<String>) -> Self {
        Self {
            method: Method::Get,
            path: path.into(),
            headers: BTreeMap::new(),
            body: Vec::new(),
        }
    }

    /// Add one request header.
    #[must_use]
    pub fn with_header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.insert(name.to_owned(), value.into());
        self
    }

    /// A write with a body.
    #[must_use]
    pub fn post(path: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        Self {
            method: Method::Post,
            path: path.into(),
            headers: BTreeMap::new(),
            body: body.into(),
        }
    }
}

/// One internal response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceResponse {
    /// The HTTP status.
    pub status: u16,
    /// The content type, when the service set one.
    pub content_type: Option<String>,
    /// The entity tag, when the service set one.
    pub etag: Option<String>,
    /// The body bytes; a reference or a small envelope, never a candle series.
    pub body: Vec<u8>,
}

impl ServiceResponse {
    /// A successful JSON response.
    #[must_use]
    pub fn json(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            content_type: Some("application/json".to_owned()),
            etag: None,
            body: body.into(),
        }
    }
}

/// Why an internal call did not produce a response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    /// The service could not be reached.
    Unreachable {
        /// Which service.
        service: Service,
        /// The transport message.
        message: String,
    },
    /// The call exceeded its deadline.
    Timeout {
        /// Which service.
        service: Service,
    },
    /// The service has no URL configured.
    Unconfigured {
        /// Which service.
        service: Service,
    },
    /// The service answered with a status the caller must handle.
    Status {
        /// Which service.
        service: Service,
        /// The HTTP status.
        status: u16,
        /// The upstream message, when it carried one.
        message: String,
    },
    /// The service answered with bytes the contract cannot decode.
    InvalidResponse {
        /// Which service.
        service: Service,
        /// What was wrong.
        message: String,
    },
    /// The configured base URL is not an absolute HTTP URL.
    InvalidBase {
        /// Which service.
        service: Service,
        /// What was wrong.
        message: String,
    },
    /// The call stopped because the process began draining.
    Interrupted {
        /// Which service.
        service: Service,
    },
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable { service, message } => {
                write!(formatter, "{} is unreachable: {message}", service.name())
            }
            Self::Timeout { service } => {
                write!(formatter, "{} did not answer in time", service.name())
            }
            Self::Unconfigured { service } => write!(
                formatter,
                "{} is not configured; set {}",
                service.name(),
                service.url_variable()
            ),
            Self::Status {
                service,
                status,
                message,
            } => write!(formatter, "{} answered {status}: {message}", service.name()),
            Self::InvalidResponse { service, message } => {
                write!(
                    formatter,
                    "{} answered an invalid body: {message}",
                    service.name()
                )
            }
            Self::InvalidBase { service, message } => {
                write!(
                    formatter,
                    "the {} base URL is invalid: {message}",
                    service.name()
                )
            }
            Self::Interrupted { service } => write!(
                formatter,
                "the {} call stopped because the process is draining",
                service.name()
            ),
        }
    }
}

impl std::error::Error for ClientError {}

/// The one seam through which a service calls another.
pub trait ServiceClient: Send + Sync {
    /// Send one request to one service.
    fn send(
        &self,
        service: Service,
        request: ServiceRequest,
    ) -> BoxFuture<Result<ServiceResponse, ClientError>>;
}

/// The data service's internal reads, as the gateway and the worker use them.
///
/// The trait carries references only: a config bundle, a dataset object, and a
/// count. A fake implements it without a socket, so a route or a worker hop is
/// tested against the contract rather than a live service.
pub trait DataClient: Send + Sync {
    /// Read one config bundle, or `None` when the profile holds no such config.
    fn config_bundle(
        &self,
        profile: ProfileId,
        config_id: String,
    ) -> BoxFuture<Result<Option<ConfigBundle>, ClientError>>;

    /// Read one dataset object, or `None` when the profile holds no such dataset.
    fn dataset_object(
        &self,
        profile: ProfileId,
        data_version: String,
    ) -> BoxFuture<Result<Option<DatasetObject>, ClientError>>;

    /// Read one pack's canonical bytes, or `None` when the pack is unknown.
    fn pack_bytes(
        &self,
        profile: ProfileId,
        pack_id: Digest,
    ) -> BoxFuture<Result<Option<Vec<u8>>, ClientError>>;

    /// Read one signal artifact's canonical bytes, or `None` when it is unknown.
    fn signal_bytes(
        &self,
        profile: ProfileId,
        artifact_id: Digest,
    ) -> BoxFuture<Result<Option<Vec<u8>>, ClientError>>;

    /// How many datasets a profile holds.
    fn dataset_count(&self, profile: ProfileId) -> BoxFuture<Result<u64, ClientError>>;

    /// Register one consumer reference to a dataset.
    fn register_dataset_reference(
        &self,
        profile: ProfileId,
        data_version: quantrule_api::Digest,
        reference_id: String,
        kind: DatasetReferenceKind,
    ) -> BoxFuture<Result<DatasetReferenceCount, ClientError>>;

    /// Release one consumer reference to a dataset.
    fn release_dataset_reference(
        &self,
        profile: ProfileId,
        data_version: quantrule_api::Digest,
        reference_id: String,
    ) -> BoxFuture<Result<DatasetReferenceCount, ClientError>>;
}

impl DataClient for HttpServiceClient {
    fn config_bundle(
        &self,
        profile: ProfileId,
        config_id: String,
    ) -> BoxFuture<Result<Option<ConfigBundle>, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            client
                .get_json(
                    Service::Data,
                    Some(&profile),
                    format!("/internal/v1/configs/{config_id}"),
                )
                .await
        })
    }

    fn dataset_object(
        &self,
        profile: ProfileId,
        data_version: String,
    ) -> BoxFuture<Result<Option<DatasetObject>, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            client
                .get_json(
                    Service::Data,
                    Some(&profile),
                    format!("/internal/v1/datasets/{data_version}"),
                )
                .await
        })
    }

    fn pack_bytes(
        &self,
        profile: ProfileId,
        pack_id: Digest,
    ) -> BoxFuture<Result<Option<Vec<u8>>, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            client
                .get_bytes(
                    Service::Data,
                    Some(&profile),
                    format!("/internal/v1/packs/{pack_id}/bytes"),
                )
                .await
        })
    }

    fn signal_bytes(
        &self,
        profile: ProfileId,
        artifact_id: Digest,
    ) -> BoxFuture<Result<Option<Vec<u8>>, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            client
                .get_bytes(
                    Service::Data,
                    Some(&profile),
                    format!("/internal/v1/artifacts/signal/{artifact_id}/bytes"),
                )
                .await
        })
    }

    fn dataset_count(&self, profile: ProfileId) -> BoxFuture<Result<u64, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            client
                .get_json(
                    Service::Data,
                    Some(&profile),
                    "/internal/v1/profile/datasets/count".to_owned(),
                )
                .await?
                .ok_or(ClientError::InvalidResponse {
                    service: Service::Data,
                    message: "the dataset count is missing".to_owned(),
                })
        })
    }

    fn register_dataset_reference(
        &self,
        profile: ProfileId,
        data_version: quantrule_api::Digest,
        reference_id: String,
        kind: DatasetReferenceKind,
    ) -> BoxFuture<Result<DatasetReferenceCount, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            let request = DatasetReferenceRequest { reference_id, kind };
            client
                .post_json_for_retryable(
                    Service::Data,
                    &profile,
                    format!("/internal/v1/datasets/{data_version}/references"),
                    &request,
                )
                .await
        })
    }

    fn release_dataset_reference(
        &self,
        profile: ProfileId,
        data_version: quantrule_api::Digest,
        reference_id: String,
    ) -> BoxFuture<Result<DatasetReferenceCount, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            let request = ReleaseDatasetReferenceRequest { reference_id };
            client
                .post_json_for_retryable(
                    Service::Data,
                    &profile,
                    format!("/internal/v1/datasets/{data_version}/references/release"),
                    &request,
                )
                .await
        })
    }
}

type DatasetReferenceMap = BTreeMap<(String, String), std::collections::BTreeSet<String>>;

type ContentBytes = std::sync::Arc<std::sync::Mutex<BTreeMap<(String, String), Vec<u8>>>>;

/// An in-memory data client, for route and worker tests.
#[derive(Debug, Clone, Default)]
pub struct MemoryDataClient {
    configs: std::sync::Arc<std::sync::Mutex<BTreeMap<(String, String), ConfigBundle>>>,
    datasets: std::sync::Arc<std::sync::Mutex<BTreeMap<(String, String), DatasetObject>>>,
    packs: ContentBytes,
    signals: ContentBytes,
    references: std::sync::Arc<std::sync::Mutex<DatasetReferenceMap>>,
}

impl MemoryDataClient {
    /// Add one config for a profile.
    pub fn add_config(&self, profile: &ProfileId, bundle: ConfigBundle) {
        self.configs.lock().expect("the config map").insert(
            (profile.as_str().to_owned(), bundle.config_id.clone()),
            bundle,
        );
    }

    /// Add one dataset for a profile.
    pub fn add_dataset(&self, profile: &ProfileId, object: DatasetObject) {
        self.datasets.lock().expect("the dataset map").insert(
            (profile.as_str().to_owned(), object.data_version.to_string()),
            object,
        );
    }

    /// Add one pack's canonical bytes for a profile.
    pub fn add_pack_bytes(&self, profile: &ProfileId, pack_id: &Digest, bytes: Vec<u8>) {
        self.packs
            .lock()
            .expect("the pack map")
            .insert((profile.as_str().to_owned(), pack_id.to_string()), bytes);
    }

    /// Add one signal artifact's canonical bytes for a profile.
    pub fn add_signal_bytes(&self, profile: &ProfileId, artifact_id: &Digest, bytes: Vec<u8>) {
        self.signals.lock().expect("the signal map").insert(
            (profile.as_str().to_owned(), artifact_id.to_string()),
            bytes,
        );
    }

    /// The reference ids registered for one dataset, in identity order.
    #[must_use]
    pub fn reference_ids(&self, profile: &ProfileId, data_version: &str) -> Vec<String> {
        self.references
            .lock()
            .expect("the reference map")
            .get(&(profile.as_str().to_owned(), data_version.to_owned()))
            .map(|ids| ids.iter().cloned().collect())
            .unwrap_or_default()
    }
}

impl DataClient for MemoryDataClient {
    fn config_bundle(
        &self,
        profile: ProfileId,
        config_id: String,
    ) -> BoxFuture<Result<Option<ConfigBundle>, ClientError>> {
        let configs = std::sync::Arc::clone(&self.configs);
        Box::pin(async move {
            Ok(configs
                .lock()
                .expect("the config map")
                .get(&(profile.as_str().to_owned(), config_id))
                .cloned())
        })
    }

    fn dataset_object(
        &self,
        profile: ProfileId,
        data_version: String,
    ) -> BoxFuture<Result<Option<DatasetObject>, ClientError>> {
        let datasets = std::sync::Arc::clone(&self.datasets);
        Box::pin(async move {
            Ok(datasets
                .lock()
                .expect("the dataset map")
                .get(&(profile.as_str().to_owned(), data_version))
                .cloned())
        })
    }

    fn pack_bytes(
        &self,
        profile: ProfileId,
        pack_id: Digest,
    ) -> BoxFuture<Result<Option<Vec<u8>>, ClientError>> {
        let packs = std::sync::Arc::clone(&self.packs);
        Box::pin(async move {
            Ok(packs
                .lock()
                .expect("the pack map")
                .get(&(profile.as_str().to_owned(), pack_id.to_string()))
                .cloned())
        })
    }

    fn signal_bytes(
        &self,
        profile: ProfileId,
        artifact_id: Digest,
    ) -> BoxFuture<Result<Option<Vec<u8>>, ClientError>> {
        let signals = std::sync::Arc::clone(&self.signals);
        Box::pin(async move {
            Ok(signals
                .lock()
                .expect("the signal map")
                .get(&(profile.as_str().to_owned(), artifact_id.to_string()))
                .cloned())
        })
    }

    fn dataset_count(&self, profile: ProfileId) -> BoxFuture<Result<u64, ClientError>> {
        let datasets = std::sync::Arc::clone(&self.datasets);
        Box::pin(async move {
            Ok(datasets
                .lock()
                .expect("the dataset map")
                .keys()
                .filter(|(owner, _)| owner.as_str() == profile.as_str())
                .count() as u64)
        })
    }

    fn register_dataset_reference(
        &self,
        profile: ProfileId,
        data_version: quantrule_api::Digest,
        reference_id: String,
        _kind: DatasetReferenceKind,
    ) -> BoxFuture<Result<DatasetReferenceCount, ClientError>> {
        let references = std::sync::Arc::clone(&self.references);
        Box::pin(async move {
            let mut map = references.lock().expect("the reference map");
            let ids = map
                .entry((profile.as_str().to_owned(), data_version.to_string()))
                .or_default();
            ids.insert(reference_id);
            Ok(DatasetReferenceCount {
                references: ids.len() as u64,
            })
        })
    }

    fn release_dataset_reference(
        &self,
        profile: ProfileId,
        data_version: quantrule_api::Digest,
        reference_id: String,
    ) -> BoxFuture<Result<DatasetReferenceCount, ClientError>> {
        let references = std::sync::Arc::clone(&self.references);
        Box::pin(async move {
            let mut map = references.lock().expect("the reference map");
            let key = (profile.as_str().to_owned(), data_version.to_string());
            let count = if let Some(ids) = map.get_mut(&key) {
                ids.remove(&reference_id);
                ids.len() as u64
            } else {
                0
            };
            Ok(DatasetReferenceCount { references: count })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_request_reaches_the_named_service() {
        let client = MemoryClient::new();
        client.respond(
            Service::Data,
            "/internal/v1/datasets/sha256:x",
            ServiceResponse::json(200, b"{}".to_vec()),
        );
        let response = client
            .send(
                Service::Data,
                ServiceRequest::get("/internal/v1/datasets/sha256:x"),
            )
            .await
            .expect("response");
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type.as_deref(), Some("application/json"));
        assert_eq!(client.calls().len(), 1);
        assert_eq!(client.calls()[0].0, Service::Data);
        assert_eq!(client.calls()[0].1.method, Method::Get);
    }

    #[tokio::test]
    async fn a_path_on_another_service_is_not_found() {
        let client = MemoryClient::new();
        client.respond(
            Service::Evidence,
            "/runs",
            ServiceResponse::json(200, b"[]".to_vec()),
        );
        let error = client
            .send(Service::Data, ServiceRequest::get("/runs"))
            .await
            .expect_err("no data route");
        assert_eq!(
            error,
            ClientError::Unconfigured {
                service: Service::Data
            }
        );
    }

    #[test]
    fn every_service_has_one_name_and_one_variable() {
        let services = [
            Service::Data,
            Service::Model,
            Service::Evidence,
            Service::Simulation,
            Service::Backtest,
            Service::Strategy,
        ];
        for (index, service) in services.iter().enumerate() {
            assert!(!service.name().is_empty());
            assert!(service.url_variable().starts_with("QR_"));
            assert!(service.url_variable().ends_with("_URL"));
            assert!(
                !services[..index]
                    .iter()
                    .any(|other| other.name() == service.name())
            );
        }
    }

    #[test]
    fn a_missing_url_names_the_service_and_the_variable() {
        let message = ClientError::Unconfigured {
            service: Service::Model,
        }
        .to_string();
        assert!(message.contains("model"), "{message}");
        assert!(message.contains("QR_MODEL_URL"), "{message}");
    }
}
