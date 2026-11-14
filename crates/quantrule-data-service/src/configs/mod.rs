//! The configuration catalog.
//!
//! A catalog entry is a **run config**: the composition root that names the
//! data, model, execution, and portfolio files. The repository mount is the
//! read-only source; the catalog stores the body, the validity, and a content
//! hash so a run can refer to the exact configuration.

pub mod scan;
pub mod store;
pub mod upload;

pub use scan::{ScanError, scan};
pub use store::{ConfigCatalog, ConfigRecord, ConfigStoreError, MemoryConfigs};
pub use upload::{UploadError, build};
