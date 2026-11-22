//! Canonical pack parsing at the registry boundary.
//!
//! The application-service parser is the one definition of what a pack is. The
//! registry calls it once at creation, writes the canonical bytes it produced
//! to the object store, and never re-serialises the document afterwards. A
//! stored pack therefore keeps the identity it was created with even if a
//! serializer changes later.

use quantrule_api::{Digest, PackKind};
use quantrule_app::pack::{ExecutionPack, PackEnvelope, PortfolioPack, StrategyPack};
use serde_json::Value;

/// Why an uploaded pack could not become a registry row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackParseError(pub String);

impl std::fmt::Display for PackParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for PackParseError {}

/// A parsed pack and the canonical bytes its id covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPack {
    /// The pack kind.
    pub kind: PackKind,
    /// The SHA-256 of `canonical`.
    pub pack_id: Digest,
    /// The non-empty human name.
    pub name: String,
    /// The human version label.
    pub version: String,
    /// The owner label.
    pub owner: String,
    /// The canonical bytes, including one trailing newline.
    pub canonical: Vec<u8>,
}

/// Parses one uploaded pack document.
///
/// # Errors
///
/// Returns [`PackParseError`] when the bytes are not JSON, the envelope is
/// invalid, the kind-specific payload is rejected, or the computed identity
/// disagrees with the canonical bytes.
pub fn parse_pack(bytes: &[u8]) -> Result<ParsedPack, PackParseError> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| PackParseError(format!("the pack is not JSON: {error}")))?;
    let envelope = PackEnvelope::parse(&value, "pack.json")
        .map_err(|error| PackParseError(error.to_string()))?;
    let kind = envelope.metadata().kind();
    let parsed = match kind {
        PackKind::Strategy => {
            let pack = StrategyPack::parse(&value, "pack.json")
                .map_err(|error| PackParseError(error.to_string()))?;
            parsed(kind, pack.metadata(), pack.id(), pack.canonical_bytes())
        }
        PackKind::Portfolio => {
            let pack = PortfolioPack::parse(&value, "pack.json")
                .map_err(|error| PackParseError(error.to_string()))?;
            parsed(kind, pack.metadata(), pack.id(), pack.canonical_bytes())
        }
        PackKind::Execution => {
            let pack = ExecutionPack::parse(&value, "pack.json")
                .map_err(|error| PackParseError(error.to_string()))?;
            parsed(kind, pack.metadata(), pack.id(), pack.canonical_bytes())
        }
    };
    if Digest::sha256(&parsed.canonical) != parsed.pack_id {
        return Err(PackParseError(
            "the canonical pack bytes do not hash to the computed pack id".to_owned(),
        ));
    }
    Ok(parsed)
}

fn parsed(
    kind: PackKind,
    metadata: &quantrule_app::pack::PackMetadata,
    id: &Digest,
    canonical: &[u8],
) -> ParsedPack {
    ParsedPack {
        kind,
        pack_id: id.clone(),
        name: metadata.name().to_owned(),
        version: metadata.version().to_owned(),
        owner: metadata.owner().to_owned(),
        canonical: canonical.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn document(seed: &str) -> Value {
        json!({
            "schema": "quantrule.pack/v1",
            "kind": "portfolio",
            "version": "1.0.0",
            "name": format!("policy-{seed}"),
            "owner": "operator",
            "payload": {
                "sizing": {
                    "mode": "equity_fraction",
                    "fraction": 0.05,
                    "quantity_step": 0.0001
                },
                "risk": {
                    "per_trade_risk": 0.01,
                    "max_exposure": 0.5,
                    "max_open_positions": 1
                },
                "drawdown": {"limit": 0.15, "action": "halt"},
                "rebalance": "none",
                "constraints": []
            }
        })
    }

    #[test]
    fn a_pack_is_canonical_and_identity_stable() {
        let text = serde_json::to_string(&document("a")).expect("encode");
        let parsed = parse_pack(text.as_bytes()).expect("the pack is valid");
        assert_eq!(parsed.kind, PackKind::Portfolio);
        assert_eq!(parsed.name, "policy-a");
        assert_eq!(Digest::sha256(&parsed.canonical), parsed.pack_id);
        let again = parse_pack(&parsed.canonical).expect("canonical bytes reparse");
        assert_eq!(again.pack_id, parsed.pack_id);
        assert_eq!(again.canonical, parsed.canonical);
    }

    #[test]
    fn two_documents_differing_in_order_share_one_identity() {
        let left = serde_json::to_string(&document("a")).expect("encode");
        let mut object = document("a");
        object["payload"]["risk"]["max_exposure"] = json!(0.5);
        let right = serde_json::to_string(&object).expect("encode");
        assert_eq!(
            parse_pack(left.as_bytes()).expect("left").pack_id,
            parse_pack(right.as_bytes()).expect("right").pack_id
        );
    }

    #[test]
    fn an_unknown_field_is_refused() {
        let mut object = document("a");
        object["payload"]["venue"] = json!("binance");
        let text = serde_json::to_string(&object).expect("encode");
        assert!(parse_pack(text.as_bytes()).is_err());
    }
}
