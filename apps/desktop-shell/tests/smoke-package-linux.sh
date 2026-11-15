#!/usr/bin/env bash
# Prove the packaged supervisor starts six sidecars from the package.
#
# The six real service binaries need the embedded backends (#693/#694), so this
# smoke replaces them with the test stub in a throwaway extraction of the
# AppImage. The supervisor itself is the packaged one. That proves the resource
# layout, the launch contract, health, and stop without claiming a run or a
# digest: those need the real backends and belong to the parity suite.
#
# Usage:
#   smoke-package-linux.sh path/to/QuantRule_<version>_amd64.AppImage
#   QR_STUB_BIN=path/to/stub-sidecar smoke-package-linux.sh <artifact>
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
artifact="${1:-}"
services=(qr-gateway qr-data qr-model qr-backtest qr-evidence qr-simulation)
tmp=""
supervisor_pid=""

fail() {
  echo "package-smoke: $1" >&2
  exit 1
}

cleanup() {
  if [[ -n "$supervisor_pid" ]] && kill -0 "$supervisor_pid" 2>/dev/null; then
    kill "$supervisor_pid" 2>/dev/null || true
    wait "$supervisor_pid" 2>/dev/null || true
  fi
  if [[ -n "$tmp" ]]; then
    rm -rf "$tmp"
  fi
}
trap cleanup EXIT

[[ -n "$artifact" && -f "$artifact" ]] || fail "usage: smoke-package-linux.sh <AppImage>"
artifact="$(cd "$(dirname "$artifact")" && pwd)/$(basename "$artifact")"
command -v curl >/dev/null 2>&1 || fail "curl is required"

stub="${QR_STUB_BIN:-}"
if [[ -z "$stub" ]]; then
  cargo build --quiet --release --manifest-path "$root/supervisor/Cargo.toml" --bin stub-sidecar
  stub="$root/supervisor/target/release/stub-sidecar"
fi
[[ -x "$stub" ]] || fail "missing the stub binary at $stub"

tmp="$(mktemp -d)"
(
  cd "$tmp"
  "$artifact" --appimage-extract >/dev/null
)
tree="$tmp/squashfs-root"
sidecars="$(find "$tree" -type d -name sidecars -print -quit)"
[[ -n "$sidecars" ]] || fail "the package has no sidecars directory"
supervisor="$sidecars/qr-desktop"
[[ -x "$supervisor" ]] || fail "the package has no qr-desktop"
resource="$(dirname "$sidecars")"

for name in "${services[@]}"; do
  cp "$stub" "$sidecars/$name"
  chmod +x "$sidecars/$name"
done

base=28700
for _ in $(seq 1 20); do
  if ! (exec 3<>"/dev/tcp/127.0.0.1/$base") 2>/dev/null; then
    break
  fi
  base=$((base + 20))
done
internal=$((base + 10))
markers="$tmp/markers"
mkdir -p "$markers"

QR_STUB_MARKER_DIR="$markers" "$supervisor" \
  --profile desktop \
  --resource-dir "$resource" \
  --sidecar-dir "$sidecars" \
  --data-dir "$tmp/data" \
  --gateway-port "$base" \
  --internal-base "$internal" \
  --health-timeout-ms 15000 \
  >"$tmp/supervisor.log" 2>&1 &
supervisor_pid=$!

ready=0
for _ in $(seq 1 150); do
  if curl -fsS --max-time 1 "http://127.0.0.1:$base/health" >/dev/null 2>&1; then
    ready=1
    break
  fi
  if ! kill -0 "$supervisor_pid" 2>/dev/null; then
    cat "$tmp/supervisor.log" >&2
    fail "the packaged supervisor exited before becoming healthy"
  fi
  sleep 0.1
done
[[ "$ready" -eq 1 ]] || {
  cat "$tmp/supervisor.log" >&2
  fail "the packaged supervisor did not become healthy"
}

for name in "${services[@]}"; do
  [[ -s "$markers/$name.pid" ]] || fail "$name did not start"
done
record="$tmp/data/supervisor/ports.json"
for _ in $(seq 1 100); do
  if grep -q '"state": "ready"' "$record" 2>/dev/null; then
    break
  fi
  if ! kill -0 "$supervisor_pid" 2>/dev/null; then
    cat "$tmp/supervisor.log" >&2
    fail "the packaged supervisor exited before recording readiness"
  fi
  sleep 0.1
done
grep -q '"state": "ready"' "$record" 2>/dev/null || fail "ports.json is not ready"

pids=()
for name in "${services[@]}"; do
  pids+=("$(cat "$markers/$name.pid")")
done
kill "$supervisor_pid" 2>/dev/null || true
wait "$supervisor_pid" 2>/dev/null || true
supervisor_pid=""

for pid in "${pids[@]}"; do
  for _ in $(seq 1 100); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.1
  done
  kill -0 "$pid" 2>/dev/null && fail "sidecar pid $pid survived the supervisor"
done

echo "package-smoke ok: the AppImage supervisor started six sidecars and stopped them"
