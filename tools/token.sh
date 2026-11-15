#!/usr/bin/env bash
# Print one new bearer token for the gateway's remote profile.
#
# The token is 32 random bytes rendered as 64 lowercase hexadecimal characters.
# That form is safe in a shell, an HTTP header, and a Compose environment value,
# and it carries 256 bits of entropy. This script writes no file and no log; the
# value exists on standard output only, and no token is committed to the
# repository.
#
# Usage: tools/token.sh
#
# The caller stores the value, for example:
#   export QR_BEARER_TOKEN="$(./tools/token.sh)"
set -euo pipefail

die() {
  echo "token: $*" >&2
  exit 1
}

if command -v openssl >/dev/null 2>&1; then
  token=$(openssl rand -hex 32)
else
  token=$(od -An -N32 -tx1 /dev/urandom | tr -d ' \n')
fi

case "$token" in
  '' | *[!0-9a-f]*) die 'the random source did not produce hexadecimal bytes' ;;
esac
[ "${#token}" -eq 64 ] || die 'the random source did not produce 32 bytes'

printf '%s\n' "$token"
