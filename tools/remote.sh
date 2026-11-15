#!/usr/bin/env bash
# Start the stack for remote use: one published port, one bearer token.
#
# The gateway is the only container a client can reach. This script exports
# `QR_BEARER_TOKEN` to Compose, starts the stack, refuses to continue if any
# container other than the gateway publishes a host port, and checks the token
# matrix before it prints the token and an API example:
#
#   no token      -> 401 auth.unauthorized with WWW-Authenticate: Bearer
#   wrong token   -> 401 auth.unauthorized
#   correct token -> the route's normal status (200 on GET /profiles)
#
# `QR_BEARER_TOKEN` is read from the environment when set and generated with
# tools/token.sh otherwise. The token is printed to the terminal; this script
# writes it to no file, and it is not committed or logged. Docker holds it in
# the gateway container's environment while the stack runs.
#
# Remote exposure is still a network decision. Do not publish :8080 to the
# public internet: use WireGuard or Tailscale, or terminate TLS at a reverse
# proxy in front of the gateway. The browser client and the shell do not send
# the Authorization header, so their API calls answer 401 against this profile;
# use curl or another API client that sets the header until a client token
# feature lands.
#
# Usage: tools/remote.sh
#
# Environment:
#   QR_BEARER_TOKEN  reuse a token instead of generating one (32+ characters)
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
compose() {
  "${compose_command[@]}" "$@"
}
compose version >/dev/null 2>&1 || die "cannot run '${COMPOSE:-docker compose}'"

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
export QR_BEARER_TOKEN="$token"

printf 'remote: starting the stack with the token enforced\n'
compose up -d --build --wait || die 'the stack did not start healthy'

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

status=$(curl -sS --max-time 10 -o /dev/null -w '%{http_code}' -H "Authorization: Bearer $token" "$url/profiles") ||
  die "cannot reach $url/profiles with the token"
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
printf '  export QR_BEARER_TOKEN=%q\n' "$token"
printf '  curl -fsS -H "Authorization: Bearer $QR_BEARER_TOKEN" %s/profiles\n' "$url"
printf '  curl -fsS -H "Authorization: Bearer $QR_BEARER_TOKEN" -H "X-QuantRule-Profile: default" %s/catalog/datasets\n' "$url"
printf '\nTLS and network: never expose :8080 to the public internet. Use WireGuard or\n'
printf 'Tailscale, or terminate TLS at a reverse proxy in front of the gateway.\n'
printf '\nRestart: use make remote again, not make up; make up starts the\n'
printf 'unauthenticated local profile.\n'
printf '\nBrowser and shell: neither sends Authorization, so their API calls answer 401.\n'
printf 'Remote browser use needs a proxy that injects the token, or the private-network\n'
printf 'gateway with no token; use curl or an API client that sets the header.\n'
