#!/usr/bin/env bash
# Create the local deployment secret files.
#
# The application services receive every secret as a mounted file under
# `/run/secrets`; only `<NAME>_FILE` and non-secret values reach the container
# environment. This script creates the values once and never overwrites them on
# a normal `make up`: rotating a live database password without a coordinated
# restart is a deliberate operation, not a side effect of starting the stack.
#
# Usage:
#   tools/secrets.sh                 create missing files under ./secrets
#   tools/secrets.sh --out DIR       write to DIR instead
#   tools/secrets.sh --force         rotate every value (explicit operation)
#
# The files are mode 600 and the directory is mode 700. No value is printed.
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
out="${QR_SECRETS_DIR:-$root/secrets}"
force=0

while [ "$#" -gt 0 ]; do
  case "$1" in
    --out)
      [ "$#" -ge 2 ] || { echo "secrets: --out needs a directory" >&2; exit 2; }
      out="$2"
      shift 2
      ;;
    --force)
      force=1
      shift
      ;;
    -h | --help)
      sed -n '2,15p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "secrets: unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

umask 077
mkdir -p "$out"
chmod 700 "$out"

env_file="$out/compose.env"
if [ "$force" -eq 0 ] && [ -f "$env_file" ]; then
  # Read the existing infrastructure passwords so the generated role URLs and
  # the running containers keep one identity. The file is our own KEY=VALUE
  # output and contains only URL-safe hexadecimal values.
  set -a
  # shellcheck disable=SC1090
  . "$env_file"
  set +a
elif [ "$force" -eq 0 ]; then
  if find "$out" -maxdepth 1 -type f ! -name compose.env ! -name bearer_token -print -quit | grep -q .; then
    echo "secrets: $out has secret files but no compose.env; remove the directory or rerun with --force" >&2
    exit 1
  fi
fi

random_hex() {
  if command -v openssl >/dev/null 2>&1; then
    openssl rand -hex "$1"
  else
    od -An -N"$1" -tx1 /dev/urandom | tr -d ' \n'
  fi
}

if [ "$force" -eq 1 ] || [ ! -f "$env_file" ]; then
  QR_POSTGRES_PASSWORD="${QR_POSTGRES_PASSWORD:-$(random_hex 32)}"
  QR_RABBITMQ_PASSWORD="${QR_RABBITMQ_PASSWORD:-$(random_hex 32)}"
  QR_OBJECT_STORE_PASSWORD="${QR_OBJECT_STORE_PASSWORD:-$(random_hex 32)}"
  {
    printf 'QR_POSTGRES_PASSWORD=%s\n' "$QR_POSTGRES_PASSWORD"
    printf 'QR_RABBITMQ_PASSWORD=%s\n' "$QR_RABBITMQ_PASSWORD"
    printf 'QR_OBJECT_STORE_PASSWORD=%s\n' "$QR_OBJECT_STORE_PASSWORD"
  } > "$env_file"
  chmod 600 "$env_file"
fi

write_secret() {
  local name="$1" value="$2"
  if [ "$force" -eq 0 ] && [ -f "$out/$name" ]; then
    return 0
  fi
  printf '%s\n' "$value" > "$out/$name"
  chmod 600 "$out/$name"
}

write_secret qr_gateway_database_url \
  "postgres://qr_gateway_app:$QR_POSTGRES_PASSWORD@postgres:5432/qr_gateway"
write_secret qr_catalog_database_url \
  "postgres://qr_catalog_app:$QR_POSTGRES_PASSWORD@postgres:5432/qr_catalog"
write_secret qr_evidence_database_url \
  "postgres://qr_evidence_app:$QR_POSTGRES_PASSWORD@postgres:5432/qr_evidence"
write_secret qr_simulation_database_url \
  "postgres://qr_simulation_app:$QR_POSTGRES_PASSWORD@postgres:5432/qr_simulation"
write_secret qr_gateway_owner_database_url \
  "postgres://qr_gateway_owner:$QR_POSTGRES_PASSWORD@postgres:5432/qr_gateway"
write_secret qr_catalog_owner_database_url \
  "postgres://qr_catalog_owner:$QR_POSTGRES_PASSWORD@postgres:5432/qr_catalog"
write_secret qr_evidence_owner_database_url \
  "postgres://qr_evidence_owner:$QR_POSTGRES_PASSWORD@postgres:5432/qr_evidence"
write_secret qr_simulation_owner_database_url \
  "postgres://qr_simulation_owner:$QR_POSTGRES_PASSWORD@postgres:5432/qr_simulation"
write_secret queue_url \
  "amqp://quantrule:$QR_RABBITMQ_PASSWORD@rabbitmq:5672/%2f"
write_secret object_store_access_key "quantrule"
write_secret object_store_secret_key "$QR_OBJECT_STORE_PASSWORD"

echo "secrets: ready in $out (mode 700, files mode 600)"
if [ ! -f "$out/bearer_token" ]; then
  echo "secrets: the local stack stays unauthenticated; 'make remote' writes a token file"
fi
