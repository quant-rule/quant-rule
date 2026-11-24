//! The ingress configuration: Traefik static and dynamic files and keepalived.
//!
//! The static file is checked in because it never depends on the stack name.
//! The dynamic file names the stack's gateway service and is rendered from the
//! descriptor. Keepalived floats the descriptor VIP across the edge hosts; its
//! configuration is rendered per ingress host and mounted into a host-network
//! unit. This module never writes a certificate value into a log or a message.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::descriptor::v3::Descriptor;
use crate::error::Error;

use super::yaml::Yaml;
use super::{stack_name, write_public};

/// The external config that carries the checked-in static Traefik file.
pub(crate) const TRAEFIK_CONFIG: &str = "quantrule_traefik";
/// The external config that carries the rendered dynamic Traefik file.
pub(crate) const TRAEFIK_DYNAMIC_CONFIG: &str = "quantrule_traefik_dynamic";
/// The external config that carries the operator certificate.
pub(crate) const INGRESS_CERT_CONFIG: &str = "quantrule_ingress_cert";
/// The external secret that carries the operator private key.
pub(crate) const INGRESS_KEY_SECRET: &str = "quantrule_ingress_key";

/// The VMAC/VRRP router id shared by the two edge hosts.
const VIRTUAL_ROUTER_ID: u64 = 51;
/// The highest priority; the first descriptor host is the preferred master.
const MASTER_PRIORITY: u64 = 101;
/// Every other ingress host advertises one lower priority.
const BACKUP_PRIORITY: u64 = 100;
/// The 443 check script; the pinned image must provide a BusyBox-style `nc`.
const HEALTH_CHECK: &str = "nc -z 127.0.0.1 443";

pub(crate) const STATIC_CONFIG: &str = include_str!("../../../../deploy/v3/traefik.yml");

/// Write the static and dynamic Traefik files for a stack render.
pub(super) fn render_swarm(
    descriptor: &Descriptor,
    out: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<(), Error> {
    let path = out.join("quantrule_traefik.yml");
    write_public(&path, STATIC_CONFIG.as_bytes())?;
    written.push(path);

    let path = out.join("traefik-dynamic.yml");
    write_public(&path, dynamic_yaml(descriptor).as_bytes())?;
    written.push(path);
    Ok(())
}

/// Write one keepalived file for every ingress host.
pub(super) fn render_infra(
    descriptor: &Descriptor,
    out: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<(), Error> {
    for (index, host) in descriptor.ingress.hosts.iter().enumerate() {
        let path = out.join(format!("keepalived-{host}.conf"));
        write_public(&path, keepalived_conf(descriptor, host, index)?.as_bytes())?;
        written.push(path);
    }
    Ok(())
}

/// Build the dynamic HTTP configuration.
pub(crate) fn dynamic_yaml(descriptor: &Descriptor) -> String {
    let stack = stack_name(descriptor);
    let gateway = format!("http://{stack}_gateway:8080");
    let mut response_headers = BTreeMap::new();
    response_headers.insert("Server".to_owned(), Yaml::string(""));

    let headers = Yaml::map()
        .with("contentTypeNosniff", Yaml::Bool(true))
        .with("customResponseHeaders", Yaml::Map(response_headers))
        .with("referrerPolicy", Yaml::string("no-referrer"))
        .with("stsIncludeSubdomains", Yaml::Bool(true))
        .with("stsPreload", Yaml::Bool(false))
        .with("stsSeconds", Yaml::number(31_536_000_u64));
    let body = Yaml::map().with(
        "buffering",
        Yaml::map().with("maxRequestBodyBytes", Yaml::number(536_870_912_u64)),
    );
    let router = Yaml::map()
        .with("entryPoints", Yaml::list([Yaml::string("websecure")]))
        .with(
            "middlewares",
            Yaml::list([
                Yaml::string("security-headers"),
                Yaml::string("request-body"),
            ]),
        )
        .with(
            "rule",
            Yaml::string(format!("Host(`{}`)", descriptor.ingress.dns)),
        )
        .with("service", Yaml::string("gateway"))
        .with("tls", Yaml::map());
    let service = Yaml::map().with(
        "loadBalancer",
        Yaml::map().with(
            "servers",
            Yaml::list([Yaml::map().with("url", Yaml::string(gateway))]),
        ),
    );
    let http = Yaml::map()
        .with(
            "middlewares",
            Yaml::map()
                .with("request-body", body)
                .with("security-headers", headers),
        )
        .with("routers", Yaml::map().with("gateway", router))
        .with("services", Yaml::map().with("gateway", service));
    Yaml::map().with("http", http).render()
}

/// Build one edge host's keepalived configuration.
///
/// # Errors
///
/// Returns a rule when an ingress host has no public address or when fewer than
/// two ingress hosts are declared. Both are refused by validation first; this is
/// the defensive renderer boundary.
pub(crate) fn keepalived_conf(
    descriptor: &Descriptor,
    host: &str,
    index: usize,
) -> Result<String, Error> {
    let ingress = &descriptor.ingress;
    let vip = ingress.vip.as_deref().ok_or_else(|| {
        Error::rule(
            "deploy.render.ingress",
            "ingress.vip",
            "must declare a floating IPv4 address",
        )
    })?;
    let own = public_address(descriptor, host)?;
    let mut peers = Vec::new();
    for other in &ingress.hosts {
        if other == host {
            continue;
        }
        peers.push(public_address(descriptor, other)?);
    }
    if peers.is_empty() {
        return Err(Error::rule(
            "deploy.render.ingress",
            "ingress.hosts",
            "keepalived needs at least two ingress hosts",
        ));
    }
    let priority = if index == 0 {
        MASTER_PRIORITY
    } else {
        BACKUP_PRIORITY
    };
    let state = if index == 0 { "MASTER" } else { "BACKUP" };
    let mut peer_lines = String::new();
    for peer in peers {
        peer_lines.push_str(&format!("        {peer}\n"));
    }
    Ok(format!(
        "# Generated by qr-deploy for {host}; copy to /etc/quantrule/keepalived.conf.\n\
         # The pinned image must provide keepalived and a BusyBox-style `nc`.\n\
         vrrp_script chk_traefik {{\n\
         \x20   script \"{HEALTH_CHECK}\"\n\
         \x20   interval 1\n\
         \x20   timeout 1\n\
         \x20   fall 3\n\
         \x20   rise 2\n\
         }}\n\
         \n\
         vrrp_instance VI_1 {{\n\
         \x20   state {state}\n\
         \x20   interface {interface}\n\
         \x20   virtual_router_id {VIRTUAL_ROUTER_ID}\n\
         \x20   priority {priority}\n\
         \x20   advert_int 1\n\
         \x20   unicast_src_ip {own}\n\
         \x20   unicast_peer {{\n\
         {peer_lines}\
         \x20   }}\n\
         \x20   virtual_ipaddress {{\n\
         \x20       {vip}/32 dev {interface}\n\
         \x20   }}\n\
         \x20   track_script {{\n\
         \x20       chk_traefik\n\
         \x20   }}\n\
         }}\n",
        interface = ingress.interface,
        vip = vip,
    ))
}

fn public_address<'a>(descriptor: &'a Descriptor, host: &str) -> Result<&'a str, Error> {
    descriptor
        .hosts
        .get(host)
        .and_then(|value| value.public_address.as_deref())
        .ok_or_else(|| {
            Error::rule(
                "deploy.render.ingress",
                format!("hosts.{host}.public_address"),
                "an ingress host must declare a public address for keepalived",
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor() -> Descriptor {
        serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
            .expect("v3 example")
    }

    #[test]
    fn traefik_static_has_only_the_secure_entrypoint_and_an_internal_metrics_port() {
        for expected in [
            "websecure:",
            "address: \":443\"",
            "readTimeout: 900s",
            "writeTimeout: 900s",
            "idleTimeout: 900s",
            "dashboard: false",
            "insecure: false",
            "minVersion: VersionTLS12",
            "format: json",
            "Authorization: drop",
            "certFile: /etc/traefik/certs/ingress.crt",
            "keyFile: /etc/traefik/certs/ingress.key",
            "metrics:",
            "address: \":8082\"",
            "entryPoint: metrics",
        ] {
            assert!(STATIC_CONFIG.contains(expected), "{expected}");
        }
        for forbidden in [
            "address: \":80\"",
            "web:",
            "acme:",
            "certificatesResolvers:",
            "dashboard: true",
            "insecure: true",
        ] {
            assert!(!STATIC_CONFIG.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn traefik_dynamic_names_the_host_stack_and_security_headers() {
        let text = dynamic_yaml(&descriptor());
        for expected in [
            "Host(`quantrule.dev`)",
            "http://quantrule-ha_gateway:8080",
            "security-headers",
            "request-body",
            "stsSeconds: 31536000",
            "stsIncludeSubdomains: true",
            "stsPreload: false",
            "contentTypeNosniff: true",
            "referrerPolicy: \"no-referrer\"",
            "Server: \"\"",
            "maxRequestBodyBytes: 536870912",
            "websecure",
            "tls: {}",
        ] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
        assert!(!text.contains("basicAuth"), "{text}");
    }

    #[test]
    fn keepalived_floats_the_vip_between_the_edge_hosts() {
        let first = keepalived_conf(&descriptor(), "edge-1", 0).expect("edge-1");
        assert!(first.contains("state MASTER"), "{first}");
        assert!(first.contains("priority 101"), "{first}");
        assert!(first.contains("unicast_src_ip 203.0.113.10"), "{first}");
        assert!(first.contains("203.0.113.11"), "{first}");
        assert!(first.contains("203.0.113.20/32 dev eth0"), "{first}");
        assert!(first.contains("interface eth0"), "{first}");
        assert!(first.contains("virtual_router_id 51"), "{first}");
        assert!(first.contains("advert_int 1"), "{first}");
        assert!(first.contains("nc -z 127.0.0.1 443"), "{first}");
        assert!(first.contains("fall 3"), "{first}");
        assert!(first.contains("rise 2"), "{first}");

        let second = keepalived_conf(&descriptor(), "edge-2", 1).expect("edge-2");
        assert!(second.contains("state BACKUP"), "{second}");
        assert!(second.contains("priority 100"), "{second}");
        assert!(second.contains("unicast_src_ip 203.0.113.11"), "{second}");
        assert!(second.contains("203.0.113.10"), "{second}");
        assert!(second.contains("203.0.113.20/32 dev eth0"), "{second}");
    }

    #[test]
    fn keepalived_uses_the_descriptor_interface() {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
                .expect("example");
        value["ingress"]["interface"] = serde_json::json!("enp3s0");
        let descriptor: Descriptor = serde_json::from_value(value).expect("descriptor");
        let text = keepalived_conf(&descriptor, "edge-1", 0).expect("edge-1");
        assert!(text.contains("interface enp3s0"), "{text}");
        assert!(text.contains("203.0.113.20/32 dev enp3s0"), "{text}");
    }
}
