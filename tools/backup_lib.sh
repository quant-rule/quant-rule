#!/usr/bin/env bash
# Shared constants and checks for tools/backup.sh and tools/restore.sh.
#
# Sourced, never executed. The archive layout is one invariant: both scripts
# must agree on the schema token, the manifest name, the payload paths, and the
# container names. Keeping them here means a new payload kind is added once.
#
# The scripts run on the operator host and call `docker compose`; they add no
# runtime dependency to the image and never read a client path.

# The archive schema and the names inside every archive. The schema moved to v2
# when one dump became one dump per logical database; restore refuses a v1
# archive rather than guessing that it covers all four.
BACKUP_SCHEMA='quantrule.backup/v2'
BACKUP_MANIFEST='manifest.json'
BACKUP_DATABASES='qr_gateway qr_catalog qr_evidence qr_simulation'
BACKUP_DATABASES_DIR='postgres'
BACKUP_OBJECTS_DIR='objects'
BACKUP_ARCHIVE_PREFIX='quantrule-'

# The three durable stores. The broker is deliberately absent: a message is
# derived from a run row and is rebuilt, never restored.
BACKUP_STATE_SERVICES='postgres minio rabbitmq'

# The object-store client is the `mc` bundled in the MinIO image. The client
# talks to the live service over the Compose network, so no host mount and no
# second published port are needed.
BACKUP_MINIO_ALIAS='qr-backup'
BACKUP_MINIO_CONFIG='/tmp/qr-backup-mc'

# Diagnostics go to stderr so stdout stays a machine-readable summary.
backup_log() {
  printf '%s\n' "$*" >&2
}

backup_die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

# The repository root, derived from this file rather than the caller's cwd.
backup_root() {
  local tools_dir
  tools_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
  (cd -- "$tools_dir/.." && pwd)
}

# Run Compose from the repository root so the file and project resolve the same
# way no matter where the operator invoked the script. COMPOSE_PROJECT_NAME is
# inherited, which is what keeps a test stack separate from the default one.
backup_compose() {
  (cd -- "$(backup_root)" && docker compose "$@")
}

backup_require_commands() {
  local command
  for command in "$@"; do
    command -v "$command" >/dev/null 2>&1 || backup_die "$command is required"
  done
}

# SHA-256 of one file, lowercase hexadecimal. sha256sum is the Linux default;
# shasum covers the macOS host that runs Docker Desktop's Linux backend.
backup_sha256() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
  else
    backup_die 'sha256sum or shasum is required'
  fi
}

# The size in bytes, without the platform's stat spelling.
backup_file_bytes() {
  wc -c < "$1" | tr -d '[:space:]'
}

# The detached sidecar format: digest, two spaces, base name.
backup_sha256_line() {
  local file="$1"
  local display="$2"
  printf '%s  %s\n' "$(backup_sha256 "$file")" "$display"
}

# A manifest path or object key must be relative, separator-clean, and made of
# characters the application writes. A value outside that set is a refusal:
# it would make the manifest ambiguous and the extraction unsafe.
backup_safe_relative() {
  local value="$1"
  case "$value" in
    '' | /* | */ | *..* | *\\* | *$'\n'* | *$'\t'*) return 1 ;;
  esac
  printf '%s' "$value" | grep -Eq '^[A-Za-z0-9._+:-]+(/[A-Za-z0-9._+:-]+)*$'
}

# Services that are running now, one per line.
backup_running_services() {
  backup_compose ps --services --status running
}

backup_service_running() {
  local services
  services=$(backup_running_services)
  case $'\n'"$services"$'\n' in
    *$'\n'"$1"$'\n'*) return 0 ;;
  esac
  return 1
}

# The application services that must be quiet for one consistent snapshot.
# Anything not a durable store is an application process, including a future
# data/model/simulation container, so the list stays correct when the split
# lands without the script having to enumerate the seven binaries.
backup_app_services() {
  local service
  local state_service
  local found
  while IFS= read -r service; do
    [ -n "$service" ] || continue
    found=0
    for state_service in $BACKUP_STATE_SERVICES; do
      if [ "$service" = "$state_service" ]; then
        found=1
        break
      fi
    done
    [ "$found" -eq 0 ] && printf '%s\n' "$service"
  done < <(backup_running_services)
}

# Execute one shell body inside the running MinIO container with an alias that
# points at that same container. The caller supplies the body; this function
# owns only the client setup. The credentials are already service environment.
backup_minio_run() {
  local script="$1"
  backup_compose exec -T \
    -e "QR_BACKUP_BUCKET=${QR_OBJECT_BUCKET:-quantrule}" \
    -e "SSL_CERT_FILE=/certs/CAs/ca.crt" \
    minio sh -c "
      set -e
      export MC_CONFIG_DIR=$BACKUP_MINIO_CONFIG MC_UPDATE=off
      command -v mc >/dev/null 2>&1 || {
        echo 'the object-store image does not provide mc; keep the pinned MinIO image' >&2
        exit 1
      }
      mc alias set $BACKUP_MINIO_ALIAS https://127.0.0.1:9000 \
        \"\$MINIO_ROOT_USER\" \"\$MINIO_ROOT_PASSWORD\" >/dev/null
      $script"
}

# Stop the application services and print the list that must be started again.
# The caller owns the restart so a failure path restores the prior state too.
backup_quiesce() {
  local services
  services=$(backup_app_services | tr '\n' ' ')
  services=${services% }
  if [ -n "$services" ]; then
    backup_log "quiescing: $services"
    # Service names are Compose identifiers; word splitting is intended here.
    backup_compose stop $services >/dev/null
  fi
  printf '%s\n' "$services"
}
