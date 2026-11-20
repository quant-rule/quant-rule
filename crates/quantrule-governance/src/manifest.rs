//! The archive manifest.
//!
//! A manifest is the evidence boundary between archive and erase. It records
//! the source commit, every relative path, its byte count, and its SHA-256.
//! `archive_id` is the SHA-256 over the canonical file list, so a manifest can
//! be verified before any source file is removed.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The manifest schema.
pub const SCHEMA: &str = "quantrule.history-manifest/v1";

/// One file in an archive manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRecord {
    /// Path relative to the repository root.
    pub path: String,
    /// File size in bytes.
    pub bytes: u64,
    /// SHA-256 as `sha256:<hex>`.
    pub sha256: String,
}

/// A parsed archive manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Manifest schema.
    pub schema: String,
    /// Evidence kind.
    pub kind: String,
    /// Source commit, or `unknown` outside Git.
    pub source_commit: String,
    /// Creation time in Unix seconds.
    pub created_unix: u64,
    /// Deterministic digest of the file list.
    pub archive_id: String,
    /// Files in path order.
    pub files: Vec<FileRecord>,
}

/// Hash one byte slice as `sha256:<hex>`.
#[must_use]
pub fn digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{}", hex(&hasher.finalize()))
}

/// Hash a file.
///
/// # Errors
///
/// Returns the filesystem error when the file cannot be read.
pub fn digest_file(path: &Path) -> Result<String, std::io::Error> {
    let bytes = fs::read(path)?;
    Ok(digest(&bytes))
}

/// Compute the deterministic archive identity over a file list.
#[must_use]
pub fn archive_id(files: &[FileRecord]) -> String {
    let mut canonical = String::new();
    for file in files {
        canonical.push_str(&file.path);
        canonical.push('\t');
        canonical.push_str(&file.bytes.to_string());
        canonical.push('\t');
        canonical.push_str(&file.sha256);
        canonical.push('\n');
    }
    digest(canonical.as_bytes())
}

impl Manifest {
    /// Build a manifest from ordered file records.
    #[must_use]
    pub fn new(
        kind: &str,
        source_commit: String,
        created_unix: u64,
        files: Vec<FileRecord>,
    ) -> Self {
        let archive_id = archive_id(&files);
        Self {
            schema: SCHEMA.to_owned(),
            kind: kind.to_owned(),
            source_commit,
            created_unix,
            archive_id,
            files,
        }
    }

    /// Serialize the manifest as canonical pretty JSON.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "schema": self.schema,
            "kind": self.kind,
            "source_commit": self.source_commit,
            "created_unix": self.created_unix,
            "archive_id": self.archive_id,
            "files": self.files.iter().map(|file| json!({
                "path": file.path,
                "bytes": file.bytes,
                "sha256": file.sha256,
            })).collect::<Vec<_>>(),
        })
    }

    /// Parse a manifest and reject a schema or digest mismatch.
    ///
    /// # Errors
    ///
    /// Returns a human-readable error when the JSON shape or the digest is invalid.
    pub fn from_json(value: &Value) -> Result<Self, String> {
        let schema = value
            .get("schema")
            .and_then(Value::as_str)
            .ok_or_else(|| String::from("manifest schema is missing"))?;
        if schema != SCHEMA {
            return Err(format!("unsupported manifest schema {schema}"));
        }
        let kind = value
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| String::from("manifest kind is missing"))?
            .to_owned();
        let source_commit = value
            .get("source_commit")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        let created_unix = value
            .get("created_unix")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let recorded_id = value
            .get("archive_id")
            .and_then(Value::as_str)
            .ok_or_else(|| String::from("manifest archive_id is missing"))?
            .to_owned();
        let rows = value
            .get("files")
            .and_then(Value::as_array)
            .ok_or_else(|| String::from("manifest files are missing"))?;
        let mut files = Vec::with_capacity(rows.len());
        for row in rows {
            let path = row
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| String::from("manifest file path is missing"))?
                .to_owned();
            let bytes = row
                .get("bytes")
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("manifest file {path} has no byte count"))?;
            let sha256 = row
                .get("sha256")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("manifest file {path} has no hash"))?
                .to_owned();
            files.push(FileRecord {
                path,
                bytes,
                sha256,
            });
        }
        let manifest = Self {
            schema: schema.to_owned(),
            kind,
            source_commit,
            created_unix,
            archive_id: recorded_id,
            files,
        };
        if archive_id(&manifest.files) != manifest.archive_id {
            return Err(String::from(
                "manifest archive_id does not match its file list",
            ));
        }
        Ok(manifest)
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}
