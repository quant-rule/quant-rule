#!/usr/bin/env bash
# Run a repeated soak over the live Compose stack.
#
# Each cycle cycles the model/backtest worker count, submits one fixed v4
# verification run, checks the canonical digest, and samples health, queue
# depth, PostgreSQL connections, disk, and memory. The default is the 72-hour
# release gate; a shorter duration is a sample, not the gate.
#
# Usage:
#   tools/soak.sh --plan
#   tools/soak.sh --duration 72h --cycles 1,2
#   tools/soak.sh --duration 30m --interval 5m --cycles 1,2
#
# Environment:
#   QUANTRULE_API_URL   Gateway base (default http://127.0.0.1:8080/api/v1)
#   QUANTRULE_TIMEOUT   Seconds per submitted run (default 300)
#   SOAK_EVIDENCE       Evidence directory (default deploy/v3/evidence/soak-<UTC>)
#   SOAK_MIN_MEM_MB     Required available memory (default 512)
#   SOAK_MIN_DISK_GB    Required free disk (default 5)
#
# The script starts nothing. Start the stack with `make up` first. It reads no
# secret into output and writes value-free JSON plus the verification log.
set -euo pipefail

root="$(cd "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

api_url=${QUANTRULE_API_URL:-http://127.0.0.1:8080/api/v1}
run_timeout=${QUANTRULE_TIMEOUT:-300}
duration=72h
interval=30m
cycles=1,2
expected="sha256:a0372f02db7bc65460906eba3242943da8c56f26c46fde470726cee856bc4cea"
evidence=${SOAK_EVIDENCE:-}
min_mem_mb=${SOAK_MIN_MEM_MB:-512}
min_disk_gb=${SOAK_MIN_DISK_GB:-5}
plan=0

usage() {
  awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
}

die() {
  printf 'soak: %s\n' "$*" >&2
  exit 1
}

parse_duration() {
  local value=$1
  case "$value" in
    *h) printf '%s' $(( ${value%h} * 3600 )) ;;
    *m) printf '%s' $(( ${value%m} * 60 )) ;;
    *s) printf '%s' "${value%s}" ;;
    ''|*[!0-9]*) die "duration must be 72h, 30m, 45s, or whole seconds: $value" ;;
    *) printf '%s' "$value" ;;
  esac
}

parse_args() {
  while [ $# -gt 0 ]; do
    case "$1" in
      --plan | --dry-run) plan=1; shift ;;
      --duration)
        [ $# -ge 2 ] || die "--duration needs a value"
        duration=$2
        shift 2
        ;;
      --interval)
        [ $# -ge 2 ] || die "--interval needs a value"
        interval=$2
        shift 2
        ;;
      --cycles)
        [ $# -ge 2 ] || die "--cycles needs a value"
        cycles=$2
        shift 2
        ;;
      --expected)
        [ $# -ge 2 ] || die "--expected needs a value"
        expected=${2#canonical digest }
        shift 2
        ;;
      --evidence)
        [ $# -ge 2 ] || die "--evidence needs a value"
        evidence=$2
        shift 2
        ;;
      -h | --help) usage; exit 0 ;;
      *) die "unknown argument $1" ;;
    esac
  done
}

print_plan() {
  cat <<'PLAN'
soak: plan (no Docker, network, secret, or host is read)
soak: preflight: MemAvailable >= SOAK_MIN_MEM_MB (default 512 MiB), free disk >= SOAK_MIN_DISK_GB (default 5 GiB), docker info, gateway healthy
soak: cycles: alternate the configured worker counts with `make scale N=<n>`
soak: each cycle: one fixed v4 run via tools/verify.sh --expected <digest>, then health, queue depth, PostgreSQL connections, disk, and memory samples
soak: fail: unhealthy gateway, queue depth above zero after a run, digest mismatch, DB connections above baseline + 20, available memory below 256 MiB, or disk below 1 GiB
soak: evidence: value-free JSON and per-cycle verification logs under the evidence directory
soak: 72h is the release gate; a shorter --duration is a sample, not the gate
PLAN
}

parse_args "$@"
if [ "$plan" -eq 1 ]; then
  print_plan
  exit 0
fi

duration_s=$(parse_duration "$duration")
interval_s=$(parse_duration "$interval")
[ "$duration_s" -gt 0 ] || die "duration must be greater than zero"
[ "$interval_s" -gt 0 ] || die "interval must be greater than zero"
case "$min_mem_mb" in ''|*[!0-9]*) die "SOAK_MIN_MEM_MB must be a whole number" ;; esac
case "$min_disk_gb" in ''|*[!0-9]*) die "SOAK_MIN_DISK_GB must be a whole number" ;; esac
case "$run_timeout" in ''|*[!0-9]*) die "QUANTRULE_TIMEOUT must be a whole number" ;; esac
[ -n "$cycles" ] || die "--cycles must name at least one worker count"
IFS=',' read -r -a worker_list <<<"$cycles"
for worker in "${worker_list[@]}"; do
  case "$worker" in ''|*[!0-9]*) die "worker counts must be whole numbers: $cycles" ;; esac
  [ "$worker" -ge 1 ] || die "worker counts must be at least 1: $cycles"
done

for command in curl jq docker make; do
  command -v "$command" >/dev/null 2>&1 || die "$command is required"
done
[ -f "$root/secrets/compose.env" ] || die "secrets/compose.env is missing; run make secrets"
[ -d "$root/pki" ] || die "pki/ is missing; run make pki"
[ -x "$root/tools/verify.sh" ] || die "tools/verify.sh is missing"

mem_avail_kb=$(awk '/^MemAvailable:/ { print $2 }' /proc/meminfo)
disk_avail_kb=$(df -Pk "$root" | awk 'NR == 2 { print $4 }')
[ -n "$mem_avail_kb" ] && [ -n "$disk_avail_kb" ] || die "cannot read memory or disk"
[ "$mem_avail_kb" -ge $((min_mem_mb * 1024)) ] || {
  printf 'soak: UNVERIFIED: MemAvailable %s MiB < %s MiB\n' \
    $((mem_avail_kb / 1024)) "$min_mem_mb" >&2
  exit 1
}
[ "$disk_avail_kb" -ge $((min_disk_gb * 1024 * 1024)) ] || {
  printf 'soak: UNVERIFIED: free disk %s GiB < %s GiB\n' \
    $((disk_avail_kb / 1024 / 1024)) "$min_disk_gb" >&2
  exit 1
}
docker info >/dev/null 2>&1 || die "docker info failed"
health=$(curl -fsS --max-time 5 "$api_url/health" 2>/dev/null) || die "gateway is not reachable at $api_url"
state=$(printf '%s' "$health" | jq -r '.state // empty')
[ "$state" = healthy ] || die "gateway state is '${state:-missing}', not healthy"

if [ -z "$evidence" ]; then
  evidence="$root/deploy/v3/evidence/soak-$(date -u +%Y%m%dT%H%M%SZ)"
fi
mkdir -p "$evidence"
ndjson="$evidence/soak.ndjson"
: >"$ndjson"

started=$(date -u +%Y-%m-%dT%H:%M:%SZ)
started_epoch=$(date -u +%s)
total_cycles=$(( (duration_s + interval_s - 1) / interval_s ))
[ "$total_cycles" -ge 1 ] || total_cycles=1
export QR_SECRETS_DIR="$root/secrets" QR_PKI_DIR="$root/pki"
compose=(docker compose --env-file "$root/secrets/compose.env")

db_connections() {
  "${compose[@]}" exec -T postgres sh -c \
    'psql -U "$POSTGRES_USER" -d postgres -tAc "select count(*) from pg_stat_activity"' \
    2>/dev/null | tr -d '[:space:]' || printf 'unknown'
}

sample() {
  local health_json queue db disk mem
  health_json=$(curl -fsS --max-time 5 "$api_url/health" 2>/dev/null || true)
  queue=$(printf '%s' "$health_json" | jq -r '.queue_depth // 0' 2>/dev/null || printf '0')
  db=$(db_connections)
  disk=$(df -Pk "$root" | awk 'NR == 2 { print $4 }')
  mem=$(awk '/^MemAvailable:/ { print $2 }' /proc/meminfo)
  printf '%s\t%s\t%s\t%s\n' "$queue" "$db" "$disk" "$mem"
}

printf 'soak: start %s duration=%ss interval=%ss cycles=%s evidence=%s\n' \
  "$started" "$duration_s" "$interval_s" "$cycles" "$evidence"
failures=0
baseline_db=""
last_mem=""
for index in $(seq 1 "$total_cycles"); do
  worker=${worker_list[$(( (index - 1) % ${#worker_list[@]} ))]}
  cycle_start=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  log="$evidence/cycle-$(printf '%02d' "$index").log"
  if ! make scale N="$worker" >"$log" 2>&1; then
    printf 'soak: cycle %s scale N=%s failed; see %s\n' "$index" "$worker" "$log" >&2
    failures=$((failures + 1))
    break
  fi
  before=$(sample)
  set +e
  ./tools/verify.sh --api-url "$api_url" --timeout "$run_timeout" --expected "$expected" \
    >>"$log" 2>&1
  status=$?
  set -e
  after=$(sample)
  cycle_end=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  cycle_health=$(curl -fsS --max-time 5 "$api_url/health" 2>/dev/null \
    | jq -r '.state // empty' 2>/dev/null || true)
  queue=${after%%$'\t'*}
  rest=${after#*$'\t'}
  db=${rest%%$'\t'*}
  rest=${rest#*$'\t'}
  disk=${rest%%$'\t'*}
  mem=${rest#*$'\t'}
  [ -n "$baseline_db" ] || baseline_db=$db
  if [ "$status" -ne 0 ]; then
    failures=$((failures + 1))
    printf 'soak: cycle %s verify failed with exit %s; see %s\n' "$index" "$status" "$log" >&2
  fi
  if [ "$cycle_health" != healthy ]; then
    failures=$((failures + 1))
    printf 'soak: cycle %s gateway is not healthy: %s\n' "$index" "${cycle_health:-missing}" >&2
  fi
  if [ "$queue" != 0 ]; then
    failures=$((failures + 1))
    printf 'soak: cycle %s queue depth is %s, expected 0\n' "$index" "$queue" >&2
  fi
  if [ "$db" != unknown ] && [ "$baseline_db" != unknown ] && [ "$db" -gt $((baseline_db + 20)) ]; then
    failures=$((failures + 1))
    printf 'soak: cycle %s PostgreSQL connections %s exceed baseline %s + 20\n' "$index" "$db" "$baseline_db" >&2
  fi
  if [ "$mem" -lt 262144 ]; then
    failures=$((failures + 1))
    printf 'soak: cycle %s available memory %s MiB < 256 MiB\n' "$index" "$((mem / 1024))" >&2
  fi
  if [ "$disk" -lt 1048576 ]; then
    failures=$((failures + 1))
    printf 'soak: cycle %s free disk %s MiB < 1 GiB\n' "$index" "$((disk / 1024))" >&2
  fi
  last_mem=$mem
  jq -n \
    --arg v "quantrule.soak-cycle/v1" \
    --arg cycle "$index" --arg workers "$worker" \
    --arg start "$cycle_start" --arg end "$cycle_end" \
    --arg log "$log" --argjson status "$status" \
    --argjson queue "$queue" --argjson db "$db" \
    --argjson disk_kb "$disk" --argjson mem_kb "$mem" \
    --arg expected "$expected" \
    '{v:$v, cycle:($cycle|tonumber), workers:($workers|tonumber), start:$start, end:$end,
      verify_status:$status, expected:$expected, queue_depth:$queue,
      db_connections:$db, disk_avail_kb:$disk_kb, mem_avail_kb:$mem_kb, log:$log}' \
    >>"$ndjson"
  if [ "$failures" -ne 0 ]; then
    break
  fi
  if [ "$index" -lt "$total_cycles" ]; then
    sleep "$interval_s"
  fi
done

finished=$(date -u +%Y-%m-%dT%H:%M:%SZ)
result=pass
[ "$failures" -eq 0 ] || result=fail
jq -s --arg v "quantrule.soak/v1" --arg started "$started" --arg finished "$finished" \
  --arg duration "$duration_s" --arg interval "$interval_s" --arg cycles "$cycles" \
  --arg result "$result" --argjson failures "$failures" --argjson total "$total_cycles" \
  '{v:$v, started:$started, finished:$finished, duration_s:($duration|tonumber),
    interval_s:($interval|tonumber), cycles:$cycles, planned_cycles:$total,
    failures:$failures, result:$result, cycles_data:.}' \
  "$ndjson" >"$evidence/summary.json"
printf 'soak: %s cycles=%s failures=%s evidence=%s\n' "$result" "$total_cycles" "$failures" "$evidence"
[ "$result" = pass ]
