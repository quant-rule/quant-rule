#!/usr/bin/env bash
# Restore the durable state of a clean Compose stack from one backup archive.
#
# The archive sidecar is checked first, then every payload file against the
# in-archive manifest, and only then is anything written to Postgres or the
# object store. A missing sidecar, a changed byte, a missing file, an entry
# outside the archive root, or a partial four-database set is a refusal.
#
# A non-empty stack is refused unless --force is given. --force replaces the
# four application schemas and the bucket contents; it is an explicit operator
# decision, never an automatic fallback. A restore without --yes prompts when
# stdin is a terminal and refuses when it is not.
#
# Restore writes objects before rows: an interrupted object copy leaves
# unreferenced blobs, while a row set without its bytes would be a run that
# looks present but cannot verify. The queue is not restored; messages are
# rebuilt from run rows after the stack starts.
#
# Usage: tools/restore.sh ARCHIVE [--yes] [--force] [--check]
#   ARCHIVE   the quantrule-*.tar.gz written by tools/backup.sh
#   --yes     do not prompt (required when stdin is not a terminal)
#   --force   replace a non-empty database and bucket
#   --check   verify the sidecar, manifest, four-database set, and hashes only;
#             no Docker command runs and nothing is written
#
# Environment:
#   COMPOSE_PROJECT_NAME  the Compose project to restore (default: quantrule)
#   QR_OBJECT_BUCKET      the bucket to restore (default: quantrule)
#   QR_POSTGRES_USER      the database role (default: quantrule)
set -euo pipefail
umask 077

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=tools/backup_lib.sh
. "$script_dir/backup_lib.sh"

usage() {
  sed -n '2,29p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

archive=''
assume_yes=0
force=0
check_only=0
while [ $# -gt 0 ]; do
  case "$1" in
    --yes)
      assume_yes=1
      shift
      ;;
    --force)
      force=1
      shift
      ;;
    --check)
      check_only=1
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    -*)
      backup_die "unknown argument: $1"
      ;;
    *)
      [ -z "$archive" ] || backup_die 'restore accepts one archive'
      archive=$1
      shift
      ;;
  esac
done
[ -n "$archive" ] || backup_die 'usage: tools/restore.sh ARCHIVE [--yes] [--force] [--check]'
[ -f "$archive" ] || backup_die "the archive $archive does not exist"
archive=$(cd -- "$(dirname -- "$archive")" && pwd)/$(basename -- "$archive")
sidecar="$archive.sha256"
[ -f "$sidecar" ] ||
  backup_die "the archive has no .sha256 sidecar; an archive written by tools/backup.sh always has one"

backup_require_commands tar find sort awk wc
if [ "$check_only" -eq 0 ]; then
  backup_require_commands docker
  backup_compose version >/dev/null
fi

# 1. The archive as a whole.
read -r expected_archive_hash expected_archive_name < "$sidecar" ||
  backup_die 'the sidecar is empty'
observed_archive_hash=$(backup_sha256 "$archive")
[ "$expected_archive_hash" = "$observed_archive_hash" ] ||
  backup_die "the archive hash does not match its sidecar: expected $expected_archive_hash, observed $observed_archive_hash"
[ "$expected_archive_name" = "$(basename -- "$archive")" ] ||
  backup_die "the sidecar names '$expected_archive_name', not '$(basename -- "$archive")'"

archive_name=$(basename -- "$archive")
root_name=${archive_name%.tar.gz}
[ "$root_name" != "$archive_name" ] && [ -n "$root_name" ] ||
  backup_die 'the archive name does not end in .tar.gz'

# 2. The entry list, before anything is extracted.
entries=$(tar -tzf "$archive") || backup_die 'the archive cannot be read'
first_entry=${entries%%$'\n'*}
[ "$first_entry" = "$root_name/" ] ||
  backup_die "the archive root is not $root_name/"
while IFS= read -r entry; do
  [ -n "$entry" ] || continue
  case "$entry" in
    "$root_name" | "$root_name"/*) ;;
    *) backup_die "the archive entry '$entry' is outside $root_name/" ;;
  esac
  relative=${entry#"$root_name"/}
  [ -n "$relative" ] || continue
  backup_safe_relative "${relative%/}" ||
    backup_die "the archive entry '$entry' is not a safe relative path"
done <<EOF
$entries
EOF

while IFS= read -r listing; do
  case "$listing" in
    d* | -*) ;;
    *) backup_die 'the archive contains an entry that is not a regular file or directory' ;;
  esac
done < <(tar -tvzf "$archive")

stage=$(mktemp -d "${TMPDIR:-/tmp}/quantrule-restore.XXXXXX")
cleanup() {
  local status=$?
  trap - EXIT
  if [ -n "${restart_services:-}" ]; then
    # Service names are Compose identifiers; word splitting is intended here.
    backup_compose start $restart_services >/dev/null 2>&1 ||
      backup_log "warning: could not restart: $restart_services"
  fi
  rm -rf "$stage"
  exit "$status"
}
trap cleanup EXIT

tar -C "$stage" -xzf "$archive"
payload="$stage/$root_name"
manifest="$payload/$BACKUP_MANIFEST"
[ -f "$manifest" ] || backup_die 'the archive has no manifest'
first_manifest=$(sed -n '1p' "$manifest")
[ "$first_manifest" = "# $BACKUP_SCHEMA" ] ||
  backup_die "the manifest schema is not $BACKUP_SCHEMA"

# 3. Every payload file against the manifest. Nothing has touched the stack.
expected_list="$stage/expected-files"
: > "$expected_list"
database_entries=""
database_count=0
object_entries=0
object_bytes=0
while IFS='|' read -r kind path bytes hash database key; do
  case "$kind" in
    '' | '#'*) continue ;;
    database | object) ;;
    *) backup_die "the manifest has an unknown entry kind: $kind" ;;
  esac
  backup_safe_relative "$path" ||
    backup_die "the manifest path '$path' is not a safe relative path"
  if [ -z "$bytes" ] || [ -z "$hash" ]; then
    backup_die "the manifest entry '$path' is incomplete"
  fi
  file="$payload/$path"
  [ -f "$file" ] || backup_die "the archive is incomplete: $path is missing"
  actual_bytes=$(backup_file_bytes "$file")
  [ "$actual_bytes" = "$bytes" ] ||
    backup_die "size mismatch for $path: expected $bytes, observed $actual_bytes"
  actual_hash=$(backup_sha256 "$file")
  [ "$actual_hash" = "$hash" ] ||
    backup_die "hash mismatch for $path: expected $hash, observed $actual_hash"
  printf '%s\n' "$path" >> "$expected_list"
  case "$kind" in
    database)
      [ -z "$key" ] || backup_die "the database entry '$database' carries an object key"
      backup_safe_relative "$database" ||
        backup_die "the manifest database '$database' is not a safe name"
      case " $BACKUP_DATABASES " in
        *" $database "*) ;;
        *) backup_die "the manifest names an unknown database: $database" ;;
      esac
      case " $database_entries " in
        *" $database "*) backup_die "the manifest names $database twice" ;;
      esac
      [ "$path" = "$BACKUP_DATABASES_DIR/$database.dump" ] ||
        backup_die "the manifest path '$path' does not match database '$database'"
      database_entries="$database_entries $database"
      database_count=$((database_count + 1))
      ;;
    object)
      [ -z "$database" ] || backup_die "the object entry '$key' carries a database"
      backup_safe_relative "$key" ||
        backup_die "the manifest object key '$key' is not a safe relative path"
      [ "$path" = "$BACKUP_OBJECTS_DIR/$key" ] ||
        backup_die "the manifest path '$path' does not match object key '$key'"
      object_entries=$((object_entries + 1))
      object_bytes=$((object_bytes + actual_bytes))
      ;;
  esac
done < "$manifest"
for database in $BACKUP_DATABASES; do
  case " $database_entries " in
    *" $database "*) ;;
    *) backup_die "the archive is incomplete: it has no $database dump; restore refuses a partial four-database set" ;;
  esac
done
[ "$database_count" -eq 4 ] ||
  backup_die "the manifest must name exactly four databases, not $database_count"

# An extra file in the archive is not covered by a hash, so it is a refusal.
actual_list="$stage/actual-files"
(
  cd "$payload"
  find . -type f | sed 's|^\./||' | grep -Fxv "$BACKUP_MANIFEST" | LC_ALL=C sort
) > "$actual_list"
LC_ALL=C sort -o "$expected_list" "$expected_list"
if ! cmp -s "$expected_list" "$actual_list"; then
  backup_log 'the archive holds files the manifest does not name:'
  comm -13 "$expected_list" "$actual_list" >&2 || true
  backup_die 'the archive does not match its manifest'
fi

if [ "$check_only" -eq 1 ]; then
  printf 'archive: %s\n' "$archive"
  printf 'databases: %s dumps\n' "$database_count"
  printf 'objects: %s files, %s bytes\n' "$object_entries" "$object_bytes"
  printf 'check: sidecar, entries, manifest, four-database set, and payload hashes verified; no stack was touched\n'
  exit 0
fi

if [ "$assume_yes" -ne 1 ]; then
  [ -t 0 ] ||
    backup_die 'restore needs --yes when stdin is not a terminal'
  printf 'This replaces the database and bucket of project %s. Type "restore" to continue: ' \
    "${COMPOSE_PROJECT_NAME:-quantrule}" >&2
  read -r answer
  [ "$answer" = 'restore' ] || backup_die 'restore cancelled'
fi

# 4. The target. Only the two stores are needed to inspect and write.
backup_compose up -d --wait postgres minio >/dev/null
restart_services=$(backup_quiesce)

postgres_user=${QR_POSTGRES_USER:-quantrule}
bucket=${QR_OBJECT_BUCKET:-quantrule}
# The four databases are separate, so a table in any of them makes the target
# non-empty. The check is per database; pg_restore must see a clean one.
tables=0
for database in $BACKUP_DATABASES; do
  count=$(backup_compose exec -T postgres psql -U "$postgres_user" -d "$database" -tAc \
    "SELECT count(*) FROM pg_catalog.pg_tables
     WHERE schemaname NOT IN ('pg_catalog', 'information_schema')")
  count=${count//[[:space:]]/}
  case "$count" in
    '' | *[!0-9]*) backup_die "cannot count the tables in $database" ;;
  esac
  tables=$((tables + count))
done
objects=$(backup_minio_run 'mc ls --recursive "qr-backup/$QR_BACKUP_BUCKET" | wc -l')
objects=${objects//[[:space:]]/}
case "$objects" in
  '' | *[!0-9]*) backup_die 'cannot count the bucket objects' ;;
esac

if [ "$tables" -ne 0 ] || [ "$objects" -ne 0 ]; then
  if [ "$force" -ne 1 ]; then
    backup_die "the stack is not clean ($tables tables, $objects objects); use --force to replace it"
  fi
  backup_log "replacing $tables tables and $objects objects"
  for database in $BACKUP_DATABASES; do
    case "$database" in
      qr_gateway) schemas='gateway' ;;
      qr_catalog) schemas='catalog' ;;
      qr_evidence) schemas='runs, research, stage' ;;
      qr_simulation) schemas='simulation' ;;
      *) backup_die "unknown database: $database" ;;
    esac
    backup_compose exec -T postgres psql -v ON_ERROR_STOP=1 -U "$postgres_user" -d "$database" \
      -c "DROP SCHEMA IF EXISTS $schemas CASCADE" >/dev/null
  done
  if [ "$objects" -ne 0 ]; then
    backup_minio_run 'mc rm --recursive --force "qr-backup/$QR_BACKUP_BUCKET" >/dev/null 2>&1 || true'
  fi
fi

# 5. Write. Objects first, then the rows that refer to them.
backup_log "restoring $object_entries objects ($object_bytes bytes)"
if ! tar -C "$payload/$BACKUP_OBJECTS_DIR" -cf - . | backup_minio_run \
  'rm -rf /tmp/qr-restore-objects
   mkdir -p /tmp/qr-restore-objects
   tar -C /tmp/qr-restore-objects -xf -
   mc mirror --overwrite --quiet /tmp/qr-restore-objects "qr-backup/$QR_BACKUP_BUCKET" >/dev/null'; then
  backup_die 'the object restore failed'
fi

backup_log 'restoring the four databases'
for database in $BACKUP_DATABASES; do
  if ! backup_compose exec -T postgres \
    pg_restore -U "$postgres_user" -d "$database" \
    --no-owner --no-privileges --exit-on-error \
    < "$payload/$BACKUP_DATABASES_DIR/$database.dump"; then
    backup_die "pg_restore failed for $database"
  fi
done

printf 'archive: %s\n' "$archive"
printf 'databases: %s dumps restored\n' "$database_count"
printf 'objects: %s files, %s bytes restored\n' "$object_entries" "$object_bytes"
printf 'start the stack with make up when it was stopped for the restore\n'
