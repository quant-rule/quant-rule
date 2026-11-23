//! Descriptor versions and the version-first loader.
//!
//! A deployment descriptor is JSON. The `v` field names the contract, so the
//! loader inspects it before it parses a typed document. A v2 document stays a
//! v2 document even when it carries a field that only v3 understands: the
//! typed v2 parser rejects the field rather than silently upgrading the file.

pub mod v2;
pub mod v3;

use std::path::Path;

use serde::Deserialize;

use crate::error::Error;

pub use v2::{
    CONTRACT, Descriptor, GatewayExposure, InternalExposure, PROFILE, REQUIRED_SERVICES, Service,
    Tls, load_env, secrets_path,
};

/// One parsed deployment document.
#[derive(Debug, Clone)]
pub enum Document {
    /// A `quantrule.deploy/v2` multi-host document.
    V2(Box<v2::Descriptor>),
    /// A `quantrule.deploy/v3` Swarm HA document.
    V3(Box<v3::Descriptor>),
}

impl Document {
    /// The contract version the document declares.
    #[must_use]
    pub fn contract(&self) -> &'static str {
        match self {
            Self::V2(_) => v2::CONTRACT,
            Self::V3(_) => v3::CONTRACT,
        }
    }
}

#[derive(Deserialize)]
struct VersionProbe {
    v: String,
}

/// Read and parse a descriptor by its declared contract version.
///
/// The version is read from the raw JSON first, so a document that declares
/// v2 cannot be accepted by the v3 parser and vice versa. Unknown fields are
/// still rejected by the selected typed parser.
///
/// # Errors
///
/// Returns [`Error::Read`] when the file is absent, [`Error::Parse`] when the
/// JSON or the selected schema is malformed, and [`Error::Rule`] with
/// `deploy.version` when `v` names no supported contract.
pub fn load(path: &Path) -> Result<Document, Error> {
    let text = std::fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    parse(&text, path)
}

/// Parse descriptor text after reading its `v` field.
///
/// This is the seam the CLI and tests share; it never guesses a version from
/// the shape of the document.
///
/// # Errors
///
/// Returns the same failures as [`load`], with `path` used only for the parse
/// message.
pub fn parse(text: &str, path: &Path) -> Result<Document, Error> {
    let probe: VersionProbe = serde_json::from_str(text).map_err(|error| Error::Parse {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    match probe.v.as_str() {
        v2::CONTRACT => serde_json::from_str(text)
            .map(|descriptor| Document::V2(Box::new(descriptor)))
            .map_err(|error| parse_error(path, error)),
        v3::CONTRACT => serde_json::from_str(text)
            .map(|descriptor| Document::V3(Box::new(descriptor)))
            .map_err(|error| parse_error(path, error)),
        other => Err(Error::rule(
            "deploy.version",
            "v",
            format!(
                "unsupported contract {other}; expected {} or {}",
                v2::CONTRACT,
                v3::CONTRACT
            ),
        )),
    }
}

fn parse_error(path: &Path, error: serde_json::Error) -> Error {
    Error::Parse {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path() -> &'static Path {
        Path::new("deploy.json")
    }

    fn v2_text() -> String {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../deploy/multi-host/deploy.example.json"
        ))
        .expect("the v2 example is JSON");
        value["image"]["digest"] = serde_json::json!(format!("sha256:{}", "3".repeat(64)));
        value.to_string()
    }

    fn v3_text() -> String {
        include_str!("../../../../deploy/v3/deploy.example.json").to_owned()
    }

    #[test]
    fn the_loader_dispatches_on_the_version_field() {
        let v2 = parse(&v2_text(), path()).expect("v2 parses");
        assert!(matches!(v2, Document::V2(_)));
        assert_eq!(v2.contract(), v2::CONTRACT);

        let v3 = parse(&v3_text(), path()).expect("v3 parses");
        assert!(matches!(v3, Document::V3(_)));
        assert_eq!(v3.contract(), v3::CONTRACT);
    }

    #[test]
    fn an_unknown_contract_is_refused_with_a_stable_code() {
        let text = v2_text().replace(v2::CONTRACT, "quantrule.deploy/v9");
        let error = parse(&text, path()).expect_err("an unknown contract");
        assert!(error.to_string().contains("deploy.version"), "{error}");
        assert!(error.to_string().contains("v"), "{error}");
    }

    #[test]
    fn a_v2_document_with_a_v3_field_is_refused() {
        let mut value: serde_json::Value = serde_json::from_str(&v2_text()).expect("json");
        value["stateful"] = serde_json::json!({});
        let error = parse(&value.to_string(), path()).expect_err("a v3 field in v2");
        assert!(error.to_string().contains("unknown field"), "{error}");
    }

    #[test]
    fn a_v3_document_with_a_v2_field_is_refused() {
        let mut value: serde_json::Value = serde_json::from_str(&v3_text()).expect("json");
        value["network_gate"] = serde_json::json!("wireguard");
        let error = parse(&value.to_string(), path()).expect_err("a v2 field in v3");
        assert!(error.to_string().contains("unknown field"), "{error}");
    }

    #[test]
    fn a_missing_required_v3_field_is_refused() {
        let mut value: serde_json::Value = serde_json::from_str(&v3_text()).expect("json");
        value.as_object_mut().expect("object").remove("profile");
        let error = parse(&value.to_string(), path()).expect_err("a missing profile");
        assert!(error.to_string().contains("missing field"), "{error}");
        assert!(error.to_string().contains("profile"), "{error}");
    }

    #[test]
    fn a_published_service_port_field_is_refused() {
        let mut value: serde_json::Value = serde_json::from_str(&v3_text()).expect("json");
        value["services"]["gateway"]["public"] = serde_json::json!(true);
        let error = parse(&value.to_string(), path()).expect_err("a published port");
        assert!(error.to_string().contains("unknown field"), "{error}");
    }
}
