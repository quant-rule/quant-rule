//! The opt-in all-service runtime digest probe.
//!
//! `qr-deploy check --remote docker --runtime` runs the descriptor image's own
//! `qr-gateway --healthcheck --print` mode on the descriptor overlay against
//! each service endpoint and compares the reported `image_digest` with the
//! descriptor. This is the one check that starts a short-lived container; it is
//! never reached by the default check or by `--remote docker` alone. Rows name
//! the service and digest, never a secret, an environment value, or a full
//! response body.

use serde::Deserialize;

use crate::descriptor::v3::Descriptor;
use crate::render::app_image;

use super::remote::{DockerQuery, Inspector, append};
use super::report::{CheckReport, CheckRow};

/// The all-service runtime digest inspector.
struct RuntimeInspector<'a> {
    descriptor: &'a Descriptor,
    docker: &'a dyn DockerQuery,
}

impl<'a> RuntimeInspector<'a> {
    fn new(descriptor: &'a Descriptor, docker: &'a dyn DockerQuery) -> Self {
        Self { descriptor, docker }
    }
}

impl Inspector for RuntimeInspector<'_> {
    fn inspect(&self) -> Result<Vec<CheckRow>, String> {
        inspect_runtime(self.descriptor, self.docker)
    }
}

/// Append the all-service runtime digest rows when the operator opted in.
pub fn append_runtime(report: &mut CheckReport, descriptor: &Descriptor, docker: &dyn DockerQuery) {
    append(
        report,
        "remote",
        "runtime",
        &RuntimeInspector::new(descriptor, docker),
    );
}

#[derive(Debug, Deserialize, Default)]
struct RuntimeHealth {
    #[serde(default)]
    image_digest: Option<String>,
}

/// The seven service endpoints whose reported digest must equal the descriptor.
fn runtime_table() -> [(&'static str, u16, &'static str); 7] {
    [
        ("gateway", 8080, "/version"),
        ("data", 8080, "/version"),
        ("model", 8080, "/version"),
        ("strategy", 8080, "/version"),
        ("backtest", 9100, "/health"),
        ("evidence", 8080, "/version"),
        ("simulation", 8080, "/version"),
    ]
}

fn inspect_runtime(
    descriptor: &Descriptor,
    docker: &dyn DockerQuery,
) -> Result<Vec<CheckRow>, String> {
    let image = app_image(descriptor);
    let network = &descriptor.network.overlay;
    let mut rows = Vec::new();
    for (service, default_port, path) in runtime_table() {
        // The backtest worker serves no public API: its `/health` lives on the
        // internal metrics listener, not the unused service bind.
        let port = if service == "backtest" {
            default_port
        } else {
            descriptor
                .services
                .get(service)
                .map_or(default_port, |value| value.port)
        };
        let url = format!("http://{service}:{port}{path}");
        match docker.runtime_probe(network, &image, &url) {
            Ok(body) => {
                match serde_json::from_str::<RuntimeHealth>(&body) {
                    Ok(report) => match report.image_digest {
                        Some(digest) if digest == descriptor.image.digest => rows.push(
                            CheckRow::ok("remote", format!("runtime {service}"), "digest", digest),
                        ),
                        Some(digest) => rows.push(CheckRow::failed(
                            "remote",
                            format!("runtime {service}"),
                            "mismatch",
                            format!("reports {digest}, expected {}", descriptor.image.digest),
                        )),
                        None => rows.push(CheckRow::failed(
                            "remote",
                            format!("runtime {service}"),
                            "missing",
                            "the response has no image_digest",
                        )),
                    },
                    Err(error) => rows.push(CheckRow::failed(
                        "remote",
                        format!("runtime {service}"),
                        "unreadable",
                        format!("cannot parse {url}: {error}"),
                    )),
                }
            }
            Err(error) => rows.push(CheckRow::failed(
                "remote",
                format!("runtime {service}"),
                "unavailable",
                format!("deploy.remote.runtime: {error}"),
            )),
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests;
