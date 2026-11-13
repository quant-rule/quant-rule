//! The data service.
//!
//! Ingestion turns uploaded bytes into a dataset identity. The parser hands the
//! raw fields to the data engine, which validates and normalizes them; the
//! canonical line of each candle is what the version hashes.

pub mod catalog;
pub mod ingest;
pub mod inspect;

pub use catalog::{CatalogError, DatasetCatalog, MemoryCatalog};
pub use ingest::{IngestError, Ingested, ingest};
pub use inspect::{InspectError, Inspection, inspect, ladder};
