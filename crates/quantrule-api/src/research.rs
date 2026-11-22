//! Content references and run lineage for research artifacts.
//!
//! A `quantrule.run/v5` document and an evidence lineage record name an
//! artifact by its id and content hash; they never embed a pack payload or a
//! signal row. The two values are the same content address, so a record that
//! names two different values for one object is refused before its bytes are
//! read.
//!
//! The types are transport-free. The application service parses a run document
//! into them, and the evidence service stores the lineage record in its own
//! schema; neither embeds the other's storage.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::digest::Digest;
use crate::pack::{PackKind, PackState};

/// The kind of object a run reference names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceKind {
    /// The upstream model bundle.
    ModelArtifact,
    /// The human-owned alpha definition.
    StrategyPack,
    /// The computed signal output of one model artifact and strategy pack.
    SignalArtifact,
    /// The human-owned portfolio policy.
    PortfolioPolicy,
    /// The human-owned execution-simulation policy.
    ExecutionModel,
}

impl ReferenceKind {
    /// The stable wire and hash tag.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ModelArtifact => "model_artifact",
            Self::StrategyPack => "strategy_pack",
            Self::SignalArtifact => "signal_artifact",
            Self::PortfolioPolicy => "portfolio_policy",
            Self::ExecutionModel => "execution_model",
        }
    }

    /// Parses the stable wire code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "model_artifact" => Some(Self::ModelArtifact),
            "strategy_pack" => Some(Self::StrategyPack),
            "signal_artifact" => Some(Self::SignalArtifact),
            "portfolio_policy" => Some(Self::PortfolioPolicy),
            "execution_model" => Some(Self::ExecutionModel),
            _ => None,
        }
    }

    /// The pack kind this reference must resolve to, when it names a pack.
    #[must_use]
    pub const fn pack_kind(self) -> Option<PackKind> {
        match self {
            Self::StrategyPack => Some(PackKind::Strategy),
            Self::PortfolioPolicy => Some(PackKind::Portfolio),
            Self::ExecutionModel => Some(PackKind::Execution),
            Self::ModelArtifact | Self::SignalArtifact => None,
        }
    }
}

impl fmt::Display for ReferenceKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

/// A run's id-and-hash reference to one immutable object.
///
/// The two fields are the same SHA-256 value. Keeping both matches the run
/// contract and makes an inconsistent document visible without reading bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentReference {
    /// The object id.
    pub id: Digest,
    /// The content hash of the stored bytes.
    pub hash: Digest,
}

impl ContentReference {
    /// Builds a reference from its two content-address fields.
    #[must_use]
    pub const fn new(id: Digest, hash: Digest) -> Self {
        Self { id, hash }
    }

    /// Whether the id and hash name the same content address.
    #[must_use]
    pub fn is_consistent(&self) -> bool {
        self.id == self.hash
    }

    /// The wire form, for a provenance record.
    ///
    /// The map is the serializer's ordered map, so the same reference always
    /// produces the same bytes.
    #[must_use]
    pub fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("id".to_owned(), Value::String(self.id.to_string()));
        object.insert("hash".to_owned(), Value::String(self.hash.to_string()));
        Value::Object(object)
    }

    /// Whether these bytes carry this reference's content hash.
    #[must_use]
    pub fn matches(&self, bytes: &[u8]) -> bool {
        Digest::sha256(bytes) == self.hash
    }
}

/// The pack metadata a lineage record keeps as provenance.
///
/// This is the pack as it was when the run cited it. Lifecycle state is
/// recorded too, because a later retire does not rewrite the history of a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackProvenance {
    /// The pack kind.
    pub kind: PackKind,
    /// The non-empty human name.
    pub name: String,
    /// The human version label.
    pub version: String,
    /// The owner label; it is not an authorisation boundary.
    pub owner: String,
    /// The lifecycle state at the time the lineage was recorded.
    pub state: PackState,
}

/// One artifact a run cites, with pack provenance when it names a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunArtifactReference {
    /// The role the object plays in the run.
    pub kind: ReferenceKind,
    /// The object id.
    pub id: Digest,
    /// The content hash of the stored bytes.
    pub hash: Digest,
    /// The pack metadata, present exactly for a pack reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack: Option<PackProvenance>,
}

impl RunArtifactReference {
    /// Whether the id and hash name the same content address.
    #[must_use]
    pub fn is_consistent(&self) -> bool {
        self.id == self.hash
    }

    /// Whether a pack provenance is present exactly when the kind names a pack.
    #[must_use]
    pub fn has_expected_provenance(&self) -> bool {
        match (self.kind.pack_kind(), self.pack.as_ref()) {
            (None, None) => true,
            (Some(expected), Some(pack)) => pack.kind == expected,
            _ => false,
        }
    }
}

/// The lineage a run record keeps: every object the run cited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunLineage {
    /// The run the lineage belongs to.
    pub run_id: crate::run::RunId,
    /// The references, in the fixed provenance order.
    pub references: Vec<RunArtifactReference>,
    /// When the lineage was first recorded, RFC 3339 UTC.
    pub recorded_at: String,
}

/// A request to record a run's lineage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordRunLineageRequest {
    /// The references the run cites. Empty is valid for a run with no packs.
    pub references: Vec<RunArtifactReference>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::{PackKind, PackState};

    fn digest(seed: &[u8]) -> Digest {
        Digest::sha256(seed)
    }

    #[test]
    fn reference_kinds_round_trip_their_wire_codes() {
        for kind in [
            ReferenceKind::ModelArtifact,
            ReferenceKind::StrategyPack,
            ReferenceKind::SignalArtifact,
            ReferenceKind::PortfolioPolicy,
            ReferenceKind::ExecutionModel,
        ] {
            assert_eq!(ReferenceKind::from_code(kind.code()), Some(kind));
            let text = serde_json::to_string(&kind).expect("encode");
            assert_eq!(text, format!("\"{}\"", kind.code()));
        }
        assert_eq!(ReferenceKind::from_code("book"), None);
    }

    #[test]
    fn a_content_reference_round_trips_and_reports_inconsistency() {
        let id = digest(b"one");
        let reference = ContentReference::new(id.clone(), id.clone());
        assert!(reference.is_consistent());
        assert!(reference.matches(b"one"));
        let text = serde_json::to_string(&reference).expect("encode");
        let back: ContentReference = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, reference);
        let wrong = ContentReference::new(id, digest(b"two"));
        assert!(!wrong.is_consistent());
    }

    #[test]
    fn a_pack_reference_needs_its_own_pack_kind() {
        let id = digest(b"pack");
        let pack = PackProvenance {
            kind: PackKind::Strategy,
            name: "compression-fade-5m".to_owned(),
            version: "1.0.0".to_owned(),
            owner: "operator".to_owned(),
            state: PackState::Frozen,
        };
        let reference = RunArtifactReference {
            kind: ReferenceKind::StrategyPack,
            id: id.clone(),
            hash: id.clone(),
            pack: Some(pack.clone()),
        };
        assert!(reference.has_expected_provenance());
        let wrong = RunArtifactReference {
            pack: Some(PackProvenance {
                kind: PackKind::Portfolio,
                ..pack
            }),
            ..reference.clone()
        };
        assert!(!wrong.has_expected_provenance());
        let missing = RunArtifactReference {
            pack: None,
            ..reference.clone()
        };
        assert!(!missing.has_expected_provenance());
        let unexpected = RunArtifactReference {
            kind: ReferenceKind::ModelArtifact,
            pack: reference.pack,
            ..reference
        };
        assert!(!unexpected.has_expected_provenance());
    }

    #[test]
    fn a_lineage_request_rejects_an_unknown_field() {
        let text = "{\"references\":[],\"path\":\"/tmp\"}";
        assert!(serde_json::from_str::<RecordRunLineageRequest>(text).is_err());
    }
}
