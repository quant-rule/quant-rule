#!/usr/bin/env bash
# Start the stack for remote use: one published port, one bearer token.
#
# The gateway is the only container a client can reach. This script writes the
# token to a mode-600 file, layers `compose.remote.yaml` over the base file,
# starts the stack, refuses to continue if any container other than the gateway
# publishes a host port, and checks the token matrix before it prints the token
# and an API example:
#
#   no token      -> 401 auth.unauthorized with WWW-Authenticate: Bearer
#   wrong token   -> 401 auth.unauthorized
#   correct token -> the route's normal status (200 on GET /profiles)
#
# `QR_BEARER_TOKEN` is read from the environment when set and generated with
# tools/token.sh otherwise. The value is written to
# `$QR_SECRETS_DIR/bearer_token` (default `secrets/`) at mode 600 and mounted at
# `/run/secrets/bearer_token`; it is never a container environment value. The
# gateway re-reads the file every `QR_TOKEN_RELOAD_SECONDS` and on `SIGHUP`.
#
# Remote exposure is still a network decision. Do not publish :8080 to the
# public internet: use WireGuard or Tailscale, or terminate TLS at a reverse
# proxy in front of the gateway. The browser enters the token on its Connection
# screen and the shell reads --token, QUANTRULE_TOKEN or QUANTRULE_TOKEN_FILE,
# so both send Authorization: Bearer on every API call.
#
# Usage: tools/remote.sh
#
# Environment:
#   QR_BEARER_TOKEN  reuse a token instead of generating one (32+ characters)
#   QR_SECRETS_DIR   secret directory (default <root>/secrets)
#   QR_URL           URL to probe and print (default http://localhost:8080)
#   COMPOSE          Compose command (default "docker compose")
set -euo pipefail
umask 077

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "$script_dir/.." && pwd)
cd "$root"

die() {
  echo "remote: $*" >&2
  exit 1
}

case "${1:-}" in
  '') ;;
  -h | --help)
    awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
    exit 0
    ;;
  *)
    die "unknown argument: $1"
    ;;
esac

command -v docker >/dev/null 2>&1 || die 'docker is required'
command -v curl >/dev/null 2>&1 || die 'curl is required to check /health and the token'

read -r -a compose_command <<< "${COMPOSE:-docker compose}"
secrets_dir="${QR_SECRETS_DIR:-$root/secrets}"
compose() {
  QR_SECRETS_DIR="$secrets_dir" "${compose_command[@]}" \
    --env-file "$secrets_dir/compose.env" \
    -f compose.yaml -f compose.remote.yaml "$@"
}
"${compose_command[@]}" version >/dev/null 2>&1 || die "cannot run '${COMPOSE:-docker compose}'"

"$script_dir/secrets.sh" --out "$secrets_dir" >/dev/null
set -a
# shellcheck disable=SC1090
. "$secrets_dir/compose.env"
set +a

# The local stack may use the documented loopback defaults; a remotely exposed
# stack may not. Refuse the known default before any container starts.
for variable in QR_POSTGRES_PASSWORD QR_OBJECT_STORE_PASSWORD QR_RABBITMQ_PASSWORD; do
  value="${!variable:-}"
  if [ -z "$value" ] || [ "$value" = "quantrule" ]; then
    die "$variable must be set to a non-default value for a remote stack"
  fi
done

url=${QR_URL:-http://localhost:8080}
url=${url%/}

token=${QR_BEARER_TOKEN:-}
if [ -z "$token" ]; then
  token=$("$script_dir/token.sh") || die 'cannot generate a token'
  printf 'remote: generated a new bearer token\n'
else
  printf 'remote: using QR_BEARER_TOKEN from the environment\n'
fi
if [ "${#token}" -lt 32 ]; then
  die 'QR_BEARER_TOKEN must be at least 32 characters; run tools/token.sh'
fi
case "$token" in
  *[[:space:]]*) die 'QR_BEARER_TOKEN must not contain whitespace' ;;
esac
printf '%s\n' "$token" > "$secrets_dir/bearer_token"
chmod 600 "$secrets_dir/bearer_token"

printf 'remote: starting the stack with the token enforced\n'
compose up -d --build --wait || die 'the stack did not start healthy'

# The entrypoint stages the mounted file for UID 10001 at startup. Install the
# current value into that staged path as well, so a second `make remote` with a
# rotated token is live without recreating the container. Only the root helper
# can change the staged file's owner; the service itself stays UID 10001.
compose cp "$secrets_dir/bearer_token" gateway:/run/quantrule-secrets/bearer_token \
  || die 'cannot install the token file in the gateway container'
compose exec -T -u 0 gateway sh -c \
  'chown 10001:10001 /run/quantrule-secrets/bearer_token && chmod 400 /run/quantrule-secrets/bearer_token' \
  || die 'cannot set the staged token file owner'
compose kill -s HUP gateway >/dev/null 2>&1 || die 'cannot signal the gateway to reload the token'

# A gateway healthcheck proves the process is up; it does not prove that the
# host port set is the one the remote profile promises.
gateway_found=0
published=()
mapfile -t containers < <(compose ps -q)
for id in "${containers[@]}"; do
  name=$(docker inspect --format '{{.Name}}' "$id")
  ports=$(docker inspect --format '{{range $p, $bindings := .HostConfig.PortBindings}}{{range $bindings}}{{.HostPort}}->{{$p}} {{end}}{{end}}' "$id")
  ports=$(printf '%s' "$ports" | tr -s '[:space:]' '\n' | sed '/^$/d' | sort -u | paste -sd, -)
  [ -n "$ports" ] || continue
  published+=("$name $ports")
  case "$name" in
    *-gateway-* | */gateway)
      gateway_found=1
      [ "$ports" = '8080->8080/tcp' ] || die "the gateway publishes '$ports', expected 8080->8080/tcp"
      ;;
    *)
      die "$name publishes '$ports'; only the gateway may publish a host port"
      ;;
  esac
done
[ "$gateway_found" -eq 1 ] || die 'the gateway has no published :8080 port'

curl -fsS --max-time 10 "$url/health" >/dev/null || die "the gateway is not healthy at $url/health"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
status=$(curl -sS --max-time 10 -o "$work/body" -D "$work/headers" -w '%{http_code}' "$url/profiles") ||
  die "cannot reach $url/profiles"
[ "$status" = 401 ] || die "a request without a token returned $status, expected 401"
grep -qi '^www-authenticate: Bearer' "$work/headers" ||
  die 'the 401 response has no WWW-Authenticate: Bearer header'
grep -q 'auth.unauthorized' "$work/body" ||
  die 'the 401 response does not carry auth.unauthorized'

status=$(curl -sS --max-time 10 -o /dev/null -w '%{http_code}' -H 'Authorization: Bearer wrong-token' "$url/profiles") ||
  die "cannot reach $url/profiles with a wrong token"
[ "$status" = 401 ] || die "a wrong token returned $status, expected 401"

status=0
for _ in $(seq 1 10); do
  status=$(curl -sS --max-time 10 -o /dev/null -w '%{http_code}' -H "Authorization: Bearer $token" "$url/profiles") ||
    die "cannot reach $url/profiles with the token"
  [ "$status" = 200 ] && break
  sleep 1
done
[ "$status" = 200 ] || die "the configured token returned $status on /profiles, expected 200"

printf '\nQuantRule remote profile ready → %s\n' "$url"
printf 'Published host ports:\n'
for entry in "${published[@]}"; do
  printf '  %s\n' "$entry"
done
printf '\nToken check: no token 401, wrong token 401, configured token 200 (/profiles)\n'
printf '\nBearer token (printed once; keep it out of git and logs):\n'
printf '  %s\n' "$token"
printf '\nAPI usage:\n'
printf '  export QUANTRULE_TOKEN=%q\n' "$token"
printf '  curl -fsS -H "Authorization: Bearer $QUANTRULE_TOKEN" %s/profiles\n' "$url"
printf '  curl -fsS -H "Authorization: Bearer $QUANTRULE_TOKEN" -H "X-QuantRule-Profile: default" %s/catalog/datasets\n' "$url"
printf '\nRotation: export QR_BEARER_TOKEN from tools/token.sh and run make remote again,\n'
printf 'or replace the host file, copy it into the gateway, and send SIGHUP:\n'
printf '  %s\n' "$secrets_dir/bearer_token"
printf 'See docs/manual/08-operations.md for the exact copy and SIGHUP commands.\n'
printf 'Clients re-export the token after a rotation.\n'
printf '\nTLS and network: never expose :8080 to the public internet. Use WireGuard or\n'
printf 'Tailscale, or terminate TLS at a reverse proxy in front of the gateway.\n'
printf '\nRestart: use make remote again, not make up; make up starts the\n'
printf 'unauthenticated local profile.\n'
printf '\nClients: the browser enters the token on the Connection screen; the shell\n'
printf 'reads --token, QUANTRULE_TOKEN or QUANTRULE_TOKEN_FILE. Both send\n'
printf 'Authorization: Bearer on every API call.\n'
