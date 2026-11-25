#!/usr/bin/env bash
# Shared state and helpers for the v3 chaos cases.
#
# Sourced by `deploy/v3/chaos.sh`. It defines functions and state only; it never
# contacts Docker, SSH, jq, or a secret at source time.

chaos=${chaos:-0}
skip_chaos=${skip_chaos:-1}
chaos_status="Unverified"
chaos_profile=${QR_CHAOS_PROFILE:-default}
chaos_digest=""
chaos_work=""
chaos_stopped=()
chaos_shell_bin=""
chaos_data_version=""
chaos_config_id=""
chaos_config_seq=0

chaos_say() {
  printf 'ha: chaos: %s\n' "$*"
}

chaos_fail() {
  unverified "chaos: $*"
}

chaos_evidence_base() {
  local base=${QR_HA_EVIDENCE:-${QR_SWARM_EVIDENCE:-}}
  if [ -z "$base" ]; then
    base="deploy/v3/evidence/ha-$(date -u +%Y-%m-%dT%H%M%SZ)"
  fi
  case "$base" in
    *.json) base=${base%.json} ;;
  esac
  printf '%s' "$base"
}

chaos_write() {
  local row=$1 status=$2 case_name=$3 rto=$4 digest=$5 note=$6 extra=${7:-'{}'}
  local directory
  directory=$(chaos_evidence_base)
  mkdir -p "$directory"
  jq -n \
    --arg schema "quantrule.ha-evidence/v1" \
    --arg row "$row" \
    --arg status "$status" \
    --arg case_name "$case_name" \
    --arg profile "$profile" \
    --arg recorded_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --arg rto_seconds "$rto" \
    --arg digest "$digest" \
    --arg note "$note" \
    --argjson extra "$extra" \
    '{
      schema: $schema,
      row: $row,
      status: $status,
      case: $case_name,
      profile: $profile,
      recorded_at: $recorded_at,
      rto_seconds: ($rto_seconds | tonumber),
      digest: $digest,
      note: $note,
      real_ha: "not claimed by this script",
      details: $extra
    }' > "$directory/$row.json"
  chaos_say "evidence $row ($status) -> $directory/$row.json"
}

chaos_cleanup() {
  local entry host container
  for entry in ${chaos_stopped[@]+"${chaos_stopped[@]}"}; do
    host=${entry%%:*}
    container=${entry#*:}
    # shellcheck disable=SC2086
    remote "$host" "$docker_bin start $container" >/dev/null 2>&1 || true
  done
  chaos_stopped=()
  if [ -n "${chaos_work:-}" ]; then
    rm -rf "$chaos_work"
    chaos_work=""
  fi
}

chaos_require_lab() {
  [ "$profile" = "lab-single-host" ] \
    || chaos_fail "the chaos cases are lab-only; use a lab-single-host descriptor or --skip-chaos"
  [ -f config/examples/verify-mini/run.json ] \
    || chaos_fail "config/examples/verify-mini/run.json is missing"
  command -v jq >/dev/null 2>&1 || chaos_fail "jq is required"
  command -v curl >/dev/null 2>&1 || chaos_fail "curl is required"
}

chaos_shell() {
  if [ -n "${QUANTRULE_SHELL:-}" ]; then
    chaos_shell_bin=$QUANTRULE_SHELL
  else
    cargo build --quiet -p quantrule-shell >/dev/null 2>&1 \
      || chaos_fail "cannot build the shell client; set QUANTRULE_SHELL"
    chaos_shell_bin="${CARGO_TARGET_DIR:-$root/target}/debug/quantrule"
  fi
  [ -x "$chaos_shell_bin" ] || chaos_fail "the shell client is not executable"
}

chaos_curl() {
  if [ -n "${cacert:-}" ]; then
    curl --cacert "$cacert" "$@"
  else
    curl "$@"
  fi
}

chaos_dataset() {
  local output
  output=$("$chaos_shell_bin" --profile "$chaos_profile" dataset upload \
    config/examples/verify-mini/candles.csv --name candles.csv) \
    || chaos_fail "the dataset upload failed"
  chaos_data_version=$(printf '%s\n' "$output" | sed -n 's/^data_version: //p')
  [ -n "$chaos_data_version" ] || chaos_fail "the dataset upload returned no data version"
}

chaos_config() {
  local directory=config/examples/verify-mini body response status names name source
  if [ -n "${chaos_work:-}" ]; then
    rm -rf "$chaos_work"
  fi
  chaos_work=$(mktemp -d "${TMPDIR:-/tmp}/quantrule-chaos.XXXXXX")
  body="$chaos_work/bundle.json"
  # A run identity is derived from the data version and the config hash, so the
  # fixed verify-mini config is idempotent: a second submit returns the run
  # that already succeeded. Name every chaos run uniquely so the kill case gets
  # a fresh, in-flight run. The name is recorded in provenance and the manifest,
  # both outside the canonical digest; the computed artifacts are unchanged.
  chaos_config_seq=$((chaos_config_seq + 1))
  chaos_run_name="chaos-kill-$(date -u +%Y%m%dT%H%M%SZ)-$chaos_config_seq"
  jq --arg name "$chaos_run_name" '.name = $name' "$directory/run.json" \
    > "$chaos_work/run.json" \
    || chaos_fail "cannot name the chaos run"
  names=$(jq -er '
    ["run.json"] + ([.data, .model, .execution, .portfolio, .strategy]
      | map(select(. != null))) | unique | .[]
  ' "$directory/run.json") || chaos_fail "cannot read the fixed config"
  jq -n '{name: "chaos-kill", files: {}}' > "$body"
  while IFS= read -r name; do
    [ -n "$name" ] || continue
    if [ "$name" = "run.json" ]; then
      source="$chaos_work/run.json"
    else
      source="$directory/$name"
    fi
    jq --arg name "$name" --rawfile content "$source" \
      '.files[$name] = $content' "$body" > "$body.next"
    mv "$body.next" "$body"
  done <<< "$names"
  response="$chaos_work/config.json"
  status=$(chaos_curl -sS -o "$response" -w '%{http_code}' \
    -X POST -H "X-QuantRule-Profile: $chaos_profile" \
    -H 'content-type: application/json' \
    --data-binary @"$body" "$api_url/catalog/configs") \
    || chaos_fail "the gateway refused the config upload"
  case "$status" in 200 | 201) ;; *) chaos_fail "the config upload returned HTTP $status" ;; esac
  chaos_config_id=$(sed -n 's/.*"config_id":"\([^"]*\)".*/\1/p' "$response")
  [ -n "$chaos_config_id" ] || chaos_fail "the config upload returned no config id"
}

chaos_submit() {
  local output run_id
  output=$("$chaos_shell_bin" --profile "$chaos_profile" run submit \
    "$chaos_data_version" "$chaos_config_id") \
    || chaos_fail "the run submission failed"
  run_id=$(printf '%s\n' "$output" | sed -n 's/^run_id: //p')
  [ -n "$run_id" ] || chaos_fail "the run submission returned no run id"
  printf '%s' "$run_id"
}

chaos_detail() {
  "$chaos_shell_bin" --profile "$chaos_profile" run status "$1"
}

chaos_field() {
  chaos_detail "$1" | sed -n "s/^$2: //p" | head -n 1
}

chaos_wait_run() {
  local run_id=$1 deadline=$((SECONDS + ${QR_CHAOS_TIMEOUT:-300})) state
  while :; do
    state=$(chaos_field "$run_id" status)
    case "$state" in
      succeeded) return 0 ;;
      failed | cancelled) chaos_fail "run $run_id ended $state" ;;
    esac
    [ "$SECONDS" -lt "$deadline" ] || chaos_fail "run $run_id did not finish in time"
    sleep 2
  done
}

# The run detail must not show a succeeded run without a digest, and every
# artifact it references must be fetchable. This is the partial-publish check.
chaos_assert_no_partial() {
  local run_id=$1 state digest name code
  state=$(chaos_field "$run_id" status)
  digest=$(chaos_field "$run_id" digest)
  if [ "$state" = "succeeded" ] && [ "$digest" = "none" ]; then
    chaos_fail "run $run_id is succeeded with no digest"
  fi
  while IFS= read -r name; do
    [ -n "$name" ] || continue
    code=$(chaos_curl -sS -o /dev/null -w '%{http_code}' \
      -H "X-QuantRule-Profile: $chaos_profile" \
      "$api_url/runs/${run_id//:/%3A}/artifacts/$name") \
      || chaos_fail "cannot reach the gateway for artifact $name"
    [ "$code" = "200" ] || chaos_fail "referenced artifact $name returned HTTP $code"
  done <<< "$(chaos_detail "$run_id" | sed -n 's/^artifact \([^:]*\):.*/\1/p')"
}

chaos_assert_one_digest() {
  local run_id=$1 digest duplicates
  chaos_wait_run "$run_id"
  digest=$(chaos_field "$run_id" digest)
  [ -n "$digest" ] && [ "$digest" != "none" ] \
    || chaos_fail "run $run_id succeeded without a digest"
  duplicates=$(chaos_detail "$run_id" | sed -n 's/^artifact \([^:]*\):.*/\1/p' | sort | uniq -d)
  [ -z "$duplicates" ] || chaos_fail "run $run_id lists a duplicate artifact"
  [ "$digest" = "sha256:$chaos_digest" ] \
    || chaos_fail "run $run_id digest $digest differs from baseline sha256:$chaos_digest"
  printf '%s' "$digest"
}

chaos_verify_expected() {
  local phase=$1 args
  args=(--api-url "$api_url" --timeout "$verify_timeout" --expected "sha256:$chaos_digest")
  [ -n "${cacert:-}" ] && args+=(--cacert "$cacert")
  "$verify" "${args[@]}" >/dev/null \
    || chaos_fail "$phase: tools/verify.sh --expected did not reproduce the baseline"
}

chaos_task_on_host() {
  local service=$1 host=$2
  # shellcheck disable=SC2086
  remote "$host" "$docker_bin ps --filter label=com.docker.swarm.service.name=${stack}_$service --format '{{.ID}}'" \
    | head -n 1
}

chaos_kill_on_host() {
  local host=$1 container=$2
  [ -n "$container" ] || chaos_fail "no task container to kill on $host"
  # shellcheck disable=SC2086
  remote "$host" "$docker_bin kill --signal=SIGKILL $container" >/dev/null \
    || chaos_fail "cannot SIGKILL $container on $host"
}

chaos_leader_term() {
  remote "$chaos_pg_host" \
    "$docker_bin exec -u postgres quantrule-patroni-$chaos_pg_host psql -d qr_evidence -tAc \"SELECT term FROM runs.leader_lease WHERE singleton;\"" \
    | tr -d '[:space:]'
}
