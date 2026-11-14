#!/usr/bin/env bash
# Back up the durable state of the Compose stack into one timestamped archive.
#
# The archive holds a custom-format Postgres dump, an object mirror of the
# bucket, and a manifest that records the SHA-256 of every payload file. A
# detached `.sha256` sidecar covers the archive itself. `tools/restore.sh`
# verifies both before it writes anything.
#
# The application services are stopped for the duration, so the dump and the
# mirror describe one instant rather than two races. Postgres, the object store,
# and the broker stay up. The services that were running are started again on
# exit, including the failure path. The queue is not archived: a message is
# derived from a run row and is rebuilt after a restore.
#
# Usage: tools/backup.sh [--out DIR]
#   --out DIR   archive directory; default $QR_BACKUP_DIR or
#               $QUANTRULE_DATA_ROOT/backups
#
# Environment:
#   COMPOSE_PROJECT_NAME  the Compose project to back up (default: quantrule)
#   QR_OBJECT_BUCKET      the bucket to mirror (default: quantrule)
#   QR_POSTGRES_USER      the database role (default: quantrule)
#   QR_POSTGRES_DB        the database name (default: quantrule)
set -euo pipefail
umask 077

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=tools/backup_lib.sh
. "$script_dir/backup_lib.sh"

usage() {
  sed -n '2,23p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

output_dir=${QR_BACKUP_DIR:-${QUANTRULE_DATA_ROOT:-$HOME/quant-rule-data}/backups}
while [ $# -gt 0 ]; do
  case "$1" in
    --out)
      [ $# -ge 2 ] || backup_die '--out needs a directory'
      output_dir=$2
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      backup_die "unknown argument: $1"
      ;;
  esac
done

backup_require_commands docker tar find sort awk wc
backup_compose version >/dev/null

backup_service_running postgres || backup_die 'the postgres service is not running'
backup_service_running minio || backup_die 'the minio service is not running'

mkdir -p "$output_dir"
output_dir=$(cd -- "$output_dir" && pwd)
created_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
stamp=$(printf '%s' "$created_at" | tr ':' '-')
name="${BACKUP_ARCHIVE_PREFIX}${stamp}"
archive="$output_dir/$name.tar.gz"
sidecar="$archive.sha256"
[ ! -e "$archive" ] || backup_die "$archive already exists"

stage=$(mktemp -d "${TMPDIR:-/tmp}/quantrule-backup.XXXXXX")
payload="$stage/$name"
postgres_file="$payload/$BACKUP_POSTGRES_FILE"
objects_dir="$payload/$BACKUP_OBJECTS_DIR"
mkdir -p "$(dirname -- "$postgres_file")" "$objects_dir"

restart_services=''
cleanup() {
  local status=$?
  trap - EXIT
  if [ -n "$restart_services" ]; then
    # Service names are Compose identifiers; word splitting is intended here.
    backup_compose start $restart_services >/dev/null 2>&1 ||
      backup_log "warning: could not restart: $restart_services"
  fi
  rm -rf "$stage"
  exit "$status"
}
trap cleanup EXIT

restart_services=$(backup_quiesce)

postgres_user=${QR_POSTGRES_USER:-quantrule}
postgres_db=${QR_POSTGRES_DB:-quantrule}
backup_log 'dumping postgres'
if ! backup_compose exec -T postgres \
  pg_dump -U "$postgres_user" -d "$postgres_db" \
  --format=custom --no-owner --no-privileges > "$postgres_file"; then
  backup_die 'pg_dump failed'
fi
[ -s "$postgres_file" ] || backup_die 'pg_dump produced no bytes'

bucket=${QR_OBJECT_BUCKET:-quantrule}
backup_log "mirroring bucket $bucket"
if ! backup_minio_run "rm -rf /tmp/qr-backup-objects
  mkdir -p /tmp/qr-backup-objects
  mc mirror --overwrite --quiet \"$BACKUP_MINIO_ALIAS/\$QR_BACKUP_BUCKET\" /tmp/qr-backup-objects >/dev/null
  tar -C /tmp/qr-backup-objects -cf - ." | tar -C "$objects_dir" -xf -; then
  backup_die 'the object mirror failed'
fi

manifest="$payload/$BACKUP_MANIFEST"
object_count=0
object_bytes=0
{
  printf '# %s\n' "$BACKUP_SCHEMA"
  printf '# created_at=%s\n' "$created_at"
  printf '# project=%s\n' "${COMPOSE_PROJECT_NAME:-quantrule}"
  printf '# archive=%s\n' "$name.tar.gz"
  printf '# bucket=%s\n' "$bucket"
  printf '# fields: kind, archive-path, bytes, sha256, object-key\n'
  printf 'postgres\t%s\t%s\t%s\n' \
    "$BACKUP_POSTGRES_FILE" \
    "$(backup_file_bytes "$postgres_file")" \
    "$(backup_sha256 "$postgres_file")"
  while IFS= read -r file; do
    relative=${file#"$objects_dir"/}
    backup_safe_relative "$relative" ||
      backup_die "the object key '$relative' is not a safe archive path"
    bytes=$(backup_file_bytes "$file")
    object_count=$((object_count + 1))
    object_bytes=$((object_bytes + bytes))
    printf 'object\t%s/%s\t%s\t%s\t%s\n' \
      "$BACKUP_OBJECTS_DIR" "$relative" "$bytes" "$(backup_sha256 "$file")" "$relative"
  done < <(find "$objects_dir" -type f | LC_ALL=C sort)
} > "$manifest"

# A partial archive is never published under the final name. COPYFILE_DISABLE
# keeps macOS from adding AppleDouble members that the manifest does not list.
partial="$output_dir/.$name.tar.gz.part"
COPYFILE_DISABLE=1 tar -C "$stage" -czf "$partial" "$name"
mv "$partial" "$archive"
backup_sha256_line "$archive" "$name.tar.gz" > "$sidecar"

printf 'archive: %s\n' "$archive"
printf 'sha256: %s\n' "$(backup_sha256 "$archive")"
printf 'postgres: %s bytes\n' "$(backup_file_bytes "$postgres_file")"
printf 'objects: %s files, %s bytes\n' "$object_count" "$object_bytes"
printf 'sidecar: %s\n' "$sidecar"
