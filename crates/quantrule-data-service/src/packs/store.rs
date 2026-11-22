//! Pack registry storage behind one trait.
//!
//! Registration is content-addressed and label-safe: the same canonical bytes
//! are idempotent, and a `(name, version)` label that already names another
//! pack is refused. The object store is the single authority for the bytes;
//! the row carries the object key, content hash and size. Lifecycle transitions
//! are guarded per record, so a freeze cannot race a validate into an unlawful
//! state and a frozen pack cannot be mutated.

use quantrule_api::{Digest, PackKind, PackRecord, PackState, PackSummary, ProfileId};
use quantrule_runtime::adapters::BoxFuture;

/// A pack where the registry stores its metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPack {
    /// The SHA-256 of the canonical bytes.
    pub pack_id: Digest,
    /// The pack kind.
    pub kind: PackKind,
    /// The non-empty human name.
    pub name: String,
    /// The human version label.
    pub version: String,
    /// The owner label.
    pub owner: String,
    /// The object key the canonical bytes are stored under.
    pub object_key: String,
    /// The canonical byte count.
    pub bytes: u64,
    /// The creation time, RFC 3339 UTC.
    pub created_at: String,
}

/// The optional filters on a pack list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PackFilter {
    /// Only this kind, when set.
    pub kind: Option<PackKind>,
    /// Only this lifecycle state, when set.
    pub state: Option<PackState>,
}

/// The outcome of a registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterOutcome {
    /// The stored record.
    pub record: PackRecord,
    /// Whether this call created the row.
    pub created: bool,
}

/// Why a registry operation failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackRegistryError {
    /// The pack is unknown in this profile.
    NotFound,
    /// The `(name, version)` label already names another pack.
    LabelConflict {
        /// The pack the label already names.
        existing: Digest,
        /// The name that collided.
        name: String,
        /// The version that collided.
        version: String,
    },
    /// Freeze requires a validated pack.
    NotValidated(PackState),
    /// Retire requires a frozen pack.
    NotFrozen(PackState),
    /// The pack is frozen and refuses the operation.
    Frozen,
    /// The pack is retired and refuses the operation.
    Retired,
    /// The store refused the operation.
    Store(String),
}

impl std::fmt::Display for PackRegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("the pack is unknown"),
            Self::LabelConflict {
                existing,
                name,
                version,
            } => write!(
                formatter,
                "pack {name} version {version} already names {existing}"
            ),
            Self::NotValidated(state) => {
                write!(formatter, "freeze requires a validated pack, not {state}")
            }
            Self::NotFrozen(state) => {
                write!(formatter, "retire requires a frozen pack, not {state}")
            }
            Self::Frozen => formatter.write_str("a frozen pack cannot be mutated"),
            Self::Retired => formatter.write_str("a retired pack is terminal"),
            Self::Store(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for PackRegistryError {}

/// Where research packs live.
pub trait PackRegistry: Send + Sync {
    /// Register one pack. The same content is idempotent.
    fn register(
        &self,
        profile: ProfileId,
        pack: NewPack,
    ) -> BoxFuture<Result<RegisterOutcome, PackRegistryError>>;

    /// Read one pack by content id within a profile.
    fn get(
        &self,
        profile: ProfileId,
        pack_id: String,
    ) -> BoxFuture<Result<Option<PackRecord>, PackRegistryError>>;

    /// List a profile's packs, ordered by name, version and id.
    fn list(
        &self,
        profile: ProfileId,
        filter: PackFilter,
    ) -> BoxFuture<Result<Vec<PackSummary>, PackRegistryError>>;

    /// Record a successful validation and move a draft to validated.
    ///
    /// A second validation of an already validated pack appends a record and
    /// leaves the state unchanged. A frozen or retired pack is refused.
    fn validate(
        &self,
        profile: ProfileId,
        pack_id: String,
        validated_at: String,
    ) -> BoxFuture<Result<PackRecord, PackRegistryError>>;

    /// Freeze a validated pack. Freezing a frozen pack is idempotent.
    fn freeze(
        &self,
        profile: ProfileId,
        pack_id: String,
        frozen_at: String,
    ) -> BoxFuture<Result<PackRecord, PackRegistryError>>;

    /// Retire a frozen pack from new runs. Retiring a retired pack is
    /// idempotent; the row and bytes are never deleted.
    fn retire(
        &self,
        profile: ProfileId,
        pack_id: String,
        retired_at: String,
    ) -> BoxFuture<Result<PackRecord, PackRegistryError>>;
}
