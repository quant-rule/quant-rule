//! MinIO configuration, bucket policy, PKI, and SANs for the v3 stateful tier.
//!
//! The renderer writes one environment file and one bucket policy, the MinIO
//! server leaf, and the `minio` + every `minio-<host>` SAN set. The application
//! connects to the HAProxy pair through the shared `minio` alias; MinIO nodes
//! never carry that alias, so one node loss cannot strand the client.
//!
//! The server reads its root credentials from the mounted mode-600 files. The
//! rendered config carries the `_FILE` references, never a value. Erasure
//! coding is explicit: `MINIO_STORAGE_CLASS_STANDARD=EC:<n>`, with `auto`
//! derived from the drive layout and validated so one node loss keeps the write
//! quorum.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::descriptor::v3::Descriptor;
use crate::error::Error;
use crate::validate::secrets::read_public_file;
use crate::validate::v3::LoadedFiles;

use super::{create_dir, safe_component, write_private, write_public};

/// The MinIO S3 client port; TLS only.
pub const MINIO_PORT: u16 = 9000;

/// The in-container configuration path sourced by the unit command.
pub const MINIO_CONFIG_TARGET: &str = "/etc/minio/minio.conf";

/// The in-container certificate directory read by `minio server`.
pub const MINIO_CERTS_TARGET: &str = "/certs";

/// The host path of the MinIO config.
pub const MINIO_CONFIG_HOST: &str = "/etc/quantrule/minio/minio.conf";

/// The host path of the least-privilege bucket policy.
pub const MINIO_POLICY_HOST: &str = "/etc/quantrule/minio/quantrule-app-policy.json";

/// The bucket the application owns.
pub const MINIO_BUCKET: &str = "quantrule";

/// The root secret files mounted into the MinIO unit.
pub const MINIO_ROOT_USER_FILE: &str = "/run/secrets/minio_root_user";
/// The root password file mounted into the MinIO unit.
pub const MINIO_ROOT_PASSWORD_FILE: &str = "/run/secrets/minio_root_password";

/// Write the per-node config, the bucket policy, and the MinIO PKI leaf.
pub fn render(
    descriptor: &Descriptor,
    files: &LoadedFiles,
    out: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<(), Error> {
    for host in &descriptor.stateful.minio.nodes {
        safe_component(host, "stateful.minio.nodes")?;
        let path = out.join(format!("minio-{host}.conf"));
        write_private(&path, minio_conf(descriptor, host)?.as_bytes())?;
        written.push(path);
    }
    let path = out.join("minio-policy.json");
    write_public(&path, bucket_policy().as_bytes())?;
    written.push(path);

    let pki = out.join("pki").join("minio");
    create_dir(&pki)?;
    create_dir(&pki.join("CAs"))?;
    let object_ca = read_public_file(
        &descriptor.endpoints.object_store.tls_ca,
        "endpoints.object_store.tls_ca",
    )
    .map_err(|error| {
        Error::rule(
            "deploy.endpoint.tls_ca",
            "endpoints.object_store.tls_ca",
            format!("cannot use the object-store CA bundle for the MinIO leaf: {error}"),
        )
    })?;
    for (path, content, private) in [
        (
            pki.join("public.crt"),
            files.stateful_pki.minio.certificate.as_str(),
            false,
        ),
        (
            pki.join("private.key"),
            files.stateful_pki.minio.key.as_str(),
            true,
        ),
        (pki.join("CAs").join("ca.crt"), object_ca.as_str(), false),
    ] {
        if private {
            write_private(&path, content.as_bytes())?;
        } else {
            write_public(&path, content.as_bytes())?;
        }
        written.push(path);
    }
    Ok(())
}

/// The DNS names the MinIO leaf must cover.
///
/// The shared `minio` name is the application front door on HAProxy; every
/// MinIO node also needs its unique `minio-<host>` name for inter-node traffic.
#[must_use]
pub fn minio_sans(descriptor: &Descriptor) -> Vec<String> {
    let mut names = BTreeSet::from(["minio".to_owned()]);
    for host in &descriptor.stateful.minio.nodes {
        names.insert(format!("minio-{host}"));
    }
    names.into_iter().collect()
}

/// Build one node's `minio.conf` environment file.
///
/// # Errors
///
/// Returns a rule when the node host is not declared or the parity is invalid.
/// The in-cluster names are overlay aliases, never the host underlay address.
pub fn minio_conf(descriptor: &Descriptor, host: &str) -> Result<String, Error> {
    host_by_name(descriptor, host, "stateful.minio.nodes")?;
    let mut volumes = Vec::new();
    for node in &descriptor.stateful.minio.nodes {
        host_by_name(descriptor, node, "stateful.minio.nodes")?;
        let drives = descriptor.stateful.minio.drives[descriptor
            .stateful
            .minio
            .nodes
            .iter()
            .position(|candidate| candidate == node)
            .unwrap_or(0)];
        for drive in 1..=drives {
            volumes.push(format!("https://minio-{node}:{MINIO_PORT}/data{drive}"));
        }
    }
    let parity = crate::validate::v3::minio_parity(descriptor)?;
    let endpoint = format!(
        "{}://{}:{}",
        descriptor.endpoints.object_store.scheme,
        descriptor.endpoints.object_store.host,
        MINIO_PORT
    );
    Ok(format!(
        "# Generated by qr-deploy; copy to {MINIO_CONFIG_HOST}.\n\
         # The unit command exports the root credentials from the mounted files,\n\
         # sources this file, and execs `minio server --certs-dir {MINIO_CERTS_TARGET}`.\n\
         # No secret value is written here.\n\
         MINIO_VOLUMES=\"{volumes}\"\n\
         MINIO_SERVER_URL=\"{endpoint}\"\n\
         MINIO_BROWSER=\"off\"\n\
         MINIO_STORAGE_CLASS_STANDARD=\"EC:{parity}\"\n\
         MINIO_ROOT_USER_FILE=\"{MINIO_ROOT_USER_FILE}\"\n\
         MINIO_ROOT_PASSWORD_FILE=\"{MINIO_ROOT_PASSWORD_FILE}\"\n\
         MINIO_CONFIG_ENV_FILE=\"{MINIO_CONFIG_TARGET}\"\n\
         MINIO_REGION=\"us-east-1\"\n",
        volumes = volumes.join(" "),
    ))
}

/// The least-privilege policy for `qr_app`: the `quantrule` bucket only.
#[must_use]
pub fn bucket_policy() -> String {
    format!(
        r#"{{
  "Version": "2012-10-17",
  "Statement": [
    {{
      "Effect": "Allow",
      "Action": [
        "s3:GetBucketLocation",
        "s3:GetBucketVersioning",
        "s3:ListBucket",
        "s3:ListBucketMultipartUploads"
      ],
      "Resource": [
        "arn:aws:s3:::{MINIO_BUCKET}"
      ]
    }},
    {{
      "Effect": "Allow",
      "Action": [
        "s3:AbortMultipartUpload",
        "s3:DeleteObject",
        "s3:GetObject",
        "s3:ListMultipartUploadParts",
        "s3:PutObject"
      ],
      "Resource": [
        "arn:aws:s3:::{MINIO_BUCKET}/*"
      ]
    }}
  ]
}}
"#
    )
}

fn host_by_name<'a>(
    descriptor: &'a Descriptor,
    host: &str,
    field: &str,
) -> Result<&'a crate::descriptor::v3::Host, Error> {
    descriptor.hosts.get(host).ok_or_else(|| {
        Error::rule(
            "deploy.host.undeclared",
            field,
            format!("{host} is not declared in hosts"),
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

    fn lab() -> Descriptor {
        serde_json::from_str(include_str!("../../../../deploy/v3/deploy.lab.json")).expect("v3 lab")
    }

    #[test]
    fn the_conf_has_explicit_volumes_tls_and_no_secret_value() {
        let descriptor = descriptor();
        let text = minio_conf(&descriptor, "infra-1").expect("conf");
        for expected in [
            "MINIO_VOLUMES=\"https://minio-infra-1:9000/data1 https://minio-infra-2:9000/data1 https://minio-infra-3:9000/data1 https://minio-infra-4:9000/data1\"",
            "MINIO_SERVER_URL=\"https://minio:9000\"",
            "MINIO_BROWSER=\"off\"",
            "MINIO_STORAGE_CLASS_STANDARD=\"EC:2\"",
            "MINIO_ROOT_USER_FILE=\"/run/secrets/minio_root_user\"",
            "MINIO_ROOT_PASSWORD_FILE=\"/run/secrets/minio_root_password\"",
            "MINIO_CONFIG_ENV_FILE=\"/etc/minio/minio.conf\"",
            "MINIO_REGION=\"us-east-1\"",
        ] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
        for name in ["minio_root_user", "minio_root_password"] {
            let value = std::fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../deploy/v3/secrets")
                    .join(name),
            )
            .expect("fixture secret");
            assert!(!text.contains(value.trim()), "{name}: {text}");
        }
        for host in descriptor.hosts.values() {
            assert!(!text.contains(&host.address), "{}", host.address);
        }
        assert!(!text.contains("9001"), "{text}");
    }

    #[test]
    fn the_lab_conf_lists_every_drive_and_uses_ec3() {
        let descriptor = lab();
        let text = minio_conf(&descriptor, "lab-1").expect("conf");
        for expected in [
            "https://minio-lab-1:9000/data2",
            "https://minio-lab-2:9000/data2",
            "https://minio-lab-3:9000/data2",
            "MINIO_STORAGE_CLASS_STANDARD=\"EC:3\"",
            "MINIO_SERVER_URL=\"https://minio:9000\"",
        ] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
    }

    #[test]
    fn the_sans_cover_the_front_door_and_every_node() {
        assert_eq!(
            minio_sans(&descriptor()),
            vec![
                "minio".to_owned(),
                "minio-infra-1".to_owned(),
                "minio-infra-2".to_owned(),
                "minio-infra-3".to_owned(),
                "minio-infra-4".to_owned(),
            ]
        );
        assert_eq!(
            minio_sans(&lab()),
            vec![
                "minio".to_owned(),
                "minio-lab-1".to_owned(),
                "minio-lab-2".to_owned(),
                "minio-lab-3".to_owned(),
            ]
        );
    }

    #[test]
    fn the_policy_is_bucket_scoped_and_has_no_admin_action() {
        let policy = bucket_policy();
        for expected in [
            "\"Version\": \"2012-10-17\"",
            "\"s3:GetObject\"",
            "\"s3:PutObject\"",
            "\"s3:DeleteObject\"",
            "\"s3:ListBucket\"",
            "\"arn:aws:s3:::quantrule\"",
            "\"arn:aws:s3:::quantrule/*\"",
        ] {
            assert!(policy.contains(expected), "{expected}: {policy}");
        }
        for forbidden in [
            "s3:*",
            "admin:",
            "s3:CreateBucket",
            "s3:DeleteBucket",
            "s3:PutBucketPolicy",
            "arn:aws:s3:::*",
        ] {
            assert!(!policy.contains(forbidden), "{forbidden}: {policy}");
        }
    }
}
