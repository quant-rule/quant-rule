//! Contract tests for the Compose backup and restore tools.
//!
//! These tests are Docker-free. They build a scratch v2 archive with four
//! database dumps and one object, then exercise `tools/restore.sh --check` on a
//! complete archive, a partial set, and a changed payload. The live round trip
//! remains `tools/test_backup.sh`, which needs Docker and is not part of the
//! fast tier.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

const DATABASES: [&str; 4] = ["qr_gateway", "qr_catalog", "qr_evidence", "qr_simulation"];

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-backup-tools-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn hash(bytes: &[u8]) -> String {
    quantrule_api::sha256_hex(bytes)
}

struct Payload {
    path: String,
    bytes: Vec<u8>,
    database: Option<&'static str>,
    key: Option<String>,
}

fn payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();
    for database in DATABASES {
        let bytes = format!("custom-format dump for {database}\n").into_bytes();
        payloads.push(Payload {
            path: format!("postgres/{database}.dump"),
            bytes,
            database: Some(database),
            key: None,
        });
    }
    let object = b"scratch object bytes\n".to_vec();
    let object_hash = hash(&object);
    payloads.push(Payload {
        path: format!("objects/artifacts/{object_hash}"),
        bytes: object,
        database: None,
        key: Some(format!("artifacts/{object_hash}")),
    });
    payloads
}

fn write_archive(directory: &Path, name: &str, payloads: &[Payload]) -> PathBuf {
    let root = directory.join(name);
    std::fs::create_dir_all(&root).expect("root");
    let mut manifest = String::from(
        "# quantrule.backup/v2\n\
         # fields: kind|archive-path|bytes|sha256|database|object-key\n",
    );
    for payload in payloads {
        let destination = root.join(&payload.path);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).expect("payload dir");
        }
        std::fs::write(&destination, &payload.bytes).expect("payload");
        let digest = hash(&payload.bytes);
        let key = payload.key.as_deref().unwrap_or_default();
        if let Some(database) = payload.database {
            manifest.push_str(&format!(
                "database|{}|{}|{}|{database}|\n",
                payload.path,
                payload.bytes.len(),
                digest
            ));
        } else {
            manifest.push_str(&format!(
                "object|{}|{}|{}||{}\n",
                payload.path,
                payload.bytes.len(),
                digest,
                key
            ));
        }
    }
    std::fs::write(root.join("manifest.json"), manifest).expect("manifest");
    let archive = directory.join(format!("{name}.tar.gz"));
    let status = Command::new("tar")
        .args(["-C", directory.to_str().expect("path"), "-czf"])
        .arg(&archive)
        .arg(name)
        .status()
        .expect("tar");
    assert!(status.success());
    let sidecar = directory.join(format!("{name}.tar.gz.sha256"));
    std::fs::write(
        &sidecar,
        format!(
            "{}  {name}.tar.gz\n",
            hash(&std::fs::read(&archive).expect("archive"))
        ),
    )
    .expect("sidecar");
    archive
}

fn restore_check(archive: &Path) -> (bool, String, String) {
    let output = Command::new(repo("tools/restore.sh"))
        .arg(archive)
        .arg("--check")
        .output()
        .expect("restore --check");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn a_complete_scratch_archive_passes_the_check_mode() {
    let directory = temp_dir();
    let archive = write_archive(&directory, "quantrule-scratch", &payloads());
    let (ok, stdout, stderr) = restore_check(&archive);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("databases: 4 dumps"), "{stdout}");
    assert!(stdout.contains("objects: 1 files"), "{stdout}");
    assert!(stdout.contains("no stack was touched"), "{stdout}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_partial_four_database_set_is_refused() {
    let directory = temp_dir();
    let mut payloads = payloads();
    payloads.retain(|payload| payload.database != Some("qr_simulation"));
    let archive = write_archive(&directory, "quantrule-partial", &payloads);
    let (ok, _, stderr) = restore_check(&archive);
    assert!(!ok, "{stderr}");
    assert!(
        stderr.contains("partial four-database set"),
        "the refusal did not name the partial set: {stderr}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_changed_payload_is_refused_by_its_hash() {
    let directory = temp_dir();
    let archive = write_archive(&directory, "quantrule-changed", &payloads());
    let unpack = directory.join("unpack");
    std::fs::create_dir_all(&unpack).expect("unpack");
    let status = Command::new("tar")
        .args(["-C", unpack.to_str().expect("path"), "-xzf"])
        .arg(&archive)
        .status()
        .expect("extract");
    assert!(status.success());
    let dump = unpack.join("quantrule-changed/postgres/qr_gateway.dump");
    let mut bytes = std::fs::read(&dump).expect("dump");
    bytes[0] = b'X';
    std::fs::write(&dump, bytes).expect("tamper");
    let changed = directory.join("quantrule-changed.tar.gz");
    std::fs::remove_file(&changed).expect("remove original");
    let status = Command::new("tar")
        .args(["-C", unpack.to_str().expect("path"), "-czf"])
        .arg(&changed)
        .arg("quantrule-changed")
        .status()
        .expect("archive");
    assert!(status.success());
    std::fs::write(
        directory.join("quantrule-changed.tar.gz.sha256"),
        format!(
            "{}  quantrule-changed.tar.gz\n",
            hash(&std::fs::read(&changed).expect("changed"))
        ),
    )
    .expect("sidecar");
    let (ok, _, stderr) = restore_check(&changed);
    assert!(!ok, "{stderr}");
    assert!(stderr.contains("hash mismatch"), "{stderr}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn every_changed_script_parses_and_the_plan_names_the_four_databases() {
    for script in [
        "tools/backup.sh",
        "tools/backup_lib.sh",
        "tools/restore.sh",
        "tools/test_backup.sh",
        "tools/stateful.sh",
        "tools/release.sh",
        "deploy/v3/images/patroni/build.sh",
    ] {
        let status = Command::new("bash")
            .arg("-n")
            .arg(repo(script))
            .status()
            .expect("bash -n");
        assert!(status.success(), "{script} does not parse");
    }

    let output = Command::new(repo("tools/test_backup.sh"))
        .arg("--plan")
        .output()
        .expect("test_backup --plan");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for database in DATABASES {
        assert!(stdout.contains(database), "{database}: {stdout}");
    }
    assert!(stdout.contains("no Docker"), "{stdout}");

    let output = Command::new(repo("deploy/v3/images/patroni/build.sh"))
        .arg("--plan")
        .output()
        .expect("build --plan");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("batonogov/patroni-docker@sha256:"),
        "{stdout}"
    );
    assert!(stdout.contains("no push and no pull"), "{stdout}");
}
