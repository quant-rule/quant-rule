//! Content references from a run to its packs and upstream artifacts.
//!
//! A `quantrule.run/v5` document never embeds a pack payload or a signal row.
//! It names each object by its id and content hash, and the loader is given the
//! bytes through a [`RunReferenceResolver`]. This module owns the wire shape,
//! the deterministic SHA-256 check, the narrow upstream-artifact headers, and
//! the in-memory resolver used by tests and by a caller that already holds the
//! bytes. It does not store objects, expose a command, or decide lifecycle; the
//! persistent registry is a later work item.
//!
//! Every reference carries both `id` and `hash`. They are the same content
//! address in v5, so a document that names two different values for one object
//! is refused before its bytes are read.

use std::collections::BTreeMap;

use quantrule_api::Digest;
pub use quantrule_api::{ContentReference, ReferenceKind};
use quantrule_data::Timeframe;
use serde_json::{Map, Value};

use crate::config::read;
use crate::config::{ConfigError, parse_timeframe};

use super::error::PackError;

/// The schema of a computed signal artifact.
pub const SIGNAL_ARTIFACT_SCHEMA: &str = "quantrule.signal-artifact/v1";
/// The schema of the model bundle a signal artifact is computed from.
pub const MODEL_ARTIFACT_SCHEMA: &str = "quantrule.model-bundle/v1";

/// Parses one `{ "id": …, "hash": … }` reference.
///
/// # Errors
///
/// Returns a [`ConfigError`] naming the file and JSON path when a field is
/// missing, unknown, not a SHA-256 digest, or the two values disagree.
pub fn parse_content_reference(
    file: &str,
    path: &str,
    value: &Value,
) -> Result<ContentReference, ConfigError> {
    let object = read::object(file, path, value)?;
    read::check_fields(file, path, object, &["id", "hash"])?;
    let id = digest(
        file,
        &read::child(path, "id"),
        read::field(file, path, object, "id")?,
    )?;
    let hash = digest(
        file,
        &read::child(path, "hash"),
        read::field(file, path, object, "hash")?,
    )?;
    if id != hash {
        return Err(ConfigError::inconsistent(
            file,
            path,
            "one content id and the same content hash",
            &format!("id {id}, hash {hash}"),
        ));
    }
    Ok(ContentReference::new(id, hash))
}

/// Supplies the bytes a run v5 references.
///
/// The resolver is the seam a later persistent registry implements. A caller
/// that already holds the bytes, such as a config bundle or a test, can use
/// [`MemoryReferences`] instead.
pub trait RunReferenceResolver {
    /// Returns the stored bytes for `reference`, or `None` when it is unknown.
    fn resolve(&self, kind: ReferenceKind, reference: &ContentReference) -> Option<&[u8]>;
}

/// An in-memory resolver keyed by reference kind and content hash.
///
/// The map is ordered and the key includes the kind, so the same bytes may back
/// two kinds without one lookup standing in for the other.
#[derive(Debug, Clone, Default)]
pub struct MemoryReferences {
    entries: BTreeMap<(ReferenceKind, Digest), Vec<u8>>,
}

impl MemoryReferences {
    /// Creates an empty resolver.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores bytes and returns the reference that names them.
    ///
    /// The id and hash are the SHA-256 of the bytes, so a caller cannot build a
    /// mismatched reference through this method. This suits a single stored
    /// object (a pack or a signal artifact). A model artifact is a file set;
    /// use [`Self::insert_at`] with its canonical artifact-set digest instead.
    pub fn insert(&mut self, kind: ReferenceKind, bytes: impl Into<Vec<u8>>) -> ContentReference {
        let bytes = bytes.into();
        let hash = Digest::sha256(&bytes);
        self.entries.insert((kind, hash.clone()), bytes);
        ContentReference {
            id: hash.clone(),
            hash,
        }
    }

    /// Stores bytes for an already-built reference.
    ///
    /// This exists for fixtures that exercise a missing or mismatched hash; the
    /// loader always re-hashes the stored bytes before it parses them.
    pub fn insert_at(
        &mut self,
        kind: ReferenceKind,
        reference: ContentReference,
        bytes: impl Into<Vec<u8>>,
    ) {
        self.entries
            .insert((kind, reference.hash.clone()), bytes.into());
    }

    /// Returns the stored bytes for a reference.
    #[must_use]
    pub fn get(&self, kind: ReferenceKind, reference: &ContentReference) -> Option<&[u8]> {
        self.entries
            .get(&(kind, reference.hash.clone()))
            .map(Vec::as_slice)
    }

    /// The number of stored objects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no object is stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl RunReferenceResolver for MemoryReferences {
    fn resolve(&self, kind: ReferenceKind, reference: &ContentReference) -> Option<&[u8]> {
        self.get(kind, reference)
    }
}

/// The five references a run v5 carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReferences {
    /// The upstream model bundle, when the run reuses one.
    pub model_artifact: Option<ContentReference>,
    /// The alpha definition.
    pub strategy_pack: ContentReference,
    /// The computed signal output, present only in signal mode.
    pub signal_artifact: Option<ContentReference>,
    /// The portfolio policy.
    pub portfolio_policy: ContentReference,
    /// The execution-simulation policy.
    pub execution_model: ContentReference,
}

impl RunReferences {
    /// The reference for one kind, when the run carries it.
    #[must_use]
    pub fn for_kind(&self, kind: ReferenceKind) -> Option<&ContentReference> {
        match kind {
            ReferenceKind::ModelArtifact => self.model_artifact.as_ref(),
            ReferenceKind::StrategyPack => Some(&self.strategy_pack),
            ReferenceKind::SignalArtifact => self.signal_artifact.as_ref(),
            ReferenceKind::PortfolioPolicy => Some(&self.portfolio_policy),
            ReferenceKind::ExecutionModel => Some(&self.execution_model),
        }
    }

    /// Every present reference, in the fixed provenance order.
    pub fn each(&self) -> impl Iterator<Item = (ReferenceKind, &ContentReference)> {
        [
            (ReferenceKind::ModelArtifact, self.model_artifact.as_ref()),
            (ReferenceKind::StrategyPack, Some(&self.strategy_pack)),
            (ReferenceKind::SignalArtifact, self.signal_artifact.as_ref()),
            (ReferenceKind::PortfolioPolicy, Some(&self.portfolio_policy)),
            (ReferenceKind::ExecutionModel, Some(&self.execution_model)),
        ]
        .into_iter()
        .filter_map(|(kind, reference)| reference.map(|reference| (kind, reference)))
    }

    /// The provenance value for every present reference.
    ///
    /// An absent model artifact or signal artifact is omitted, so the record
    /// says what the run used without inventing a placeholder hash.
    #[must_use]
    pub fn to_value(&self) -> Value {
        let mut object = Map::new();
        for (kind, reference) in self.each() {
            object.insert(kind.code().to_owned(), reference.to_value());
        }
        Value::Object(object)
    }

    /// The deterministic compact JSON text of the provenance value.
    #[must_use]
    pub fn to_json_text(&self) -> String {
        self.to_value().to_string()
    }
}

/// The identity fields a run v5 reads from a computed signal artifact.
///
/// The artifact owns its rows; this header is only what the run must agree with
/// before it can cite the artifact. The signal artifact writer is a later work
/// item, so the reader deliberately does not interpret a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalArtifactHeader {
    /// The strategy pack the artifact was computed from.
    pub strategy_pack_id: Digest,
    /// The model artifact the artifact was computed from.
    pub model_artifact_id: Digest,
    /// The input series identity recorded by the artifact.
    pub data_version: String,
    /// The timeframe the rows belong to.
    pub timeframe: Timeframe,
    /// The first date the artifact covers, as written.
    pub period_from: String,
    /// The last date the artifact covers, as written.
    pub period_to: String,
}

/// The identity fields a run v5 reads from a model bundle manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelArtifactHeader {
    /// The content id computed over the manifest's artifact set.
    pub artifact_id: Digest,
    /// The model registry version the manifest records.
    pub model_version: String,
    /// The manifest's computed timeframes.
    pub ladder: Vec<String>,
}

/// Resolves one reference and verifies its content hash.
///
/// # Errors
///
/// Returns a [`ConfigError`] when the resolver has no bytes for the reference,
/// or when the stored bytes hash to another value.
pub(crate) fn resolve_bytes<'a>(
    resolver: &'a dyn RunReferenceResolver,
    kind: ReferenceKind,
    reference: &ContentReference,
    file: &str,
    path: &str,
) -> Result<&'a [u8], ConfigError> {
    let bytes = resolver.resolve(kind, reference).ok_or_else(|| {
        ConfigError::missing_reference(file, path, &format!("a resolvable {kind} reference"))
    })?;
    let observed = Digest::sha256(bytes);
    if observed != reference.hash {
        return Err(ConfigError::inconsistent(
            file,
            path,
            &format!("bytes with content hash {}", reference.hash),
            &observed.to_string(),
        ));
    }
    Ok(bytes)
}

/// Maps a pack parser's error to the run reference that named the pack.
pub(crate) fn pack_error(file: &str, prefix: &str, error: PackError) -> ConfigError {
    match error {
        PackError::Document(error) => error.prefix_path(prefix),
        PackError::WrongKind { expected, found } => ConfigError::inconsistent(
            file,
            prefix,
            &format!("a {} pack", expected.code()),
            &format!("a {} pack", found.code()),
        ),
        PackError::NonCanonical { .. } => ConfigError::inconsistent(
            file,
            prefix,
            "canonical stored pack bytes",
            "non-canonical bytes",
        ),
    }
}

/// Parses a computed signal artifact's identity header.
///
/// # Errors
///
/// Returns a [`ConfigError`] when the bytes are not JSON, the schema is another
/// artifact, a required identity field is absent or mistyped, or a period or
/// timeframe is invalid. The rows are required to be an array and are not read.
pub fn parse_signal_artifact(
    bytes: &[u8],
    file: &str,
    path: &str,
) -> Result<SignalArtifactHeader, ConfigError> {
    let value = json(bytes, file, path)?;
    let object = read::object(file, path, &value)?;
    read::check_fields(
        file,
        path,
        object,
        &[
            "schema",
            "strategy_pack_id",
            "model_artifact_id",
            "data_version",
            "timeframe",
            "period",
            "rows",
        ],
    )?;
    let schema = read::string(
        file,
        &read::child(path, "schema"),
        read::field(file, path, object, "schema")?,
    )?;
    if schema != SIGNAL_ARTIFACT_SCHEMA {
        return Err(ConfigError::schema(
            file,
            &read::child(path, "schema"),
            SIGNAL_ARTIFACT_SCHEMA,
            schema,
        ));
    }
    let strategy_pack_id = digest(
        file,
        &read::child(path, "strategy_pack_id"),
        read::field(file, path, object, "strategy_pack_id")?,
    )?;
    let model_artifact_id = digest(
        file,
        &read::child(path, "model_artifact_id"),
        read::field(file, path, object, "model_artifact_id")?,
    )?;
    let data_version = read::non_empty_string(
        file,
        &read::child(path, "data_version"),
        read::field(file, path, object, "data_version")?,
    )?
    .to_owned();
    let timeframe = parse_timeframe(
        file,
        &read::child(path, "timeframe"),
        read::field(file, path, object, "timeframe")?,
    )?;
    let period_path = read::child(path, "period");
    let period = read::object(
        file,
        &period_path,
        read::field(file, path, object, "period")?,
    )?;
    read::check_fields(file, &period_path, period, &["from", "to"])?;
    let period_from = read::non_empty_string(
        file,
        &read::child(&period_path, "from"),
        read::field(file, &period_path, period, "from")?,
    )?
    .to_owned();
    let period_to = read::non_empty_string(
        file,
        &read::child(&period_path, "to"),
        read::field(file, &period_path, period, "to")?,
    )?
    .to_owned();
    let _rows = read::array(
        file,
        &read::child(path, "rows"),
        read::field(file, path, object, "rows")?,
    )?;
    Ok(SignalArtifactHeader {
        strategy_pack_id,
        model_artifact_id,
        data_version,
        timeframe,
        period_from,
        period_to,
    })
}

/// Parses a model bundle manifest's identity header.
///
/// The artifact id is the canonical digest over the manifest's `(name, sha256)`
/// entries, which is the same identity the diagnostic stage contract records.
///
/// # Errors
///
/// Returns a [`ConfigError`] when the bytes are not JSON, the schema is another
/// artifact, a required field is absent or mistyped, or the artifact set is
/// empty or names a file twice.
pub fn parse_model_artifact(
    bytes: &[u8],
    file: &str,
    path: &str,
) -> Result<ModelArtifactHeader, ConfigError> {
    let value = json(bytes, file, path)?;
    let object = read::object(file, path, &value)?;
    let schema = read::string(
        file,
        &read::child(path, "schema"),
        read::field(file, path, object, "schema")?,
    )?;
    if schema != MODEL_ARTIFACT_SCHEMA {
        return Err(ConfigError::schema(
            file,
            &read::child(path, "schema"),
            MODEL_ARTIFACT_SCHEMA,
            schema,
        ));
    }
    let model_version = read::non_empty_string(
        file,
        &read::child(path, "model_version"),
        read::field(file, path, object, "model_version")?,
    )?
    .to_owned();
    let _data_version = read::non_empty_string(
        file,
        &read::child(path, "data_version"),
        read::field(file, path, object, "data_version")?,
    )?;
    let ladder_path = read::child(path, "ladder");
    let ladder_values = read::array(
        file,
        &ladder_path,
        read::field(file, path, object, "ladder")?,
    )?;
    if ladder_values.is_empty() {
        return Err(ConfigError::range(
            file,
            &ladder_path,
            "a non-empty model ladder",
            "[]",
        ));
    }
    let mut ladder = Vec::with_capacity(ladder_values.len());
    for (index, item) in ladder_values.iter().enumerate() {
        ladder.push(
            read::non_empty_string(file, &read::index(&ladder_path, index), item)?.to_owned(),
        );
    }
    let artifacts_path = read::child(path, "artifacts");
    let artifacts = read::array(
        file,
        &artifacts_path,
        read::field(file, path, object, "artifacts")?,
    )?;
    if artifacts.is_empty() {
        return Err(ConfigError::range(
            file,
            &artifacts_path,
            "a non-empty artifact set",
            "[]",
        ));
    }
    let mut entries: Vec<(String, Digest)> = Vec::with_capacity(artifacts.len());
    for (index, item) in artifacts.iter().enumerate() {
        let item_path = read::index(&artifacts_path, index);
        let entry = read::object(file, &item_path, item)?;
        read::check_fields(file, &item_path, entry, &["name", "bytes", "sha256"])?;
        let name = read::non_empty_string(
            file,
            &read::child(&item_path, "name"),
            read::field(file, &item_path, entry, "name")?,
        )?
        .to_owned();
        let sha256 = digest(
            file,
            &read::child(&item_path, "sha256"),
            read::field(file, &item_path, entry, "sha256")?,
        )?;
        if entries.iter().any(|(existing, _)| existing == &name) {
            return Err(ConfigError::inconsistent(
                file,
                &read::child(&item_path, "name"),
                "a unique artifact name",
                &name,
            ));
        }
        entries.push((name, sha256));
    }
    let artifact_id = Digest::canonical(&entries);
    Ok(ModelArtifactHeader {
        artifact_id,
        model_version,
        ladder,
    })
}

fn json(bytes: &[u8], file: &str, path: &str) -> Result<Value, ConfigError> {
    serde_json::from_slice(bytes)
        .map_err(|error| ConfigError::invalid_json_at(file, path, &error.to_string()))
}

fn digest(file: &str, path: &str, value: &Value) -> Result<Digest, ConfigError> {
    let text = read::string(file, path, value)?;
    Digest::parse(text)
        .map_err(|error| ConfigError::range(file, path, "a sha256 digest", &error.to_string()))
}

#[cfg(test)]
mod tests;
