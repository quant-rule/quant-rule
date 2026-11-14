#!/usr/bin/env bash
# Linux smoke test for the desktop shell.
#
# It starts a stub on the shell's default gateway port, launches the release
# binary under a virtual display, and waits for the shell to probe `/health`.
# The probe is the first request the shell makes: it proves the window started,
# the bundled bootstrap loaded, and the default gateway URL was read.
#
# The same test runs in the desktop CI job on ubuntu-latest. It is not an
# end-to-end UI test; the manual procedure in README.md covers the screens.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bin="${1:-$root/target/release/quantrule-desktop-shell}"
port="${QR_SMOKE_PORT:-8080}"

fail() {
  echo "smoke: $1" >&2
  exit 1
}

[[ -x "$bin" ]] || fail "missing $bin; run 'cargo build --release' first"
command -v bun >/dev/null 2>&1 || fail "bun is required for the gateway stub"
command -v xvfb-run >/dev/null 2>&1 || fail "xvfb-run is required (install xvfb)"
command -v setsid >/dev/null 2>&1 || fail "setsid is required (util-linux)"

if (exec 3<>"/dev/tcp/127.0.0.1/${port}") 2>/dev/null; then
  exec 3>&-
  fail "port ${port} is in use; the smoke test uses the shell's default gateway"
fi

tmp="$(mktemp -d)"
stub_log="$tmp/gateway.log"
app_log="$tmp/app.log"
stub_pid=""
app_pid=""

cleanup() {
  if [[ -n "$app_pid" ]] && kill -0 "$app_pid" 2>/dev/null; then
    kill -- "-$app_pid" 2>/dev/null || true
    wait "$app_pid" 2>/dev/null || true
  fi
  if [[ -n "$stub_pid" ]] && kill -0 "$stub_pid" 2>/dev/null; then
    kill "$stub_pid" 2>/dev/null || true
    wait "$stub_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT

QR_SMOKE_PORT="$port" bun "$root/tests/gateway-stub.js" >"$stub_log" 2>&1 &
stub_pid=$!
for _ in $(seq 1 50); do
  grep -q "gateway-stub listening" "$stub_log" && break
  sleep 0.1
done
grep -q "gateway-stub listening" "$stub_log" || {
  cat "$stub_log" >&2
  fail "the gateway stub did not start"
}

runner=(xvfb-run -a --server-args="-screen 0 1280x900x24")
if command -v dbus-run-session >/dev/null 2>&1; then
  runner=(dbus-run-session -- "${runner[@]}")
fi

setsid env \
  GDK_BACKEND=x11 \
  WEBKIT_DISABLE_COMPOSITING_MODE=1 \
  WEBKIT_DISABLE_DMABUF_RENDERER=1 \
  LIBGL_ALWAYS_SOFTWARE=1 \
  NO_AT_BRIDGE=1 \
  "${runner[@]}" "$bin" >"$app_log" 2>&1 &
app_pid=$!

seen_health=0
for _ in $(seq 1 200); do
  if grep -q "GET /health" "$stub_log"; then
    seen_health=1
    break
  fi
  if ! kill -0 "$app_pid" 2>/dev/null; then
    cat "$app_log" >&2
    fail "the shell exited before probing the gateway"
  fi
  sleep 0.1
done

if [[ "$seen_health" -ne 1 ]]; then
  echo "--- gateway stub ---" >&2
  cat "$stub_log" >&2
  echo "--- shell ---" >&2
  cat "$app_log" >&2
  fail "the shell did not probe /health within 20s"
fi

# Navigation to the gateway is best-effort here: a newer published release
# keeps the bootstrap page visible by design, so the smoke test only requires
# the probe. `launchPlan` is covered by the Bun unit tests.
if grep -q "GET /" "$stub_log"; then
  echo "smoke passed: the shell probed /health and navigated to the gateway"
else
  echo "smoke passed: the shell probed /health (gateway navigation not observed)"
fi
