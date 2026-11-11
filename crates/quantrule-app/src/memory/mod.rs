//! The local research memory.
//!
//! The memory is the application service's durable store for workspaces,
//! datasets, configurations, runs, metrics, claims, and notes. It is a local
//! SQLite database; no engine reads it, and no network path reaches it.
//!
//! Two rules keep it reproducible:
//!
//! - every timestamp is supplied by the caller, so the store never reads the
//!   clock;
//! - every query and export is ordered by stable IDs, so the same state always
//!   produces the same bytes.
//!
//! The schema identity is [`SCHEMA`]. A migration is additive and numbered; it
//! never rewrites the current version in place. The logical export and the file
//! backup are separate paths: export is JSON for portability, backup is a
//! consistent SQLite file for restore.

mod schema;
mod store;
mod transfer;
mod types;

#[cfg(test)]
mod tests;

pub use schema::{CURRENT_VERSION, SCHEMA};
pub use store::ResearchMemory;
pub use transfer::{
    EXPORT_SCHEMA, backup, export_bytes, export_json, export_to_file, import_json, restore,
};
pub use types::{
    ClaimRecord, ConfigRecord, DatasetRecord, MemoryError, MetricRecord, MetricValue, NewClaim,
    NewConfig, NewDataset, NewNote, NewRun, NewWorkspace, NoteRecord, RunRecord, WorkspaceRecord,
};
