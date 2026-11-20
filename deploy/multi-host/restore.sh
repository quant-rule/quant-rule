#!/usr/bin/env bash
# Restore a multi-host deployment from an archive written by backup.sh.
#
# Run this on the infrastructure host after stopping the gateway, compute, and
# core services. The wrapper points tools/restore.sh at the infrastructure
# Compose project; the existing verifier checks the sidecar and every payload
# before writing anything. The queue is not restored; run rows rebuild it.
#
# Usage:
#   deploy/multi-host/restore.sh ARCHIVE --application-stopped [--yes] [--force]
#
# Environment:
#   QR_INFRA_COMPOSE_FILE  default deploy/multi-host/compose.infra.yaml
#   QR_INFRA_ENV_FILE      default deploy/multi-host/env/infra.env
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "$script_dir/../.." && pwd)
compose=${QR_INFRA_COMPOSE_FILE:-$root/deploy/multi-host/compose.infra.yaml}
env_file=${QR_INFRA_ENV_FILE:-$root/deploy/multi-host/env/infra.env}

usage() {
  sed -n '2,14p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

application_stopped=0
args=()
while [ $# -gt 0 ]; do
  case "$1" in
    --application-stopped)
      application_stopped=1
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      args+=("$1")
      shift
      ;;
  esac
done

[ "$application_stopped" -eq 1 ] ||
  { echo 'restore: stop the application hosts first, then pass --application-stopped' >&2; exit 2; }
[ -f "$compose" ] || { echo "restore: compose file not found: $compose" >&2; exit 2; }
[ -f "$env_file" ] || { echo "restore: infra env not found: $env_file" >&2; exit 2; }

# The archive tools read these shell variables; COMPOSE_ENV_FILES only feeds
# Compose. Source the same file so custom database, bucket, and queue names
# reach the dump, mirror, and restore.
# shellcheck disable=SC1090
set -a
. "$env_file"
set +a

exec env \
  COMPOSE_FILE="$compose" \
  COMPOSE_PROJECT_NAME=quantrule-infra \
  COMPOSE_ENV_FILES="$env_file" \
  "$root/tools/restore.sh" "${args[@]}"
