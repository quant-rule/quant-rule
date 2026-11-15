//! The filesystem content-addressed store adapter.
//!
//! A logical key is split on `/`; every segment is validated and then mapped
//! to a filesystem-safe name. The mapping is deterministic and identical on
//! every platform: `%` and `:` are percent-encoded, a trailing dot or space is
//! encoded, and a Windows device name has its first character encoded. The
//! logical key stays the caller's key, while the physical path is portable.
//!
//! The embedded desktop profile has no MinIO. This module implements the same
//! [`ObjectStore`] trait over a directory: the key becomes a mapped relative
//! path, the bytes are the object, and a publish is a temporary file in the
//! destination directory followed by a rename (ADR-2026-11-12-12).
//!
//! Content addressing is the caller's rule and this adapter enforces it for the
//! keys that carry an address. A key whose last segment is a bare SHA-256 — the
//! `artifacts/{sha256}` blob layout and the `datasets/{sha256}` upload layout —
//! is verified on write and on read, so a corrupted file cannot be served as a
//! valid object. A key that names an identity rather than its bytes is moved
//! without verification.
//!
//! The root directory belongs to the operator. A key is validated against
//! traversal, so no key can leave the root; a symlink planted inside the root is
//! outside this module's threat model. `exists` is a metadata check and reads no
//! bytes, so content verification happens on `get`.
//!
//! An object is immutable once published. Repeating the same bytes is a no-op
//! rather than a second write, and a different body under an existing key is
//! refused. Every current key is either content-addressed or unique per run, so
//! a replacement is a defect; refusing it also keeps the publish atomic on a
//! platform whose rename cannot replace an existing file (Windows).

use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use quantrule_api::sha256_hex;

use crate::adapters::{Backend, BoxFuture, ObjectStore, StoreError};

/// A counter that keeps two temporary files in one process distinct.
static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

/// The prefix of a temporary file. The name is hidden and never a final key.
const TEMP_PREFIX: &str = ".quantrule-tmp-";

/// A content-addressed object store over a directory.
#[derive(Debug, Clone)]
pub struct FilesystemStore {
    root: PathBuf,
}

impl FilesystemStore {
    /// Build a store rooted at `root`.
    ///
    /// The directory is created by the first write; reading from a root that
    /// does not exist reports a missing object rather than an error.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The directory this store owns.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Whether the root is a writable directory.
    ///
    /// This is a liveness probe, not a content check: it creates the root when
    /// it is absent and writes and removes one temporary file, so a read-only
    /// or missing volume reports down at `/health` before the first run fails.
    /// The probe never names a key, so it cannot collide with an object.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the root cannot be created, is not a
    /// directory, or refuses the probe file.
    pub async fn check(&self) -> Result<(), StoreError> {
        let root = self.root.clone();
        tokio::task::spawn_blocking(move || probe_root(&root))
            .await
            .map_err(|error| StoreError(format!("the object-store probe failed: {error}")))?
    }

    /// Resolve a key to a path inside the root, refusing anything that could
    /// leave it.
    ///
    /// Each segment is mapped to a Windows-safe filename. The mapping is
    /// deterministic and used by `put`, `get`, and `exists` alike.
    fn path(&self, key: &str) -> Result<PathBuf, StoreError> {
        validate_key(key)?;
        let mut path = self.root.clone();
        for segment in key.split('/') {
            let encoded = encode_segment(segment);
            if encoded.len() > 255 || encoded.encode_utf16().count() > 255 {
                return Err(StoreError(format!(
                    "the object key {key:?} has a segment that is not representable as a Windows filename"
                )));
            }
            path.push(encoded);
        }
        Ok(path)
    }
}

impl ObjectStore for FilesystemStore {
    fn backend(&self) -> Backend {
        Backend::Filesystem
    }

    fn put(&self, key: &str, bytes: Vec<u8>) -> BoxFuture<Result<(), StoreError>> {
        let resolved = self
            .path(key)
            .and_then(|path| verify_content(key, &bytes).map(|()| path));
        let key = key.to_owned();
        Box::pin(async move {
            let path = resolved?;
            tokio::task::spawn_blocking(move || publish(&path, &bytes))
                .await
                .map_err(|error| StoreError(format!("the writer for {key} failed: {error}")))?
                .map_err(|error| StoreError(format!("write {key}: {error}")))
        })
    }

    fn get(&self, key: &str) -> BoxFuture<Result<Vec<u8>, StoreError>> {
        let resolved = self.path(key);
        let key = key.to_owned();
        Box::pin(async move {
            let path = resolved?;
            let bytes = tokio::fs::read(&path).await.map_err(|error| {
                if error.kind() == io::ErrorKind::NotFound {
                    StoreError(format!("object {key} does not exist"))
                } else {
                    StoreError(format!("read {key}: {error}"))
                }
            })?;
            verify_content(&key, &bytes)?;
            Ok(bytes)
        })
    }

    fn exists(&self, key: &str) -> BoxFuture<Result<bool, StoreError>> {
        let resolved = self.path(key);
        let key = key.to_owned();
        Box::pin(async move {
            let path = resolved?;
            match tokio::fs::metadata(&path).await {
                Ok(metadata) => Ok(metadata.is_file()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
                Err(error) => Err(StoreError(format!("inspect {key}: {error}"))),
            }
        })
    }
}

/// Create the root and prove it accepts a file.
///
/// The probe file carries the temporary prefix, so a reader that lists the
/// directory during the probe cannot mistake it for an object; it is removed
/// before the probe returns.
fn probe_root(root: &Path) -> Result<(), StoreError> {
    std::fs::create_dir_all(root)
        .map_err(|error| StoreError(format!("cannot create {}: {error}", root.display())))?;
    let path = root.join(format!("{TEMP_PREFIX}probe-{}", std::process::id()));
    let written = std::fs::write(&path, []).and_then(|()| std::fs::remove_file(&path));
    written.map_err(|error| {
        StoreError(format!(
            "the object-store root {} is not writable: {error}",
            root.display()
        ))
    })
}

/// Refuse a key that is not a portable relative path.///
/// The rule mirrors the gateway's artifact-name check: absolute, empty, `.`,
/// `..`, and backslash segments are refused. Characters a common filesystem
/// cannot store portably are refused too, so a key that is accepted on Linux
/// is not silently mangled on another platform.
fn validate_key(key: &str) -> Result<(), StoreError> {
    if key.is_empty() {
        return Err(StoreError("an object key cannot be empty".to_owned()));
    }
    if key.len() > 1024 {
        return Err(StoreError(format!(
            "the object key is {} bytes; the limit is 1024",
            key.len()
        )));
    }
    if key.starts_with('/') || key.starts_with('\\') {
        return Err(StoreError(format!("the object key {key:?} is absolute")));
    }
    for character in key.chars() {
        if character == '\\' {
            return Err(StoreError(format!(
                "the object key {key:?} contains a backslash"
            )));
        }
        if character.is_control() {
            return Err(StoreError(format!(
                "the object key {key:?} contains a control character"
            )));
        }
        if matches!(character, '<' | '>' | '"' | '|' | '?' | '*') {
            return Err(StoreError(format!(
                "the object key {key:?} contains the reserved character {character:?}"
            )));
        }
    }
    for segment in key.split('/') {
        if segment.is_empty() {
            return Err(StoreError(format!(
                "the object key {key:?} has an empty segment"
            )));
        }
        if segment == "." || segment == ".." {
            return Err(StoreError(format!(
                "the object key {key:?} has a {segment:?} segment"
            )));
        }
        if segment.len() > 255 {
            return Err(StoreError(format!(
                "the object key {key:?} has a segment longer than 255 bytes"
            )));
        }
    }
    Ok(())
}

/// Windows device names are reserved as a file name and as the stem before an
/// extension. A key segment matching one is encoded rather than refused, so a
/// logical key such as `runs/CON/manifest.json` still round-trips.
const WINDOWS_RESERVED_STEMS: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "CLOCK$", "COM1", "COM2", "COM3", "COM4",
    "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7",
    "LPT8", "LPT9", "COM¹", "COM²", "COM³", "LPT¹", "LPT²", "LPT³",
];

const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";

/// Map one key segment to a filename every supported filesystem accepts.
///
/// The mapping is injective: `%` is always escaped, so a literal escape
/// sequence in the key can never collide with an encoded character. Colon and
/// the characters a Windows filename forbids are percent-encoded. A trailing
/// dot or space is encoded because Windows strips it during path resolution,
/// and the first character of a reserved device name is encoded so `CON` and
/// `CON.txt` do not resolve to a device.
fn encode_segment(segment: &str) -> String {
    let trailing = segment.trim_end_matches(['.', ' ']).len();
    let reserved = is_windows_reserved(segment);
    let mut encoded = String::with_capacity(segment.len());
    for (index, character) in segment.char_indices() {
        let encode = character == '%'
            || character == ':'
            || is_windows_invalid(character)
            || (index >= trailing && matches!(character, '.' | ' '))
            || (reserved && index == 0);
        if encode {
            push_percent_encoded(&mut encoded, character);
        } else {
            encoded.push(character);
        }
    }
    encoded
}

/// Whether a segment names a Windows device before its first extension.
fn is_windows_reserved(segment: &str) -> bool {
    let stem = segment.split('.').next().unwrap_or(segment);
    let stem = stem.trim_end_matches([' ', '.']);
    WINDOWS_RESERVED_STEMS
        .iter()
        .any(|reserved| stem.eq_ignore_ascii_case(reserved))
}

/// The characters Windows refuses in a file name. Most are rejected by
/// [`validate_key`]; encoding them here keeps the mapping itself total.
fn is_windows_invalid(character: char) -> bool {
    character.is_control() || matches!(character, '<' | '>' | '"' | '|' | '?' | '*' | '\\')
}

/// Append one UTF-8 character as uppercase percent-encoded bytes.
fn push_percent_encoded(encoded: &mut String, character: char) {
    let mut buffer = [0_u8; 4];
    for byte in character.encode_utf8(&mut buffer).bytes() {
        encoded.push('%');
        encoded.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
    }
}

/// The digest a content-addressed key promises, when the key carries one.
///
/// The system stores an object under the SHA-256 of its bytes, so a bare digest
/// as the last segment is the object's address: `artifacts/{sha256}` and
/// `datasets/{sha256}` are the current layouts. A `runs/{run_id}/manifest.json`
/// key names a run, not the manifest bytes, so it is not verified against its
/// name.
fn content_digest(key: &str) -> Option<&str> {
    let digest = key.rsplit('/').next()?;
    if digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Some(digest)
    } else {
        None
    }
}

/// Reject bytes that do not match a content address in the key.
fn verify_content(key: &str, bytes: &[u8]) -> Result<(), StoreError> {
    if let Some(expected) = content_digest(key) {
        let actual = sha256_hex(bytes);
        if actual != expected {
            return Err(StoreError(format!(
                "the bytes under {key} hash to sha256:{actual}"
            )));
        }
    }
    Ok(())
}

/// Publish an immutable object, or confirm an identical one is already there.
///
/// A retry that lands on the same bytes succeeds without a second write, which
/// keeps a republish atomic even on a platform whose rename cannot replace an
/// existing file. A different body under an existing key is a defect: every
/// current key is content-addressed or unique per run, so there is nothing to
/// replace.
fn publish(path: &Path, bytes: &[u8]) -> io::Result<()> {
    match std::fs::read(path) {
        Ok(existing) if existing == bytes => return Ok(()),
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "an object with different bytes already exists under this key",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    write_atomic(path, bytes)
}

/// Atomically publish `bytes` at `path`.
///
/// The temporary file lives in the destination directory, so the rename stays
/// on one filesystem. The file is fsynced before the rename and the directory
/// after it: a crash leaves either the previous object or the complete new one,
/// and a stale temporary file is never visible under the key.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp = stage_temp(path, bytes)?;
    if let Err(error) = commit_temp(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(error);
    }
    Ok(())
}

/// Write a complete temporary file next to `path` without publishing it.
///
/// A caller that stops here simulates a killed writer: the final key is absent
/// and the partial bytes stay in a file no reader resolves.
fn stage_temp(path: &Path, bytes: &[u8]) -> io::Result<PathBuf> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "the object path has no parent")
    })?;
    std::fs::create_dir_all(parent)?;
    loop {
        let temp = parent.join(format!(
            "{TEMP_PREFIX}{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        let written = file.write_all(bytes).and_then(|()| file.sync_all());
        if let Err(error) = written {
            drop(file);
            let _ = std::fs::remove_file(&temp);
            return Err(error);
        }
        return Ok(temp);
    }
}

/// Rename a temporary file onto its final key and make the rename durable.
fn commit_temp(temp: &Path, path: &Path) -> io::Result<()> {
    std::fs::rename(temp, path)?;
    sync_directory(path)
}

/// Fsync the directory holding `path`, so the rename survives a power loss.
fn sync_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::fs::File;
        if let Some(parent) = path.parent() {
            File::open(parent)?.sync_all()?;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use quantrule_api::Digest;

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);

    fn test_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "quantrule-filesystem-{}-{tag}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn content_key(bytes: &[u8]) -> String {
        format!("artifacts/{}", Digest::sha256(bytes).as_hex())
    }

    #[tokio::test]
    async fn a_put_is_readable_and_reports_the_filesystem_backend() {
        let root = test_root("put");
        let store = FilesystemStore::new(&root);
        let key = content_key(b"one");

        store.put(&key, b"one".to_vec()).await.expect("put");
        assert_eq!(store.get(&key).await.expect("get"), b"one");
        assert!(store.exists(&key).await.expect("exists"));
        assert!(!store.exists("artifacts/absent").await.expect("missing"));
        assert_eq!(store.backend(), Backend::Filesystem);

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[tokio::test]
    async fn a_nested_key_creates_its_directories() {
        let root = test_root("nested");
        let store = FilesystemStore::new(&root);
        let key = "runs/sha256:0123456789abcdef/manifest.json";

        store.put(key, b"{}".to_vec()).await.expect("put");
        assert_eq!(store.get(key).await.expect("get"), b"{}");
        let path = store.path(key).expect("resolve");
        assert!(path.is_file());
        let relative = path.strip_prefix(&root).expect("inside the root");
        assert!(!relative.to_string_lossy().contains(':'));

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[tokio::test]
    async fn a_run_key_with_a_colon_round_trips_without_a_colon_on_disk() {
        let root = test_root("colon");
        let store = FilesystemStore::new(&root);
        let hex = Digest::sha256(b"run identity").as_hex().to_owned();
        let key = format!("runs/sha256:{hex}/manifest.json");
        let path = store.path(&key).expect("resolve");
        let relative = path.strip_prefix(&root).expect("inside the root");
        assert!(!relative.to_string_lossy().contains(':'));
        assert!(
            relative.ends_with(
                Path::new("runs")
                    .join(format!("sha256%3A{hex}"))
                    .join("manifest.json")
            )
        );

        store.put(&key, b"manifest".to_vec()).await.expect("put");
        assert!(store.exists(&key).await.expect("exists"));
        assert_eq!(store.get(&key).await.expect("get"), b"manifest");
        store
            .put(&key, b"manifest".to_vec())
            .await
            .expect("an identical republish is a no-op");
        assert!(path.is_file());

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[tokio::test]
    async fn a_percent_in_a_key_is_escaped_and_does_not_collide_with_an_escape() {
        let root = test_root("percent");
        let store = FilesystemStore::new(&root);
        let hex = Digest::sha256(b"run identity").as_hex().to_owned();
        let colon = format!("runs/sha256:{hex}/manifest.json");
        let literal_escape = format!("runs/sha256%3A{hex}/manifest.json");
        let plain = "runs/50%/manifest.json";
        let escaped = "runs/50%25/manifest.json";

        let colon_relative = store
            .path(&colon)
            .expect("colon")
            .strip_prefix(&root)
            .expect("inside")
            .to_path_buf();
        let literal_relative = store
            .path(&literal_escape)
            .expect("literal")
            .strip_prefix(&root)
            .expect("inside")
            .to_path_buf();
        assert_ne!(colon_relative, literal_relative);
        assert_eq!(
            literal_relative,
            Path::new("runs")
                .join(format!("sha256%253A{hex}"))
                .join("manifest.json")
        );
        assert_eq!(
            store
                .path(plain)
                .expect("plain")
                .strip_prefix(&root)
                .expect("inside"),
            Path::new("runs").join("50%25").join("manifest.json")
        );
        assert_eq!(
            store
                .path(escaped)
                .expect("escaped")
                .strip_prefix(&root)
                .expect("inside"),
            Path::new("runs").join("50%2525").join("manifest.json")
        );

        store.put(plain, b"percent".to_vec()).await.expect("put");
        store.put(escaped, b"escaped".to_vec()).await.expect("put");
        assert_eq!(store.get(plain).await.expect("get"), b"percent");
        assert_eq!(store.get(escaped).await.expect("get"), b"escaped");

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[tokio::test]
    async fn windows_reserved_names_and_trailing_characters_are_encoded() {
        let root = test_root("reserved");
        let store = FilesystemStore::new(&root);
        for (key, expected) in [
            ("runs/CON/manifest.json", "runs/%43ON/manifest.json"),
            ("runs/con.txt/manifest.json", "runs/%63on.txt/manifest.json"),
            ("runs/COM1/manifest.json", "runs/%43OM1/manifest.json"),
            (
                "runs/trailing./manifest.json",
                "runs/trailing%2E/manifest.json",
            ),
            (
                "runs/trailing /manifest.json",
                "runs/trailing%20/manifest.json",
            ),
            ("runs/.../manifest.json", "runs/%2E%2E%2E/manifest.json"),
        ] {
            let path = store.path(key).expect("resolve");
            let relative = path.strip_prefix(&root).expect("inside the root");
            assert_eq!(relative, Path::new(expected), "{key}");
            assert!(!relative.to_string_lossy().contains(':'), "{key}");
            store.put(key, b"body".to_vec()).await.expect("put");
            assert_eq!(store.get(key).await.expect("get"), b"body");
            assert!(store.exists(key).await.expect("exists"));
        }

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[tokio::test]
    async fn a_staged_object_is_not_visible_until_it_is_committed() {
        let root = test_root("staged");
        let store = FilesystemStore::new(&root);
        let key = "runs/run/manifest.json";
        let target = store.path(key).expect("resolve");

        // The temporary file holds the full body, but no reader resolves it.
        let temp = stage_temp(&target, b"manifest").expect("stage");
        assert!(!store.exists(key).await.expect("exists"));
        assert!(store.get(key).await.is_err());

        commit_temp(&temp, &target).expect("commit");
        assert_eq!(store.get(key).await.expect("get"), b"manifest");

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[tokio::test]
    async fn an_identical_republish_succeeds_and_a_different_body_is_refused() {
        let root = test_root("immutable");
        let store = FilesystemStore::new(&root);

        let key = "runs/run/manifest.json";
        store.put(key, b"manifest".to_vec()).await.expect("first");
        store
            .put(key, b"manifest".to_vec())
            .await
            .expect("an identical republish is a no-op");
        let error = store
            .put(key, b"another manifest".to_vec())
            .await
            .expect_err("a different body must be refused");
        assert!(error.0.contains("different bytes"), "{error}");
        assert_eq!(store.get(key).await.expect("get"), b"manifest");

        // The same rule applies to a content-addressed retry.
        let bytes = b"artifact".to_vec();
        let content = content_key(&bytes);
        store
            .put(&content, bytes.clone())
            .await
            .expect("first blob");
        store
            .put(&content, bytes.clone())
            .await
            .expect("an identical blob republish is a no-op");
        assert_eq!(store.get(&content).await.expect("get"), bytes);

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[tokio::test]
    async fn a_killed_writer_leaves_no_visible_object() {
        let root = test_root("killed");
        let store = FilesystemStore::new(&root);
        let key = "runs/run/manifest.json";
        let target = store.path(key).expect("resolve");

        // A kill between the temporary write and the rename leaves the temp.
        let temp = stage_temp(&target, b"partial").expect("stage");
        assert!(temp.is_file());
        assert!(!store.exists(key).await.expect("exists"));
        assert!(store.get(key).await.is_err());

        // A restarted process reads the same directories and still sees nothing.
        let restarted = FilesystemStore::new(&root);
        assert!(!restarted.exists(key).await.expect("exists"));
        assert!(restarted.get(key).await.is_err());

        // A later complete publish is the first thing any reader can see.
        store.put(key, b"complete".to_vec()).await.expect("put");
        assert_eq!(restarted.get(key).await.expect("get"), b"complete");

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    const KILL_TARGET_ENV: &str = "QR_FILESYSTEM_KILL_TARGET";
    const KILL_MARKER_ENV: &str = "QR_FILESYSTEM_KILL_MARKER";

    /// The child half of the process-kill test.
    ///
    /// It stages a complete temporary file, records its path, and sleeps until
    /// the parent kills it before the rename. Run by the normal suite without
    /// the environment variable, where it does nothing.
    #[test]
    fn staged_write_helper_for_the_kill_test() {
        let Some(target) = std::env::var_os(KILL_TARGET_ENV) else {
            return;
        };
        let marker = std::env::var_os(KILL_MARKER_ENV).expect("the kill test sets a marker path");
        let temp = stage_temp(Path::new(&target), b"partial").expect("stage");
        std::fs::write(&marker, temp.display().to_string()).expect("marker");
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }

    #[tokio::test]
    async fn a_killed_writer_process_leaves_no_visible_object() {
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};

        let root = test_root("process-kill");
        let key = "runs/run/manifest.json";
        let store = FilesystemStore::new(&root);
        let target = store.path(key).expect("resolve");
        let marker = root.join("kill-marker");
        std::fs::create_dir_all(&root).expect("root");

        let mut child = Command::new(std::env::current_exe().expect("the test binary"))
            .args([
                "--exact",
                "adapters::filesystem::tests::staged_write_helper_for_the_kill_test",
                "--nocapture",
            ])
            .env(KILL_TARGET_ENV, &target)
            .env(KILL_MARKER_ENV, &marker)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn the writer");

        // Wait for the child to stage the object, then kill it at that boundary.
        let deadline = Instant::now() + Duration::from_secs(10);
        while !marker.exists() {
            if let Some(status) = child.try_wait().expect("inspect the writer") {
                panic!("the writer exited before it staged the object: {status}");
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("the writer did not stage the object within ten seconds");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        child.kill().expect("kill the writer");
        child.wait().expect("reap the writer");

        let temp = std::fs::read_to_string(&marker).expect("read the marker");
        assert!(
            Path::new(&temp).is_file(),
            "the staged temporary file remains after the kill"
        );
        assert!(!store.exists(key).await.expect("exists"));
        assert!(store.get(key).await.is_err());
        let restarted = FilesystemStore::new(&root);
        assert!(!restarted.exists(key).await.expect("exists"));

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn only_a_bare_digest_last_segment_is_content_addressed() {
        let hex = Digest::sha256(b"x").as_hex().to_owned();
        let artifact = format!("artifacts/{hex}");
        let dataset = format!("datasets/{hex}");
        let nested = format!("runs/{hex}/manifest.json");
        assert_eq!(content_digest(&artifact), Some(hex.as_str()));
        assert_eq!(content_digest(&dataset), Some(hex.as_str()));
        assert_eq!(content_digest(&nested), None);
        assert_eq!(content_digest("artifacts/not-a-digest"), None);
    }

    #[tokio::test]
    async fn a_corrupt_object_fails_the_content_check() {
        let root = test_root("corrupt");
        let store = FilesystemStore::new(&root);
        let key = content_key(b"expected");
        let path = store.path(&key).expect("resolve");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        std::fs::write(&path, b"corrupt").expect("plant");

        let error = store.get(&key).await.expect_err("corrupt bytes must fail");
        assert!(error.0.contains("hash to sha256:"), "{error}");

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[tokio::test]
    async fn a_put_that_does_not_match_its_address_writes_nothing() {
        let root = test_root("mismatch");
        let store = FilesystemStore::new(&root);
        let key = content_key(b"expected");

        let error = store
            .put(&key, b"other".to_vec())
            .await
            .expect_err("a mismatch must be refused");
        assert!(error.0.contains("hash to sha256:"), "{error}");
        assert!(!store.exists(&key).await.expect("exists"));

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_path_outside_the_root_is_refused() {
        let root = test_root("traversal");
        let store = FilesystemStore::new(&root);
        let long = "a".repeat(256);
        for key in [
            "",
            "/etc/passwd",
            "\\windows\\system32",
            "../escape",
            "artifacts/../../escape",
            "runs//manifest.json",
            "runs/./manifest.json",
            "runs/..",
            "runs\\manifest.json",
            "runs/manifest\n.json",
            "runs/manifest?.json",
            long.as_str(),
        ] {
            assert!(
                store.get(key).await.is_err(),
                "{key:?} must be refused on read"
            );
            assert!(
                store.put(key, b"x".to_vec()).await.is_err(),
                "{key:?} must be refused on write"
            );
            assert!(
                store.exists(key).await.is_err(),
                "{key:?} must be refused on exists"
            );
        }
        assert!(!root.exists(), "no refused key may create the root");

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn the_same_bytes_at_the_same_key_hash_equal() {
        let root = test_root("digest");
        let store = FilesystemStore::new(&root);
        let bytes = b"deterministic artifact".to_vec();
        let key = content_key(&bytes);

        store.put(&key, bytes.clone()).await.expect("first");
        let first = store.get(&key).await.expect("read first");
        store.put(&key, bytes).await.expect("second");
        let second = store.get(&key).await.expect("read second");

        assert_eq!(first, second);
        assert_eq!(Digest::sha256(&first), Digest::sha256(&second));

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[tokio::test]
    async fn the_store_is_usable_behind_the_object_store_trait() {
        use std::sync::Arc;

        let root = test_root("trait");
        let store: Arc<dyn ObjectStore> = Arc::new(FilesystemStore::new(&root));
        let bytes = b"trait object".to_vec();
        let key = content_key(&bytes);

        store.put(&key, bytes.clone()).await.expect("put");
        assert_eq!(store.backend(), Backend::Filesystem);
        assert_eq!(store.get(&key).await.expect("get"), bytes);
        assert!(store.exists(&key).await.expect("exists"));

        std::fs::remove_dir_all(&root).expect("cleanup");
    }
}
