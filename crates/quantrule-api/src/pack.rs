//! The persistent research pack registry contract.
//!
//! A pack is an immutable, versioned, content-addressed document. The
//! `pack_id` is the SHA-256 of the canonical bytes; lifecycle state is
//! metadata outside the hash. The catalog row is what a client lists and what a
//! run cites, and the bytes live in both the catalog file table and the object
//! store under the same content key.
//!
//! Lifecycle vocabulary is shared with the application-service parser so the
//! registry, the wire contract, and the in-process guards cannot drift.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::digest::Digest;

/// The kind of document carried in a pack payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackKind {
    /// A human-owned alpha definition.
    Strategy,
    /// A human-owned portfolio policy definition.
    Portfolio,
    /// A human-owned execution-simulation definition.
    Execution,
}

impl PackKind {
    /// The stable wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Strategy => "strategy",
            Self::Portfolio => "portfolio",
            Self::Execution => "execution",
        }
    }

    /// Parses the stable wire code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "strategy" => Some(Self::Strategy),
            "portfolio" => Some(Self::Portfolio),
            "execution" => Some(Self::Execution),
            _ => None,
        }
    }
}

impl fmt::Display for PackKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

/// One pack lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackState {
    /// Editable and validatable; not usable in a run.
    Draft,
    /// Schema-valid and ready for the freeze decision.
    Validated,
    /// Immutable and usable in new runs.
    Frozen,
    /// Readable for existing runs only; refused for new runs.
    Retired,
}

impl PackState {
    /// The stable wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Validated => "validated",
            Self::Frozen => "frozen",
            Self::Retired => "retired",
        }
    }

    /// Parses the stable wire code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "draft" => Some(Self::Draft),
            "validated" => Some(Self::Validated),
            "frozen" => Some(Self::Frozen),
            "retired" => Some(Self::Retired),
            _ => None,
        }
    }

    /// Whether the lifecycle permits a move from this state to `next`.
    ///
    /// `validated -> draft` is the rejection path. A frozen pack cannot return
    /// to an earlier state, and a retired pack is terminal.
    #[must_use]
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::Validated)
                | (Self::Validated, Self::Draft)
                | (Self::Validated, Self::Frozen)
                | (Self::Frozen, Self::Retired)
        )
    }

    /// Whether a new run may reference a pack in this state.
    ///
    /// Only a frozen pack is usable for a new run. A retired pack remains
    /// readable for runs that already cite it.
    #[must_use]
    pub const fn allows_new_runs(self) -> bool {
        matches!(self, Self::Frozen)
    }
}

impl fmt::Display for PackState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

/// A pack in a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackSummary {
    /// The SHA-256 of the canonical pack bytes.
    pub pack_id: Digest,
    /// The pack kind.
    pub kind: PackKind,
    /// The non-empty human name.
    pub name: String,
    /// The human version label, which is not the identity.
    pub version: String,
    /// The owner label; it is not an authorisation boundary.
    pub owner: String,
    /// The lifecycle state.
    pub state: PackState,
    /// The canonical byte count.
    pub bytes: u64,
    /// When the row was created, RFC 3339 UTC.
    pub created_at: String,
    /// When validation last succeeded, RFC 3339 UTC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validated_at: Option<String>,
    /// When the pack was frozen, RFC 3339 UTC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frozen_at: Option<String>,
    /// When the pack was retired, RFC 3339 UTC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retired_at: Option<String>,
}

/// One validation attempt recorded against a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackValidationRecord {
    /// The validator identity, such as `pack.schema/v1`.
    pub validator: String,
    /// Whether the pack passed.
    pub valid: bool,
    /// The failure message when the pack did not pass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// When the attempt was recorded, RFC 3339 UTC.
    pub created_at: String,
}

/// One append-only lifecycle transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackLifecycleEvent {
    /// The state before the transition; absent for creation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<PackState>,
    /// The state after the transition.
    pub to: PackState,
    /// The operation, such as `create`, `validate`, `freeze`, or `retire`.
    pub action: String,
    /// An optional human reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// When the transition committed, RFC 3339 UTC.
    pub created_at: String,
}

/// A pack in detail, including its object key and history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackRecord {
    /// The list projection.
    pub summary: PackSummary,
    /// The object key inside the content-addressed store.
    pub object_key: String,
    /// The SHA-256 of the canonical bytes, equal to `summary.pack_id`.
    pub sha256: Digest,
    /// Validation attempts, oldest first.
    pub validations: Vec<PackValidationRecord>,
    /// Lifecycle transitions, oldest first.
    pub events: Vec<PackLifecycleEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(seed: &[u8]) -> Digest {
        Digest::sha256(seed)
    }

    #[test]
    fn pack_enums_round_trip_their_wire_codes() {
        for kind in [PackKind::Strategy, PackKind::Portfolio, PackKind::Execution] {
            assert_eq!(PackKind::from_code(kind.code()), Some(kind));
            let text = serde_json::to_string(&kind).expect("encode");
            assert_eq!(text, format!("\"{}\"", kind.code()));
        }
        for state in [
            PackState::Draft,
            PackState::Validated,
            PackState::Frozen,
            PackState::Retired,
        ] {
            assert_eq!(PackState::from_code(state.code()), Some(state));
            let text = serde_json::to_string(&state).expect("encode");
            assert_eq!(text, format!("\"{}\"", state.code()));
        }
        assert_eq!(PackKind::from_code("book"), None);
        assert_eq!(PackState::from_code("deleted"), None);
    }

    #[test]
    fn only_a_frozen_pack_is_usable_for_a_new_run() {
        assert!(!PackState::Draft.allows_new_runs());
        assert!(!PackState::Validated.allows_new_runs());
        assert!(PackState::Frozen.allows_new_runs());
        assert!(!PackState::Retired.allows_new_runs());
    }

    #[test]
    fn a_pack_record_round_trips() {
        let pack_id = digest(b"pack");
        let record = PackRecord {
            summary: PackSummary {
                pack_id: pack_id.clone(),
                kind: PackKind::Strategy,
                name: "compression-fade-5m".to_owned(),
                version: "1.0.0".to_owned(),
                owner: "operator".to_owned(),
                state: PackState::Frozen,
                bytes: 512,
                created_at: "2026-11-22T00:00:00Z".to_owned(),
                validated_at: Some("2026-11-22T00:01:00Z".to_owned()),
                frozen_at: Some("2026-11-22T00:02:00Z".to_owned()),
                retired_at: None,
            },
            object_key: format!("packs/{}", pack_id.as_hex()),
            sha256: pack_id,
            validations: vec![PackValidationRecord {
                validator: "pack.schema/v1".to_owned(),
                valid: true,
                message: None,
                created_at: "2026-11-22T00:01:00Z".to_owned(),
            }],
            events: vec![PackLifecycleEvent {
                from: None,
                to: PackState::Draft,
                action: "create".to_owned(),
                reason: None,
                created_at: "2026-11-22T00:00:00Z".to_owned(),
            }],
        };
        let text = serde_json::to_string(&record).expect("encode");
        let back: PackRecord = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, record);
    }
}
