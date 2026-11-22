//! The in-memory pack registry.
//!
//! It carries the same rules as the Postgres store: one content-addressed row
//! per profile, one label per profile, and guarded lifecycle transitions. It
//! stores metadata only; the canonical bytes are the object store's authority.
//! It is the fake the fast gate uses, so every production rule a route test
//! relies on must hold here too.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use quantrule_api::{
    Digest, PackLifecycleEvent, PackRecord, PackState, PackSummary, PackValidationRecord, ProfileId,
};
use quantrule_runtime::adapters::BoxFuture;

use super::store::{NewPack, PackFilter, PackRegistry, PackRegistryError, RegisterOutcome};

const VALIDATOR: &str = "pack.schema/v1";

#[derive(Debug, Default)]
struct MemoryState {
    rows: BTreeMap<(String, String), PackRecord>,
}

impl MemoryState {
    fn label_owner(&self, profile: &str, name: &str, version: &str) -> Option<&str> {
        self.rows
            .iter()
            .find(|((owner, _), record)| {
                owner == profile && record.summary.name == name && record.summary.version == version
            })
            .map(|((_, pack_id), _)| pack_id.as_str())
    }

    fn transition(record: &PackRecord, next: PackState, action: &str, at: &str) -> PackRecord {
        let mut record = record.clone();
        let from = record.summary.state;
        record.summary.state = next;
        record.events.push(PackLifecycleEvent {
            from: Some(from),
            to: next,
            action: action.to_owned(),
            reason: None,
            created_at: at.to_owned(),
        });
        record
    }
}

/// An in-memory pack registry.
#[derive(Debug, Clone, Default)]
pub struct MemoryPacks {
    inner: Arc<Mutex<MemoryState>>,
}

fn summary(pack: &NewPack) -> PackSummary {
    PackSummary {
        pack_id: pack.pack_id.clone(),
        kind: pack.kind,
        name: pack.name.clone(),
        version: pack.version.clone(),
        owner: pack.owner.clone(),
        state: PackState::Draft,
        bytes: pack.bytes,
        created_at: pack.created_at.clone(),
        validated_at: None,
        frozen_at: None,
        retired_at: None,
    }
}

impl PackRegistry for MemoryPacks {
    fn register(
        &self,
        profile: ProfileId,
        pack: NewPack,
    ) -> BoxFuture<Result<RegisterOutcome, PackRegistryError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| PackRegistryError::Store("the pack lock is poisoned".to_owned()))?;
            let owner = profile.as_str().to_owned();
            let key = (owner.clone(), pack.pack_id.to_string());
            if let Some(existing) = state.rows.get(&key).cloned() {
                return Ok(RegisterOutcome {
                    record: existing,
                    created: false,
                });
            }
            if let Some(existing) = state
                .label_owner(&owner, &pack.name, &pack.version)
                .map(str::to_owned)
                && existing != pack.pack_id.to_string()
            {
                return Err(PackRegistryError::LabelConflict {
                    existing: Digest::parse(&existing).map_err(|error| {
                        PackRegistryError::Store(format!("a stored pack id is malformed: {error}"))
                    })?,
                    name: pack.name,
                    version: pack.version,
                });
            }
            let record = PackRecord {
                summary: summary(&pack),
                object_key: pack.object_key,
                sha256: pack.pack_id.clone(),
                validations: Vec::new(),
                events: vec![PackLifecycleEvent {
                    from: None,
                    to: PackState::Draft,
                    action: "create".to_owned(),
                    reason: None,
                    created_at: pack.created_at.clone(),
                }],
            };
            state.rows.insert(key, record.clone());
            Ok(RegisterOutcome {
                record,
                created: true,
            })
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        pack_id: String,
    ) -> BoxFuture<Result<Option<PackRecord>, PackRegistryError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            Ok(inner
                .lock()
                .map_err(|_| PackRegistryError::Store("the pack lock is poisoned".to_owned()))?
                .rows
                .get(&(profile.as_str().to_owned(), pack_id))
                .cloned())
        })
    }

    fn list(
        &self,
        profile: ProfileId,
        filter: PackFilter,
    ) -> BoxFuture<Result<Vec<PackSummary>, PackRegistryError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| PackRegistryError::Store("the pack lock is poisoned".to_owned()))?;
            let mut records: Vec<&PackRecord> = state
                .rows
                .iter()
                .filter(|((owner, _), record)| {
                    owner == profile.as_str()
                        && filter.kind.is_none_or(|kind| kind == record.summary.kind)
                        && filter
                            .state
                            .is_none_or(|state| state == record.summary.state)
                })
                .map(|(_, record)| record)
                .collect();
            records.sort_by(|left, right| {
                left.summary
                    .name
                    .cmp(&right.summary.name)
                    .then_with(|| left.summary.version.cmp(&right.summary.version))
                    .then_with(|| left.summary.pack_id.cmp(&right.summary.pack_id))
            });
            Ok(records
                .into_iter()
                .map(|record| record.summary.clone())
                .collect())
        })
    }

    fn validate(
        &self,
        profile: ProfileId,
        pack_id: String,
        validated_at: String,
    ) -> BoxFuture<Result<PackRecord, PackRegistryError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| PackRegistryError::Store("the pack lock is poisoned".to_owned()))?;
            let key = (profile.as_str().to_owned(), pack_id);
            let record = state
                .rows
                .get(&key)
                .cloned()
                .ok_or(PackRegistryError::NotFound)?;
            let mut record = match record.summary.state {
                PackState::Frozen => return Err(PackRegistryError::Frozen),
                PackState::Retired => return Err(PackRegistryError::Retired),
                PackState::Draft => {
                    let mut next = MemoryState::transition(
                        &record,
                        PackState::Validated,
                        "validate",
                        &validated_at,
                    );
                    next.summary.validated_at = Some(validated_at.clone());
                    next
                }
                PackState::Validated => record,
            };
            record.validations.push(PackValidationRecord {
                validator: VALIDATOR.to_owned(),
                valid: true,
                message: None,
                created_at: validated_at,
            });
            state.rows.insert(key, record.clone());
            Ok(record)
        })
    }

    fn freeze(
        &self,
        profile: ProfileId,
        pack_id: String,
        frozen_at: String,
    ) -> BoxFuture<Result<PackRecord, PackRegistryError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| PackRegistryError::Store("the pack lock is poisoned".to_owned()))?;
            let key = (profile.as_str().to_owned(), pack_id);
            let record = state
                .rows
                .get(&key)
                .cloned()
                .ok_or(PackRegistryError::NotFound)?;
            let record = match record.summary.state {
                PackState::Frozen => record,
                PackState::Retired => return Err(PackRegistryError::Retired),
                PackState::Draft => {
                    return Err(PackRegistryError::NotValidated(PackState::Draft));
                }
                PackState::Validated => {
                    let mut next =
                        MemoryState::transition(&record, PackState::Frozen, "freeze", &frozen_at);
                    next.summary.frozen_at = Some(frozen_at);
                    next
                }
            };
            state.rows.insert(key, record.clone());
            Ok(record)
        })
    }

    fn retire(
        &self,
        profile: ProfileId,
        pack_id: String,
        retired_at: String,
    ) -> BoxFuture<Result<PackRecord, PackRegistryError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| PackRegistryError::Store("the pack lock is poisoned".to_owned()))?;
            let key = (profile.as_str().to_owned(), pack_id);
            let record = state
                .rows
                .get(&key)
                .cloned()
                .ok_or(PackRegistryError::NotFound)?;
            let record = match record.summary.state {
                PackState::Retired => record,
                PackState::Frozen => {
                    let mut next =
                        MemoryState::transition(&record, PackState::Retired, "retire", &retired_at);
                    next.summary.retired_at = Some(retired_at);
                    next
                }
                other => return Err(PackRegistryError::NotFrozen(other)),
            };
            state.rows.insert(key, record.clone());
            Ok(record)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::parse_pack;
    use quantrule_api::PackKind;
    use serde_json::json;

    fn profile(name: &str) -> ProfileId {
        ProfileId::parse(name).expect("profile")
    }

    fn portfolio(version: &str, exposure: f64) -> NewPack {
        let document = json!({
            "schema": "quantrule.pack/v1",
            "kind": "portfolio",
            "version": version,
            "name": "single-position-conservative",
            "owner": "operator",
            "payload": {
                "sizing": {
                    "mode": "equity_fraction",
                    "fraction": 0.05,
                    "quantity_step": 0.0001
                },
                "risk": {
                    "per_trade_risk": 0.01,
                    "max_exposure": exposure,
                    "max_open_positions": 1
                },
                "drawdown": {"limit": 0.15, "action": "halt"},
                "rebalance": "none",
                "constraints": []
            }
        });
        let bytes = serde_json::to_vec(&document).expect("encode");
        let parsed = parse_pack(&bytes).expect("the pack is valid");
        NewPack {
            pack_id: parsed.pack_id.clone(),
            kind: parsed.kind,
            name: parsed.name,
            version: parsed.version,
            owner: parsed.owner,
            object_key: format!("packs/{}", parsed.pack_id.as_hex()),
            bytes: parsed.canonical.len() as u64,
            created_at: "2026-11-22T00:00:00Z".to_owned(),
        }
    }

    #[tokio::test]
    async fn the_same_content_is_idempotent_and_the_label_is_unique() {
        let registry = MemoryPacks::default();
        let first = portfolio("1.0.0", 0.5);
        let outcome = registry
            .register(profile("default"), first.clone())
            .await
            .expect("register");
        assert!(outcome.created);
        assert_eq!(outcome.record.summary.state, PackState::Draft);
        let again = registry
            .register(profile("default"), first.clone())
            .await
            .expect("idempotent");
        assert!(!again.created);
        assert_eq!(again.record.summary.pack_id, first.pack_id);

        let changed = portfolio("1.0.0", 0.25);
        assert_ne!(changed.pack_id, first.pack_id);
        let error = registry
            .register(profile("default"), changed)
            .await
            .expect_err("the label must not change bytes");
        assert!(matches!(error, PackRegistryError::LabelConflict { .. }));
    }

    #[tokio::test]
    async fn freeze_is_guarded_and_idempotent() {
        let registry = MemoryPacks::default();
        let pack = portfolio("1.0.0", 0.5);
        let id = pack.pack_id.to_string();
        registry
            .register(profile("default"), pack)
            .await
            .expect("register");
        assert!(matches!(
            registry
                .freeze(profile("default"), id.clone(), "t1".to_owned())
                .await,
            Err(PackRegistryError::NotValidated(_))
        ));
        let validated = registry
            .validate(profile("default"), id.clone(), "t2".to_owned())
            .await
            .expect("validate");
        assert_eq!(validated.summary.state, PackState::Validated);
        let frozen = registry
            .freeze(profile("default"), id.clone(), "t3".to_owned())
            .await
            .expect("freeze");
        assert_eq!(frozen.summary.state, PackState::Frozen);
        let again = registry
            .freeze(profile("default"), id.clone(), "t4".to_owned())
            .await
            .expect("freeze is idempotent");
        assert_eq!(again, frozen);
        assert!(matches!(
            registry
                .validate(profile("default"), id.clone(), "t5".to_owned())
                .await,
            Err(PackRegistryError::Frozen)
        ));
        let retired = registry
            .retire(profile("default"), id.clone(), "t6".to_owned())
            .await
            .expect("retire");
        assert_eq!(retired.summary.state, PackState::Retired);
        let again = registry
            .retire(profile("default"), id, "t7".to_owned())
            .await
            .expect("retire is idempotent");
        assert_eq!(again, retired);
    }

    #[tokio::test]
    async fn the_list_is_ordered_and_filtered() {
        let registry = MemoryPacks::default();
        let first = portfolio("1.0.0", 0.5);
        let second = portfolio("1.0.1", 0.5);
        registry
            .register(profile("default"), second.clone())
            .await
            .expect("second");
        registry
            .register(profile("default"), first.clone())
            .await
            .expect("first");
        let listed = registry
            .list(profile("default"), PackFilter::default())
            .await
            .expect("list");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].version, "1.0.0");
        assert_eq!(listed[1].version, "1.0.1");
        assert!(
            registry
                .list(
                    profile("default"),
                    PackFilter {
                        kind: Some(PackKind::Strategy),
                        state: None,
                    }
                )
                .await
                .expect("filtered")
                .is_empty()
        );
    }
}
