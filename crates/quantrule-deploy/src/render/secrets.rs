//! Secret, PKI, and manifest assembly for a v3 render.
//!
//! The source `secrets.files` values are read once by validation and handed to
//! this module. Each value is written to `secrets/<name>` at mode 600; the
//! manifest lists names and paths, never a value or a fingerprint. Public CA
//! bundles are copied to `pki/` at mode 644; an ingress private key is copied
//! at mode 600 and only when the descriptor selected `tls: cert`.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::descriptor::v3::{Descriptor, IngressTls};
use crate::error::Error;
use crate::validate::secrets::{read_public_file, scope};
use crate::validate::v3::LoadedFiles;

use super::{create_dir, safe_component, write_private, write_public};

/// Write the secret and PKI material common to every target.
pub fn render(
    descriptor: &Descriptor,
    files: &LoadedFiles,
    out: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<(), Error> {
    let secrets_dir = out.join("secrets");
    create_dir(&secrets_dir)?;
    let mut entries = Vec::with_capacity(files.secrets.len());
    for (name, value) in &files.secrets {
        safe_component(name, &format!("secrets.files.{name}"))?;
        let path = secrets_dir.join(name);
        write_private(&path, format!("{value}\n").as_bytes())?;
        written.push(path);
        entries.push(SecretEntry {
            name: name.clone(),
            path: format!("secrets/{name}"),
            mode: "600".to_owned(),
            swarm_name: name.clone(),
            scope: scope(name).to_owned(),
        });
    }
    let manifest = SecretManifest {
        v: "quantrule.deploy/v3",
        note: "values are not listed; app secrets are Swarm objects, stateful and MinIO root secrets are copied by tools/stateful.sh",
        files: entries,
    };
    let text = serde_json::to_string_pretty(&manifest).map_err(|error| Error::Invalid {
        field: "secrets/manifest.json".to_owned(),
        message: error.to_string(),
    })?;
    let path = secrets_dir.join("manifest.json");
    write_private(&path, format!("{text}\n").as_bytes())?;
    written.push(path);

    let pki_dir = out.join("pki");
    create_dir(&pki_dir)?;
    for (name, source, field) in [
        (
            "database-ca.crt",
            descriptor.endpoints.database.tls_ca.as_str(),
            "endpoints.database.tls_ca",
        ),
        (
            "queue-ca.crt",
            descriptor.endpoints.queue.tls_ca.as_str(),
            "endpoints.queue.tls_ca",
        ),
        (
            "object-store-ca.crt",
            descriptor.endpoints.object_store.tls_ca.as_str(),
            "endpoints.object_store.tls_ca",
        ),
    ] {
        let contents = read_public_file(source, field)?;
        let path = pki_dir.join(name);
        write_public(&path, contents.as_bytes())?;
        written.push(path);
    }
    if descriptor.ingress.tls == IngressTls::Cert {
        let certificate = files
            .ingress_certificate
            .as_deref()
            .ok_or_else(|| Error::Invalid {
                field: "ingress.files.certificate".to_owned(),
                message: "cert mode needs a readable certificate".to_owned(),
            })?;
        let path = pki_dir.join("ingress.crt");
        write_public(&path, certificate.as_bytes())?;
        written.push(path);
        let key = files.ingress_key.as_deref().ok_or_else(|| Error::Invalid {
            field: "ingress.files.key".to_owned(),
            message: "cert mode needs a readable private key".to_owned(),
        })?;
        let path = pki_dir.join("ingress.key");
        write_private(&path, key.as_bytes())?;
        written.push(path);
    }
    Ok(())
}

#[derive(Serialize)]
struct SecretManifest {
    v: &'static str,
    note: &'static str,
    files: Vec<SecretEntry>,
}

#[derive(Serialize)]
struct SecretEntry {
    name: String,
    path: String,
    mode: String,
    swarm_name: String,
    scope: String,
}
