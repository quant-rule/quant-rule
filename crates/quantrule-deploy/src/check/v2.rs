//! The `quantrule.deploy/v2` checker, unchanged.
//!
//! Every HTTP service exposes `/version` and `/health`; the gateway also
//! exposes `/ready`. The tool compares the reported image digest with the
//! descriptor and requires a healthy state. The backtest worker has no HTTP
//! surface, so the operator checks its container on the compute host.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::Deserialize;
use ureq::tls::{Certificate, RootCerts, TlsConfig};

use crate::descriptor::Descriptor;
use crate::error::Error;
use crate::validate::{expand, validate_with_secrets};

/// One service check row.
#[derive(Debug, Clone)]
pub struct CheckRow {
    /// The service name.
    pub service: String,
    /// The base URL that was probed.
    pub url: String,
    /// The observed state, or `not-http` for the worker.
    pub state: String,
    /// The observed image digest.
    pub digest: Option<String>,
    /// A failure detail, or `None` on success.
    pub error: Option<String>,
}

impl CheckRow {
    /// Whether the row passed.
    #[must_use]
    pub fn ok(&self) -> bool {
        self.state != "not-http" && self.error.is_none()
    }

    /// Whether the row could not be probed and is reported as skipped.
    #[must_use]
    pub fn skipped(&self) -> bool {
        self.state == "not-http"
    }
}

#[derive(Debug, Deserialize)]
struct Version {
    #[serde(default)]
    image_digest: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Health {
    state: String,
}

/// Probe every HTTP service in the descriptor.
///
/// # Errors
///
/// Returns [`Error::Invalid`] or [`Error::MissingSecret`] when the descriptor
/// cannot be expanded. Per-service failures are returned as rows with an
/// `error`, so one unreachable host does not hide the others.
pub fn check(
    descriptor: &Descriptor,
    secrets: &BTreeMap<String, String>,
) -> Result<Vec<CheckRow>, Error> {
    validate_with_secrets(descriptor, secrets)?;
    let timeout = Duration::from_secs(5);
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let mut rows = Vec::new();
    for name in ["gateway", "data", "model", "evidence", "simulation"] {
        let service = &descriptor.services[name];
        let host = &descriptor.hosts[&service.host];
        let Some(port) = service.port else {
            continue;
        };
        let base = format!("http://{}:{port}", host.address);
        rows.push(check_one(
            &agent,
            name,
            &base,
            &descriptor.image.digest,
            name == "gateway",
        ));
    }
    let backtest = &descriptor.services["backtest"];
    let backtest_host = &descriptor.hosts[&backtest.host];
    rows.push(CheckRow {
        service: "backtest".to_owned(),
        url: backtest_host.address.clone(),
        state: "not-http".to_owned(),
        digest: None,
        error: None,
    });
    rows.push(check_object_store(descriptor, secrets)?);
    Ok(rows)
}

/// Probe the object store over the scheme the descriptor names.
///
/// The endpoint is `https://` in the deployed profile, so the probe trusts the
/// descriptor's CA bundle instead of the platform roots. The public gateway
/// probe stays plain `http`: ingress TLS belongs to the deployed proxy, not to
/// the internal HTTP connector.
fn check_object_store(
    descriptor: &Descriptor,
    secrets: &BTreeMap<String, String>,
) -> Result<CheckRow, Error> {
    let endpoint = expand(&descriptor.endpoints.object_store, secrets)?;
    let url = format!("{}/minio/health/live", endpoint.trim_end_matches('/'));
    let mut row = CheckRow {
        service: "object_store".to_owned(),
        url: url.clone(),
        state: "unknown".to_owned(),
        digest: None,
        error: None,
    };
    let mut config = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(5)));
    if endpoint.starts_with("https://") {
        let pem = match std::fs::read(&descriptor.endpoints.object_store_tls_ca) {
            Ok(pem) => pem,
            Err(error) => {
                row.error = Some(format!(
                    "cannot read {}: {error}",
                    descriptor.endpoints.object_store_tls_ca
                ));
                return Ok(row);
            }
        };
        let certificate = match Certificate::from_pem(&pem) {
            Ok(certificate) => certificate,
            Err(error) => {
                row.error = Some(format!("invalid object-store CA bundle: {error}"));
                return Ok(row);
            }
        };
        config = config.tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::from(vec![certificate]))
                .build(),
        );
    }
    let agent = ureq::Agent::new_with_config(config.build());
    match agent.get(&url).call() {
        Ok(response) if (200..300).contains(&response.status().as_u16()) => {
            row.state = "healthy".to_owned();
        }
        Ok(response) => {
            row.state = "error".to_owned();
            row.error = Some(format!("HTTP {}", response.status().as_u16()));
        }
        Err(error) => {
            row.state = "error".to_owned();
            row.error = Some(error.to_string());
        }
    }
    Ok(row)
}

fn check_one(
    agent: &ureq::Agent,
    service: &str,
    base: &str,
    expected_digest: &str,
    ready: bool,
) -> CheckRow {
    let version_url = format!("{base}/version");
    let health_url = format!("{base}/{}", if ready { "ready" } else { "health" });
    let mut row = CheckRow {
        service: service.to_owned(),
        url: base.to_owned(),
        state: "unknown".to_owned(),
        digest: None,
        error: None,
    };
    let version: Version = match get_json(agent, &version_url) {
        Ok(version) => version,
        Err(error) => {
            row.error = Some(error);
            return row;
        }
    };
    row.digest = version.image_digest.clone();
    if version.image_digest.as_deref() != Some(expected_digest) {
        row.error = Some(format!(
            "image digest {} does not match {expected_digest}",
            version.image_digest.as_deref().unwrap_or("missing")
        ));
    }
    let health: Health = match get_json(agent, &health_url) {
        Ok(health) => health,
        Err(error) => {
            row.error = Some(error);
            return row;
        }
    };
    row.state = health.state.clone();
    if health.state != "healthy"
        && let Some(error) = row.error.as_mut()
    {
        error.push_str("; ");
        error.push_str(&format!("state is {}", health.state));
    } else if health.state != "healthy" {
        row.error = Some(format!("state is {}", health.state));
    }
    row
}

fn get_json<T: for<'de> Deserialize<'de>>(agent: &ureq::Agent, url: &str) -> Result<T, String> {
    let response = agent.get(url).call().map_err(|error| error.to_string())?;
    let status = response.status().as_u16();
    let body = response
        .into_body()
        .read_to_vec()
        .map_err(|error| error.to_string())?;
    if !(200..300).contains(&status) {
        return Err(format!("HTTP {status}"));
    }
    serde_json::from_slice(&body).map_err(|error| format!("invalid JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_and_health_parse_the_contract_subset() {
        let version: Version = serde_json::from_str(
            r#"{"product":"1.3.0","api":"quantrule.api/v1","image_digest":"sha256:aa"}"#,
        )
        .expect("version");
        assert_eq!(version.image_digest.as_deref(), Some("sha256:aa"));
        let health: Health = serde_json::from_str(r#"{"state":"healthy"}"#).expect("health");
        assert_eq!(health.state, "healthy");
    }
}

#[cfg(test)]
mod row_tests {
    use super::*;

    fn row(state: &str, error: Option<&str>) -> CheckRow {
        CheckRow {
            service: "service".to_owned(),
            url: "http://127.0.0.1:1".to_owned(),
            state: state.to_owned(),
            digest: None,
            error: error.map(str::to_owned),
        }
    }

    #[test]
    fn a_worker_row_is_skipped_not_ok() {
        let row = row("not-http", None);
        assert!(row.skipped());
        assert!(!row.ok());
    }

    #[test]
    fn a_healthy_row_is_ok_and_a_failure_is_not() {
        assert!(row("healthy", None).ok());
        assert!(!row("healthy", Some("mismatch")).ok());
    }
}
