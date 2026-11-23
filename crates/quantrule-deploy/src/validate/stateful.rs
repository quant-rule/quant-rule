//! The stateful PKI leaves Patroni and etcd present.
//!
//! `tools/pki.sh` writes the Postgres server leaf and the etcd server, peer, and
//! client leaves under the same internal CA. The descriptor names the CA bundle
//! (`endpoints.database.tls_ca`); this module treats its parent as the PKI root
//! and reads the leaves once so validation and rendering share one read. A
//! missing, symlinked, oversized, or world-readable private key fails closed.

use std::path::Path;

use crate::descriptor::v3::Descriptor;
use crate::error::Error;

use super::secrets::{read_pem_file, read_public_file};
use super::v3::rule;

/// One certificate and its private key.
#[derive(Debug)]
pub struct Leaf {
    /// The PEM certificate.
    pub certificate: String,
    /// The PEM private key.
    pub key: String,
}

/// The etcd server, peer, and client leaves.
#[derive(Debug)]
pub struct EtcdLeaves {
    /// The client-listener certificate.
    pub server: Leaf,
    /// The peer-listener certificate, which also acts as the peer client.
    pub peer: Leaf,
    /// The Patroni etcd client certificate.
    pub client: Leaf,
}

/// The stateful PKI read once for validation and rendering.
#[derive(Debug)]
pub struct StatefulFiles {
    /// The PostgreSQL server certificate and key.
    pub postgres: Leaf,
    /// The etcd leaves.
    pub etcd: EtcdLeaves,
}

/// Read the stateful PKI leaves from the CA bundle's parent directory.
///
/// # Errors
///
/// Returns [`Error::Rule`] naming the logical file when a leaf is missing,
/// symlinked, not a regular file, oversized, unreadable, or when a private key
/// is not mode 600.
pub fn load(descriptor: &Descriptor) -> Result<StatefulFiles, Error> {
    let root = Path::new(&descriptor.endpoints.database.tls_ca)
        .parent()
        .ok_or_else(|| {
            rule(
                "deploy.endpoint.tls_ca",
                "endpoints.database.tls_ca",
                "must name a file inside a PKI directory",
            )
        })?;
    let postgres = leaf(root, "postgres", "server", "pki.postgres.server")?;
    let etcd_server = leaf(root, "etcd", "server", "pki.etcd.server")?;
    let etcd_peer = leaf(root, "etcd", "peer", "pki.etcd.peer")?;
    let etcd_client = leaf(root, "etcd", "client", "pki.etcd.client")?;
    Ok(StatefulFiles {
        postgres,
        etcd: EtcdLeaves {
            server: etcd_server,
            peer: etcd_peer,
            client: etcd_client,
        },
    })
}

fn leaf(root: &Path, directory: &str, name: &str, field: &str) -> Result<Leaf, Error> {
    let directory = root.join(directory);
    let certificate_path = directory.join(format!("{name}.crt"));
    let key_path = directory.join(format!("{name}.key"));
    let certificate =
        read_public_file(&certificate_path.to_string_lossy(), &format!("{field}.crt"))
            .map_err(|error| contextual(error, &certificate_path, field))?;
    let key = read_pem_file(&key_path.to_string_lossy(), &format!("{field}.key"), true)
        .map_err(|error| contextual(error, &key_path, field))?;
    Ok(Leaf { certificate, key })
}

/// Keep the stable code while naming the path the operator must install.
fn contextual(error: Error, path: &Path, field: &str) -> Error {
    match error {
        Error::Rule { code, .. } => rule(
            code,
            field,
            format!(
                "cannot use the PKI leaf at {}; run tools/pki.sh or install the leaf at mode 600",
                path.display()
            ),
        ),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "qr-deploy-stateful-pki-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temp dir");
        path
    }

    fn write(path: &Path, content: &str, mode: u32) {
        std::fs::write(path, content).expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("mode");
        }
        #[cfg(not(unix))]
        let _ = mode;
    }

    fn fixture(directory: &Path) -> Descriptor {
        let root = directory.join("pki");
        std::fs::create_dir_all(root.join("postgres")).expect("postgres");
        std::fs::create_dir_all(root.join("etcd")).expect("etcd");
        write(&root.join("ca.crt"), "ca\n", 0o644);
        for (directory, name) in [
            ("postgres", "server"),
            ("etcd", "server"),
            ("etcd", "peer"),
            ("etcd", "client"),
        ] {
            write(
                &root.join(directory).join(format!("{name}.crt")),
                "cert\n",
                0o644,
            );
            write(
                &root.join(directory).join(format!("{name}.key")),
                "key\n",
                0o600,
            );
        }
        let mut value: Value =
            serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
                .expect("example");
        value["endpoints"]["database"]["tls_ca"] = serde_json::json!(root.join("ca.crt"));
        serde_json::from_value(value).expect("descriptor")
    }

    #[test]
    fn the_leaf_layout_is_read_from_the_ca_parent() {
        let directory = temp_dir();
        let descriptor = fixture(&directory);
        let files = load(&descriptor).expect("pki");
        assert_eq!(files.postgres.certificate, "cert");
        assert_eq!(files.etcd.client.key, "key");
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn a_world_readable_private_key_is_refused() {
        let directory = temp_dir();
        let descriptor = fixture(&directory);
        let path = directory.join("pki/postgres/server.key");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
                .expect("mode 644");
        }
        let error = load(&descriptor).expect_err("a readable key");
        assert!(error.to_string().contains("deploy.secret.mode"), "{error}");
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn a_missing_leaf_fails_closed() {
        let directory = temp_dir();
        let descriptor = fixture(&directory);
        std::fs::remove_file(directory.join("pki/etcd/peer.crt")).expect("remove");
        let error = load(&descriptor).expect_err("a missing peer cert");
        assert!(
            error.to_string().contains("deploy.file.unreadable"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(directory);
    }
}
