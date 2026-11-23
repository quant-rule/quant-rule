#!/usr/bin/env bash
# RabbitMQ broker-loss phase for the v3 `ha` lab tier.
#
# Sourced by `deploy/v3/test.sh` after the database phase. It stops the quorum
# leader broker with `docker stop`, polls a surviving node for two running nodes,
# a new quorum leader, `check_if_all_queues_have_quorum`, and no alarms, then
# proves that a publisher-confirmed quorum message is still readable through the
# HAProxy `rabbitmq` alias while the leader is down. It restores the broker with
# `docker start`, waits for three nodes and quorum, publishes and consumes a
# second probe, checks the app `/health`/`/ready`, compares the canonical digest
# when it is available, and writes value-free evidence. The end-to-end digest is
# `Unverified` until the MinIO phase and `tools/verify.sh` succeed; only the queue result is
# `Simulated HA`. A missing lab precondition is `UNVERIFIED`, never a pass.

q_stopped_host=""
q_probe_queue="qr.lab.queue-probe"
q_probe_bin=""
q_url_file=""
q_digest_status="Unverified"
q_digest_before=""
q_digest_after=""
q_publish_survived=0
q_restore_published=0
q_definitions_verified=0

q_say() {
  printf 'ha: queue: %s\n' "$*"
}

q_fail() {
  unverified "queue: $*"
}

q_container() {
  printf 'quantrule-rabbitmq-%s' "$1"
}

q_hosts() {
  local value
  value=$(jq -r "$1[]" "$descriptor") || q_fail "cannot read $1"
  [ -n "$value" ] || q_fail "the descriptor declares no $1"
  printf '%s\n' "$value"
}

q_first() {
  printf '%s\n' "$1" | head -n 1
}

# The leader broker host from a `rabbitmq-queues quorum_status` block.
q_leader_host() {
  local text=$1 leader
  leader=$(printf '%s\n' "$text" \
    | sed -n 's/.*Leader:[[:space:]]*\(rabbit@[^[:space:]]*\).*/\1/p' \
    | head -n 1)
  [ -n "$leader" ] || return 1
  case "$leader" in
    rabbit@rabbitmq-*) printf '%s' "${leader#rabbit@rabbitmq-}" ;;
    *) return 1 ;;
  esac
}

q_quorum_status() {
  local host=$1 queue=$2
  # shellcheck disable=SC2086
  remote "$host" "$docker_bin exec -u rabbitmq $(q_container "$host") rabbitmq-queues quorum_status $queue"
}

q_eval() {
  local host=$1 expression=$2
  # shellcheck disable=SC2086
  remote "$host" "$docker_bin exec -u rabbitmq $(q_container "$host") rabbitmqctl -q eval '$expression'" \
    | tr -d '[:space:]'
}

q_running_nodes() {
  q_eval "$1" 'length(rabbit_nodes:list_running()).'
}

q_alarm_count() {
  q_eval "$1" 'length(rabbit_alarm:get_alarms()).'
}

q_all_queues_have_quorum() {
  local host=$1
  # shellcheck disable=SC2086
  remote "$host" "$docker_bin exec -u rabbitmq $(q_container "$host") rabbitmq-diagnostics -q check_if_all_queues_have_quorum" \
    >/dev/null 2>&1
}

q_probe_binary() {
  if [ -n "${QR_QUEUE_PROBE_BIN:-}" ]; then
    q_probe_bin=$QR_QUEUE_PROBE_BIN
  else
    cargo build --quiet -p quantrule-runtime --example queue_probe >/dev/null 2>&1 \
      || q_fail "cannot build the queue probe example; set QR_QUEUE_PROBE_BIN"
    q_probe_bin="${CARGO_TARGET_DIR:-$root/target}/debug/examples/queue_probe"
  fi
  [ -x "$q_probe_bin" ] || q_fail "the queue probe $q_probe_bin is not executable"
}

q_probe_environment() {
  q_probe_binary
  q_url_file=${QR_QUEUE_URL_FILE:-${QR_STATEFUL_QUEUE_URL:-$(jq -r '.secrets.files.queue_url // empty' "$descriptor")}}
  [ -n "$q_url_file" ] || q_fail "the descriptor names no queue_url; set QR_QUEUE_URL_FILE"
  [ -r "$q_url_file" ] \
    || q_fail "the queue_url file $q_url_file is not readable; set QR_QUEUE_URL_FILE"
  [ -n "$cacert" ] && [ -r "$cacert" ] \
    || q_fail "the queue CA bundle is not readable; set QR_SWARM_CACERT"
}

q_probe() {
  local action=$1 payload=${2:-}
  "$q_probe_bin" "$action" "$q_probe_queue" "$payload" "$q_url_file" "$cacert"
}

q_probe_retry() {
  local action=$1 payload=${2:-} attempt
  for attempt in $(seq 1 15); do
    if q_probe "$action" "$payload" >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done
  return 1
}

# The app reconnect backoff can be up to 30 s; retry the gateway/ingress check
# before declaring the queue loss a failure.
q_targeted_retry() {
  local phase=$1 attempt
  for attempt in $(seq 1 30); do
    if ( targeted_check "$phase" ) >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done
  targeted_check "$phase"
}

queue_preflight() {
  [ "$profile" = "lab-single-host" ] \
    || q_fail "the broker-loss phase is lab-only; use a lab-single-host descriptor or --skip-queue"
  q_broker_hosts=$(q_hosts '.stateful.rabbitmq.nodes')
  q_haproxy_hosts=$(q_hosts '.stateful.haproxy.nodes')
  [ "$(printf '%s\n' "$q_broker_hosts" | wc -l)" -eq 3 ] \
    || q_fail "the broker-loss phase needs exactly three brokers"
  q_generation=${QR_QUEUE_GENERATION:-1}
  case "$q_generation" in
    '' | *[!0-9]*) q_fail "QR_QUEUE_GENERATION must be a positive integer" ;;
  esac
  [ "$q_generation" -gt 0 ] || q_fail "QR_QUEUE_GENERATION must be positive"
  q_queues="runs.backtest.g$q_generation runs.evidence.g$q_generation runs.dead-letter.g$q_generation stages.worker.g$q_generation"

  local host
  for host in $q_broker_hosts $q_haproxy_hosts; do
    # shellcheck disable=SC2086
    $ssh_bin "$(host_address "$host")" true >/dev/null 2>&1 \
      || q_fail "cannot SSH to $host ($(host_address "$host")); set QR_SWARM_SSH"
  done
  for host in $q_broker_hosts; do
    local running
    running=$(remote "$host" "$docker_bin inspect --format '{{.State.Running}}' $(q_container "$host")" 2>/dev/null || true)
    [ "$running" = "true" ] || q_fail "broker $(q_container "$host") is not running on $host"
    # shellcheck disable=SC2086
    remote "$host" "$docker_bin exec -u rabbitmq $(q_container "$host") rabbitmq-diagnostics -q ping" \
      >/dev/null 2>&1 || q_fail "RabbitMQ on $host does not answer rabbitmq-diagnostics -q ping"
  done
  q_first_broker=$(q_first "$q_broker_hosts")
  [ "$(q_running_nodes "$q_first_broker")" = "3" ] \
    || q_fail "the cluster does not report three running nodes"
  [ "$(q_alarm_count "$q_first_broker")" = "0" ] \
    || q_fail "the cluster reports an alarm"
  local status
  # shellcheck disable=SC2086
  status=$(remote "$q_first_broker" "$docker_bin exec -u rabbitmq $(q_container "$q_first_broker") rabbitmqctl cluster_status") \
    || q_fail "cannot read cluster_status"
  for host in $q_broker_hosts; do
    printf '%s\n' "$status" | grep -Fq "rabbit@rabbitmq-$host" \
      || q_fail "cluster_status omits rabbit@rabbitmq-$host"
  done
  q_definitions_verified=0
  DESCRIPTOR="$descriptor" \
    QR_QUEUE_DEFINITIONS_SSH="$ssh_bin" \
    QR_QUEUE_DEFINITIONS_DOCKER="$docker_bin" \
    QR_QUEUE_DEFINITIONS_GENERATION="$q_generation" \
    ./tools/queue-definitions.sh verify --live \
    || q_fail "the live definitions export does not match the four HA-005 quorum queues, bindings, or no-policy rule"
  q_definitions_verified=1

  local haproxy
  haproxy=$(q_first "$q_haproxy_hosts")
  q_haproxy_container="quantrule-haproxy-$haproxy"
  local haproxy_config
  haproxy_config=$(remote "$haproxy" "$docker_bin exec $q_haproxy_container cat /usr/local/etc/haproxy/haproxy.cfg" 2>/dev/null) \
    || q_fail "cannot read the HAProxy config on $haproxy"
  printf '%s\n' "$haproxy_config" | grep -Fq "frontend rabbitmq" \
    || q_fail "HAProxy on $haproxy has no rabbitmq frontend"
  printf '%s\n' "$haproxy_config" | grep -Fq "bind *:5671" \
    || q_fail "HAProxy on $haproxy does not bind 5671"
  remote "$haproxy" "$docker_bin exec $q_haproxy_container sh -c 'nc -z -w 5 rabbitmq 5671'" \
    >/dev/null 2>&1 || q_fail "the HAProxy rabbitmq:5671 front door does not answer"
  q_probe_environment
  q_leader=$(q_quorum_status "$q_first_broker" "runs.backtest.g$q_generation" \
    | sed -n 's/.*Leader:[[:space:]]*\(rabbit@[^[:space:]]*\).*/\1/p' | head -n 1)
  [ -n "$q_leader" ] || q_fail "no quorum leader for runs.backtest.g$q_generation"
  q_leader_host=${q_leader#rabbit@rabbitmq-}
  q_say "preflight: $q_leader leads runs.backtest.g$q_generation; $q_definitions_verified definitions verified; HAProxy $haproxy answers rabbitmq:5671"
}

q_verify_optional() {
  local phase=$1 output digest args
  if [ ! -x "$verify" ]; then
    q_digest_status="Unverified"
    q_say "$phase: verification script $verify is not executable; end-to-end digest Unverified"
    return 0
  fi
  args=(--api-url "$api_url" --timeout "$verify_timeout")
  [ -n "$cacert" ] && args+=(--cacert "$cacert")
  if output=$("$verify" "${args[@]}" 2>&1); then
    digest=$(printf '%s\n' "$output" | sed -n 's/^canonical digest sha256:\([0-9a-f]*\)$/\1/p')
    if [ -n "$digest" ]; then
      q_digest_status="Measured"
      q_digest_after=$digest
      q_say "$phase: canonical digest sha256:$digest"
    else
      q_digest_status="Unverified"
      q_say "$phase: the verification run printed no digest; end-to-end Unverified"
    fi
  else
    q_digest_status="Unverified"
    q_say "$phase: end-to-end digest Unverified (a stack precondition or no successful tools/verify.sh); queue evidence stays queue-level"
  fi
}

queue_baseline() {
  targeted_check "queue baseline"
  q_verify_optional "queue baseline"
  q_digest_before=$q_digest_after
  q_probe drain >/dev/null || q_fail "cannot drain stale probe messages"
  q_before_payload="quantrule-lab-queue-probe-before-g$q_generation"
  q_probe publish "$q_before_payload" >/dev/null \
    || q_fail "the broker did not confirm the baseline probe publish"
  q_say "baseline: leader $q_leader; confirmed publish $q_before_payload on $q_probe_queue"
}

queue_loss() {
  local old_leader=$q_leader old_host=$q_leader_host survivor deadline
  survivor=$(printf '%s\n' "$q_broker_hosts" | grep -Fxv "$old_host" | head -n 1)
  [ -n "$survivor" ] || q_fail "no surviving broker to observe"
  q_survivor_host=$survivor
  q_stopped_host=$old_host
  remote "$old_host" "$docker_bin stop $(q_container "$old_host")" >/dev/null \
    || q_fail "cannot stop the leader broker on $old_host"
  q_say "loss: stopped quorum leader $old_leader on $old_host"
  q_loss_start=$(date +%s)
  deadline=$(( q_loss_start + 120 ))
  q_new_leader=""
  q_rto=0
  while [ "$(date +%s)" -lt "$deadline" ]; do
    local running alarms leader new_leader
    running=$(q_running_nodes "$survivor" 2>/dev/null || true)
    alarms=$(q_alarm_count "$survivor" 2>/dev/null || true)
    leader=$(q_quorum_status "$survivor" "runs.backtest.g$q_generation" 2>/dev/null || true)
    new_leader=$(printf '%s\n' "$leader" \
      | sed -n 's/.*Leader:[[:space:]]*\(rabbit@[^[:space:]]*\).*/\1/p' | head -n 1)
    if [ "$running" = "2" ] && [ "$alarms" = "0" ] \
      && [ -n "$new_leader" ] && [ "$new_leader" != "$old_leader" ]; then
      if q_all_queues_have_quorum "$survivor"; then
        q_new_leader=$new_leader
        q_rto=$(( $(date +%s) - q_loss_start ))
        break
      fi
    fi
    sleep 1
  done
  [ -n "$q_new_leader" ] \
    || q_fail "the cluster did not reach two running nodes, a new quorum leader, all queues quorum, and no alarms within 120 s"
  q_say "loss: two running nodes, new quorum leader $q_new_leader, all queues have quorum, no alarms; measured gap ${q_rto}s"
  q_targeted_retry "after queue broker loss"
  if q_probe_retry consume "$q_before_payload"; then
    q_publish_survived=1
    q_say "loss: the acknowledged publish survived and was consumed through rabbitmq:5671"
  else
    q_fail "the acknowledged publish did not survive the broker loss"
  fi
}

queue_restore() {
  local host=$q_stopped_host deadline
  [ -n "$host" ] || q_fail "no stopped broker to restore"
  remote "$host" "$docker_bin start $(q_container "$host")" >/dev/null \
    || q_fail "cannot start the broker on $host"
  q_stopped_host=""
  q_say "restore: starting $host"
  deadline=$(( $(date +%s) + 180 ))
  q_restored=""
  local probe_host=${q_survivor_host:-$q_first_broker}
  while [ "$(date +%s)" -lt "$deadline" ]; do
    local running alarms status
    running=$(q_running_nodes "$probe_host" 2>/dev/null || true)
    alarms=$(q_alarm_count "$probe_host" 2>/dev/null || true)
    # shellcheck disable=SC2086
    status=$(remote "$probe_host" "$docker_bin exec -u rabbitmq $(q_container "$probe_host") rabbitmqctl cluster_status" 2>/dev/null || true)
    if [ "$running" = "3" ] && [ "$alarms" = "0" ] \
      && printf '%s\n' "$status" | grep -Fq "rabbit@rabbitmq-$host" \
      && q_all_queues_have_quorum "$probe_host" \
      && q_all_queues_have_quorum "$host"; then
      q_restored=1
      break
    fi
    sleep 1
  done
  [ -n "$q_restored" ] \
    || q_fail "the restored broker did not rejoin three running nodes with quorum and no alarms within 180 s"
  q_say "restore: three running nodes, quorum restored, $host rejoined"
  q_targeted_retry "after queue broker restore"
  negative_controls
  q_after_payload="quantrule-lab-queue-probe-after-g$q_generation"
  q_probe drain >/dev/null || q_fail "cannot drain the probe queue after restore"
  q_probe publish "$q_after_payload" >/dev/null \
    || q_fail "the restored cluster did not confirm a publish"
  q_probe_retry consume "$q_after_payload" \
    || q_fail "the restored cluster did not serve the second acknowledged publish"
  q_restore_published=1
  q_say "restore: the restored cluster accepted and consumed a second acknowledged publish"
  if [ -n "$q_digest_before" ]; then
    q_verify_optional "after queue restore"
    [ "$q_digest_status" = "Measured" ] \
      || q_fail "the canonical digest became unavailable after the queue restore"
    [ "$q_digest_after" = "$q_digest_before" ] \
      || q_fail "the canonical digest changed across the broker loss"
  else
    q_digest_status="Unverified"
    q_say "restore: end-to-end digest Unverified (baseline was not measured)"
  fi
  if [ -f "$out/stateful-manifest.json" ]; then
    qr_deploy check "$descriptor" --remote docker --overlay --out "$out" \
      || q_fail "the full remote/overlay check failed after the queue restore"
  else
    qr_deploy check "$descriptor" --remote docker --overlay \
      || q_fail "the full remote/overlay check failed after the queue restore"
  fi
}

queue_evidence() {
  local base path
  base=${QR_SWARM_EVIDENCE:-deploy/v3/evidence/$(date -u +%Y-%m-%dT%H%M%SZ).json}
  case "$base" in
    *.json) path="${base%.json}.queue.json" ;;
    *) path="$base.queue.json" ;;
  esac
  mkdir -p "$(dirname "$path")"
  jq -n \
    --arg evidence "Simulated HA" \
    --arg profile "$profile" \
    --arg recorded_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --arg scope "queue" \
    --argjson generation "$q_generation" \
    --arg queues "$q_queues" \
    --arg leader_before "$q_leader" \
    --arg leader_after "$q_new_leader" \
    --arg broker_stopped "$q_leader_host" \
    --argjson rto_seconds "$q_rto" \
    --argjson running_after_loss 2 \
    --argjson alarms 0 \
    --arg publish_before "$q_before_payload" \
    --arg publish_after "$q_after_payload" \
    --arg digest_status "$q_digest_status" \
    --arg digest_before "$([ -n "$q_digest_before" ] && printf 'sha256:%s' "$q_digest_before" || printf 'Unverified')" \
    --arg digest_after "$([ -n "$q_digest_after" ] && printf 'sha256:%s' "$q_digest_after" || printf 'Unverified')" \
    --arg definitions "live export verified; users/permissions stripped; no value or hash recorded" \
    --arg end_to_end "Unverified until the MinIO phase and tools/verify.sh succeed" \
    --arg real_ha "HA-014 only" \
    '{
      evidence: $evidence,
      scope: $scope,
      profile: $profile,
      recorded_at: $recorded_at,
      generation: $generation,
      queues: ($queues | split(" ")),
      broker_loss: {
        stopped: $broker_stopped,
        leader_before: $leader_before,
        leader_after: $leader_after,
        running_nodes_after_loss: $running_after_loss,
        all_queues_have_quorum: true,
        alarms: $alarms,
        rto_seconds: $rto_seconds
      },
      acknowledged_publish: {
        before: $publish_before,
        after: $publish_after,
        survived_one_broker_loss: true,
        restored_cluster_accepted: true
      },
      app: {health: "healthy", ready: "healthy", path: "https://<dns>/health and /ready"},
      definitions: $definitions,
      digests: {
        status: $digest_status,
        before: $digest_before,
        after: $digest_after
      },
      claims: {
        queue_level: "Simulated HA",
        end_to_end: $end_to_end,
        real_ha: $real_ha
      }
    }' > "$path"
  q_say "evidence: $path (Simulated HA at the queue level; end-to-end $q_digest_status)"
}

q_cleanup() {
  if [ -n "$q_stopped_host" ]; then
    remote "$q_stopped_host" "$docker_bin start $(q_container "$q_stopped_host")" >/dev/null 2>&1 || true
    q_stopped_host=""
  fi
}

queue_failover() {
  trap q_cleanup EXIT
  queue_preflight
  queue_baseline
  queue_loss
  queue_restore
  queue_evidence
  q_cleanup
  trap - EXIT
  q_say "result: Simulated HA (queue); end-to-end digest $q_digest_status; Real HA is HA-014"
}
