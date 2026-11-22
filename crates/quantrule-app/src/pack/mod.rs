//! Research artifact packs: the envelope, the kind payloads, and identity.
//!
//! A pack is an immutable, versioned, content-addressed definition. The
//! envelope and the kind-specific payload are hashed; lifecycle state and the
//! registry row are metadata outside the hash. This module owns parsing,
//! validation, the canonical content hash, duplicate-label rejection, and the
//! lifecycle guards. It does not store bytes, expose a command, or compute a
//! signal; those belong to the owning service and later issues.
//!
//! Strategy and portfolio policy packs are implemented here. The
//! execution-simulation parser arrives with its own pack; the envelope already
//! recognises its kind so a parser cannot silently accept it.

pub mod canonical;
mod core;
pub mod envelope;
pub mod error;
pub mod identity;
pub mod lifecycle;
pub mod portfolio;
pub mod strategy;

pub use canonical::canonical_json;
pub use envelope::{PACK_SCHEMA, PackEnvelope, PackKind, PackMetadata};
pub use error::PackError;
pub use identity::{PackIdentity, PackIdentityConflict, PackIndex};
pub use lifecycle::{PackState, PackStateError};
pub use portfolio::{PortfolioConstraint, PortfolioPack, PortfolioPackPayload, RebalanceRule};
pub use strategy::{StrategyMode, StrategyPack, StrategyPackPayload};
