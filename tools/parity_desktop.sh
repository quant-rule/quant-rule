#!/usr/bin/env bash
# Desktop/server parity for the verify-mini fixture.
#
# One fixture, two deployment profiles. The desktop side runs six service
# binaries with QR_PROFILE=desktop, one absolute QR_DATA_DIR, and the URL
# environment the supervisor sets; the server side runs the same binaries with
# QR_PROFILE=server against Postgres, MinIO, and RabbitMQ, or points at a stack
# that is already running. The canonical digest, every canonical artifact byte,
# and the run status projection (manifest entry set and stage states) must
# agree. The server profile is the oracle: a mismatch fails, and the digest is
# never adjusted.
#
# The desktop side never needs Docker. The server side does when it brings up
# its own dependencies through Compose; --server-url points at a running stack
# instead, and --desktop-only with --expected compares against a server digest
# recorded by tools/verify.sh.
#
# A seeded difference fails: --expected sha256:000... exits nonzero and prints
# both values. That is the negative control for this harness.
#
# Release tier, not the fast gate: it starts six processes per profile and, on
# the server side, three containers.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

# The image identity both profiles record, so the run id can be compared too.
# It is the v1.0.0 image digest: a pinned fixture identity for run identity, not
# a claim that this tree is that image. The canonical digest does not depend on
# it.
default_image_digest="sha256:1250438fd3a28c91f193f39bb45890d484c96ecddb554875f56bd995786ef4a0"

desktop_only=0
expected=""
server_url=""
server_compose=0
supervisor_bin=""
services_dir="$root/target/debug"
image_digest="$default_image_digest"
gateway_port=18080
internal_base=18081
timeout=300
repeat=1
keep=0
build=1

usage() {
  cat <<'USAGE'
usage: tools/parity_desktop.sh [--desktop-only] [--expected <digest>]
                               [--server-url <url>] [--server-compose]
                               [--via-supervisor <qr-desktop>]
                               [--services-dir <dir>] [--image-digest <digest>]
                               [--gateway-port <n>] [--internal-base <n>]
                               [--timeout <seconds>] [--repeat <n>] [--keep]
                               [--no-build]

Runs the verify-mini fixture on the desktop profile and compares the canonical
digest, the canonical artifact bytes, and the run status with the server
profile. The desktop side starts six sidecars and uses no Docker.

  --desktop-only  Do not contact or start a server profile. With --expected this
                  is the pre-package parity check against a recorded digest.
  --expected      A server canonical digest. The desktop digest must equal it.
  --server-url    A server-profile gateway base URL, for example
                  http://127.0.0.1:8080/api/v1.
  --server-compose
                  Bring up Postgres, MinIO, and RabbitMQ under an isolated
                  Compose project, run the six binaries in the server profile,
                  and tear the project down afterwards. The server side is the
                  only part that needs Docker.
  --via-supervisor
                  Start the desktop side through qr-desktop instead of six
                  direct children. The six sidecars must already be in
                  --services-dir.
  --services-dir  Where the six service binaries and the shell live
                  (default: target/debug).
  --image-digest  The QR_IMAGE_DIGEST both profiles record, so run ids compare
                  (default: the v1.0.0 image digest).
  --gateway-port  The desktop gateway port (default: 18080).
  --internal-base The desktop data port; model, evidence, and simulation take
                  the next three (default: 18081).
  --timeout       Seconds to wait for each run (default: 300).
  --repeat        How many rounds to run (default: 1). Round 1 starts desktop
                  first and round 2 starts server first, so a profile that
                  depends on first-run state is visible.
  --keep          Keep the evidence and logs under the printed work directory.
  --no-build      Do not build the binaries first.

Exit status is 0 only when every requested comparison passes.
USAGE
}

fail() {
  printf 'parity: %s\n' "$*" >&2
  exit 1
}

say() {
  printf 'parity: %s\n' "$*" >&2
}

usage_error() {
  printf 'parity: %s\n' "$*" >&2
  usage >&2
  exit 2
}

while [ $# -gt 0 ]; do
  case "$1" in
    --desktop-only)
      desktop_only=1
      shift
      ;;
    --expected)
      [ $# -ge 2 ] || usage_error "--expected needs a value"
      expected="$2"
      shift 2
      ;;
    --server-url)
      [ $# -ge 2 ] || usage_error "--server-url needs a value"
      server_url="$2"
      shift 2
      ;;
    --server-compose)
      server_compose=1
      shift
      ;;
    --via-supervisor)
      [ $# -ge 2 ] || usage_error "--via-supervisor needs a path"
      supervisor_bin="$2"
      shift 2
      ;;
    --services-dir)
      [ $# -ge 2 ] || usage_error "--services-dir needs a value"
      services_dir="$2"
      shift 2
      ;;
    --image-digest)
      [ $# -ge 2 ] || usage_error "--image-digest needs a value"
      image_digest="$2"
      shift 2
      ;;
    --gateway-port)
      [ $# -ge 2 ] || usage_error "--gateway-port needs a value"
      gateway_port="$2"
      shift 2
      ;;
    --internal-base)
      [ $# -ge 2 ] || usage_error "--internal-base needs a value"
      internal_base="$2"
      shift 2
      ;;
    --timeout)
      [ $# -ge 2 ] || usage_error "--timeout needs a value"
      timeout="$2"
      shift 2
      ;;
    --repeat)
      [ $# -ge 2 ] || usage_error "--repeat needs a value"
      repeat="$2"
      shift 2
      ;;
    --keep)
      keep=1
      shift
      ;;
    --no-build)
      build=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage_error "unknown argument $1"
      ;;
  esac
done

for value in "$gateway_port" "$internal_base" "$timeout" "$repeat"; do
  case "$value" in
    ''|*[!0-9]*) usage_error "numeric options must be whole numbers" ;;
  esac
done
[ "$timeout" -gt 0 ] || usage_error "--timeout must be greater than zero"
[ "$repeat" -gt 0 ] || usage_error "--repeat must be at least one"

if [ -z "$server_url" ] && [ "$server_compose" -eq 0 ] && [ "$desktop_only" -eq 0 ]; then
  if [ -n "$expected" ]; then
    desktop_only=1
  else
    usage_error "choose --server-url, --server-compose, or --desktop-only with --expected"
  fi
fi
if [ "$desktop_only" -eq 1 ] && [ "$server_compose" -eq 1 ]; then
  usage_error "--desktop-only and --server-compose are mutually exclusive"
fi

command -v curl >/dev/null || fail "curl is required to talk to the gateway"
command -v python3 >/dev/null || fail "python3 is required by tools/verify.sh"

services_dir="$(cd "$services_dir" && pwd)" || fail "the services directory does not exist"
shell="$services_dir/quantrule"

if [ "$build" -eq 1 ]; then
  say "building the six service binaries and the shell"
  cargo build --quiet --bins \
    -p quantrule-gateway \
    -p quantrule-data-service \
    -p quantrule-model-service \
    -p quantrule-backtest-service \
    -p quantrule-evidence-service \
    -p quantrule-simulation-service \
    -p quantrule-shell
fi

for binary in qr-gateway qr-data qr-model qr-backtest qr-evidence qr-simulation; do
  [ -x "$services_dir/$binary" ] || fail "missing $services_dir/$binary; run cargo build --bins"
done
[ -x "$shell" ] || fail "missing the shell at $shell; run cargo build -p quantrule-shell"
if [ -n "$supervisor_bin" ] && [ ! -x "$supervisor_bin" ]; then
  fail "the supervisor at $supervisor_bin is not executable"
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/quantrule-parity.XXXXXX")"
compose_project="quantrule-parity"
compose_started=0
declare -a desktop_pids=()
declare -a server_pids=()
declare -a common=()
desktop_logs=""
supervisor_pid=""

stop_pids() {
  local -n pids="$1"
  local pid
  for pid in "${pids[@]:-}"; do
    [ -n "$pid" ] || continue
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  done
  pids=()
}

stop_desktop_profile() {
  if [ -n "$supervisor_pid" ]; then
    kill "$supervisor_pid" 2>/dev/null || true
    wait "$supervisor_pid" 2>/dev/null || true
    supervisor_pid=""
    # The guards end their services when the control socket closes.
    sleep 1
  fi
  stop_pids desktop_pids
}

cleanup() {
  local status=$?
  trap - EXIT INT TERM
  stop_desktop_profile || true
  stop_pids server_pids || true
  if [ "$compose_started" -eq 1 ]; then
    say "stopping the parity Compose project"
    docker compose -p "$compose_project" -f "$root/compose.yaml" -f "$work/compose.override.yaml" \
      down -v --remove-orphans >/dev/null 2>&1 || true
  fi
  if [ "$keep" -eq 0 ]; then
    rm -rf "$work"
  else
    say "evidence kept in $work"
  fi
  exit "$status"
}
trap cleanup EXIT INT TERM

port_free() {
  local port="$1"
  ! (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null
}

require_ports() {
  local profile="$1"
  shift
  for port in "$@"; do
    port_free "$port" || fail "127.0.0.1:$port is in use; stop it or pass a different port (${profile} profile)"
  done
}

health_healthy() {
  local url="$1" body
  body="$(curl -fsS --max-time 2 "$url/health" 2>/dev/null)" || return 1
  printf '%s' "$body" | grep -q '"state":"healthy"'
}

wait_for_url() {
  local url="$1" deadline
  deadline=$((SECONDS + 60))
  while [ "$SECONDS" -lt "$deadline" ]; do
    if health_healthy "$url"; then
      return 0
    fi
    sleep 1
  done
  return 1
}

wait_for_profile() {
  local label="$1" ports="$2"
  # ports: gateway data model evidence simulation
  local gateway data model evidence simulation
  read -r gateway data model evidence simulation <<<"$ports"
  wait_for_url "http://127.0.0.1:$gateway" || return 1
  wait_for_url "http://127.0.0.1:$data" || return 1
  wait_for_url "http://127.0.0.1:$model" || return 1
  wait_for_url "http://127.0.0.1:$evidence" || return 1
  wait_for_url "http://127.0.0.1:$simulation" || return 1
  local pid
  if [ "$label" = server ]; then
    for pid in "${server_pids[@]:-}"; do
      [ -n "$pid" ] || continue
      kill -0 "$pid" 2>/dev/null || return 1
    done
  elif [ -z "$supervisor_pid" ]; then
    for pid in "${desktop_pids[@]:-}"; do
      [ -n "$pid" ] || continue
      kill -0 "$pid" 2>/dev/null || return 1
    done
  fi
  return 0
}

start_desktop_child() {
  local name="$1" port="$2"
  shift 2
  local bind=()
  [ "$port" != "-" ] && bind=(QR_BIND="127.0.0.1:$port")
  env -u QR_DATABASE_URL -u QR_OBJECT_STORE_URL -u QR_OBJECT_BUCKET \
    -u QR_QUEUE_URL -u QR_BEARER_TOKEN \
    "${common[@]}" "${bind[@]}" "$@" "$services_dir/$name" >"$desktop_logs/$name.log" 2>&1 &
  desktop_pids+=("$!")
}

start_desktop_raw() {
  local round="$1" data_dir="$2" logs="$3" port_gateway="$4" port_base="$5"
  local port_model=$((port_base + 1))
  local port_evidence=$((port_base + 2))
  local port_simulation=$((port_base + 3))
  common=(
    QR_PROFILE=desktop
    QR_DATA_DIR="$data_dir"
    QR_CONFIG_ROOT="$root/config"
    QR_IMAGE_DIGEST="$image_digest"
  )
  desktop_logs="$logs"
  start_desktop_child qr-data "$port_base" QR_DATA_URL="http://127.0.0.1:$port_base"
  start_desktop_child qr-model "$port_model" QR_DATA_URL="http://127.0.0.1:$port_base"
  start_desktop_child qr-evidence "$port_evidence" QR_DATA_URL="http://127.0.0.1:$port_base"
  start_desktop_child qr-simulation "$port_simulation"
  start_desktop_child qr-backtest - \
    QR_DATA_URL="http://127.0.0.1:$port_base" \
    QR_MODEL_URL="http://127.0.0.1:$port_model" \
    QR_EVIDENCE_URL="http://127.0.0.1:$port_evidence"
  start_desktop_child qr-gateway "$port_gateway" \
    QR_DATA_URL="http://127.0.0.1:$port_base" \
    QR_MODEL_URL="http://127.0.0.1:$port_model" \
    QR_EVIDENCE_URL="http://127.0.0.1:$port_evidence" \
    QR_SIMULATION_URL="http://127.0.0.1:$port_simulation"
  say "round $round: desktop profile started direct on $port_gateway"
}

start_desktop_supervisor() {
  local round="$1" data_dir="$2" logs="$3" port_gateway="$4" port_base="$5"
  mkdir -p "$data_dir"
  QR_DATA_DIR="$data_dir" QR_CONFIG_ROOT="$root/config" QR_IMAGE_DIGEST="$image_digest" \
    "$supervisor_bin" --profile desktop \
    --data-dir "$data_dir" \
    --sidecar-dir "$services_dir" \
    --gateway-port "$port_gateway" \
    --internal-base "$port_base" \
    --health-timeout-ms 60000 \
    >"$logs/supervisor.log" 2>&1 &
  supervisor_pid=$!
  say "round $round: desktop profile started through qr-desktop (pid $supervisor_pid)"
}

# Print "gateway data model evidence simulation" once the ports are known.
desktop_ports() {
  local data_dir="$1" fixed_gateway="$2" fixed_base="$3"
  if [ -z "$supervisor_pid" ]; then
    printf '%s %s %s %s %s\n' "$fixed_gateway" "$fixed_base" \
      "$((fixed_base + 1))" "$((fixed_base + 2))" "$((fixed_base + 3))"
    return 0
  fi
  local file="$data_dir/supervisor/ports.json"
  for _ in $(seq 1 60); do
    if [ -f "$file" ] && python3 - "$file" <<'PY' 2>/dev/null
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    record = json.load(handle)
ports = {service["name"]: service["port"] for service in record["services"]}
gateway = record["gateway"].rsplit(":", 1)[1]
print(gateway, ports["qr-data"], ports["qr-model"], ports["qr-evidence"], ports["qr-simulation"])
PY
    then
      return 0
    fi
    kill -0 "$supervisor_pid" 2>/dev/null || return 1
    sleep 1
  done
  return 1
}

start_server_profile() {
  local round="$1" logs="$2" pg="$3" s3="$4" amqp="$5"
  local port_gateway="$6" port_base="$7"
  local common=(
    QR_PROFILE=server
    QR_DATABASE_URL="postgres://quantrule:quantrule@127.0.0.1:$pg/quantrule"
    QR_OBJECT_STORE_URL="http://127.0.0.1:$s3"
    QR_OBJECT_BUCKET=quantrule
    QR_QUEUE_URL="amqp://quantrule:quantrule@127.0.0.1:$amqp/%2f"
    QR_CONFIG_ROOT="$root/config"
    QR_IMAGE_DIGEST="$image_digest"
    AWS_ACCESS_KEY_ID=quantrule
    AWS_SECRET_ACCESS_KEY=quantrule
    AWS_REGION=us-east-1
  )
  start_server_child() {
    local name="$1" port="$2" migrate="$3"
    shift 3
    local bind=()
    [ "$port" != "-" ] && bind=(QR_BIND="127.0.0.1:$port")
    env "${common[@]}" QR_MIGRATE="$migrate" "${bind[@]}" "$@" \
      "$services_dir/$name" >"$logs/$name.log" 2>&1 &
    server_pids+=("$!")
  }
  start_server_child qr-data "$port_base" 1
  start_server_child qr-model $((port_base + 1)) 0 QR_DATA_URL="http://127.0.0.1:$port_base"
  start_server_child qr-evidence $((port_base + 2)) 1 QR_DATA_URL="http://127.0.0.1:$port_base"
  start_server_child qr-simulation $((port_base + 3)) 1
  start_server_child qr-backtest - 0 \
    QR_DATA_URL="http://127.0.0.1:$port_base" \
    QR_MODEL_URL="http://127.0.0.1:$((port_base + 1))" \
    QR_EVIDENCE_URL="http://127.0.0.1:$((port_base + 2))"
  start_server_child qr-gateway "$port_gateway" 1 \
    QR_DATA_URL="http://127.0.0.1:$port_base" \
    QR_MODEL_URL="http://127.0.0.1:$((port_base + 1))" \
    QR_EVIDENCE_URL="http://127.0.0.1:$((port_base + 2))" \
    QR_SIMULATION_URL="http://127.0.0.1:$((port_base + 3))"
  say "round $round: server profile started on $port_gateway"
}

# Run the fixture through one gateway and retain its evidence. The verify
# output goes to stderr and a log, so a caller that captures this function's
# stdout gets only the digest.
run_fixture() {
  local label="$1" round="$2" api="$3" evidence="$4" expected_value="$5"
  local log="$work/verify-$label-$round.log" extra=()
  if [ -n "$expected_value" ]; then
    extra=(--expected "$expected_value")
  fi
  if ! QUANTRULE_SHELL="$shell" "$root/tools/verify.sh" \
    --api-url "$api" \
    --evidence-dir "$evidence" \
    --timeout "$timeout" \
    "${extra[@]}" >"$log" 2>&1; then
    cat "$log" >&2
    return 1
  fi
  cat "$log" >&2
}

compare_round() {
  local round="$1" desktop="$2" server="$3"
  local desktop_digest server_digest files
  desktop_digest="$(cat "$desktop/digest")"
  server_digest="$(cat "$server/digest")"
  if [ "$desktop_digest" != "$server_digest" ]; then
    fail "round $round: canonical digests differ: desktop sha256:$desktop_digest, server sha256:$server_digest"
  fi
  if ! diff -ru "$desktop/artifacts" "$server/artifacts" >"$work/round-$round-artifacts.diff" 2>&1; then
    tail -n 40 "$work/round-$round-artifacts.diff" >&2 || true
    fail "round $round: the canonical artifacts differ"
  fi
  if ! diff -ru "$desktop/status" "$server/status" >"$work/round-$round-status.diff" 2>&1; then
    tail -n 40 "$work/round-$round-status.diff" >&2 || true
    fail "round $round: the run status or manifest entry set differs"
  fi
  files="$(find "$desktop/artifacts" -type f | wc -l | tr -d ' ')"
  say "round $round: canonical digest sha256:$desktop_digest; $files canonical artifacts byte-identical; run status matches"
  printf '%s' "$desktop_digest"
}

published_port() {
  local override="$1" service="$2" container="$3" mapping
  mapping="$(docker compose -p "$compose_project" -f "$root/compose.yaml" -f "$override" \
    port "$service" "$container")"
  printf '%s' "${mapping##*:}"
}

compose_up() {
  local override="$work/compose.override.yaml"
  cat >"$override" <<'YAML'
services:
  postgres:
    ports:
      - "127.0.0.1::5432"
  minio:
    ports:
      - "127.0.0.1::9000"
  rabbitmq:
    ports:
      - "127.0.0.1::5672"
YAML
  command -v docker >/dev/null || fail "--server-compose needs docker"
  say "starting Postgres, MinIO, and RabbitMQ under the $compose_project Compose project"
  docker compose -p "$compose_project" -f "$root/compose.yaml" -f "$override" \
    up -d --wait postgres minio rabbitmq >"$work/compose-up.log" 2>&1 \
    || { tail -n 40 "$work/compose-up.log" >&2 || true; fail "the Compose dependencies did not become healthy"; }
  compose_started=1
  local project container pg s3 amqp
  container="$(docker compose -p "$compose_project" -f "$root/compose.yaml" -f "$override" ps -q postgres)"
  project="$(docker inspect -f '{{index .Config.Labels "com.docker.compose.project"}}' "$container")"
  [ "$project" = "$compose_project" ] \
    || fail "the Compose project label is '$project', not '$compose_project'; refusing to touch another stack"
  pg="$(published_port "$override" postgres 5432)"
  s3="$(published_port "$override" minio 9000)"
  amqp="$(published_port "$override" rabbitmq 5672)"
  printf '%s\t%s\t%s\n' "$pg" "$s3" "$amqp" >"$work/server-ports.tsv"
  say "Compose dependencies on Postgres $pg, MinIO $s3, RabbitMQ $amqp"
}

desktop_round() {
  local round="$1"
  local prefix="$work/round-$round/desktop"
  local data_dir="$prefix/data" logs="$prefix/logs"
  mkdir -p "$data_dir" "$logs"
  local port_base="$internal_base"
  require_ports desktop "$gateway_port" "$port_base" "$((port_base + 1))" \
    "$((port_base + 2))" "$((port_base + 3))"
  if [ -n "$supervisor_bin" ]; then
    start_desktop_supervisor "$round" "$data_dir" "$logs" "$gateway_port" "$port_base"
  else
    start_desktop_raw "$round" "$data_dir" "$logs" "$gateway_port" "$port_base"
  fi
  local ports gateway actual
  ports="$(desktop_ports "$data_dir" "$gateway_port" "$port_base")" \
    || { stop_desktop_profile; fail "round $round: the desktop ports were not published"; }
  read -r gateway _ <<<"$ports"
  if ! wait_for_profile desktop "$ports"; then
    for log in "$logs"/*.log; do
      [ -f "$log" ] || continue
      printf 'parity: %s\n' "--- $log" >&2
      tail -n 20 "$log" >&2 || true
    done
    stop_desktop_profile
    fail "round $round: the desktop profile did not become healthy"
  fi
  say "round $round: desktop profile healthy on $gateway"
  run_fixture desktop "$round" "http://127.0.0.1:$gateway/api/v1" "$prefix/evidence" "$expected" \
    || { stop_desktop_profile; fail "round $round: the desktop fixture failed"; }
  stop_desktop_profile
  cat "$prefix/evidence/digest"
}

server_round() {
  local round="$1" pg="$2" s3="$3" amqp="$4"
  local prefix="$work/round-$round/server"
  local logs="$prefix/logs"
  mkdir -p "$logs"
  local port_gateway=$((gateway_port + 100))
  local port_base=$((internal_base + 100))
  require_ports server "$port_gateway" "$port_base" "$((port_base + 1))" \
    "$((port_base + 2))" "$((port_base + 3))"
  start_server_profile "$round" "$logs" "$pg" "$s3" "$amqp" "$port_gateway" "$port_base"
  local ports
  ports="$port_gateway $port_base $((port_base + 1)) $((port_base + 2)) $((port_base + 3))"
  if ! wait_for_profile server "$ports"; then
    for log in "$logs"/*.log; do
      [ -f "$log" ] || continue
      printf 'parity: %s\n' "--- $log" >&2
      tail -n 20 "$log" >&2 || true
    done
    stop_pids server_pids
    fail "round $round: the server profile did not become healthy"
  fi
  say "round $round: server profile healthy on $port_gateway"
  run_fixture server "$round" "http://127.0.0.1:$port_gateway/api/v1" "$prefix/evidence" "" \
    || { stop_pids server_pids; fail "round $round: the server fixture failed"; }
  stop_pids server_pids
  cat "$prefix/evidence/digest"
}

if [ "$server_compose" -eq 1 ]; then
  command -v docker >/dev/null || fail "--server-compose needs docker"
  compose_up
  IFS=$'\t' read -r server_pg server_s3 server_amqp <"$work/server-ports.tsv"
fi

declare -a round_digests=()
for round in $(seq 1 "$repeat"); do
  say "round $round of $repeat"
  desktop_digest=""
  server_digest=""
  server_first=0
  if [ $((round % 2)) -eq 0 ]; then
    server_first=1
  fi

  if [ "$server_first" -eq 1 ] && [ "$desktop_only" -eq 0 ]; then
    if [ "$server_compose" -eq 1 ]; then
      server_digest="$(server_round "$round" "$server_pg" "$server_s3" "$server_amqp")" || exit 1
    else
      run_fixture server "$round" "${server_url%/}" "$work/round-$round/server/evidence" "" \
        || fail "round $round: the server fixture failed"
      server_digest="$(cat "$work/round-$round/server/evidence/digest")"
    fi
  fi

  desktop_digest="$(desktop_round "$round")" || exit 1

  if [ "$desktop_only" -eq 0 ]; then
    if [ -z "$server_digest" ]; then
      if [ "$server_compose" -eq 1 ]; then
        server_digest="$(server_round "$round" "$server_pg" "$server_s3" "$server_amqp")" || exit 1
      else
        run_fixture server "$round" "${server_url%/}" "$work/round-$round/server/evidence" "" \
          || fail "round $round: the server fixture failed"
        server_digest="$(cat "$work/round-$round/server/evidence/digest")"
      fi
    fi
    digest="$(compare_round "$round" "$work/round-$round/desktop/evidence" "$work/round-$round/server/evidence")" \
      || exit 1
  else
    digest="$desktop_digest"
    say "round $round: desktop canonical digest sha256:$digest (no server comparison)"
  fi
  round_digests+=("$digest")
done

for digest in "${round_digests[@]:1}"; do
  [ "$digest" = "${round_digests[0]}" ] || fail "rounds disagree: ${round_digests[*]}"
done

if [ "$desktop_only" -eq 1 ] && [ -z "$expected" ]; then
  printf 'parity: desktop ok sha256:%s\n' "${round_digests[0]}"
else
  printf 'parity: ok sha256:%s\n' "${round_digests[0]}"
fi
