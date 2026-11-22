//! The computed-artifact client seam.
//!
//! The signal writer owns the object-store write and asks the data service to
//! record the catalog row. The request carries the content id, the kind and the
//! byte count; the service verifies the stored object before it publishes a row.
//! `MemoryArtifacts` is the fake a service test uses without a database.

use std::collections::BTreeMap;

use quantrule_api::{ArtifactKind, ArtifactRecord, ArtifactSummary, Digest, ProfileId};
use quantrule_service_api::{ArtifactRegistration, ArtifactRegistrationRequest};

use crate::adapters::BoxFuture;
use crate::client::{ClientError, HttpServiceClient, Service};

/// The data service's metadata-only artifact registration.
pub trait ArtifactClient: Send + Sync {
    /// Register one already-stored artifact.
    ///
    /// A duplicate returns the stored row with `created == false`; it is never
    /// an error. The data service fails closed when the object is missing or its
    /// hash and size disagree with the id.
    fn register(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        artifact_id: Digest,
        bytes: u64,
    ) -> BoxFuture<Result<ArtifactRegistration, ClientError>>;
}

impl ArtifactClient for HttpServiceClient {
    fn register(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        artifact_id: Digest,
        bytes: u64,
    ) -> BoxFuture<Result<ArtifactRegistration, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            let request = ArtifactRegistrationRequest { bytes };
            client
                .post_json_for(
                    Service::Data,
                    &profile,
                    format!("/internal/v1/artifacts/{}/{}", kind.code(), artifact_id),
                    &request,
                )
                .await
        })
    }
}

type Rows = BTreeMap<(String, String, String), ArtifactRecord>;

/// An in-memory artifact catalog for service tests.
#[derive(Debug, Clone, Default)]
pub struct MemoryArtifacts {
    rows: std::sync::Arc<std::sync::Mutex<Rows>>,
    calls: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl MemoryArtifacts {
    /// The stored rows of one kind for a profile, ordered by id.
    #[must_use]
    pub fn records(&self, profile: &ProfileId, kind: ArtifactKind) -> Vec<ArtifactRecord> {
        self.rows
            .lock()
            .expect("the artifact lock")
            .iter()
            .filter(|((owner, stored_kind, _), _)| {
                owner == profile.as_str() && stored_kind == kind.code()
            })
            .map(|(_, record)| record.clone())
            .collect()
    }

    /// How many registrations this fake has answered.
    #[must_use]
    pub fn calls(&self) -> u64 {
        self.calls.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl ArtifactClient for MemoryArtifacts {
    fn register(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        artifact_id: Digest,
        bytes: u64,
    ) -> BoxFuture<Result<ArtifactRegistration, ClientError>> {
        self.calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let rows = std::sync::Arc::clone(&self.rows);
        Box::pin(async move {
            let mut rows = rows.lock().map_err(|_| ClientError::InvalidResponse {
                service: Service::Data,
                message: "the artifact lock is poisoned".to_owned(),
            })?;
            let key = (
                profile.as_str().to_owned(),
                kind.code().to_owned(),
                artifact_id.to_string(),
            );
            if let Some(record) = rows.get(&key) {
                return Ok(ArtifactRegistration {
                    record: record.clone(),
                    created: false,
                });
            }
            let record = ArtifactRecord {
                summary: ArtifactSummary {
                    artifact_id: artifact_id.clone(),
                    kind,
                    bytes,
                    created_at: "2026-11-22T00:00:00Z".to_owned(),
                },
                object_key: format!("artifacts/{}", artifact_id.as_hex()),
                sha256: artifact_id,
            };
            rows.insert(key, record.clone());
            Ok(ArtifactRegistration {
                record,
                created: true,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::Path;
    use axum::http::StatusCode;
    use axum::routing::post;
    use axum::{Json, Router};

    fn profile() -> ProfileId {
        ProfileId::parse("default").expect("profile")
    }

    #[tokio::test]
    async fn registration_is_idempotent_and_reports_the_first_writer() {
        let artifacts = MemoryArtifacts::default();
        let id = Digest::sha256(b"signal");
        let first = artifacts
            .register(profile(), ArtifactKind::Signal, id.clone(), 6)
            .await
            .expect("registration");
        let second = artifacts
            .register(profile(), ArtifactKind::Signal, id.clone(), 6)
            .await
            .expect("duplicate");
        assert!(first.created);
        assert!(!second.created);
        assert_eq!(first.record, second.record);
        assert_eq!(artifacts.records(&profile(), ArtifactKind::Signal).len(), 1);
        assert_eq!(artifacts.calls(), 2);
    }

    #[tokio::test]
    async fn the_http_client_accepts_a_created_registration() {
        let id = Digest::sha256(b"signal");
        let record = ArtifactRecord {
            summary: ArtifactSummary {
                artifact_id: id.clone(),
                kind: ArtifactKind::Signal,
                bytes: 6,
                created_at: "2026-11-22T00:00:00Z".to_owned(),
            },
            object_key: format!("artifacts/{}", id.as_hex()),
            sha256: id.clone(),
        };
        let response_id = id.clone();
        let app = Router::new().route(
            "/internal/v1/artifacts/signal/{id}",
            post(move |Path(path): Path<String>| {
                assert_eq!(path, response_id.to_string());
                let record = record.clone();
                async move {
                    (
                        StatusCode::CREATED,
                        Json(ArtifactRegistration {
                            record,
                            created: true,
                        }),
                    )
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let address = listener.local_addr().expect("address");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("artifact server");
        });
        let client =
            HttpServiceClient::new(Service::Data, &format!("http://{address}")).expect("client");
        let registration = client
            .register(profile(), ArtifactKind::Signal, id, 6)
            .await
            .expect("registration");
        assert!(registration.created);
        assert_eq!(registration.record.summary.bytes, 6);
        handle.abort();
    }
}
