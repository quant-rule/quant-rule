//! Placement, failure-domain and anti-affinity rules.
//!
//! The descriptor is a placement contract, not a scheduler hint. `swarm-ha`
//! spreads every replica across distinct hosts and distinct failure domains.
//! `single-az` keeps host anti-affinity inside one availability zone.
//! `lab-single-host` is the only profile that allows co-location, because its
//! three VMs share one physical failure domain; its evidence is `Simulated HA`.

use std::collections::BTreeSet;

use crate::descriptor::v3::{Descriptor, Profile, Service};
use crate::error::Error;

use super::v3::{distinct_domains, host_by_name, rule};

/// Validate profile-level failure-domain counts and placement.
///
/// # Errors
///
/// Returns [`Error::Rule`] with a stable placement code.
pub fn validate(descriptor: &Descriptor) -> Result<(), Error> {
    match descriptor.profile {
        Profile::SwarmHa => {
            require_hosts(descriptor, 3)?;
            let domains = distinct_domains(descriptor);
            if domains.len() < 3 {
                return Err(rule(
                    "deploy.profile.failure_domains",
                    "hosts",
                    format!(
                        "swarm-ha needs at least three distinct failure domains; found {}",
                        domains.len()
                    ),
                ));
            }
            for (name, service) in &descriptor.services {
                anti_affinity(descriptor, name, service, true)?;
            }
            ingress_anti_affinity(descriptor, true, true)?;
            stateful_spread(descriptor, true)?;
        }
        Profile::SingleAz => {
            require_hosts(descriptor, 3)?;
            // The profile asserts one availability zone; the descriptor has no
            // AZ field. Failure domains may be racks inside that zone, so the
            // same host and domain anti-affinity applies without a three-domain
            // minimum.
            for (name, service) in &descriptor.services {
                anti_affinity(descriptor, name, service, true)?;
            }
            ingress_anti_affinity(descriptor, true, false)?;
            stateful_spread(descriptor, true)?;
        }
        Profile::LabSingleHost => {
            require_hosts(descriptor, 3)?;
            // Co-location is the recorded lab exception. The ingress still
            // binds 443 in host mode, so its replicas stay on distinct hosts.
            ingress_anti_affinity(descriptor, false, false)?;
        }
    }
    Ok(())
}

fn require_hosts(descriptor: &Descriptor, minimum: usize) -> Result<(), Error> {
    if descriptor.hosts.len() < minimum {
        return Err(rule(
            "deploy.hosts.count",
            "hosts",
            format!(
                "must declare at least {minimum} hosts; found {}",
                descriptor.hosts.len()
            ),
        ));
    }
    Ok(())
}

fn anti_affinity(
    descriptor: &Descriptor,
    name: &str,
    service: &Service,
    strict_domains: bool,
) -> Result<(), Error> {
    let hosts: BTreeSet<&str> = service.hosts.iter().map(String::as_str).collect();
    if usize::try_from(service.replicas).unwrap_or(usize::MAX) > hosts.len() {
        return Err(rule(
            "deploy.service.anti_affinity",
            format!("services.{name}.replicas"),
            format!(
                "{} replicas need {} distinct hosts; only {} are named",
                service.replicas,
                service.replicas,
                hosts.len()
            ),
        ));
    }
    if strict_domains {
        let domains: BTreeSet<&str> = hosts
            .iter()
            .map(|host| descriptor.hosts[*host].failure_domain.as_str())
            .collect();
        if usize::try_from(service.replicas).unwrap_or(usize::MAX) > domains.len() {
            return Err(rule(
                "deploy.service.anti_affinity",
                format!("services.{name}.hosts"),
                format!(
                    "{} replicas need {} distinct failure domains; the named hosts cover {}",
                    service.replicas,
                    service.replicas,
                    domains.len()
                ),
            ));
        }
    }
    Ok(())
}

fn ingress_anti_affinity(
    descriptor: &Descriptor,
    strict_domains: bool,
    require_two: bool,
) -> Result<(), Error> {
    let ingress = &descriptor.ingress;
    if ingress.replicas == 0 {
        return Err(rule(
            "deploy.ingress.replicas",
            "ingress.replicas",
            "must be at least one",
        ));
    }
    if require_two && ingress.replicas < 2 {
        return Err(rule(
            "deploy.ingress.replicas",
            "ingress.replicas",
            "swarm-ha needs at least two ingress replicas",
        ));
    }
    let hosts: BTreeSet<&str> = ingress.hosts.iter().map(String::as_str).collect();
    if usize::try_from(ingress.replicas).unwrap_or(usize::MAX) > hosts.len() {
        return Err(rule(
            "deploy.ingress.anti_affinity",
            "ingress.hosts",
            format!(
                "{} ingress replicas need {} distinct hosts; only {} are named",
                ingress.replicas,
                ingress.replicas,
                hosts.len()
            ),
        ));
    }
    if strict_domains {
        let domains: BTreeSet<&str> = hosts
            .iter()
            .map(|host| descriptor.hosts[*host].failure_domain.as_str())
            .collect();
        if domains.len() < 2 {
            return Err(rule(
                "deploy.ingress.anti_affinity",
                "ingress.hosts",
                "the ingress replicas must cover at least two failure domains",
            ));
        }
    }
    Ok(())
}

fn stateful_spread(descriptor: &Descriptor, strict_domains: bool) -> Result<(), Error> {
    let units = [
        (
            "stateful.postgres.nodes",
            &descriptor.stateful.postgres.nodes,
        ),
        ("stateful.etcd.nodes", &descriptor.stateful.etcd.nodes),
        ("stateful.haproxy.nodes", &descriptor.stateful.haproxy.nodes),
        (
            "stateful.rabbitmq.nodes",
            &descriptor.stateful.rabbitmq.nodes,
        ),
        ("stateful.minio.nodes", &descriptor.stateful.minio.nodes),
    ];
    for (field, nodes) in units {
        let hosts: BTreeSet<&str> = nodes.iter().map(String::as_str).collect();
        if hosts.len() != nodes.len() {
            return Err(rule(
                "deploy.stateful.anti_affinity",
                field,
                "every stateful member must use a distinct host",
            ));
        }
        if strict_domains {
            let domains: BTreeSet<&str> = hosts
                .iter()
                .map(|host| descriptor.hosts[*host].failure_domain.as_str())
                .collect();
            if domains.len() != nodes.len() {
                return Err(rule(
                    "deploy.stateful.anti_affinity",
                    field,
                    "every stateful member must use a distinct failure domain",
                ));
            }
        }
        for node in nodes {
            host_by_name(descriptor, node, field)?;
        }
    }
    Ok(())
}
