#!/usr/bin/env bash
# Check the contents of a Linux desktop package.
#
# The AppImage is the only Linux artifact. It must carry the shell, the
# supervisor, the six service binaries, the built client, and no interpreter.
# This is a static check: it does not start the profile or run a backtest.
#
# Usage:
#   check-package.sh path/to/QuantRule_<version>_amd64.AppImage
#   check-package.sh --tree <extracted AppDir>   # a CI or local tree
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
services=(qr-gateway qr-data qr-model qr-backtest qr-evidence qr-simulation)
tmp=""
status=0

fail() {
  echo "package: $1" >&2
  status=1
}

cleanup() {
  if [[ -n "$tmp" ]]; then
    rm -rf "$tmp"
  fi
}
trap cleanup EXIT

if [[ "${1:-}" == "--tree" ]]; then
  tree="${2:-}"
  [[ -d "$tree" ]] || {
    echo "package: --tree needs an extracted directory" >&2
    exit 2
  }
elif [[ -n "${1:-}" && -f "${1:-}" ]]; then
  artifact="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
  case "$artifact" in
    *.AppImage) ;;
    *)
      echo "package: expected a .AppImage or --tree" >&2
      exit 2
      ;;
  esac
  tmp="$(mktemp -d)"
  (
    cd "$tmp"
    # Extraction does not need FUSE, which makes this check work on a headless
    # runner even when the AppImage cannot be mounted.
    "$artifact" --appimage-extract >/dev/null
  )
  tree="$tmp/squashfs-root"
else
  echo "usage: check-package.sh <artifact.AppImage> | --tree <directory>" >&2
  exit 2
fi

shell_bin="$(find "$tree" -maxdepth 4 -type f -name quantrule-desktop-shell -print -quit)"
if [[ -z "$shell_bin" ]]; then
  fail "the shell binary is missing"
fi

sidecar_dir="$(find "$tree" -type d -name sidecars -print -quit)"
if [[ -z "$sidecar_dir" ]]; then
  fail "the sidecars directory is missing"
else
  for name in "${services[@]}" qr-desktop; do
    found="$(find "$sidecar_dir" -maxdepth 1 -type f \( -name "$name" -o -name "$name.exe" \) -print -quit)"
    if [[ -z "$found" ]]; then
      fail "sidecars/$name is missing"
    elif [[ ! -x "$found" ]]; then
      fail "sidecars/$name is not executable"
    fi
  done
  if [[ -n "$(find "$sidecar_dir" -maxdepth 1 -type f -name 'stub-sidecar*' -print -quit)" ]]; then
    fail "the test stub is packaged"
  fi
fi

client="$(find "$tree" -type f -name index.html -path '*/client/*' -print -quit)"
if [[ -z "$client" ]]; then
  fail "the built client (client/index.html) is missing"
fi

python_hits="$(find "$tree" -type f \( -name 'py*' -o -name '*.py' \) -print)"
if [[ -n "$python_hits" ]]; then
  fail "the package carries Python:"
  printf '%s\n' "$python_hits" >&2
fi

runtime_hits="$(find "$tree" -type f \( -name bun -o -name node -o -name 'bun.exe' -o -name 'node.exe' \) -print)"
if [[ -n "$runtime_hits" ]]; then
  fail "the package carries a Bun or Node runtime:"
  printf '%s\n' "$runtime_hits" >&2
fi

if [[ "$status" -ne 0 ]]; then
  exit 1
fi

echo "package ok: shell, supervisor, six sidecars, client, no interpreter"
