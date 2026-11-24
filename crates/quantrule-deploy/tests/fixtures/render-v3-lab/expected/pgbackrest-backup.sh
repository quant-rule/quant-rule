#!/usr/bin/env bash
# Run the pgBackRest full, differential, verify, info, or restore command on one
# Patroni node. The scripts are copied by tools/stateful.sh; the operator's host
# timer or cron invokes `full` at the full time and `diff` at the differential
# time. No scheduler runs inside a container.
#
# backup host: lab-1
# Schedule: full 02:00 UTC, diff 14:00 UTC; install a host systemd timer or cron.
# not-off-host: lab-only in-cluster repository; never a production off-host claim
# object_archive: enabled target=quantrule-archive retention_days=30 (phase 3 owns the archive tool)
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
  PGBACKREST_NODE         Patroni host key; default lab-1
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
  printf 'backup: the phase-3 restore plan replaces the Patroni data directory on lab-1; stop Patroni and re-run with --confirm\n' >&2
  exit 1
fi

node=${PGBACKREST_NODE:-lab-1}
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
  printf 'dry-run: backup host lab-1, node %s, container %s\n' "$node" "$container"
  printf 'dry-run: pgbackrest --stanza=quantrule %s\n' "${pgbackrest_args[*]}"
  printf 'dry-run: no secret is read, no repository is written, and no stamp is created\n'
  exit 0
fi

inner=$(cat <<'INNER'
set -eu
IFS= read -r PGBACKREST_REPO1_CIPHER_PASS || [ -n "$PGBACKREST_REPO1_CIPHER_PASS" ]
IFS= read -r PGBACKREST_REPO1_S3_KEY || [ -n "$PGBACKREST_REPO1_S3_KEY" ]
IFS= read -r PGBACKREST_REPO1_S3_KEY_SECRET || [ -n "$PGBACKREST_REPO1_S3_KEY_SECRET" ]
export PGBACKREST_REPO1_CIPHER_PASS PGBACKREST_REPO1_S3_KEY PGBACKREST_REPO1_S3_KEY_SECRET
exec pgbackrest --stanza=quantrule "$@"
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
  { cat "$secret_dir/pgbackrest_cipher_pass"
cat "$secret_dir/pgbackrest_s3_access_key"
cat "$secret_dir/pgbackrest_s3_secret_key"; } | run_node "${pgbackrest_args[@]}"
  exit 0
fi

if [ "$command" = full ] || [ "$command" = diff ]; then
  { cat "$secret_dir/pgbackrest_cipher_pass"
cat "$secret_dir/pgbackrest_s3_access_key"
cat "$secret_dir/pgbackrest_s3_secret_key"; } | run_node "${pgbackrest_args[@]}"
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
printf '  pgbackrest --stanza=quantrule restore --delta\n'
printf 'backup: phase 3 owns the scratch drill, object archive, and restore evidence.\n'
