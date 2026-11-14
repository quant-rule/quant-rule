#!/usr/bin/env bash
# Reviewer verification: run one fixture twice and print one canonical digest.
#
# The digest covers results.json, *.csv, and *.ndjson under the run output and
# excludes provenance, logs, and images, exactly as the run contract defines.
# Two hosts that print the same line have reproduced the same data plane.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

config="config/examples/verify-mini/run.json"
strategy="config/examples/verify-mini/strategy.json"
expected=""

usage() {
  cat <<'USAGE'
usage: tools/verify.sh [--config <run.json>] [--strategy <strategy.json>] [--expected <digest>]

Runs the fixture twice, compares the canonical digests, and prints one line.
--expected accepts a full "canonical digest sha256:..." line or the digest.
USAGE
}

while [ $# -gt 0 ]; do
  case "$1" in
    --config)
      config="$2"
      shift 2
      ;;
    --strategy)
      strategy="$2"
      shift 2
      ;;
    --expected)
      expected="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "verify: unknown argument $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

target="${CARGO_TARGET_DIR:-target}"
shell="${QUANTRULE_SHELL:-$target/debug/quantrule}"
if [ ! -x "$shell" ]; then
  cargo build -p quantrule-shell
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/quantrule-verify.XXXXXX")"
trap 'rm -rf "$work"' EXIT

"$shell" backtest run --config "$config" --strategy "$strategy" --out "$work/a" >/dev/null
"$shell" backtest run --config "$config" --strategy "$strategy" --out "$work/b" >/dev/null

digest() {
  local directory="$1"
  (
    cd "$directory"
    find . -type f \( -name 'results.json' -o -name '*.csv' -o -name '*.ndjson' \) \
      | sed 's|^\./||' \
      | LC_ALL=C sort \
      | while IFS= read -r path; do
          hash="$(sha256sum "$path" | cut -d' ' -f1)"
          printf '%s\0sha256:%s\n' "$path" "$hash"
        done
  ) | sha256sum | cut -d' ' -f1
}

first="$(digest "$work/a")"
second="$(digest "$work/b")"
if [ "$first" != "$second" ]; then
  echo "verify failed: two runs produced different digests" >&2
  echo "  first:  sha256:$first" >&2
  echo "  second: sha256:$second" >&2
  exit 1
fi

if [ -n "$expected" ]; then
  expected="${expected#canonical digest }"
  if [ "$expected" != "sha256:$first" ]; then
    echo "verify failed: expected $expected, observed sha256:$first" >&2
    exit 1
  fi
  echo "verified sha256:$first"
else
  echo "canonical digest sha256:$first"
fi
