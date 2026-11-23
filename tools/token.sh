#!/usr/bin/env bash
# Print or write one new bearer token for the gateway's remote profile.
#
# The token is 32 random bytes rendered as 64 lowercase hexadecimal characters.
# That form is safe in a shell, an HTTP header, and a file, and it carries 256
# bits of entropy. With `--out` the value is written to a mode-600 file and only
# the path is printed; the gateway reads that file, never an environment value.
# Without `--out` the value exists on standard output only.
#
# Usage:
#   tools/token.sh                 print a new token
#   tools/token.sh --out PATH      write a new token to PATH at mode 600
#
# The caller stores the printed value, for example:
#   export QUANTRULE_TOKEN="$(./tools/token.sh)"
set -euo pipefail
umask 077

die() {
  echo "token: $*" >&2
  exit 1
}

out=""
if [ "$#" -gt 0 ]; then
  case "$1" in
    --out)
      [ "$#" -ge 2 ] || die '--out needs a path'
      out="$2"
      shift 2
      ;;
    -h | --help)
      sed -n '2,14p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
fi
[ "$#" -eq 0 ] || die "unknown argument: $1"

if command -v openssl >/dev/null 2>&1; then
  token=$(openssl rand -hex 32)
else
  token=$(od -An -N32 -tx1 /dev/urandom | tr -d ' \n')
fi

case "$token" in
  '' | *[!0-9a-f]*) die 'the random source did not produce hexadecimal bytes' ;;
esac
[ "${#token}" -eq 64 ] || die 'the random source did not produce 32 bytes'

if [ -n "$out" ]; then
  printf '%s\n' "$token" > "$out"
  chmod 600 "$out"
  printf '%s\n' "$out"
else
  printf '%s\n' "$token"
fi
