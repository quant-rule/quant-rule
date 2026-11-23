//! Descriptor validation by contract version.
//!
//! v2 keeps the original rules and messages. v3 adds stable `deploy.*` codes,
//! closed-schema cross-field rules, placement anti-affinity, TLS endpoint
//! checks, and the local file checks behind `validate --files`. The two rule
//! sets are separate so a v2 deployment cannot acquire a v3 rule by accident.

pub mod v2;
pub mod v3;

mod network;
mod placement;
pub(crate) mod secrets;

use crate::descriptor::Document;
use crate::error::Error;

pub use v2::{expand, validate, validate_with_secrets};

/// Validate a typed document without opening local files.
///
/// # Errors
///
/// Returns the first v2 [`Error::Invalid`] or v3 [`Error::Rule`].
pub fn validate_document(document: &Document) -> Result<(), Error> {
    match document {
        Document::V2(descriptor) => v2::validate(descriptor),
        Document::V3(descriptor) => v3::validate(descriptor),
    }
}

/// Validate a typed document and the local secret and certificate files.
///
/// v2 has no `--files` surface, so this is the same as [`validate_document`]
/// for a v2 document.
///
/// # Errors
///
/// Returns the first structural rule failure, an unreadable file, or a
/// credential rule failure.
pub fn validate_document_files(document: &Document) -> Result<(), Error> {
    match document {
        Document::V2(descriptor) => v2::validate(descriptor),
        Document::V3(descriptor) => v3::validate_files(descriptor),
    }
}
