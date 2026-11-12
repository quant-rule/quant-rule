//! Read and verify a backtest bundle's manifest and stage provenance.
//!
//! The reader owns the manifest schema, the canonical stage order, the stage
//! provenance schemas, and the FNV content-hash check. A stage consumer adds its
//! own artifact readers on top; this module never interprets a metric or a
//! verdict.
//!
//! A missing manifest, an unsupported schema, a malformed stage entry, a
//! data-version disagreement, or a provenance hash that does not match the
//! bytes on disk is a refusal. A measurement that is simply absent from a
//! present artifact is left to the consumer.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use quantrule_app::date::DateBounds;
use quantrule_app::fingerprint::fnv1a;

/// The backtest manifest schema.
pub(crate) const MANIFEST_SCHEMA: &str = "quantrule.backtest-manifest/v1";
/// The stage provenance schema for strategy, execution, portfolio, and later
/// stages.
pub(crate) const STAGE_SCHEMA: &str = "quantrule.stage-provenance/v1";
/// The existing model provenance schema.
pub(crate) const MODEL_PROVENANCE_SCHEMA: &str = "quantrule.provenance/v1";

/// The canonical stage order.
const STAGES: [&str; 4] = ["model", "strategy", "execution", "portfolio"];

/// A backtest bundle could not be read.
#[derive(Debug)]
pub(crate) enum ManifestError {
    /// A required artifact could not be read.
    Read {
        /// The artifact path.
        path: PathBuf,
        /// The underlying I/O error.
        source: io::Error,
    },
    /// A required artifact is not shaped like the documented bundle.
    Invalid {
        /// The artifact path.
        path: PathBuf,
        /// What is wrong, with the artifact's own field names.
        message: String,
    },
    /// An artifact uses a schema the reader does not support.
    UnsupportedSchema {
        /// The artifact path.
        path: PathBuf,
        /// The expected schema.
        expected: &'static str,
        /// The observed schema.
        observed: String,
    },
    /// A stage provenance file does not match the manifest's recorded hash.
    HashMismatch {
        /// The stage name.
        stage: String,
        /// The provenance path.
        path: PathBuf,
        /// The hash recorded in the manifest.
        expected: String,
        /// The hash of the bytes on disk.
        observed: String,
    },
}

impl fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "cannot read {}: {source}", path.display())
            }
            Self::Invalid { path, message } => {
                write!(
                    formatter,
                    "invalid evidence bundle {}: {message}",
                    path.display()
                )
            }
            Self::UnsupportedSchema {
                path,
                expected,
                observed,
            } => write!(
                formatter,
                "{} uses schema '{observed}', expected '{expected}'",
                path.display()
            ),
            Self::HashMismatch {
                stage,
                path,
                expected,
                observed,
            } => write!(
                formatter,
                "{} has provenance hash '{observed}', expected '{expected}' for stage {stage}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Invalid { .. } | Self::UnsupportedSchema { .. } | Self::HashMismatch { .. } => {
                None
            }
        }
    }
}

/// One stage's provenance, as recorded by the backtest manifest.
#[derive(Debug, Clone)]
pub(crate) struct StageProvenance {
    /// The stage name.
    pub(crate) stage: String,
    /// The stage's relative directory.
    pub(crate) path: String,
    /// The relative provenance path.
    pub(crate) provenance: String,
    /// The FNV-1a hash recorded in the manifest.
    pub(crate) provenance_hash: String,
    /// The provenance schema.
    pub(crate) schema: String,
    /// The input data version.
    pub(crate) data_version: String,
    /// The stage configuration identity.
    pub(crate) config_id: String,
    /// The parsed provenance value.
    pub(crate) value: Value,
}

/// A verified backtest manifest and its stage provenance.
#[derive(Debug, Clone)]
pub(crate) struct BundleManifest {
    /// The bundle root.
    pub(crate) root: PathBuf,
    /// The parsed manifest.
    pub(crate) manifest: Value,
    /// The FNV-1a hash of the manifest bytes.
    pub(crate) manifest_hash: String,
    /// The input data version.
    pub(crate) data_version: String,
    /// The run configuration identity.
    pub(crate) config_id: String,
    /// The code revision recorded by the build.
    pub(crate) code_commit: String,
    /// The explicit seed, or `null`.
    pub(crate) seed: Value,
    /// The requested period start, as written.
    pub(crate) period_from: Option<String>,
    /// The requested period end, as written.
    pub(crate) period_to: Option<String>,
    /// The four stage provenances, in canonical order.
    pub(crate) stages: Vec<StageProvenance>,
}

impl BundleManifest {
    /// Loads and validates a backtest bundle.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError`] when the manifest or a stage provenance is
    /// missing, unreadable, not JSON, carries an unsupported schema, disagrees
    /// with the manifest, or fails a recorded provenance hash.
    pub(crate) fn load(root: &Path) -> Result<Self, ManifestError> {
        let manifest_path = root.join("manifest.json");
        let (manifest, manifest_bytes) = read_json(&manifest_path)?;
        let manifest_object = as_object(&manifest_path, &manifest)?;
        let schema = required_str(&manifest_path, manifest_object, "schema")?;
        if schema != MANIFEST_SCHEMA {
            return Err(ManifestError::UnsupportedSchema {
                path: manifest_path,
                expected: MANIFEST_SCHEMA,
                observed: schema.to_owned(),
            });
        }

        let order = manifest_object
            .get("stage_order")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid(&manifest_path, "stage_order must be an array"))?;
        let order = order
            .iter()
            .map(Value::as_str)
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| invalid(&manifest_path, "stage_order must contain strings"))?;
        if order.as_slice() != STAGES {
            return Err(invalid(
                &manifest_path,
                "stage_order must be model, strategy, execution, portfolio",
            ));
        }

        let data_version =
            required_non_empty_str(&manifest_path, manifest_object, "data_version")?.to_owned();
        let config_id =
            required_non_empty_str(&manifest_path, manifest_object, "config_id")?.to_owned();
        let code_commit =
            required_non_empty_str(&manifest_path, manifest_object, "code_commit")?.to_owned();
        let seed = manifest_object
            .get("seed")
            .cloned()
            .ok_or_else(|| invalid(&manifest_path, "seed is required"))?;
        if !seed.is_null() && seed.as_u64().is_none() {
            return Err(invalid(
                &manifest_path,
                "seed must be null or a non-negative integer",
            ));
        }

        let period = manifest_object
            .get("period")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid(&manifest_path, "period must be an object"))?;
        let period_from = optional_date(&manifest_path, period, "from")?;
        let period_to = optional_date(&manifest_path, period, "to")?;
        DateBounds::parse(period_from.as_deref(), period_to.as_deref())
            .map_err(|error| invalid(&manifest_path, &format!("period is invalid: {error}")))?;

        let stages_value = manifest_object
            .get("stages")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid(&manifest_path, "stages must be an array"))?;
        if stages_value.len() != STAGES.len() {
            return Err(invalid(
                &manifest_path,
                "stages must name exactly the four pipeline stages",
            ));
        }
        let mut stages = Vec::with_capacity(STAGES.len());
        for (index, value) in stages_value.iter().enumerate() {
            let label = format!("$.stages[{index}]");
            let object = value
                .as_object()
                .ok_or_else(|| invalid(&manifest_path, &format!("{label} must be an object")))?;
            let stage = required_str(&manifest_path, object, "stage")?;
            if !STAGES.contains(&stage) {
                return Err(invalid(
                    &manifest_path,
                    &format!("{label}.stage '{stage}' is not a pipeline stage"),
                ));
            }
            if stages
                .iter()
                .any(|evidence: &StageProvenance| evidence.stage == stage)
            {
                return Err(invalid(
                    &manifest_path,
                    &format!("{label}.stage '{stage}' is duplicated"),
                ));
            }
            let path = required_str(&manifest_path, object, "path")?;
            if path != stage {
                return Err(invalid(
                    &manifest_path,
                    &format!("{label}.path must equal the stage name"),
                ));
            }
            let provenance = required_str(&manifest_path, object, "provenance")?;
            let expected_provenance = format!("{stage}/provenance.json");
            if provenance != expected_provenance {
                return Err(invalid(
                    &manifest_path,
                    &format!("{label}.provenance must be '{expected_provenance}'"),
                ));
            }
            let hash = required_str(&manifest_path, object, "provenance_hash")?;
            if hash.len() != 16 || !hash.bytes().all(is_lower_hex) {
                return Err(invalid(
                    &manifest_path,
                    &format!("{label}.provenance_hash must be 16 lowercase hex digits"),
                ));
            }

            let provenance_path = root.join(provenance);
            let (provenance_value, provenance_bytes) = read_json(&provenance_path)?;
            let observed = fnv1a(&provenance_bytes);
            if observed != hash {
                return Err(ManifestError::HashMismatch {
                    stage: stage.to_owned(),
                    path: provenance_path,
                    expected: hash.to_owned(),
                    observed,
                });
            }
            let provenance_object = as_object(&provenance_path, &provenance_value)?;
            let provenance_schema = required_str(&provenance_path, provenance_object, "schema")?;
            let expected_schema = if stage == "model" {
                MODEL_PROVENANCE_SCHEMA
            } else {
                STAGE_SCHEMA
            };
            if provenance_schema != expected_schema {
                return Err(ManifestError::UnsupportedSchema {
                    path: provenance_path,
                    expected: expected_schema,
                    observed: provenance_schema.to_owned(),
                });
            }
            if stage != "model" {
                let provenance_stage = required_str(&provenance_path, provenance_object, "stage")?;
                if provenance_stage != stage {
                    return Err(invalid(
                        &provenance_path,
                        &format!("stage is '{provenance_stage}', expected '{stage}'"),
                    ));
                }
            }
            let stage_data_version =
                required_non_empty_str(&provenance_path, provenance_object, "data_version")?;
            if stage_data_version != data_version {
                return Err(invalid(
                    &provenance_path,
                    &format!("data_version is '{stage_data_version}', expected '{data_version}'"),
                ));
            }
            let config_field = if stage == "model" {
                "config_hash"
            } else {
                "config_id"
            };
            let stage_config_id =
                required_non_empty_str(&provenance_path, provenance_object, config_field)?;
            if stage != "strategy" && stage_config_id != config_id {
                return Err(invalid(
                    &provenance_path,
                    &format!("{config_field} is '{stage_config_id}', expected '{config_id}'"),
                ));
            }
            stages.push(StageProvenance {
                stage: stage.to_owned(),
                path: path.to_owned(),
                provenance: provenance.to_owned(),
                provenance_hash: hash.to_owned(),
                schema: provenance_schema.to_owned(),
                data_version: stage_data_version.to_owned(),
                config_id: stage_config_id.to_owned(),
                value: provenance_value,
            });
        }

        Ok(Self {
            root: root.to_path_buf(),
            manifest,
            manifest_hash: fnv1a(&manifest_bytes),
            data_version,
            config_id,
            code_commit,
            seed,
            period_from,
            period_to,
            stages,
        })
    }

    /// Finds one stage provenance by name.
    #[must_use]
    pub(crate) fn stage(&self, name: &str) -> Option<&StageProvenance> {
        self.stages.iter().find(|stage| stage.stage == name)
    }

    /// Reads one JSON artifact relative to the bundle root.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::Read`] when the file cannot be read and
    /// [`ManifestError::Invalid`] when it is not JSON.
    pub(crate) fn read_json(&self, relative: &str) -> Result<Value, ManifestError> {
        let path = self.root.join(relative);
        let (value, _) = read_json(&path)?;
        Ok(value)
    }

    /// Reads one UTF-8 artifact relative to the bundle root.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::Read`] when the file cannot be read and
    /// [`ManifestError::Invalid`] when it is not UTF-8.
    pub(crate) fn read_string(&self, relative: &str) -> Result<String, ManifestError> {
        let path = self.root.join(relative);
        let bytes = std::fs::read(&path).map_err(|source| ManifestError::Read {
            path: path.clone(),
            source,
        })?;
        String::from_utf8(bytes).map_err(|error| invalid(&path, &format!("not UTF-8: {error}")))
    }
}

fn read_json(path: &Path) -> Result<(Value, Vec<u8>), ManifestError> {
    let bytes = std::fs::read(path).map_err(|source| ManifestError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let value = serde_json::from_slice(&bytes).map_err(|error| ManifestError::Invalid {
        path: path.to_path_buf(),
        message: format!("not JSON: {error}"),
    })?;
    Ok((value, bytes))
}

fn as_object<'a>(path: &Path, value: &'a Value) -> Result<&'a Map<String, Value>, ManifestError> {
    value
        .as_object()
        .ok_or_else(|| invalid(path, "the document root must be an object"))
}

fn required_str<'a>(
    path: &Path,
    object: &'a Map<String, Value>,
    name: &str,
) -> Result<&'a str, ManifestError> {
    object
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(path, &format!("{name} must be a string")))
}

fn required_non_empty_str<'a>(
    path: &Path,
    object: &'a Map<String, Value>,
    name: &str,
) -> Result<&'a str, ManifestError> {
    let value = required_str(path, object, name)?;
    if value.is_empty() {
        return Err(invalid(path, &format!("{name} must be non-empty")));
    }
    Ok(value)
}

fn optional_date(
    path: &Path,
    object: &Map<String, Value>,
    name: &str,
) -> Result<Option<String>, ManifestError> {
    match object.get(name) {
        Some(Value::Null) | None => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(invalid(
            path,
            &format!("period.{name} must be a string or null"),
        )),
    }
}

fn is_lower_hex(byte: u8) -> bool {
    byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
}

fn invalid(path: &Path, message: &str) -> ManifestError {
    ManifestError::Invalid {
        path: path.to_path_buf(),
        message: message.to_owned(),
    }
}
