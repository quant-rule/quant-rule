//! The data service.
//!
//! The service owns the `catalog` schema: dataset records, config records, the
//! read-only config mount, and the canonical dataset objects. It serves the
//! public routes the gateway proxies and the internal references other services
//! consume. It holds no profile store and no run store, so it cannot read a
//! schema it does not own.

#![forbid(unsafe_code)]

pub mod catalog;
pub mod configs;
pub mod ingest;
pub mod inspect;
pub mod migrations;
pub mod postgres;
pub mod routes;
mod rows;
pub mod state;
pub mod tolerant;
pub mod upload;

pub use catalog::{CatalogError, DatasetCatalog, MemoryCatalog};
pub use configs::{ConfigCatalog, ConfigRecord, ConfigStoreError, MemoryConfigs};
pub use routes::router;
pub use state::{DataServices, DataState};
