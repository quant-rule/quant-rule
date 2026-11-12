//! Read one stage bundle's `provenance.json`.
//!
//! The reader accepts the model bundle's `quantrule.provenance/v1` envelope and
//! the new `quantrule.stage-provenance/v1` envelope. It returns the identity
//! fields every stage needs: the data version, the configuration identity, and
//! the requested period. It performs no stage computation and writes nothing.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The model bundle's provenance schema.
pub const MODEL_PROVENANCE_SCHEMA: &str = "quantrule.provenance/v1";
/// The stage bundle's provenance schema.
pub const STAGE_PROVENANCE_SCHEMA: &str = "quantrule.stage-provenance/v1";

/// The identity fields a downstream stage reads from a provenance file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageProvenance {
    /// The identity of the input candle series.
    pub data_version: String,
    /// The stable configuration identity.
    pub config_id: String,
    /// The requested period start, as written.
    pub period_from: Option<String>,
    /// The requested period end, as written.
    pub period_to: Option<String>,
}

/// A provenance file could not be read or is not shaped like its contract.
#[derive(Debug)]
pub enum ProvenanceError {
    /// The file could not be read.
    Read {
        /// The artifact path.
        path: PathBuf,
        /// The underlying I/O error.
        source: io::Error,
    },
    /// The file is not shaped like the documented provenance.
    Invalid {
        /// The artifact path.
        path: PathBuf,
        /// What is wrong, using the artifact's own field names.
        message: String,
    },
}

impl fmt::Display for ProvenanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "cannot read {}: {source}", path.display())
            }
            Self::Invalid { path, message } => {
                write!(formatter, "invalid provenance {}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for ProvenanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Invalid { .. } => None,
        }
    }
}

/// Reads the model bundle's `quantrule.provenance/v1` identity.
///
/// # Errors
///
/// Returns [`ProvenanceError`] when the file is missing, unreadable, not JSON,
/// not an object, names another report, or omits an identity field.
pub fn load_model(path: &Path) -> Result<StageProvenance, ProvenanceError> {
    let object = read_object(path)?;
    let report = object.get("report").and_then(Value::as_str).unwrap_or("");
    if report != "model" {
        return Err(invalid(
            path,
            &format!("expected report 'model', found '{report}'"),
        ));
    }
    let data_version = required_non_empty(path, &object, "data_version")?;
    let config_id = object
        .get("config_hash")
        .or_else(|| object.get("config_id"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid(path, "config_hash is missing or empty"))?
        .to_owned();
    let (period_from, period_to) = period(path, object.get("requested_period"))?;
    Ok(StageProvenance {
        data_version,
        config_id,
        period_from,
        period_to,
    })
}

/// Reads a `quantrule.stage-provenance/v1` identity for one stage artifact.
///
/// # Errors
///
/// Returns [`ProvenanceError`] when the file is missing, unreadable, not JSON,
/// not an object, names another schema/stage/artifact, or omits an identity
/// field.
pub fn load_stage(
    path: &Path,
    stage: &str,
    artifact: &str,
) -> Result<StageProvenance, ProvenanceError> {
    let object = read_object(path)?;
    let schema = object.get("schema").and_then(Value::as_str).unwrap_or("");
    if schema != STAGE_PROVENANCE_SCHEMA {
        return Err(invalid(
            path,
            &format!("expected schema '{STAGE_PROVENANCE_SCHEMA}', found '{schema}'"),
        ));
    }
    let observed_stage = object.get("stage").and_then(Value::as_str).unwrap_or("");
    if observed_stage != stage {
        return Err(invalid(
            path,
            &format!("expected stage '{stage}', found '{observed_stage}'"),
        ));
    }
    let observed_artifact = object.get("artifact").and_then(Value::as_str).unwrap_or("");
    if observed_artifact != artifact {
        return Err(invalid(
            path,
            &format!("expected artifact '{artifact}', found '{observed_artifact}'"),
        ));
    }
    let data_version = required_non_empty(path, &object, "data_version")?;
    let config_id = required_non_empty(path, &object, "config_id")?;
    let (period_from, period_to) = period(path, object.get("period"))?;
    Ok(StageProvenance {
        data_version,
        config_id,
        period_from,
        period_to,
    })
}

/// Reads one UTF-8 JSON artifact.
///
/// # Errors
///
/// Returns [`ProvenanceError::Read`] when the file cannot be read and
/// [`ProvenanceError::Invalid`] when it is not a JSON object.
pub fn read_json_object(path: &Path) -> Result<serde_json::Map<String, Value>, ProvenanceError> {
    read_object(path)
}

/// Reads one UTF-8 text artifact.
///
/// # Errors
///
/// Returns [`ProvenanceError::Read`] when the file cannot be read and
/// [`ProvenanceError::Invalid`] when it is not UTF-8.
pub fn read_text(path: &Path) -> Result<String, ProvenanceError> {
    let bytes = fs::read(path).map_err(|source| ProvenanceError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    String::from_utf8(bytes)
        .map_err(|error| invalid(path, &format!("not UTF-8: {error}")))
}

fn read_object(path: &Path) -> Result<serde_json::Map<String, Value>, ProvenanceError> {
    let text = read_text(path)?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| invalid(path, &format!("not JSON: {error}")))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| invalid(path, "the provenance root is not an object"))
}

fn required_non_empty(
    path: &Path,
    object: &serde_json::Map<String, Value>,
    name: &str,
) -> Result<String, ProvenanceError> {
    object
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| invalid(path, &format!("{name} is missing or empty")))
}

fn period(
    path: &Path,
    value: Option<&Value>,
) -> Result<(Option<String>, Option<String>), ProvenanceError> {
    let Some(period) = value else {
        return Ok((None, None));
    };
    let object = period
        .as_object()
        .ok_or_else(|| invalid(path, "the period is not an object"))?;
    let from = object.get("from").and_then(Value::as_str).map(str::to_owned);
    let to = object.get("to").and_then(Value::as_str).map(str::to_owned);
    Ok((from, to))
}

fn invalid(path: &Path, message: &str) -> ProvenanceError {
    ProvenanceError::Invalid {
        path: path.to_path_buf(),
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn write(json: &str) -> PathBuf {
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "quantrule-provenance-{}-{sequence}.json",
            std::process::id()
        ));
        fs::write(&path, json).expect("the json is written");
        path
    }

    fn cleanup(path: PathBuf) {
        let _ = fs::remove_file(path);
    }

    #[test]
    fn reads_the_model_provenance() {
        let path = write(
            r#"{"report":"model","data_version":"v1","config_hash":"h1","requested_period":{"from":"2025-01-01","to":"2025-12-31"}}"#,
        );
        let provenance = load_model(&path).expect("the provenance loads");
        assert_eq!(provenance.data_version, "v1");
        assert_eq!(provenance.config_id, "h1");
        assert_eq!(provenance.period_from.as_deref(), Some("2025-01-01"));
        cleanup(path);
    }

    #[test]
    fn reads_the_stage_provenance() {
        let path = write(
            r#"{"schema":"quantrule.stage-provenance/v1","stage":"strategy","artifact":"decisions","data_version":"v1","config_id":"h1"}"#,
        );
        let provenance = load_stage(&path, "strategy", "decisions").expect("the provenance loads");
        assert_eq!(provenance.data_version, "v1");
        assert_eq!(provenance.config_id, "h1");
        cleanup(path);
    }

    #[test]
    fn rejects_the_wrong_stage_and_artifact() {
        let path = write(
            r#"{"schema":"quantrule.stage-provenance/v1","stage":"execution","artifact":"fills","data_version":"v1","config_id":"h1"}"#,
        );
        let error = load_stage(&path, "strategy", "decisions").expect_err("the stage is wrong");
        assert!(error.to_string().contains("expected stage 'strategy'"));
        cleanup(path);
    }

    #[test]
    fn rejects_a_missing_identity_field() {
        let path = write(
            r#"{"schema":"quantrule.stage-provenance/v1","stage":"strategy","artifact":"decisions","data_version":"v1"}"#,
        );
        let error = load_stage(&path, "strategy", "decisions").expect_err("the config is missing");
        assert!(error.to_string().contains("config_id is missing or empty"));
        cleanup(path);
    }
}
