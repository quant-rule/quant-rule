//! The local half of a v3 check: descriptor, placement, pins, and file names.
//!
//! This module never opens a socket and never requires Docker. The file mode
//! and credential checks run by default; a deployment check must see the local
//! secret material it is about to trust.

use crate::descriptor::v3::Descriptor;
use crate::error::Error;
use crate::render::{app_image, evidence_tier, pinned};
use crate::validate::v3::{self, distinct_domains};

use super::report::{CheckReport, CheckRow};

/// Append the descriptor, placement, image, and secret rows.
///
/// The secret and credential files are checked by default: a v3 check is a
/// deployment check, not a descriptor-only dry run.
pub fn add(report: &mut CheckReport, descriptor: &Descriptor) {
    report.push(CheckRow::ok(
        "descriptor",
        "contract",
        "valid",
        format!(
            "{} {} on {} hosts and {} services",
            descriptor.v,
            descriptor.profile.as_str(),
            descriptor.hosts.len(),
            descriptor.services.len()
        ),
    ));
    let domains = distinct_domains(descriptor);
    report.push(CheckRow::ok(
        "placement",
        "hosts",
        "spread",
        format!(
            "{} hosts in {} failure domains",
            descriptor.hosts.len(),
            domains.len()
        ),
    ));
    report.push(CheckRow::ok(
        "placement",
        "services",
        "internal-only",
        format!(
            "{} services; no internal published port",
            descriptor.services.len()
        ),
    ));
    match crate::render::swarm::stack_yaml(descriptor) {
        Ok(stack) => {
            let published = stack.matches("published:").count();
            let ports = stack.matches("ports:").count();
            let ingresses = stack.matches("\n  ingress:").count();
            if published == 1
                && ports == 1
                && ingresses == 1
                && stack.contains("published: 443")
                && stack.contains("mode: \"host\"")
            {
                report.push(CheckRow::ok(
                    "placement",
                    "ingress",
                    "single-public-port",
                    format!(
                        "{} replicas on {} hosts; rendered stack publishes only 443 in host mode",
                        descriptor.ingress.replicas,
                        descriptor.ingress.hosts.len()
                    ),
                ));
            } else {
                report.push(CheckRow::failed(
                    "placement",
                    "ingress",
                    "published-port",
                    format!(
                        "the rendered stack has {ports} ports blocks, {published} published entries, and {ingresses} ingress services; expected one 443 host-mode entry and one ingress"
                    ),
                ));
            }
        }
        Err(error) => report.push(CheckRow::failed(
            "placement",
            "ingress",
            "render-failed",
            error.to_string(),
        )),
    }
    report.push(CheckRow::ok(
        "image",
        "application",
        "pinned",
        app_image(descriptor),
    ));
    let mut units = descriptor
        .images
        .iter()
        .map(|(name, pin)| format!("{name}={}", pinned(pin)))
        .collect::<Vec<_>>();
    units.sort();
    report.push(CheckRow::ok(
        "image",
        "infrastructure",
        "pinned",
        format!("{} units; {}", units.len(), units.join(", ")),
    ));

    let files_ok = match v3::validate_files(descriptor) {
        Ok(()) => true,
        Err(error) => {
            report.push(CheckRow::failed(
                "secret",
                "files",
                "rejected",
                error.to_string(),
            ));
            false
        }
    };
    for (name, path) in &descriptor.secrets.files {
        let state = if files_ok { "checked" } else { "unchecked" };
        report.push(CheckRow::ok("secret", name.clone(), state, path.clone()));
    }
    report.push(CheckRow::ok(
        "evidence",
        descriptor.profile.as_str(),
        evidence_tier(descriptor.profile),
        profile_note(descriptor),
    ));
}

fn profile_note(descriptor: &Descriptor) -> String {
    match descriptor.profile {
        crate::descriptor::v3::Profile::LabSingleHost => {
            "three VMs on one physical host; simulated node failure only".to_owned()
        }
        crate::descriptor::v3::Profile::SwarmHa => {
            "real host or AZ failure evidence belongs to HA-014".to_owned()
        }
        crate::descriptor::v3::Profile::SingleAz => {
            "one availability zone asserted by the operator".to_owned()
        }
    }
}

/// The first local validation error, if any, without opening a socket.
pub fn validate(descriptor: &Descriptor) -> Result<(), Error> {
    v3::validate(descriptor)
}
