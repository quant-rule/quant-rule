//! pgBackRest configuration and the backup-host script for the v3 tier.
//!
//! One stanza, `quantrule`, covers all four logical databases because they share
//! one PostgreSQL cluster. Every Patroni host receives the same repository
//! configuration and pushes WAL with `archive-push`; the backup host runs the
//! rendered script, which executes `pgbackrest` inside the Patroni container of
//! the chosen node and writes the backup-age textfile only after a real success.
//!
//! The passphrase and S3 credentials are never written into the configuration,
//! a command line, or a log. The Patroni supervisor exports them from mounted
//! mode-600 files for `archive-push` and `archive-get`, and the script sends
//! them on standard input to `docker exec`.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::descriptor::v3::{
    BackupRepoKind, DEFAULT_BACKUP_PATH, Descriptor, PGBACKREST_IMAGE_REFERENCE,
};
use crate::error::Error;
use crate::validate::secrets::{BACKUP_CIPHER_SECRET, BACKUP_S3_SECRETS};

/// The only pgBackRest stanza.
pub const STANZA: &str = "quantrule";

/// The in-container pgBackRest configuration path.
pub const CONFIG_TARGET: &str = "/etc/pgbackrest/pgbackrest.conf";

/// The backup-host script path.
pub const SCRIPT_TARGET: &str = "/etc/quantrule/backup/pgbackrest-backup.sh";

/// The rendered relative path of one Patroni node's repository configuration.
#[must_use]
pub fn config_source(host: &str) -> String {
    format!("pgbackrest-{host}.conf")
}

/// The rendered relative path of the backup-host script.
pub const SCRIPT_SOURCE: &str = "pgbackrest-backup.sh";

/// The backup host: the explicit field or the first Swarm manager.
#[must_use]
pub fn backup_host(descriptor: &Descriptor) -> Option<&str> {
    descriptor.backup.host(descriptor)
}

/// The posix repository path, including the documented default.
#[must_use]
pub fn posix_path(descriptor: &Descriptor) -> Option<&str> {
    (descriptor.backup.repo.kind == BackupRepoKind::Posix).then(|| {
        descriptor
            .backup
            .repo
            .path
            .as_deref()
            .unwrap_or(DEFAULT_BACKUP_PATH)
    })
}

/// Whether the repository is in the cluster's own MinIO rather than off-host.
///
/// A posix repository is in-cluster when the backup host is also a MinIO node.
/// An S3 repository is in-cluster when its endpoint is the object-store alias or
/// a MinIO node name. Only the lab may use an in-cluster repository, and it must
/// carry the rendered `not-off-host` label.
#[must_use]
pub fn in_cluster_repo(descriptor: &Descriptor) -> bool {
    match descriptor.backup.repo.kind {
        BackupRepoKind::Posix => backup_host(descriptor).is_some_and(|host| {
            descriptor
                .stateful
                .minio
                .nodes
                .iter()
                .any(|node| node == host)
        }),
        BackupRepoKind::S3 => {
            let endpoint = descriptor.backup.repo.endpoint.as_deref().unwrap_or("");
            let host = endpoint.split(':').next().unwrap_or(endpoint);
            host == descriptor.endpoints.object_store.host
                || descriptor
                    .stateful
                    .minio
                    .nodes
                    .iter()
                    .any(|node| node == host || host == format!("minio-{node}"))
        }
    }
}

/// Build one Patroni node's `pgbackrest.conf`.
///
/// # Errors
///
/// Returns a rule when the node host is not declared. Validation refuses an
/// incomplete repository before rendering; this is the defensive boundary.
pub fn config(descriptor: &Descriptor, host: &str) -> Result<String, Error> {
    if !descriptor.hosts.contains_key(host) {
        return Err(Error::rule(
            "deploy.host.undeclared",
            "stateful.postgres.nodes",
            format!("{host} is not declared in hosts"),
        ));
    }
    let mut global: BTreeMap<&str, String> = BTreeMap::new();
    global.insert("backup-standby", "y".to_owned());
    global.insert("log-level-console", "info".to_owned());
    global.insert("log-level-file", "detail".to_owned());
    global.insert(
        "repo1-cipher-type",
        descriptor.backup.cipher.as_str().to_owned(),
    );
    global.insert("repo1-retention-diff", "7".to_owned());

    global.insert(
        "repo1-retention-full",
        descriptor.backup.retention_days.to_string(),
    );
    global.insert("repo1-retention-full-type", "time".to_owned());
    match descriptor.backup.repo.kind {
        BackupRepoKind::Posix => {
            let path = posix_path(descriptor).unwrap_or(DEFAULT_BACKUP_PATH);
            global.insert("repo1-path", path.to_owned());
        }
        BackupRepoKind::S3 => {
            let endpoint = descriptor.backup.repo.endpoint.as_deref().unwrap_or("");
            let bucket = descriptor.backup.repo.bucket.as_deref().unwrap_or("");
            let path = descriptor.backup.repo.path.as_deref().unwrap_or("/");
            let tls_ca = descriptor.backup.repo.tls_ca.as_deref().unwrap_or("");
            global.insert("repo1-path", path.to_owned());
            global.insert("repo1-s3-bucket", bucket.to_owned());
            global.insert("repo1-s3-endpoint", endpoint.to_owned());
            global.insert("repo1-s3-region", "us-east-1".to_owned());
            global.insert("repo1-s3-uri-style", "path".to_owned());
            global.insert("repo1-storage-ca-file", tls_ca.to_owned());
            global.insert("repo1-storage-verify-tls", "y".to_owned());
        }
    }
    global.insert("start-fast", "y".to_owned());

    let mut stanza: BTreeMap<&str, String> = BTreeMap::new();
    stanza.insert("pg1-path", "/var/lib/postgresql/data".to_owned());
    stanza.insert("pg1-port", "5432".to_owned());
    stanza.insert("pg1-socket-path", "/run/postgresql".to_owned());

    let mut text = String::from(
        "# Generated by qr-deploy; copy to /etc/pgbackrest/pgbackrest.conf.\n\
         # The passphrase and S3 credentials are read from mode-600 files by the\n\
         # container command; this file carries no secret and no command line.\n\
         # One stanza covers all four logical databases of the one Patroni cluster.\n",
    );
    if in_cluster_repo(descriptor) {
        text.push_str(
            "# not-off-host: lab-only in-cluster MinIO repository; never a production off-host claim\n",
        );
    }
    text.push_str(&format!(
        "# image: {PGBACKREST_IMAGE_REFERENCE} (derived; pgBackRest is not in the base image)\n"
    ));
    for (name, options) in [("global", &global), ("quantrule", &stanza)] {
        let _ = writeln!(text, "[{name}]");
        for (key, value) in options {
            let _ = writeln!(text, "{key}={value}");
        }
        text.push('\n');
    }
    Ok(text)
}

/// Build the backup-host script.
///
/// # Errors
///
/// Returns a rule when the descriptor has no PostgreSQL node or no backup host.
pub fn script(descriptor: &Descriptor) -> Result<String, Error> {
    let node = descriptor
        .stateful
        .postgres
        .nodes
        .first()
        .map(String::as_str)
        .ok_or_else(|| {
            Error::rule(
                "deploy.stateful.postgres.nodes",
                "stateful.postgres.nodes",
                "at least one Patroni node is required",
            )
        })?;
    let host = script_host(descriptor)?;
    let s3 = descriptor.backup.repo.kind == BackupRepoKind::S3;
    let archive = &descriptor.backup.object_archive;
    let archive_line = if archive.enabled {
        format!(
            "enabled target={} retention_days={} (phase 3 owns the archive tool)",
            archive.target.as_deref().unwrap_or("-"),
            archive.retention_days.unwrap_or(0)
        )
    } else {
        "disabled (phase 3 owns the archive tool)".to_owned()
    };
    let inner_reads = if s3 {
        "IFS= read -r PGBACKREST_REPO1_CIPHER_PASS || [ -n \"$PGBACKREST_REPO1_CIPHER_PASS\" ]\n\
         IFS= read -r PGBACKREST_REPO1_S3_KEY || [ -n \"$PGBACKREST_REPO1_S3_KEY\" ]\n\
         IFS= read -r PGBACKREST_REPO1_S3_KEY_SECRET || [ -n \"$PGBACKREST_REPO1_S3_KEY_SECRET\" ]\n\
         export PGBACKREST_REPO1_CIPHER_PASS PGBACKREST_REPO1_S3_KEY PGBACKREST_REPO1_S3_KEY_SECRET"
            .to_owned()
    } else {
        "IFS= read -r PGBACKREST_REPO1_CIPHER_PASS || [ -n \"$PGBACKREST_REPO1_CIPHER_PASS\" ]\n\
         export PGBACKREST_REPO1_CIPHER_PASS"
            .to_owned()
    };
    let secret_cats = if s3 {
        format!(
            "cat \"$secret_dir/{BACKUP_CIPHER_SECRET}\"\n\
             cat \"$secret_dir/{}\"\n\
             cat \"$secret_dir/{}\"",
            BACKUP_S3_SECRETS[0], BACKUP_S3_SECRETS[1]
        )
    } else {
        format!("cat \"$secret_dir/{BACKUP_CIPHER_SECRET}\"")
    };
    let inner = format!("set -eu\n{inner_reads}\nexec pgbackrest --stanza={STANZA} \"$@\"");
    let template = r#"#!/usr/bin/env bash
# Run the pgBackRest full, differential, verify, info, or restore command on one
# Patroni node. The scripts are copied by tools/stateful.sh; the operator's host
# timer or cron invokes `full` at the full time and `diff` at the differential
# time. No scheduler runs inside a container.
#
# backup host: __HOST__
# Schedule: full __FULL_AT__ UTC, diff __DIFF_AT__ UTC; install a host systemd timer or cron.
# __REPO_LABEL__
# object_archive: __ARCHIVE_LINE__
#
# The passphrase and S3 credentials are mode-600 files under $secret_dir. They
# travel on standard input to `docker exec`, never in a command line, a rendered
# configuration, or a log. A stamp is written only after a real full or diff
# success, never on a dry-run, info, verify, restore, or failure.
#
# This script is phase 1a: it does not archive objects, rebuild the queue, or
# drill DR.
set -euo pipefail
umask 077

usage() {
  cat <<'USAGE'
Usage: pgbackrest-backup.sh <full|diff|verify|info|restore> [--dry-run] [--confirm]
  full      run pgbackrest backup --type=full and stamp the age textfile
  diff      run pgbackrest backup --type=diff and stamp the age textfile
  verify    run pgbackrest verify
  info      run pgbackrest info
  restore   print the phase-3 restore command; requires --confirm and stopped Patroni
Environment:
  PGBACKREST_NODE         Patroni host key; default __NODE__
  PGBACKREST_LOCAL        auto (default), 1 for local docker, 0 for ssh
  QR_BACKUP_SSH           ssh command and options
  QR_BACKUP_SUDO          remote privilege command; empty for root
  QR_BACKUP_DOCKER        docker executable
  QR_BACKUP_SECRET_DIR    mode-600 secret directory
  QR_BACKUP_TEXTFILE_DIR  node_exporter textfile directory
USAGE
}

command=${1:-}
shift || true
dry_run=0
confirm=0
for argument in "$@"; do
  case "$argument" in
    --dry-run) dry_run=1 ;;
    --confirm) confirm=1 ;;
    -h | --help) usage; exit 0 ;;
    *) printf 'backup: unknown argument %s\n' "$argument" >&2; exit 2 ;;
  esac
done

case "$command" in
  full) pgbackrest_args=(backup --type=full) ;;
  diff) pgbackrest_args=(backup --type=diff) ;;
  verify) pgbackrest_args=(verify) ;;
  info) pgbackrest_args=(info) ;;
  restore) pgbackrest_args=(restore --delta) ;;
  -h | --help | '') usage; exit 0 ;;
  *) printf 'backup: expected full, diff, verify, info, or restore\n' >&2; exit 2 ;;
esac

if [ "$command" = restore ] && [ "$confirm" -ne 1 ]; then
  printf 'backup: the phase-3 restore plan replaces the Patroni data directory on __NODE__; stop Patroni and re-run with --confirm\n' >&2
  exit 1
fi

node=${PGBACKREST_NODE:-__NODE__}
container=quantrule-patroni-$node
ssh_bin=${QR_BACKUP_SSH:-ssh -o BatchMode=yes -o ConnectTimeout=5}
sudo_bin=${QR_BACKUP_SUDO-sudo -n}
docker_bin=${QR_BACKUP_DOCKER:-docker}
secret_dir=${QR_BACKUP_SECRET_DIR:-/etc/quantrule/secrets}
textfile_dir=${QR_BACKUP_TEXTFILE_DIR:-/srv/quantrule/node_exporter/textfile}
textfile=$textfile_dir/quantrule_backup.prom

local_run=0
case "${PGBACKREST_LOCAL:-auto}" in
  1) local_run=1 ;;
  0) local_run=0 ;;
  auto) if $docker_bin inspect "$container" >/dev/null 2>&1; then local_run=1; fi ;;
  *) printf 'backup: PGBACKREST_LOCAL must be auto, 0, or 1\n' >&2; exit 2 ;;
esac

if [ "$local_run" -eq 0 ]; then
  command -v ssh >/dev/null 2>&1 || { printf 'backup: ssh is required\n' >&2; exit 1; }
fi
for tool in $docker_bin date mktemp chmod mv mkdir; do
  command -v "$tool" >/dev/null 2>&1 || { printf 'backup: %s is required\n' "$tool" >&2; exit 1; }
done

if [ "$dry_run" -eq 1 ]; then
  printf 'dry-run: backup host __HOST__, node %s, container %s\n' "$node" "$container"
  printf 'dry-run: pgbackrest --stanza=__STANZA__ %s\n' "${pgbackrest_args[*]}"
  printf 'dry-run: no secret is read, no repository is written, and no stamp is created\n'
  exit 0
fi

inner=$(cat <<'INNER'
__INNER__
INNER
)

run_node() {
  if [ "$local_run" -eq 1 ]; then
    $docker_bin exec -i "$container" sh -c "$inner" -- "$@"
  else
    local quoted=""
    local argument
    for argument in "$@"; do
      quoted="$quoted $argument"
    done
    $ssh_bin "$node" "$sudo_bin $docker_bin exec -i $container sh -c '$inner' --$quoted"
  fi
}

if [ "$command" = info ] || [ "$command" = verify ]; then
  { __SECRET_CATS__; } | run_node "${pgbackrest_args[@]}"
  exit 0
fi

if [ "$command" = full ] || [ "$command" = diff ]; then
  { __SECRET_CATS__; } | run_node "${pgbackrest_args[@]}"
  stamp=$(date -u +%s)
  mkdir -p "$textfile_dir"
  temporary=$(mktemp "$textfile_dir/.quantrule_backup.XXXXXX")
  {
    printf '# HELP quantrule_backup_last_success_timestamp_seconds Unix time of the last successful pgBackRest backup\n'
    printf '# TYPE quantrule_backup_last_success_timestamp_seconds gauge\n'
    printf 'quantrule_backup_last_success_timestamp_seconds %s\n' "$stamp"
  } > "$temporary"
  chmod 644 "$temporary"
  mv "$temporary" "$textfile"
  printf 'backup: %s complete; stamp %s\n' "$command" "$stamp"
  exit 0
fi

# restore: phase 1a prints the exact command and does not execute a restore.
printf 'backup: phase 1a does not execute a restore; stop Patroni on %s first.\n' "$node"
printf 'backup: run the following in the pinned Patroni image with the data volume and repository config:\n'
printf '  pgbackrest --stanza=__STANZA__ restore --delta\n'
printf 'backup: phase 3 owns the scratch drill, object archive, and restore evidence.\n'
"#;
    let repo_label = if in_cluster_repo(descriptor) {
        "not-off-host: lab-only in-cluster repository; never a production off-host claim"
    } else {
        "repository: off-host S3 or dedicated posix path"
    };
    Ok(template
        .replace("__NODE__", node)
        .replace("__HOST__", host)
        .replace("__STANZA__", STANZA)
        .replace("__FULL_AT__", &descriptor.backup.schedule.full_at)
        .replace("__DIFF_AT__", &descriptor.backup.schedule.diff_at)
        .replace("__REPO_LABEL__", repo_label)
        .replace("__ARCHIVE_LINE__", &archive_line)
        .replace("__SECRET_CATS__", &secret_cats)
        .replace("__INNER__", &inner))
}

/// The backup host the script is copied to.
///
/// # Errors
///
/// Returns a rule when no host resolves.
pub fn script_host(descriptor: &Descriptor) -> Result<&str, Error> {
    backup_host(descriptor).ok_or_else(|| {
        Error::rule(
            "deploy.backup.host",
            "backup.host",
            "must resolve to a declared host",
        )
    })
}
