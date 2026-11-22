//! Research artifact packs: the envelope, the strategy payload, and identity.
//!
//! A pack is an immutable, versioned, content-addressed definition. The
//! envelope and the kind-specific payload are hashed; lifecycle state and the
//! registry row are metadata outside the hash. This module owns parsing,
//! validation, the canonical content hash, duplicate-label rejection, and the
//! lifecycle guards. It does not store bytes, expose a command, or compute a
//! signal; those belong to the owning service and later issues.
//!
//! Only the strategy kind is implemented here. Portfolio and execution parsers
//! arrive with their own packs; the envelope already recognises their kind so a
//! strategy parser cannot silently accept them.

pub mod canonical;
pub mod envelope;
pub mod error;
pub mod identity;
pub mod lifecycle;
pub mod strategy;

pub use canonical::canonical_json;
pub use envelope::{PACK_SCHEMA, PackEnvelope, PackKind, PackMetadata};
pub use error::PackError;
pub use identity::{PackIdentityConflict, PackIndex};
pub use lifecycle::{PackState, PackStateError};
pub use strategy::{StrategyMode, StrategyPack, StrategyPackPayload};
