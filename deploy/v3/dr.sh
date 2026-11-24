#!/usr/bin/env bash
# Backup restore and disaster-recovery drill for the v3 lab.
#
# The default is a scratch drill: a fresh PGDATA and a fresh object bucket in
# their own Compose project, restored from the pgBackRest repository and the
# object archive, then the same fixed run/config must reproduce the canonical
# digest. `--in-place` is a separate, destructive lab path: it stops the app and
# every Patroni node, restores all lab PGDATA directories with `--delta`, and
# starts the lab again. It requires `--confirm`, the `lab-single-host` profile,
# and a fresh full pre-restore backup.
#
# The drill never runs `docker stack rm`, `docker volume rm`, `docker secret rm`,
# `docker config rm`, `docker node rm`, `docker service rm`, `docker swarm
# leave`, `pgbackrest expire`, `stanza-delete`, `mc rm` on the source, `DROP
# DATABASE`, `DROP SCHEMA`, or `rm -rf`. It prints state, digest, and timing only;
# it never prints a token, CA body, key, or inspect body.
#
# Usage:
#   deploy/v3/dr.sh --plan
#   deploy/v3/dr.sh --target scratch --confirm
#   deploy/v3/dr.sh --target in-place --confirm
#   deploy/v3/dr.sh --target scratch --confirm --backup-label 20261124-020000F
#   deploy/v3/dr.sh --target scratch --confirm --target-time 2026-11-24T02:00:00Z
#
# Options:
#   --plan | --dry-run       Print the selected target, phases, preconditions,
#                            evidence path, and labels; read no host or secret.
#   --target scratch|in-place  Target selector (default scratch).
#   --in-place               Shorthand for --target in-place.
#   --confirm                Required for any restore.
#   --descriptor FILE        v3 descriptor (default deploy/v3/deploy.lab.json).
#   --out DIR                Render directory (default deploy/v3/out).
#   --backup-label LABEL     Restore this pgBackRest label; no fresh backup.
#   --target-time UTC        PITR target; no fresh backup.
#   --target-lsn LSN         PITR target by LSN; no fresh backup.
#   --skip-backup            Use the newest backup; backup/RPO rows Unverified.
#   --skip-markers           Do not sample markers; RPO row Unverified.
#   --skip-objects           Do not restore objects; object rows Unverified.
#   --skip-queue             Do not rebuild/re-arm the queue; row Unverified.
#   --skip-digest            Do not run the canonical digest; row Unverified.
#   --evidence FILE          Evidence JSON (default deploy/v3/evidence/<drill>.json).
#   --marker-window SECONDS  Marker window before the backup (default 60).
#   --marker-interval SECONDS Marker interval (default 10).
#   -h | --help
#
# Environment:
#   DESCRIPTOR / OUT                 shared with the other v3 tools
#   QR_DEPLOY_BIN                    qr-deploy binary; otherwise qr-deploy, then cargo
#   QR_DR_SCRATCH_ROOT               scratch root (default /srv/quantrule/dr/<drill>)
#   QR_DR_SCRATCH_PROJECT            scratch Compose project (default quantrule-dr-<drill>)
#   QR_DR_SCRATCH_PORT               scratch gateway port (default 18080)
#   QR_DR_APP_IMAGE                  resolved app image reference@sha256:
#   QR_DR_PATRONI_IMAGE              resolved derived Patroni image reference@sha256:
#   QR_DR_ARCHIVE_SOURCE_URL         object archive source endpoint for the restore
#   QR_DR_ARCHIVE_SOURCE_ACCESS_KEY_FILE / _SECRET_KEY_FILE
#   QR_DR_ARCHIVE_EVIDENCE           last object-archive evidence path
#   QR_DR_ARCHIVE_INTERVAL_SECONDS   object archive interval (default 86400)
#   QR_DR_BACKUP_SCRIPT              rendered backup script (default out/pgbackrest-backup.sh)
#   QR_DR_BACKUP_SSH                 SSH command for the lab nodes
#   QR_DR_BACKUP_SUDO                remote privilege command (default "sudo -n")
#   QR_DR_CACERT / QR_DR_API_URL     in-place ingress CA and API base
#
# The three-VM lab is `Simulated HA` at most; a scratch drill is `Measured` for
# the operations run. `Real HA` is HA-014 and is never claimed here.
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "$script_dir/../.." && pwd)
cd "$root"
umask 077

descriptor=${DESCRIPTOR:-deploy/v3/deploy.lab.json}
out=${OUT:-deploy/v3/out}
mode=scratch
confirm=0
plan=0
skip_backup=0
skip_markers=0
skip_objects=0
skip_queue=0
skip_digest=0
evidence=""
marker_window=${QR_DR_MARKER_WINDOW:-60}
marker_interval=${QR_DR_MARKER_INTERVAL:-10}
DR_TARGET_TIME=${QR_DR_TARGET_TIME:-}
DR_TARGET_LSN=${QR_DR_TARGET_LSN:-}
DR_BACKUP_LABEL=${QR_DR_BACKUP_LABEL:-}

usage() {
  awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
}

usage_error() {
  printf 'dr: %s\n\n' "$*" >&2
  usage >&2
  exit 2
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --plan | --dry-run)
      plan=1
      shift
      ;;
    --confirm)
      confirm=1
      shift
      ;;
    --target)
      [ "$#" -ge 2 ] || usage_error "--target needs a value"
      mode=$2
      shift 2
      ;;
    --in-place)
      mode=in-place
      shift
      ;;
    --descriptor)
      [ "$#" -ge 2 ] || usage_error "--descriptor needs a value"
      descriptor=$2
      shift 2
      ;;
    --out)
      [ "$#" -ge 2 ] || usage_error "--out needs a value"
      out=$2
      shift 2
      ;;
    --backup-label)
      [ "$#" -ge 2 ] || usage_error "--backup-label needs a value"
      DR_BACKUP_LABEL=$2
      shift 2
      ;;
    --target-time)
      [ "$#" -ge 2 ] || usage_error "--target-time needs a value"
      DR_TARGET_TIME=$2
      shift 2
      ;;
    --target-lsn)
      [ "$#" -ge 2 ] || usage_error "--target-lsn needs a value"
      DR_TARGET_LSN=$2
      shift 2
      ;;
    --skip-backup)
      skip_backup=1
      shift
      ;;
    --skip-markers)
      skip_markers=1
      shift
      ;;
    --skip-objects)
      skip_objects=1
      shift
      ;;
    --skip-queue)
      skip_queue=1
      shift
      ;;
    --skip-digest)
      skip_digest=1
      shift
      ;;
    --evidence)
      [ "$#" -ge 2 ] || usage_error "--evidence needs a value"
      evidence=$2
      shift 2
      ;;
    --marker-window)
      [ "$#" -ge 2 ] || usage_error "--marker-window needs a value"
      marker_window=$2
      shift 2
      ;;
    --marker-interval)
      [ "$#" -ge 2 ] || usage_error "--marker-interval needs a value"
      marker_interval=$2
      shift 2
      ;;
    -h | --help | help)
      usage
      exit 0
      ;;
    *)
      usage_error "unknown argument $1"
      ;;
  esac
done

case "$mode" in
  scratch | in-place) ;;
  *) usage_error "--target must be scratch or in-place, not '$mode'" ;;
esac
case "$marker_window$marker_interval" in
  *[!0-9]*) usage_error "the marker window and interval must be whole seconds" ;;
esac
[ "$marker_window" -ge "$marker_interval" ] && [ "$marker_interval" -gt 0 ] \
  || usage_error "the marker window must be at least one interval"
if [ -n "$DR_TARGET_TIME" ] && [ -n "$DR_TARGET_LSN" ]; then
  usage_error "--target-time and --target-lsn are mutually exclusive"
fi
if [ -n "$DR_TARGET_LSN" ]; then
  DR_TARGET_TYPE=lsn
else
  DR_TARGET_TYPE=time
fi

drill_stamp=$(date -u +%Y%m%dT%H%M%SZ)
DR_DRILL_ID="dr-$drill_stamp-$mode"
DR_MODE=$mode
DR_CONFIRM=$confirm
DR_DESCRIPTOR=$descriptor
DR_RENDER=$out
DR_SKIP_BACKUP=$skip_backup
DR_SKIP_MARKERS=$skip_markers
DR_SKIP_OBJECTS=$skip_objects
DR_SKIP_QUEUE=$skip_queue
DR_SKIP_DIGEST=$skip_digest
DR_MARKER_WINDOW=$marker_window
DR_MARKER_INTERVAL=$marker_interval
DR_ROWS=${DR_ROWS:-}
DR_LABELS=${DR_LABELS:-Unverified}

if [ "$plan" -eq 1 ]; then
  cat <<PLAN
dr: plan (no Docker, SSH, jq, qr-deploy, or secret file is read)
dr: target: $mode
dr: descriptor: $descriptor
dr: render: $out
dr: backup basis: $([ "$skip_backup" -eq 1 ] && printf 'newest backup (row Unverified)' || printf 'fresh full backup after the marker window')${DR_BACKUP_LABEL:+ label $DR_BACKUP_LABEL}${DR_TARGET_TIME:+ time $DR_TARGET_TIME}${DR_TARGET_LSN:+ lsn $DR_TARGET_LSN}
dr: marker window: ${marker_window}s every ${marker_interval}s
dr: 1 preflight: descriptor, release images, object_replication false, source/target not the live PGDATA/project/port
dr: 2 digest: a fixed run/config through tools/verify.sh before the restore
dr: 3 markers: read-only (epoch, pg_current_wal_lsn) samples from the source primary
dr: 4 restore: $([ "$mode" = scratch ] && printf 'pgBackRest into a fresh PGDATA; objects into a fresh bucket; own Compose project' || printf 'stop apps and every lab Patroni node; pgbackrest restore --delta all nodes; objects before rows')
dr: 5 queue: $([ "$skip_queue" -eq 1 ] && printf 'skipped, row Unverified' || printf 'app user and evidence/stage outbox re-arm; never publish or replay work directly')
dr: 6 digest: rerun the same run/config and compare the canonical digest; a mismatch fails
dr: 7 evidence: ${evidence:-deploy/v3/evidence/dr-<UTC>-$mode.json}
dr: preconditions: $([ "$mode" = scratch ] && printf 'lab-single-host, Docker, a resolved app and derived Patroni image, SSH/sudo to the lab, an operator archive source/target' || printf 'lab-single-host, SSH/sudo to every lab node, a fresh full pre-restore backup, a disposable lab')
dr: never: stack/volume/secret/config/node/service rm, swarm leave, pgbackrest expire/stanza-delete, mc rm on the source, DROP DATABASE/SCHEMA, rm -rf
dr: labels: Measured for executed rows; Simulated HA at most for the three-VM lab; Unverified when skipped or missing; Real HA is HA-014
dr: this plan is not evidence
PLAN
  exit 0
fi

if [ "$mode" = "in-place" ]; then
  [ "$confirm" -eq 1 ] || usage_error "--target in-place is destructive; pass --confirm"
  [ "$skip_backup" -eq 0 ] || usage_error "--in-place requires a pre-restore backup; do not pass --skip-backup"
fi
DR_TARGET_FIXED=0
[ -n "$DR_TARGET_TIME$DR_TARGET_LSN$DR_BACKUP_LABEL" ] && DR_TARGET_FIXED=1
DR_TAKE_BACKUP=1
[ "$skip_backup" -eq 1 ] && DR_TAKE_BACKUP=0
[ "$DR_TARGET_FIXED" -eq 1 ] && DR_TAKE_BACKUP=0
# In-place always takes a fresh full backup as the pre-restore safety copy, even
# when the recovery target is an older label or time.
[ "$mode" = "in-place" ] && DR_TAKE_BACKUP=1

[ "$confirm" -eq 1 ] || usage_error "a live $mode drill performs a restore; pass --confirm"
command -v jq >/dev/null 2>&1 || usage_error "jq is required for a live drill"
command -v sha256sum >/dev/null 2>&1 || usage_error "sha256sum is required for a live drill"
command -v date >/dev/null 2>&1 || usage_error "date is required for a live drill"

dr_qr_deploy() {
  if [ -n "${QR_DEPLOY_BIN:-}" ]; then
    "$QR_DEPLOY_BIN" "$@"
  elif command -v qr-deploy >/dev/null 2>&1; then
    qr-deploy "$@"
  else
    cargo run --quiet -p quantrule-deploy -- "$@"
  fi
}

. "$script_dir/dr/evidence.sh"
. "$script_dir/dr/target.sh"
. "$script_dir/dr/scratch.sh"
. "$script_dir/dr/inplace.sh"
. "$script_dir/dr/digest.sh"

DR_STARTED_UTC=$(dr_now_utc)
DR_STARTED_EPOCH=$(dr_now_epoch)
DR_WORK=$(mktemp -d "${TMPDIR:-/tmp}/quantrule-dr.XXXXXX")
chmod 700 "$DR_WORK"
: > "$DR_WORK/facts.jsonl"
if [ -z "$evidence" ]; then
  evidence="deploy/v3/evidence/$DR_DRILL_ID.json"
fi
DR_EVIDENCE=$evidence

dr_cleanup() {
  find "$DR_WORK" -type f -delete 2>/dev/null || true
  find "$DR_WORK" -depth -type d -empty -delete 2>/dev/null || true
}

dr_on_exit() {
  local status=$?
  if [ ! -f "$DR_EVIDENCE" ] && [ -f "$DR_WORK/facts.jsonl" ]; then
    [ -n "$DR_LABELS" ] || DR_LABELS=Unverified
    [ -n "$DR_ROWS" ] || DR_ROWS="drill=Unverified
"
    dr_total_timing >/dev/null 2>&1 || true
    dr_write_evidence "Unverified" >/dev/null 2>&1 || true
  fi
  dr_cleanup
  exit "$status"
}
trap dr_on_exit EXIT

dr_read_descriptor
dr_repo_facts
DR_API_URL=${QR_DR_SOURCE_API_URL:-https://$DR_INGRESS_DNS/api/v1}
DR_CACERT=${QR_DR_CACERT:-}
if [ -z "$DR_CACERT" ] || [ ! -r "$DR_CACERT" ]; then
  if [ -r "$DR_RENDER/pki/database-ca.crt" ]; then
    DR_CACERT="$DR_RENDER/pki/database-ca.crt"
  elif [ -r "$DR_DATABASE_CA" ]; then
    DR_CACERT="$DR_DATABASE_CA"
  else
    dr_unverified "the source ingress CA is not readable; set QR_DR_CACERT or render the PKI"
  fi
fi
if [ "$mode" = "scratch" ]; then
  dr_scratch_preflight
else
  dr_inplace_preflight
fi

# The pre-restore digest is taken before the marker window and the backup. A
# failure here stops before any restore.
dr_digest_pre
dr_marker_phase

if [ "$DR_TAKE_BACKUP" -eq 1 ]; then
  if [ "$DR_TARGET_FIXED" -eq 1 ]; then
    dr_take_backup 1
  else
    dr_take_backup 0
  fi
  DR_ROWS="${DR_ROWS}backup=Measured
"
else
  dr_fact "backup_status" "Unverified"
  DR_ROWS="${DR_ROWS}backup=Unverified
"
  dr_say "backup: skipped; using the newest successful backup"
fi
dr_resolve_target

if dr_check_archive_time "$DR_TARGET_EPOCH"; then
  :
else
  archive_status=$?
  if [ "$archive_status" -eq 1 ]; then
    dr_unverified "the target is after the last object archive; restore refused"
  fi
  DR_ARCHIVE_UNVERIFIED=1
fi

if [ "$mode" = "scratch" ]; then
  dr_scratch_run
else
  dr_inplace_run
fi

dr_rpo_phase
dr_object_phase
dr_digest_post
dr_finalize
