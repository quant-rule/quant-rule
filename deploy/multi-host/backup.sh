#!/usr/bin/env bash
# Back up the durable state of a multi-host deployment.
#
# Run this on the infrastructure host. The wrapper points the existing, tested
# tools/backup.sh at the infrastructure Compose project, so Postgres and MinIO
# are dumped and mirrored exactly as on the single-host stack. The queue is not
# archived; its messages are rebuilt from run rows.
#
# The application hosts are not visible from here. Stop the gateway, then the
# compute hosts, then the core hosts before running this, and pass
# --application-stopped to confirm it. The wrapper cannot verify that across
# hosts, so the confirmation is explicit.
#
# Usage:
#   deploy/multi-host/backup.sh --application-stopped [--out DIR]
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
  sed -n '2,19p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
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
  { echo 'backup: stop the application hosts first, then pass --application-stopped' >&2; exit 2; }
[ -f "$compose" ] || { echo "backup: compose file not found: $compose" >&2; exit 2; }
[ -f "$env_file" ] || { echo "backup: infra env not found: $env_file" >&2; exit 2; }

exec env \
  COMPOSE_FILE="$compose" \
  COMPOSE_PROJECT_NAME=quantrule-infra \
  COMPOSE_ENV_FILES="$env_file" \
  "$root/tools/backup.sh" "${args[@]}"
