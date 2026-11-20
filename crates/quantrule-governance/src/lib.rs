//! Repository governance tools.
//!
//! The crate is developer tooling, not a product client. It owns historical
//! repository evidence retention: list, archive, and erase with a manifest.
//! It never reads the product services and never touches a runtime record.

#![forbid(unsafe_code)]
pub mod history;
pub mod manifest;
