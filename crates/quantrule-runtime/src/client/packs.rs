//! The pack registry client seam.
//!
//! The strategy service reads a frozen pack's metadata and canonical bytes
//! through the data service. It never opens the catalog database and never
//! receives a client path. The concrete transport is the parent's
//! `HttpServiceClient`; `MemoryPacks` is the fake a service test uses without a
//! socket.

use std::collections::BTreeMap;

use quantrule_api::{Digest, PackRecord, ProfileId};

use crate::adapters::BoxFuture;
use crate::client::{ClientError, HttpServiceClient, Service};

/// The data service's pack reads.
pub trait PackClient: Send + Sync {
    /// Read one pack record, or `None` when the profile holds no such pack.
    fn pack(
        &self,
        profile: ProfileId,
        pack_id: Digest,
    ) -> BoxFuture<Result<Option<PackRecord>, ClientError>>;

    /// Read one pack's canonical bytes, or `None` when the pack is unknown.
    ///
    /// The data service re-reads the object and verifies its hash before
    /// answering; a row without matching bytes is a service error, not an empty
    /// result.
    fn pack_bytes(
        &self,
        profile: ProfileId,
        pack_id: Digest,
    ) -> BoxFuture<Result<Option<Vec<u8>>, ClientError>>;
}

impl PackClient for HttpServiceClient {
    fn pack(
        &self,
        profile: ProfileId,
        pack_id: Digest,
    ) -> BoxFuture<Result<Option<PackRecord>, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            client
                .get_json(
                    Service::Data,
                    Some(&profile),
                    format!("/internal/v1/packs/{pack_id}"),
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
}

type Rows = BTreeMap<(String, String), (PackRecord, Vec<u8>)>;

/// An in-memory pack registry for service tests.
#[derive(Debug, Clone, Default)]
pub struct MemoryPacks {
    rows: std::sync::Arc<std::sync::Mutex<Rows>>,
    record_calls: std::sync::Arc<std::sync::atomic::AtomicU64>,
    byte_calls: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl MemoryPacks {
    /// Register one record and its canonical bytes for a profile.
    pub fn add_pack(&self, profile: &ProfileId, record: PackRecord, bytes: Vec<u8>) {
        self.rows.lock().expect("the pack lock").insert(
            (
                profile.as_str().to_owned(),
                record.summary.pack_id.to_string(),
            ),
            (record, bytes),
        );
    }

    /// Remove the bytes while keeping the record, to model a missing object.
    pub fn remove_bytes(&self, profile: &ProfileId, pack_id: &Digest) -> Option<Vec<u8>> {
        self.rows
            .lock()
            .expect("the pack lock")
            .get_mut(&(profile.as_str().to_owned(), pack_id.to_string()))
            .map(|(_, bytes)| std::mem::take(bytes))
    }

    /// How many record reads this fake has answered.
    #[must_use]
    pub fn record_calls(&self) -> u64 {
        self.record_calls.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// How many byte reads this fake has answered.
    #[must_use]
    pub fn byte_calls(&self) -> u64 {
        self.byte_calls.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl PackClient for MemoryPacks {
    fn pack(
        &self,
        profile: ProfileId,
        pack_id: Digest,
    ) -> BoxFuture<Result<Option<PackRecord>, ClientError>> {
        self.record_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let rows = std::sync::Arc::clone(&self.rows);
        Box::pin(async move {
            Ok(rows
                .lock()
                .map_err(|_| ClientError::InvalidResponse {
                    service: Service::Data,
                    message: "the pack lock is poisoned".to_owned(),
                })?
                .get(&(profile.as_str().to_owned(), pack_id.to_string()))
                .map(|(record, _)| record.clone()))
        })
    }

    fn pack_bytes(
        &self,
        profile: ProfileId,
        pack_id: Digest,
    ) -> BoxFuture<Result<Option<Vec<u8>>, ClientError>> {
        self.byte_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let rows = std::sync::Arc::clone(&self.rows);
        Box::pin(async move {
            Ok(rows
                .lock()
                .map_err(|_| ClientError::InvalidResponse {
                    service: Service::Data,
                    message: "the pack lock is poisoned".to_owned(),
                })?
                .get(&(profile.as_str().to_owned(), pack_id.to_string()))
                .map(|(_, bytes)| bytes.clone()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::{PackKind, PackState, PackSummary};

    fn profile() -> ProfileId {
        ProfileId::parse("default").expect("profile")
    }

    fn record(id: &Digest) -> PackRecord {
        PackRecord {
            summary: PackSummary {
                pack_id: id.clone(),
                kind: PackKind::Strategy,
                name: "compression-fade-5m".to_owned(),
                version: "1.0.0".to_owned(),
                owner: "operator".to_owned(),
                state: PackState::Frozen,
                bytes: 4,
                created_at: "2026-11-22T00:00:00Z".to_owned(),
                validated_at: None,
                frozen_at: Some("2026-11-22T00:01:00Z".to_owned()),
                retired_at: None,
            },
            object_key: format!("packs/{}", id.as_hex()),
            sha256: id.clone(),
            validations: Vec::new(),
            events: Vec::new(),
        }
    }

    #[tokio::test]
    async fn a_registered_pack_returns_its_record_and_bytes() {
        let packs = MemoryPacks::default();
        let id = Digest::sha256(b"pack");
        packs.add_pack(&profile(), record(&id), b"pack".to_vec());
        assert_eq!(
            packs.pack(profile(), id.clone()).await.expect("record"),
            Some(record(&id))
        );
        assert_eq!(
            packs.pack_bytes(profile(), id).await.expect("bytes"),
            Some(b"pack".to_vec())
        );
        assert_eq!(packs.record_calls(), 1);
        assert_eq!(packs.byte_calls(), 1);
    }

    #[tokio::test]
    async fn a_missing_pack_and_missing_bytes_are_absent() {
        let packs = MemoryPacks::default();
        let id = Digest::sha256(b"missing");
        assert_eq!(
            packs.pack(profile(), id.clone()).await.expect("record"),
            None
        );
        assert_eq!(packs.pack_bytes(profile(), id).await.expect("bytes"), None);

        let present = Digest::sha256(b"present");
        packs.add_pack(&profile(), record(&present), b"pack".to_vec());
        packs.remove_bytes(&profile(), &present);
        assert_eq!(
            packs.pack_bytes(profile(), present).await.expect("bytes"),
            Some(Vec::new())
        );
    }
}
