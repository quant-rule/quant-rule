//! The `quantrule.pack/v1` envelope.
//!
//! The envelope is the same for every pack kind: a schema tag, a kind, a human
//! version label, a name, an owner label, and the kind-specific payload. The
//! content identity covers these bytes only; lifecycle state is metadata that
//! lives outside the hash.

use serde_json::Value;

use crate::config::ConfigError;
use crate::config::read::{self, ROOT};

use super::error::PackError;

pub use quantrule_api::PackKind;

/// The pack envelope schema.
pub const PACK_SCHEMA: &str = "quantrule.pack/v1";

/// The immutable envelope fields shared by every pack kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackMetadata {
    kind: PackKind,
    version: String,
    name: String,
    owner: String,
}

impl PackMetadata {
    /// The pack kind.
    #[must_use]
    pub const fn kind(&self) -> PackKind {
        self.kind
    }

    /// The human version label, which is not the identity.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The non-empty human name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The owner label; it is not an authorisation boundary.
    #[must_use]
    pub fn owner(&self) -> &str {
        &self.owner
    }
}

/// A parsed envelope whose payload is not yet kind-validated.
#[derive(Debug, Clone, PartialEq)]
pub struct PackEnvelope {
    metadata: PackMetadata,
    payload: Value,
}

impl PackEnvelope {
    /// Parses the envelope and rejects unknown fields.
    ///
    /// The payload object is returned as written; only the parser for its kind
    /// can decide whether it is valid. The caller still owns the final content
    /// hash, so an unvalidated envelope cannot become a pack.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] naming the file and JSON path when the envelope is
    /// not an object, carries an unknown or missing field, or declares an
    /// unsupported schema or kind.
    pub fn parse(value: &Value, file: &str) -> Result<Self, PackError> {
        let object = read::object(file, ROOT, value)?;
        read::check_fields(
            file,
            ROOT,
            object,
            &["schema", "kind", "version", "name", "owner", "payload"],
        )?;
        read::schema(file, object, PACK_SCHEMA)?;

        let kind_code = read::string(file, "$.kind", read::field(file, ROOT, object, "kind")?)?;
        let kind = PackKind::from_code(kind_code).ok_or_else(|| {
            ConfigError::range(
                file,
                "$.kind",
                "strategy, portfolio or execution",
                kind_code,
            )
        })?;
        let version = read::non_empty_string(
            file,
            "$.version",
            read::field(file, ROOT, object, "version")?,
        )?
        .to_owned();
        let name =
            read::non_empty_string(file, "$.name", read::field(file, ROOT, object, "name")?)?
                .to_owned();
        let owner =
            read::non_empty_string(file, "$.owner", read::field(file, ROOT, object, "owner")?)?
                .to_owned();
        let payload = read::object(
            file,
            "$.payload",
            read::field(file, ROOT, object, "payload")?,
        )?
        .clone();

        Ok(Self {
            metadata: PackMetadata {
                kind,
                version,
                name,
                owner,
            },
            payload: Value::Object(payload),
        })
    }

    /// The envelope metadata.
    #[must_use]
    pub fn metadata(&self) -> &PackMetadata {
        &self.metadata
    }

    /// The unvalidated payload object.
    #[must_use]
    pub fn payload(&self) -> &Value {
        &self.payload
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{PACK_SCHEMA, PackEnvelope, PackKind};
    use crate::config::ConfigErrorKind;
    use crate::pack::PackError;

    fn valid() -> serde_json::Value {
        json!({
            "schema": PACK_SCHEMA,
            "kind": "strategy",
            "version": "1.0.0",
            "name": "compression-fade-5m",
            "owner": "operator",
            "payload": {"mode": "signal"}
        })
    }

    fn document_error(error: PackError) -> crate::config::ConfigError {
        match error {
            PackError::Document(error) => error,
            other => panic!("expected a document error, got {other}"),
        }
    }

    #[test]
    fn a_valid_envelope_parses() {
        let envelope = PackEnvelope::parse(&valid(), "pack.json").expect("the envelope is valid");
        assert_eq!(envelope.metadata().kind(), PackKind::Strategy);
        assert_eq!(envelope.metadata().version(), "1.0.0");
        assert_eq!(envelope.metadata().name(), "compression-fade-5m");
        assert_eq!(envelope.metadata().owner(), "operator");
        assert_eq!(envelope.payload()["mode"], json!("signal"));
    }

    #[test]
    fn every_kind_is_recognised_by_the_envelope() {
        for kind in ["strategy", "portfolio", "execution"] {
            let mut value = valid();
            value["kind"] = json!(kind);
            let envelope = PackEnvelope::parse(&value, "pack.json").expect("a known kind");
            assert_eq!(envelope.metadata().kind().code(), kind);
        }
    }

    #[test]
    fn an_unknown_envelope_field_is_rejected() {
        let mut value = valid();
        value["tenant"] = json!("x");
        let error = document_error(
            PackEnvelope::parse(&value, "pack.json").expect_err("the field is unknown"),
        );
        assert_eq!(error.kind(), ConfigErrorKind::UnknownField);
        assert_eq!(error.path(), "$.tenant");
    }

    #[test]
    fn an_unknown_schema_is_rejected() {
        let mut value = valid();
        value["schema"] = json!("quantrule.pack/v2");
        let error = document_error(
            PackEnvelope::parse(&value, "pack.json").expect_err("the schema is wrong"),
        );
        assert_eq!(error.kind(), ConfigErrorKind::Schema);
    }

    #[test]
    fn an_unknown_kind_is_rejected() {
        let mut value = valid();
        value["kind"] = json!("book");
        let error = document_error(
            PackEnvelope::parse(&value, "pack.json").expect_err("the kind is unknown"),
        );
        assert_eq!(error.kind(), ConfigErrorKind::Range);
        assert_eq!(error.path(), "$.kind");
    }

    #[test]
    fn an_empty_name_version_or_owner_is_rejected() {
        for field in ["name", "version", "owner"] {
            let mut value = valid();
            value[field] = json!("");
            let error = document_error(
                PackEnvelope::parse(&value, "pack.json").expect_err("the label is empty"),
            );
            assert_eq!(error.kind(), ConfigErrorKind::Range);
            assert_eq!(error.path(), format!("$.{field}"));
        }
    }

    #[test]
    fn a_non_object_payload_is_rejected() {
        let mut value = valid();
        value["payload"] = json!([]);
        let error = document_error(
            PackEnvelope::parse(&value, "pack.json").expect_err("the payload is not an object"),
        );
        assert_eq!(error.kind(), ConfigErrorKind::Type);
        assert_eq!(error.path(), "$.payload");
    }

    #[test]
    fn a_missing_payload_is_rejected() {
        let mut value = valid();
        value.as_object_mut().expect("an object").remove("payload");
        let error = document_error(
            PackEnvelope::parse(&value, "pack.json").expect_err("the payload is missing"),
        );
        assert_eq!(error.kind(), ConfigErrorKind::MissingField);
        assert_eq!(error.path(), "$.payload");
    }
}
