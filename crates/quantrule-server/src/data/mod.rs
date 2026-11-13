//! The data service.
//!
//! Ingestion turns uploaded bytes into a dataset identity. The parser hands the
//! raw fields to the data engine, which validates and normalizes them; the
//! canonical line of each candle is what the version hashes.

pub mod ingest;

pub use ingest::{IngestError, Ingested, ingest};
