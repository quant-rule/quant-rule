//! Pack identity and duplicate-label rejection.
//!
//! A pack is named by the SHA-256 of its canonical bytes, but the human label
//! `(name, version)` is also unique. The same bytes may be registered again;
//! the same label pointing at different bytes is refused, because silently
//! replacing a pack would break a run that cites the old hash.

use std::collections::BTreeMap;
use std::fmt;

use quantrule_api::Digest;

use super::strategy::StrategyPack;

/// The `(name, version)` label already identifies different bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackIdentityConflict {
    name: String,
    version: String,
    existing: Digest,
    incoming: Digest,
}

impl PackIdentityConflict {
    /// The pack name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The pack version label.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The ID already registered for the label.
    #[must_use]
    pub const fn existing(&self) -> &Digest {
        &self.existing
    }

    /// The ID the caller tried to register.
    #[must_use]
    pub const fn incoming(&self) -> &Digest {
        &self.incoming
    }
}

impl fmt::Display for PackIdentityConflict {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "pack {} version {} already names {} and cannot be replaced by {}",
            self.name, self.version, self.existing, self.incoming
        )
    }
}

impl std::error::Error for PackIdentityConflict {}

/// The in-memory identity index for packs.
///
/// This is the validation seam a persistent catalog will reuse: registration is
/// idempotent for the same ID and refuses a label that already names another
/// ID. The index stores identities only; bytes and lifecycle live elsewhere.
#[derive(Debug, Clone, Default)]
pub struct PackIndex {
    by_label: BTreeMap<(String, String), Digest>,
}

impl PackIndex {
    /// Creates an empty index.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one pack identity.
    ///
    /// # Errors
    ///
    /// Returns [`PackIdentityConflict`] when `(name, version)` is already
    /// registered under a different content hash. Registering the same pack
    /// again is allowed.
    pub fn register(&mut self, pack: &StrategyPack) -> Result<(), PackIdentityConflict> {
        let label = (
            pack.metadata().name().to_owned(),
            pack.metadata().version().to_owned(),
        );
        match self.by_label.get(&label) {
            Some(existing) if existing != pack.id() => Err(PackIdentityConflict {
                name: label.0,
                version: label.1,
                existing: existing.clone(),
                incoming: pack.id().clone(),
            }),
            Some(_) => Ok(()),
            None => {
                self.by_label.insert(label, pack.id().clone());
                Ok(())
            }
        }
    }

    /// Resolves a human label to the registered content hash.
    #[must_use]
    pub fn resolve(&self, name: &str, version: &str) -> Option<&Digest> {
        self.by_label.get(&(name.to_owned(), version.to_owned()))
    }

    /// The number of registered labels.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_label.len()
    }

    /// Whether no label is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_label.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::PackIndex;
    use crate::pack::StrategyPack;

    fn pack(version: &str, reward: f64) -> StrategyPack {
        let document = serde_json::json!({
            "schema": "quantrule.pack/v1",
            "kind": "strategy",
            "version": version,
            "name": "compression-fade-5m",
            "owner": "operator",
            "payload": {
                "mode": "signal",
                "engine": "event_composite",
                "execution_timeframe": "5m",
                "context_timeframes": ["15m"],
                "event_window_bars": 1,
                "time_stop_bars": 0,
                "giveback_atr": null,
                "giveback_arm_atr": 0.0,
                "require_contiguous": true,
                "risk": {"stop_atr_multiple": 2.0, "reward_to_risk": reward},
                "entry": [
                    {"kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 1, "source": 4, "direction": 1}
                ],
                "exit": []
            }
        });
        StrategyPack::parse(&document, "pack.json").expect("the test pack is valid")
    }

    #[test]
    fn registering_the_same_pack_twice_is_idempotent() {
        let pack = pack("1.0.0", 1.5);
        let mut index = PackIndex::new();
        index.register(&pack).expect("the first registration");
        index
            .register(&pack)
            .expect("the same bytes are idempotent");
        assert_eq!(index.len(), 1);
        assert_eq!(
            index.resolve("compression-fade-5m", "1.0.0"),
            Some(pack.id())
        );
    }

    #[test]
    fn a_label_with_different_bytes_is_rejected() {
        let first = pack("1.0.0", 1.5);
        let second = pack("1.0.0", 2.0);
        assert_ne!(first.id(), second.id());
        let mut index = PackIndex::new();
        index.register(&first).expect("the first registration");
        let conflict = index
            .register(&second)
            .expect_err("the same label must not change bytes");
        assert_eq!(conflict.name(), "compression-fade-5m");
        assert_eq!(conflict.version(), "1.0.0");
        assert_eq!(conflict.existing(), first.id());
        assert_eq!(conflict.incoming(), second.id());
    }

    #[test]
    fn a_new_version_may_change_the_bytes() {
        let first = pack("1.0.0", 1.5);
        let second = pack("1.0.1", 1.5);
        let mut index = PackIndex::new();
        index.register(&first).expect("the first registration");
        index
            .register(&second)
            .expect("a new version is a new pack");
        assert_eq!(index.len(), 2);
        assert_eq!(
            index.resolve("compression-fade-5m", "1.0.0"),
            Some(first.id())
        );
        assert_eq!(
            index.resolve("compression-fade-5m", "1.0.1"),
            Some(second.id())
        );
    }
}
