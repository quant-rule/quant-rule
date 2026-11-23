//! Deterministic rendering for `quantrule.deploy/v3`.
//!
//! The v3 renderer writes the Swarm stack for the seven stateless services and
//! the one ingress, the pinned per-host Compose skeletons for the stateful and
//! observability units, the operator job scripts, the secret and PKI copies, and
//! a `render.json` manifest. It never writes a secret value into YAML, an env
//! file, a script, or a log line.
//!
//! v2 is a separate module and keeps its byte-identical output.

pub mod v2;

mod infra;
mod jobs;
mod secrets;
pub(crate) mod swarm;
mod yaml;

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::descriptor::v3::{Descriptor, ImagePin, Profile};
use crate::error::Error;

/// The seven stateless services and the binary each one starts.
pub const APP_SERVICES: [(&str, &str); 7] = [
    ("gateway", "qr-gateway"),
    ("data", "qr-data"),
    ("model", "qr-model"),
    ("strategy", "qr-strategy"),
    ("backtest", "qr-backtest"),
    ("evidence", "qr-evidence"),
    ("simulation", "qr-simulation"),
];

/// What a v3 render writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// `stack.yaml`, the bootstrap script, the jobs, and the shared material.
    Swarm,
    /// `infra/<host>.yaml` and the shared material.
    Infra,
    /// Both.
    All,
}

impl Target {
    /// Parse the `--target` value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Usage`] for a value other than `swarm`, `infra`, or
    /// `all`.
    pub fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "swarm" => Ok(Self::Swarm),
            "infra" => Ok(Self::Infra),
            "all" => Ok(Self::All),
            other => Err(Error::Usage(format!(
                "unknown --target {other}; expected swarm, infra, or all"
            ))),
        }
    }

    /// The descriptor spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Swarm => "swarm",
            Self::Infra => "infra",
            Self::All => "all",
        }
    }

    /// Whether the Swarm stack and its jobs are part of this target.
    #[must_use]
    pub fn wants_swarm(self) -> bool {
        matches!(self, Self::Swarm | Self::All)
    }

    /// Whether the per-host infrastructure files are part of this target.
    #[must_use]
    pub fn wants_infra(self) -> bool {
        matches!(self, Self::Infra | Self::All)
    }
}

/// What one v3 render produced.
#[derive(Debug, Clone)]
pub struct RenderReport {
    /// Every rendered file, in sorted order.
    pub files: Vec<PathBuf>,
    /// The canonical descriptor hash.
    pub descriptor_hash: String,
    /// The pinned application image.
    pub app_image: String,
    /// The evidence label the profile may print.
    pub evidence: &'static str,
    /// The selected target.
    pub target: Target,
}

impl RenderReport {
    /// The relative path of every rendered file, in sorted order.
    #[must_use]
    pub fn relative_files(&self, out: &Path) -> Vec<String> {
        self.files
            .iter()
            .filter_map(|path| path.strip_prefix(out).ok())
            .map(relative_text)
            .collect()
    }
}

/// Render one v3 descriptor.
///
/// # Errors
///
/// Returns the first v3 validation or file rule, an output-path rule, or a
/// write failure. No secret value appears in an error message.
pub fn render_v3(
    descriptor: &Descriptor,
    out: &Path,
    target: Target,
) -> Result<RenderReport, Error> {
    let values = crate::validate::v3::load_files(descriptor)?;
    prepare_output(out)?;
    let mut files = Vec::new();
    secrets::render(descriptor, &values, out, &mut files)?;
    if target.wants_swarm() {
        swarm::render(descriptor, out, &mut files)?;
    }
    if target.wants_infra() {
        infra::render(descriptor, out, &mut files)?;
    }
    let files = canonical_files(&files);
    let descriptor_hash = descriptor_hash(descriptor)?;
    let relative: Vec<String> = files
        .iter()
        .filter_map(|path| path.strip_prefix(out).ok())
        .map(relative_text)
        .collect();
    let manifest = RenderManifest {
        v: "quantrule.deploy/v3",
        profile: descriptor.profile.as_str(),
        evidence: evidence_tier(descriptor.profile),
        target: target.as_str(),
        descriptor_hash: &descriptor_hash,
        app_image: &app_image(descriptor),
        files: &relative,
    };
    let manifest_path = out.join("render.json");
    let text = serde_json::to_string_pretty(&manifest).map_err(|error| Error::Invalid {
        field: "render.json".to_owned(),
        message: error.to_string(),
    })?;
    write_public(&manifest_path, format!("{text}\n").as_bytes())?;
    let mut all = files;
    all.push(manifest_path);
    Ok(RenderReport {
        files: canonical_files(&all),
        descriptor_hash,
        app_image: app_image(descriptor),
        evidence: evidence_tier(descriptor.profile),
        target,
    })
}

/// The pinned image spelling: `reference@sha256:...`.
///
/// A reference that already carries a digest is reduced to its repository
/// name, so the descriptor's `digest` field is always the one that wins.
#[must_use]
pub fn pinned(pin: &ImagePin) -> String {
    let reference = pin
        .reference
        .split_once('@')
        .map_or(pin.reference.as_str(), |(name, _)| name);
    format!("{reference}@{}", pin.digest)
}

/// The application image every service runs.
#[must_use]
pub fn app_image(descriptor: &Descriptor) -> String {
    pinned(&descriptor.image)
}

/// The only evidence label a profile may print.
///
/// A real host or availability-zone failure claim belongs to HA-014; no render
/// or check output says `Real HA`.
#[must_use]
pub fn evidence_tier(profile: Profile) -> &'static str {
    match profile {
        Profile::LabSingleHost => "Simulated HA",
        Profile::SwarmHa => "HA profile (unverified)",
        Profile::SingleAz => "Single-zone HA (unverified)",
    }
}

/// The stack or deployment name.
#[must_use]
pub fn stack_name(descriptor: &Descriptor) -> &str {
    descriptor.name.as_deref().unwrap_or("quantrule")
}

/// A canonical SHA-256 hash of the typed descriptor.
fn descriptor_hash(descriptor: &Descriptor) -> Result<String, Error> {
    let bytes = serde_json::to_vec(descriptor).map_err(|error| Error::Invalid {
        field: "descriptor".to_owned(),
        message: error.to_string(),
    })?;
    Ok(format!("sha256:{}", quantrule_api::sha256_hex(&bytes)))
}

/// Render a value that came from the descriptor or a validated key.
pub(crate) fn safe_component(value: &str, field: &str) -> Result<(), Error> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.chars().any(char::is_whitespace)
    {
        return Err(Error::rule(
            "deploy.render.path",
            field,
            "must be one safe path component",
        ));
    }
    Ok(())
}

/// Create the output directory and remove only the renderer's own stale files.
fn prepare_output(out: &Path) -> Result<(), Error> {
    if out.as_os_str().is_empty() || out.parent().is_none() {
        return Err(Error::rule(
            "deploy.render.path",
            "out",
            "must not be a filesystem root",
        ));
    }
    match std::fs::symlink_metadata(out) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(Error::rule(
                "deploy.render.path",
                "out",
                "must not be a symlink",
            ));
        }
        Ok(metadata) if !metadata.is_dir() => {
            return Err(Error::rule(
                "deploy.render.path",
                "out",
                "must be a directory",
            ));
        }
        Ok(_) => {}
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(Error::Write {
                path: out.to_path_buf(),
                source,
            });
        }
    }
    create_dir(out)?;
    for name in [
        "stack.yaml",
        "infra",
        "bootstrap.sh",
        "jobs",
        "secrets",
        "pki",
        "render.json",
    ] {
        remove_existing(&out.join(name))?;
    }
    Ok(())
}

fn remove_existing(path: &Path) -> Result<(), Error> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            std::fs::remove_dir_all(path).map_err(|source| Error::Write {
                path: path.to_path_buf(),
                source,
            })
        }
        Ok(_) => std::fs::remove_file(path).map_err(|source| Error::Write {
            path: path.to_path_buf(),
            source,
        }),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::Write {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Create one directory with owner-only permissions.
pub(crate) fn create_dir(path: &Path) -> Result<(), Error> {
    std::fs::create_dir_all(path).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })?;
    set_mode(path, 0o700)
}

/// Write one public file (mode 644).
pub(crate) fn write_public(path: &Path, contents: &[u8]) -> Result<(), Error> {
    write_file(path, contents, 0o644)
}

/// Write one owner-only file (mode 600).
pub(crate) fn write_private(path: &Path, contents: &[u8]) -> Result<(), Error> {
    write_file(path, contents, 0o600)
}

/// Write one operator script (mode 700).
pub(crate) fn write_script(path: &Path, contents: &str) -> Result<(), Error> {
    write_file(path, contents.as_bytes(), 0o700)
}

fn write_file(path: &Path, contents: &[u8], mode: u32) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        create_dir_exists(parent)?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })?;
    use std::io::Write as _;
    file.write_all(contents).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })?;
    set_mode(path, mode)
}

/// Create a directory without changing an existing parent's mode.
fn create_dir_exists(path: &Path) -> Result<(), Error> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(Error::rule(
            "deploy.render.path",
            "out",
            format!("{} is not a directory", path.display()),
        )),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => create_dir(path),
        Err(source) => Err(Error::Write {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn set_mode(path: &Path, mode: u32) -> Result<(), Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).map_err(
            |source| Error::Write {
                path: path.to_path_buf(),
                source,
            },
        )?;
    }
    #[cfg(not(unix))]
    let _ = (path, mode);
    Ok(())
}

fn canonical_files(files: &[PathBuf]) -> Vec<PathBuf> {
    let mut sorted: Vec<PathBuf> = files.to_vec();
    sorted.sort();
    sorted.dedup();
    sorted
}

fn relative_text(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[derive(Serialize)]
struct RenderManifest<'a> {
    v: &'a str,
    profile: &'a str,
    evidence: &'a str,
    target: &'a str,
    descriptor_hash: &'a str,
    app_image: &'a str,
    files: &'a [String],
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "qr-render-mod-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temp dir");
        path
    }

    #[test]
    fn a_path_component_cannot_escape_the_output() {
        for value in ["", ".", "..", "a/b", "a\\b", "a b"] {
            assert!(safe_component(value, "secrets.files").is_err(), "{value:?}");
        }
        safe_component("gateway_token", "secrets.files").expect("a safe name");
    }

    #[test]
    fn a_reference_that_already_has_a_digest_is_not_doubled() {
        let pin = ImagePin {
            reference: "registry/server@sha256:old".to_owned(),
            digest: "sha256:new".to_owned(),
        };
        assert_eq!(pinned(&pin), "registry/server@sha256:new");
    }

    #[test]
    fn a_filesystem_root_is_not_an_output_directory() {
        let error = prepare_output(Path::new("/")).expect_err("root");
        assert!(error.to_string().contains("deploy.render.path"), "{error}");
    }

    #[test]
    fn stale_renderer_output_is_removed_but_operator_files_survive() {
        let out = temp_dir();
        std::fs::write(out.join("stack.yaml"), "stale").expect("stale stack");
        std::fs::create_dir_all(out.join("infra")).expect("stale infra");
        std::fs::write(out.join("infra/old.yaml"), "stale").expect("stale unit");
        std::fs::write(out.join("keep.txt"), "keep").expect("operator file");
        prepare_output(&out).expect("prepare");
        assert!(!out.join("stack.yaml").exists());
        assert!(!out.join("infra").exists());
        assert_eq!(
            std::fs::read_to_string(out.join("keep.txt")).expect("operator file"),
            "keep"
        );
        std::fs::remove_dir_all(&out).expect("cleanup");
    }
}
