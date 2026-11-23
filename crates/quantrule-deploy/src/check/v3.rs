//! The `quantrule.deploy/v3` check: local invariants plus one HTTPS ingress probe.
//!
//! The default check needs no Docker. It validates the descriptor and placement,
//! reports the pinned image and the secret names and paths, and probes
//! `https://<dns>/health`, `/ready`, and `/version` through a [`Probe`]. A test
//! supplies a recorded [`FixtureProbe`] through `QR_DEPLOY_PROBE_FIXTURE`; the
//! production path uses [`HttpProbe`] with the cert-mode CA or the system roots.
//! `--remote docker` and `--overlay` are explicit opt-ins owned by PLAT-001.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::Deserialize;
use ureq::tls::{Certificate, RootCerts, TlsConfig};

use crate::descriptor::v3::{Descriptor, IngressTls};
use crate::error::Error;
use crate::render::{app_image, evidence_tier};

use super::local;
use super::remote;
use super::report::{CheckReport, CheckRow};

/// What the check is allowed to open or inspect.
#[derive(Debug, Clone, Copy, Default)]
pub struct CheckOptions {
    /// Run the opt-in Docker inspector.
    pub remote_docker: bool,
    /// Run the opt-in overlay inspector.
    pub overlay: bool,
}

/// One HTTP response from the ingress.
#[derive(Debug, Clone)]
pub struct ProbeResponse {
    /// The HTTP status.
    pub status: u16,
    /// The response body.
    pub body: Vec<u8>,
}

/// The ingress probe seam.
pub trait Probe {
    /// Perform one GET request.
    ///
    /// # Errors
    ///
    /// Returns a transport, TLS, or fixture error as display text. It never
    /// carries a secret.
    fn get(&self, url: &str) -> Result<ProbeResponse, String>;
}

/// The production HTTPS probe.
struct HttpProbe {
    agent: ureq::Agent,
}

impl HttpProbe {
    fn new(descriptor: &Descriptor) -> Result<Self, Error> {
        let mut builder =
            ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(5)));
        if descriptor.ingress.tls == IngressTls::Cert {
            let path = &descriptor.endpoints.database.tls_ca;
            let pem = std::fs::read(path).map_err(|source| Error::Read {
                path: std::path::PathBuf::from(path),
                source,
            })?;
            let certificate = Certificate::from_pem(&pem).map_err(|error| Error::Invalid {
                field: "endpoints.database.tls_ca".to_owned(),
                message: format!("cannot parse the ingress CA bundle: {error}"),
            })?;
            builder = builder.tls_config(
                TlsConfig::builder()
                    .root_certs(RootCerts::from(vec![certificate]))
                    .build(),
            );
        }
        Ok(Self {
            agent: ureq::Agent::new_with_config(builder.build()),
        })
    }
}

impl Probe for HttpProbe {
    fn get(&self, url: &str) -> Result<ProbeResponse, String> {
        let response = self
            .agent
            .get(url)
            .call()
            .map_err(|error| error.to_string())?;
        let status = response.status().as_u16();
        let body = response
            .into_body()
            .read_to_vec()
            .map_err(|error| error.to_string())?;
        Ok(ProbeResponse { status, body })
    }
}

/// A recorded probe result, selected only when `QR_DEPLOY_PROBE_FIXTURE` names
/// a JSON file. This is the deterministic test seam; it is not evidence.
struct FixtureProbe {
    responses: BTreeMap<String, FixtureEntry>,
}

#[derive(Debug, Deserialize)]
struct FixtureEntry {
    #[serde(default)]
    status: u16,
    #[serde(default)]
    body: String,
    #[serde(default)]
    error: Option<String>,
}

impl Probe for FixtureProbe {
    fn get(&self, url: &str) -> Result<ProbeResponse, String> {
        let entry = self
            .responses
            .get(url)
            .ok_or_else(|| format!("the probe fixture has no response for {url}"))?;
        if let Some(error) = &entry.error {
            return Err(error.clone());
        }
        Ok(ProbeResponse {
            status: entry.status,
            body: entry.body.as_bytes().to_vec(),
        })
    }
}

/// Check a v3 descriptor.
///
/// # Errors
///
/// Returns [`Error::Rule`] when the descriptor itself is invalid, or
/// [`Error::Invalid`] when the CA or probe fixture cannot be read. Per-service
/// failures are rows so one bad probe does not hide the others.
pub fn check(descriptor: &Descriptor, options: CheckOptions) -> Result<CheckReport, Error> {
    let docker = remote::CommandDocker::from_env();
    check_with(descriptor, options, &docker)
}

/// Check a v3 descriptor against an explicit Docker query source.
///
/// The production path uses [`remote::CommandDocker`]; tests inject a fake so
/// the fast tier never needs a daemon. The Docker source is only touched when
/// the operator opted in.
///
/// # Errors
///
/// Returns [`Error::Rule`] when the descriptor itself is invalid, or
/// [`Error::Invalid`] when the CA or probe fixture cannot be read. A failed
/// remote row is part of the report, not an error.
pub fn check_with(
    descriptor: &Descriptor,
    options: CheckOptions,
    docker: &dyn remote::DockerQuery,
) -> Result<CheckReport, Error> {
    local::validate(descriptor)?;
    let mut report = CheckReport::new(
        descriptor.profile.as_str(),
        evidence_tier(descriptor.profile),
        app_image(descriptor),
    );
    local::add(&mut report, descriptor);
    match probe(descriptor) {
        Ok(probe) => probe_ingress(&mut report, descriptor, probe.as_ref()),
        Err(error) => report.push(CheckRow::failed(
            "ingress",
            descriptor.ingress.dns.clone(),
            "probe-unavailable",
            error.to_string(),
        )),
    }
    if options.remote_docker {
        remote::append_docker(&mut report, descriptor, docker);
    }
    if options.overlay {
        remote::append_overlay(&mut report, descriptor, docker);
    }
    Ok(report)
}

fn probe(descriptor: &Descriptor) -> Result<Box<dyn Probe>, Error> {
    if let Ok(path) = std::env::var("QR_DEPLOY_PROBE_FIXTURE") {
        let text = std::fs::read_to_string(&path).map_err(|source| Error::Read {
            path: std::path::PathBuf::from(&path),
            source,
        })?;
        let responses: BTreeMap<String, FixtureEntry> =
            serde_json::from_str(&text).map_err(|error| Error::Invalid {
                field: "QR_DEPLOY_PROBE_FIXTURE".to_owned(),
                message: error.to_string(),
            })?;
        return Ok(Box::new(FixtureProbe { responses }));
    }
    Ok(Box::new(HttpProbe::new(descriptor)?))
}

fn probe_ingress(report: &mut CheckReport, descriptor: &Descriptor, probe: &dyn Probe) {
    let base = format!("https://{}", descriptor.ingress.dns);
    for (path, kind) in [
        ("/health", "health"),
        ("/ready", "ready"),
        ("/version", "version"),
    ] {
        let url = format!("{base}{path}");
        match probe.get(&url) {
            Err(error) => report.push(CheckRow::failed("ingress", url, kind, error)),
            Ok(response) if !(200..300).contains(&response.status) => report.push(
                CheckRow::failed("ingress", url, kind, format!("HTTP {}", response.status)),
            ),
            Ok(response) if kind == "version" => {
                match parse_version(&response.body, &descriptor.image.digest) {
                    Ok(()) => {
                        let mut row = CheckRow::ok("ingress", url, kind, "digest matches");
                        row.digest = Some(descriptor.image.digest.clone());
                        report.push(row);
                    }
                    Err(error) => report.push(CheckRow::failed("ingress", url, kind, error)),
                }
            }
            Ok(response) => match parse_health(&response.body) {
                Ok(state) if state == "healthy" => {
                    report.push(CheckRow::ok("ingress", url, kind, state));
                }
                Ok(state) => {
                    report.push(CheckRow::failed(
                        "ingress",
                        url,
                        kind,
                        format!("state is {state}"),
                    ));
                }
                Err(error) => report.push(CheckRow::failed("ingress", url, kind, error)),
            },
        }
    }
}

#[derive(Deserialize)]
struct VersionBody {
    #[serde(default)]
    image_digest: Option<String>,
}

fn parse_version(body: &[u8], expected: &str) -> Result<(), String> {
    let version: VersionBody =
        serde_json::from_slice(body).map_err(|error| format!("invalid JSON: {error}"))?;
    match version.image_digest.as_deref() {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(format!("image digest {actual} does not match {expected}")),
        None => Err("version response has no image_digest".to_owned()),
    }
}

#[derive(Deserialize)]
struct HealthBody {
    state: String,
}

fn parse_health(body: &[u8]) -> Result<String, String> {
    let health: HealthBody =
        serde_json::from_slice(body).map_err(|error| format!("invalid JSON: {error}"))?;
    Ok(health.state)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor() -> Descriptor {
        serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
            .expect("v3 example")
    }

    struct Fake {
        responses: BTreeMap<String, Result<ProbeResponse, String>>,
    }

    impl Probe for Fake {
        fn get(&self, url: &str) -> Result<ProbeResponse, String> {
            self.responses
                .get(url)
                .cloned()
                .unwrap_or_else(|| Err(format!("no fixture for {url}")))
        }
    }

    fn healthy_fixture(descriptor: &Descriptor) -> Fake {
        let base = format!("https://{}", descriptor.ingress.dns);
        Fake {
            responses: BTreeMap::from([
                (
                    format!("{base}/health"),
                    Ok(ProbeResponse {
                        status: 200,
                        body: br#"{"state":"healthy"}"#.to_vec(),
                    }),
                ),
                (
                    format!("{base}/ready"),
                    Ok(ProbeResponse {
                        status: 200,
                        body: br#"{"state":"healthy"}"#.to_vec(),
                    }),
                ),
                (
                    format!("{base}/version"),
                    Ok(ProbeResponse {
                        status: 200,
                        body: format!(r#"{{"image_digest":"{}"}}"#, descriptor.image.digest)
                            .into_bytes(),
                    }),
                ),
            ]),
        }
    }

    fn report_with(probe: &Fake, descriptor: &Descriptor) -> CheckReport {
        let mut report = CheckReport::new(
            descriptor.profile.as_str(),
            evidence_tier(descriptor.profile),
            app_image(descriptor),
        );
        probe_ingress(&mut report, descriptor, probe);
        report
    }

    #[test]
    fn a_healthy_ingress_passes_and_reports_the_digest() {
        let descriptor = descriptor();
        let report = report_with(&healthy_fixture(&descriptor), &descriptor);
        assert!(!report.failed(), "{}", report.render());
        assert!(report.render().contains(&descriptor.image.digest));
    }

    #[test]
    fn a_digest_mismatch_fails() {
        let descriptor = descriptor();
        let mut fixture = healthy_fixture(&descriptor);
        fixture.responses.insert(
            format!("https://{}/version", descriptor.ingress.dns),
            Ok(ProbeResponse {
                status: 200,
                body: br#"{"image_digest":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}"#.to_vec(),
            }),
        );
        let report = report_with(&fixture, &descriptor);
        assert!(report.failed());
        assert!(report.render().contains("does not match"));
    }

    #[test]
    fn an_unhealthy_state_fails() {
        let descriptor = descriptor();
        let mut fixture = healthy_fixture(&descriptor);
        fixture.responses.insert(
            format!("https://{}/ready", descriptor.ingress.dns),
            Ok(ProbeResponse {
                status: 200,
                body: br#"{"state":"down"}"#.to_vec(),
            }),
        );
        let report = report_with(&fixture, &descriptor);
        assert!(report.failed());
        assert!(report.render().contains("state is down"));
    }

    #[test]
    fn an_unreachable_probe_fails_without_hiding_the_other_rows() {
        let descriptor = descriptor();
        let fixture = Fake {
            responses: BTreeMap::from([(
                format!("https://{}/health", descriptor.ingress.dns),
                Err("connection refused".to_owned()),
            )]),
        };
        let report = report_with(&fixture, &descriptor);
        assert!(report.failed());
        assert!(report.render().contains("connection refused"));
    }

    #[test]
    fn a_lab_profile_prints_simulated_ha_and_never_real_ha() {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
                .expect("example");
        value["profile"] = serde_json::json!("lab-single-host");
        let descriptor: Descriptor = serde_json::from_value(value).expect("descriptor");
        let report = report_with(&healthy_fixture(&descriptor), &descriptor);
        let text = report.render();
        assert!(text.contains("Simulated HA"), "{text}");
        assert!(!text.contains("Real HA"), "{text}");
    }
}
