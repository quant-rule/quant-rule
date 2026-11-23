//! The `quantrule.deploy/v3` check: local invariants plus one HTTPS ingress probe.
//!
//! The default check needs no Docker. It validates the descriptor and placement,
//! reports the pinned image, the secret names and paths, and the stateful
//! config/PKI rows, and probes
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

use super::ingress;
use super::local;
use super::remote;
use super::report::{CheckReport, CheckRow};

/// The recorded-fixture marker: a missing fixture entry is not a transport error.
pub(crate) const FIXTURE_MISSING: &str = "fixture-missing:";

/// What the check is allowed to open or inspect.
#[derive(Debug, Clone, Default)]
pub struct CheckOptions {
    /// Run the opt-in Docker inspector.
    pub remote_docker: bool,
    /// Run the opt-in overlay inspector.
    pub overlay: bool,
    /// Read this rendered tree for the stateful rows instead of rendering in
    /// memory. `QR_DEPLOY_OUT` supplies the same path on the command line.
    pub out: Option<std::path::PathBuf>,
}

/// One HTTP response from the ingress.
#[derive(Debug, Clone)]
pub struct ProbeResponse {
    /// The HTTP status.
    pub status: u16,
    /// The response headers, with lower-case names.
    pub headers: BTreeMap<String, String>,
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
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_ascii_lowercase(),
                    value.to_str().unwrap_or_default().to_owned(),
                )
            })
            .collect();
        let body = response
            .into_body()
            .read_to_vec()
            .map_err(|error| error.to_string())?;
        Ok(ProbeResponse {
            status,
            headers,
            body,
        })
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
    headers: BTreeMap<String, String>,
    #[serde(default)]
    body: String,
    #[serde(default)]
    error: Option<String>,
}

impl Probe for FixtureProbe {
    fn get(&self, url: &str) -> Result<ProbeResponse, String> {
        let entry = self.responses.get(url).ok_or_else(|| {
            format!("{FIXTURE_MISSING} the recorded probe fixture has no response for {url}")
        })?;
        if let Some(error) = &entry.error {
            return Err(error.clone());
        }
        Ok(ProbeResponse {
            status: entry.status,
            headers: entry
                .headers
                .iter()
                .map(|(name, value)| (name.to_ascii_lowercase(), value.clone()))
                .collect(),
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
    let certificates = ingress::CommandCertificate::from_env();
    super::stateful::append(
        &mut report,
        descriptor,
        options.out.as_deref(),
        &certificates,
    );
    for row in ingress::local_rows(descriptor) {
        report.push(row);
    }
    match probe(descriptor) {
        Ok(probe) => ingress::probe_rows(&mut report, descriptor, probe.as_ref()),
        Err(error) => report.push(CheckRow::failed(
            "ingress",
            descriptor.ingress.dns.clone(),
            "probe-unavailable",
            error.to_string(),
        )),
    }
    if options.remote_docker {
        remote::append_docker(&mut report, descriptor, docker);
        for row in ingress::remote_rows(descriptor, docker) {
            report.push(row);
        }
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
                .unwrap_or_else(|| Err(format!("{FIXTURE_MISSING} no fixture for {url}")))
        }
    }

    fn secure_headers() -> BTreeMap<String, String> {
        BTreeMap::from([
            (
                "strict-transport-security".to_owned(),
                "max-age=31536000; includeSubDomains".to_owned(),
            ),
            ("x-content-type-options".to_owned(), "nosniff".to_owned()),
            ("referrer-policy".to_owned(), "no-referrer".to_owned()),
        ])
    }

    fn healthy_fixture(descriptor: &Descriptor) -> Fake {
        let base = format!("https://{}", descriptor.ingress.dns);
        Fake {
            responses: BTreeMap::from([
                (
                    format!("{base}/health"),
                    Ok(ProbeResponse {
                        status: 200,
                        headers: secure_headers(),
                        body: br#"{"state":"healthy"}"#.to_vec(),
                    }),
                ),
                (
                    format!("{base}/ready"),
                    Ok(ProbeResponse {
                        status: 200,
                        headers: BTreeMap::new(),
                        body: br#"{"state":"healthy"}"#.to_vec(),
                    }),
                ),
                (
                    format!("{base}/version"),
                    Ok(ProbeResponse {
                        status: 200,
                        headers: BTreeMap::new(),
                        body: format!(r#"{{"image_digest":"{}"}}"#, descriptor.image.digest)
                            .into_bytes(),
                    }),
                ),
                (
                    format!("http://{}/", descriptor.ingress.dns),
                    Err("connection refused".to_owned()),
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
        ingress::probe_rows(&mut report, descriptor, probe);
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
                headers: BTreeMap::new(),
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
                headers: BTreeMap::new(),
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
