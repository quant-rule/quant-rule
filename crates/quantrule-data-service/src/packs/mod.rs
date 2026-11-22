//! The persistent research pack registry.
//!
//! A pack is an immutable, versioned, content-addressed document. The catalog
//! owns its metadata row, validation records and lifecycle events; the object
//! store is the single authority for the canonical bytes, which are written
//! under `packs/<hash>`. The row records the object key, hash and size and is
//! what a client lists and what a run cites.
//!
//! Lifecycle is metadata outside the content hash: `draft`, `validated`,
//! `frozen`, `retired`. Freeze and retire are explicit operations. A retired
//! pack is never deleted; it stays readable for runs that already cite it.
//! The signal artifact writer and the client commands are later work items.

pub mod memory;
mod parse;
pub mod postgres;
pub mod routes;
pub mod store;

pub use memory::MemoryPacks;
pub use parse::{PackParseError, ParsedPack, parse_pack};
pub use postgres::PgPacks;
pub use routes::routes;
pub use store::{NewPack, PackFilter, PackRegistry, PackRegistryError, RegisterOutcome};
