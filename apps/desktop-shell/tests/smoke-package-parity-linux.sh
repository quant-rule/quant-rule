#!/usr/bin/env bash
# Prove the packaged Linux desktop profile, not just the in-tree supervisor.
#
# This is release tier. It extracts the AppImage into a throwaway directory,
# checks the package payload (shell, qr-desktop, six sidecars, client, staged
# config, no interpreter), launches the packaged qr-desktop with
# QR_PROFILE=desktop and a fresh QR_DATA_DIR, completes the verify-mini fixture
# through the packaged gateway, and compares the canonical digest with the
# server profile oracle.
#
# The fixture run uses the configs staged inside the package, so a package that
# ships the right executables but the wrong examples fails. It uses the host
# `quantrule` client only because tools/verify.sh drives the HTTP contract with
# it; the client is not part of the desktop artifact.
#
# The negative control reruns the fixture in a second fresh data directory with
# a seeded digest of 64 zeroes and requires verify.sh to exit nonzero. That is
# the proof the comparison is not vacuous.
#
# Usage:
#   smoke-package-parity-linux.sh [options] path/to/QuantRule_<version>_amd64.AppImage
#
# Options:
#   --expected <digest>    Server canonical digest the package must reproduce
#                          (default: the #819 server oracle digest).
#   --image-digest <d>     QR_IMAGE_DIGEST both profiles record, held constant
#                          so run identity matches the server oracle.
#   --shell-bin <path>     Host `quantrule` client for tools/verify.sh. Defaults
#                          to target/release/quantrule, then target/debug, then
#                          builds the shell.
#   --timeout <seconds>    Fixture timeout (default: 300).
#   --no-negative-control  Skip the seeded-mismatch run.
#   --keep                 Keep the evidence and logs under the printed path.
#
# Exit status is 0 only when the package is complete, six sidecars start, the
# fixture completes, the digest matches, the seeded mismatch fails, and every
# child process is gone after the stop.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"
repository="$(cd "$root/../.." && pwd)"

services=(qr-gateway qr-data qr-model qr-backtest qr-evidence qr-simulation)
server_digest="sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa"
server_image_digest="sha256:1250438fd3a28c91f193f39bb45890d484c96ecddb554875f56bd995786ef4a0"
seeded_digest="sha256:0000000000000000000000000000000000000000000000000000000000000000"

artifact=""
expected="$server_digest"
image_digest="$server_image_digest"
shell_bin=""
timeout=300
negative_control=1
keep=0
tmp=""
tree=""
resource=""
sidecars=""
supervisor=""
supervisor_pid=""
stdin_holder=""
data_dir=""

usage() {
  sed -n '2,40p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

fail() {
  printf 'package-parity: %s\n' "$*" >&2
  exit 1
}

say() {
  printf 'package-parity: %s\n' "$*" >&2
}

while [ $# -gt 0 ]; do
  case "$1" in
    --expected)
      [ $# -ge 2 ] || fail "--expected needs a value"
      expected="$2"
      shift 2
      ;;
    --image-digest)
      [ $# -ge 2 ] || fail "--image-digest needs a value"
      image_digest="$2"
      shift 2
      ;;
    --shell-bin)
      [ $# -ge 2 ] || fail "--shell-bin needs a path"
      shell_bin="$2"
      shift 2
      ;;
    --timeout)
      [ $# -ge 2 ] || fail "--timeout needs a value"
      timeout="$2"
      shift 2
      ;;
    --no-negative-control)
      negative_control=0
      shift
      ;;
    --keep)
      keep=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    -*)
      fail "unknown argument $1"
      ;;
    *)
      [ -z "$artifact" ] || fail "only one AppImage may be given"
      artifact="$1"
      shift
      ;;
  esac
done

case "$timeout" in
  ''|*[!0-9]*) fail "--timeout must be a whole number of seconds" ;;
esac
[ "$timeout" -gt 0 ] || fail "--timeout must be greater than zero"

[[ -n "$artifact" ]] || fail "usage: smoke-package-parity-linux.sh <AppImage>"
[[ -f "$artifact" ]] || fail "the AppImage $artifact does not exist"
artifact="$(cd "$(dirname "$artifact")" && pwd)/$(basename "$artifact")"
command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v jq >/dev/null 2>&1 || fail "jq is required"

case "$expected" in
  sha256:*) ;;
  *) expected="sha256:$expected" ;;
esac
case "$image_digest" in
  sha256:*) ;;
  *) image_digest="sha256:$image_digest" ;;
esac

if [ -z "$shell_bin" ]; then
  for candidate in "$repository/target/release/quantrule" "$repository/target/debug/quantrule"; do
    if [ -x "$candidate" ]; then
      shell_bin="$candidate"
      break
    fi
  done
fi
if [ -z "$shell_bin" ]; then
  say "building the host quantrule client for tools/verify.sh"
  cargo build --quiet --release -p quantrule-shell
  shell_bin="$repository/target/release/quantrule"
fi
[ -x "$shell_bin" ] || fail "the host client $shell_bin is not executable"

tmp="$(mktemp -d "${TMPDIR:-/tmp}/quantrule-package-parity.XXXXXX")"

cleanup() {
  local status=$?
  trap - EXIT INT TERM
  stop_stdin_holder
  if [ -n "$supervisor_pid" ] && kill -0 "$supervisor_pid" 2>/dev/null; then
    kill "$supervisor_pid" 2>/dev/null || true
    wait "$supervisor_pid" 2>/dev/null || true
  fi
  if [ -n "$tree" ]; then
    # A failed stop must not leave a packaged service behind.
    local pid
    while read -r pid; do
      [ -n "$pid" ] || continue
      kill -9 "$pid" 2>/dev/null || true
    done < <(package_pids)
  fi
  if [ "$keep" -eq 1 ] && [ -n "$tmp" ]; then
    say "evidence kept in $tmp"
  elif [ -n "$tmp" ]; then
    rm -rf "$tmp"
  fi
  exit "$status"
}
trap cleanup EXIT INT TERM

start_stdin_holder() {
  local fifo="$1"
  mkfifo "$fifo"
  # A separate process holds the write end. The supervisor opens only the read
  # end, so ending this holder is the EOF the supervisor watches, exactly as
  # closing the shell's stdin would be. Holding the write end in this shell
  # would be inherited by the supervisor and the pipe would never close.
  ( exec 3>"$fifo"; exec sleep 86400 ) &
  stdin_holder=$!
}

stop_stdin_holder() {
  if [ -n "$stdin_holder" ]; then
    kill "$stdin_holder" 2>/dev/null || true
    wait "$stdin_holder" 2>/dev/null || true
    stdin_holder=""
  fi
}

# Every process whose executable lives in the extracted package. `qr-desktop`
# runs once as the supervisor and once as each guard, and the guards are the
# parents of the six services, so this is the complete child tree.
package_pids() {
  local proc exe
  for proc in /proc/[0-9]*; do
    exe="$(readlink "$proc/exe" 2>/dev/null || true)"
    case "$exe" in
      "$tree"/*) printf '%s\n' "${proc##*/}" ;;
    esac
  done
}

pids_for_binary() {
  local want="$1" proc exe
  for proc in /proc/[0-9]*; do
    exe="$(readlink "$proc/exe" 2>/dev/null || true)"
    [ "$exe" = "$want" ] || continue
    printf '%s\n' "${proc##*/}"
  done
}

wait_pids_gone() {
  local seconds="$1"
  shift
  local deadline=$((SECONDS + seconds)) pid alive
  while [ "$SECONDS" -lt "$deadline" ]; do
    alive=0
    for pid in "$@"; do
      [ -n "$pid" ] || continue
      if kill -0 "$pid" 2>/dev/null; then
        alive=1
        break
      fi
    done
    [ "$alive" -eq 0 ] && return 0
    sleep 0.1
  done
  return 1
}

pick_ports() {
  local start="$1" offset base
  for offset in $(seq 0 40); do
    base=$((start + offset * 20))
    local free=1 port
    for port in $(seq "$base" $((base + 4))); do
      if (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null; then
        exec 3>&-
        free=0
        break
      fi
    done
    if [ "$free" -eq 1 ]; then
      printf '%s' "$base"
      return 0
    fi
  done
  return 1
}

ports_field() {
  local record="$1" wanted="$2"
  if [ "$wanted" = "gateway" ]; then
    jq -r '.gateway | split(":") | last' "$record"
  else
    jq -r --arg name "$wanted" '.services[] | select(.name == $name) | .port' "$record"
  fi
}

start_stack() {
  local label="$1" base="$2" prefix="$3"
  data_dir="$prefix/data"
  local logs="$prefix/logs" fifo="$prefix/stdin.fifo"
  mkdir -p "$data_dir" "$logs"
  start_stdin_holder "$fifo"

  say "$label: starting the packaged supervisor on $base with a fresh data directory"
  QR_PROFILE=desktop QR_DATA_DIR="$data_dir" QR_IMAGE_DIGEST="$image_digest" \
    DOCKER_HOST="unix:///nonexistent/quantrule-package-smoke.sock" \
    "$supervisor" \
    --profile desktop \
    --resource-dir "$resource" \
    --sidecar-dir "$sidecars" \
    --data-dir "$data_dir" \
    --gateway-port "$base" \
    --internal-base "$((base + 10))" \
    --health-timeout-ms 120000 \
    --shutdown-grace-ms 10000 \
    --parent-stdin \
    <"$fifo" >"$logs/supervisor.log" 2>&1 &
  supervisor_pid=$!

  local record="$data_dir/supervisor/ports.json" deadline=$((SECONDS + 180))
  while [ "$SECONDS" -lt "$deadline" ]; do
    if ! kill -0 "$supervisor_pid" 2>/dev/null; then
      cat "$logs/supervisor.log" >&2
      fail "$label: the packaged supervisor exited before becoming ready"
    fi
    if [ -f "$record" ] && grep -q '"state": "ready"' "$record" 2>/dev/null; then
      break
    fi
    sleep 0.2
  done
  if ! grep -q '"state": "ready"' "$record" 2>/dev/null; then
    cat "$logs/supervisor.log" >&2
    fail "$label: the packaged supervisor did not become ready"
  fi
  cp "$record" "$prefix/ports-ready.json"

  local gateway
  gateway="$(ports_field "$record" gateway)" || {
    cat "$logs/supervisor.log" >&2
    fail "$label: cannot read the gateway port from ports.json"
  }
  gateway_port="$gateway"
  say "$label: gateway healthy on $gateway_port"
}

verify_sidecars() {
  local label="$1" record="$data_dir/supervisor/ports.json"
  jq -e '
    .profile == "desktop"
    and (.services | length) == 6
    and ([.services[].name] | sort) ==
      ["qr-backtest", "qr-data", "qr-evidence", "qr-gateway", "qr-model", "qr-simulation"]
  ' "$record" >/dev/null \
    || fail "$label: ports.json does not name the six desktop sidecars"

  local name found guards=()
  for name in "${services[@]}"; do
    found="$(pids_for_binary "$sidecars/$name")"
    [ -n "$found" ] || fail "$label: $name is not running from the package"
    guards+=("$(jq -r --arg name "$name" '.services[] | select(.name == $name) | .pid' "$record")")
  done
  for pid in "${guards[@]}"; do
    kill -0 "$pid" 2>/dev/null || fail "$label: guard pid $pid is not alive"
  done

  local gateway data model evidence simulation
  gateway="$(ports_field "$record" qr-gateway)"
  data="$(ports_field "$record" qr-data)"
  model="$(ports_field "$record" qr-model)"
  evidence="$(ports_field "$record" qr-evidence)"
  simulation="$(ports_field "$record" qr-simulation)"
  for port in "$gateway" "$data" "$model" "$evidence" "$simulation"; do
    for _ in $(seq 1 100); do
      if curl -fsS --max-time 2 "http://127.0.0.1:$port/health" 2>/dev/null | grep -q '"state":"healthy"'; then
        break
      fi
      sleep 0.1
    done
    curl -fsS --max-time 2 "http://127.0.0.1:$port/health" 2>/dev/null \
      | grep -q '"state":"healthy"' \
      || fail "$label: the service on port $port is not healthy"
  done

  local worker_guard worker_pid
  worker_guard="$(jq -r '.services[] | select(.name == "qr-backtest") | .pid' "$record")"
  worker_pid="$(pids_for_binary "$sidecars/qr-backtest")"
  [ -n "$worker_pid" ] || fail "$label: qr-backtest is not running"
  kill -0 "$worker_guard" 2>/dev/null || fail "$label: qr-backtest guard $worker_guard is not alive"
  say "$label: six sidecars running; five health endpoints healthy; qr-backtest guard $worker_guard alive"
}

run_fixture() {
  local label="$1" expected_value="$2" evidence="$3" log="$4"
  local config="$resource/config/verify-mini/run.json"
  local replica="$resource/config/verify-mini-replica/run.json"
  [ -f "$config" ] || fail "$label: the package has no staged verify-mini config"
  [ -f "$replica" ] || fail "$label: the package has no staged verify-mini replica config"
  say "$label: running the fixture from the staged config with the host client"
  QUANTRULE_SHELL="$shell_bin" "$repository/tools/verify.sh" \
    --api-url "http://127.0.0.1:$gateway_port/api/v1" \
    --config "$config" \
    --replica "$replica" \
    --evidence-dir "$evidence" \
    --expected "$expected_value" \
    --timeout "$timeout" >"$log" 2>&1
}

stop_stack() {
  local label="$1"
  local pids=()
  mapfile -t pids < <(package_pids)
  stop_stdin_holder
  local deadline=$((SECONDS + 30))
  while kill -0 "$supervisor_pid" 2>/dev/null && [ "$SECONDS" -lt "$deadline" ]; do
    sleep 0.1
  done
  if kill -0 "$supervisor_pid" 2>/dev/null; then
    kill "$supervisor_pid" 2>/dev/null || true
    fail "$label: the packaged supervisor did not stop within 30 seconds"
  fi
  local status=0
  wait "$supervisor_pid" || status=$?
  supervisor_pid=""
  [ "$status" -eq 0 ] || fail "$label: the packaged supervisor exited $status on stop"
  wait_pids_gone 20 "${pids[@]}" \
    || fail "$label: a packaged child survived the supervisor stop"
  [ -z "$(package_pids)" ] \
    || fail "$label: a packaged process appeared or survived after the stop"
  say "$label: supervisor and every packaged child stopped"
}

# --- static package contents ------------------------------------------------

(
  cd "$tmp"
  "$artifact" --appimage-extract >/dev/null
)
tree="$tmp/squashfs-root"
bash "$root/tests/check-package.sh" --tree "$tree" >&2

sidecars="$(find "$tree" -type d -name sidecars -print -quit)"
[ -n "$sidecars" ] || fail "the package has no sidecars directory"
supervisor="$sidecars/qr-desktop"
[ -x "$supervisor" ] || fail "the package has no qr-desktop supervisor"
resource="$(dirname "$sidecars")"

for name in "${services[@]}"; do
  [ -x "$sidecars/$name" ] || fail "the package has no $name sidecar"
done
[ -f "$resource/client/index.html" ] || fail "the package has no staged client"
for config_name in verify-mini verify-mini-replica; do
  [ -f "$resource/config/$config_name/run.json" ] \
    || fail "the package has no staged $config_name config"
  [ -f "$resource/config/$config_name/candles.csv" ] \
    || fail "the package has no staged $config_name data"
done
if [[ -n "$(find "$tree" -type f \( -name 'py*' -o -name '*.py' -o -name bun -o -name node \
  -o -name 'bun.exe' -o -name 'node.exe' \) -print -quit)" ]]; then
  fail "the package carries a Python, Bun, or Node runtime"
fi
say "package contents ok: shell, qr-desktop, six sidecars, client, configs, no interpreter"

# --- packaged run and digest ------------------------------------------------

base="$(pick_ports 28800)" || fail "no free loopback port block for the desktop profile"
start_stack "package" "$base" "$tmp/run"
verify_sidecars "package"
if ! run_fixture "package" "$expected" "$tmp/run/evidence" "$tmp/run/verify.log"; then
  cat "$tmp/run/verify.log" >&2
  fail "the packaged fixture failed"
fi
cat "$tmp/run/verify.log" >&2
observed="sha256:$(cat "$tmp/run/evidence/digest")"
[ "$observed" = "$expected" ] \
  || fail "the packaged digest $observed does not match the server oracle $expected"
artifacts="$(find "$tmp/run/evidence/artifacts" -type f | wc -l | tr -d ' ')"
say "packaged run reproduced $expected; $artifacts canonical artifact files downloaded"
stop_stack "package"

# --- negative control -------------------------------------------------------

if [ "$negative_control" -eq 1 ]; then
  base="$(pick_ports 28900)" || fail "no free loopback port block for the negative control"
  start_stack "negative" "$base" "$tmp/negative"
  verify_sidecars "negative"
  if run_fixture "negative" "$seeded_digest" "$tmp/negative/evidence" "$tmp/negative/verify.log"; then
    fail "the seeded mismatch did not fail; the digest comparison is vacuous"
  fi
  grep -q "expected $seeded_digest" "$tmp/negative/verify.log" \
    || fail "the seeded mismatch failed without naming the expected digest"
  say "negative control ok: the seeded mismatch $seeded_digest failed as required"
  stop_stack "negative"
fi

if [ "$keep" -eq 1 ]; then
  say "canonical digest $observed"
fi
printf 'package-parity ok: %s\n' "$observed"
