#!/usr/bin/env bash
# Observability lab phase for the v3 `ha` tier, sourced by `deploy/v3/test.sh`.
# It starts the tier, asserts the probes/jobs, proves GatewaySyntheticDown with a
# promtool rule unit, and asserts SwarmManagerQuorumAtRisk and
# GatewayReplicaDown fire and resolve. `Simulated HA` only on a full lab run;
# `Real HA` is HA-014. No stack, volume, secret, config, node, or service is
# removed. The mutations are the caller's node availability change, the gateway
# pause/unpause, and the node_exporter textfile write.

obs_sudo=${QR_OBSERVABILITY_SUDO:-sudo -n}
obs_alert_timeout=${QR_LAB_OBS_ALERT_TIMEOUT:-780}
obs_resolve_timeout=${QR_LAB_OBS_RESOLVE_TIMEOUT:-240}
obs_settle_attempts=${QR_LAB_OBS_SETTLE_ATTEMPTS:-12}
obs_settle_interval=${QR_LAB_OBS_SETTLE_INTERVAL:-5}
obs_prometheus_host=""
obs_alertmanager_host=""
obs_manager=""
obs_gateway_host=""
obs_gateway_container=""
obs_manager_down=0
obs_manager_resolved=0
obs_gateway_down=0
obs_gateway_resolved=0
obs_rule_unit_ok=0

# Run the operator tool with the lab's descriptor, render tree, SSH, and docker.
obs_tool() {
  DESCRIPTOR="$descriptor" OUT="$out" \
    QR_OBSERVABILITY_SSH="$ssh_bin" \
    QR_OBSERVABILITY_DOCKER="$docker_bin" \
    QR_OBSERVABILITY_MANAGER="$obs_manager" \
    ./tools/observability.sh "$@"
}
# Fetch one Prometheus or Alertmanager API path through the running container.
obs_prom_get() {
  local path=$1
  # shellcheck disable=SC2086
  $ssh_bin "$(host_address "$obs_prometheus_host")" \
    "$obs_sudo $docker_bin exec quantrule-prometheus wget -q -O - http://127.0.0.1:9090$path"
}
obs_alertmanager_get() {
  local path=$1
  # shellcheck disable=SC2086
  $ssh_bin "$(host_address "$obs_alertmanager_host")" \
    "$obs_sudo $docker_bin exec quantrule-alertmanager wget -q -O - http://127.0.0.1:9093$path"
}
# One instant query. The expression is URL-encoded on the harness host so the
# remote shell sees one word.
obs_query() {
  local expression=$1 encoded
  encoded=$(printf '%s' "$expression" | jq -sRr @uri)
  obs_prom_get "/api/v1/query?query=$encoded"
}
obs_value() {
  obs_query "$1" | jq -r '.data.result[0].value[1] // empty'
}
obs_alert_firing() {
  obs_prom_get "/api/v1/alerts" | jq -e --arg name "$1" \
    '.data.alerts[] | select(.labels.alertname == $name and .state == "firing")' \
    >/dev/null 2>&1
}
obs_alertmanager_has() {
  obs_alertmanager_get "/api/v2/alerts" | jq -e --arg name "$1" \
    '.[] | select(.labels.alertname == $name and .status.state == "active")' \
    >/dev/null 2>&1
}
obs_wait_alert() {
  local name=$1 timeout=$2 deadline=$((SECONDS + $2))
  say "observability: waiting for $name to fire (up to ${timeout}s)"
  while [ "$SECONDS" -lt "$deadline" ]; do
    if obs_alert_firing "$name"; then
      say "observability: $name is firing"
      return 0
    fi
    sleep 5
  done
  return 1
}
obs_wait_resolved() {
  local name=$1 timeout=$2 deadline=$((SECONDS + $2))
  say "observability: waiting for $name to resolve (up to ${timeout}s)"
  while [ "$SECONDS" -lt "$deadline" ]; do
    if ! obs_alert_firing "$name"; then
      say "observability: $name resolved"
      return 0
    fi
    sleep 5
  done
  return 1
}
obs_wait_alertmanager() {
  local name=$1 timeout=$2 deadline=$((SECONDS + $2))
  while [ "$SECONDS" -lt "$deadline" ]; do
    if obs_alertmanager_has "$name"; then
      say "observability: $name is visible in Alertmanager"
      return 0
    fi
    sleep 5
  done
  return 1
}

# A manager SSH target other than the one the caller is about to stop.
obs_live_manager() {
  local stopped=$1 host
  for host in $(jq -r '.swarm.managers[]' "$descriptor"); do
    [ "$host" = "$stopped" ] && continue
    # shellcheck disable=SC2086
    if $ssh_bin "$(host_address "$host")" true >/dev/null 2>&1; then
      host_address "$host"
      return 0
    fi
  done
  return 1
}

obs_baseline() {
  local ready health down jobs job firing unexpected attempt
  ready=$(obs_value 'probe_success{job="blackbox",check="ready"}')
  health=$(obs_value 'probe_success{job="blackbox",check="health"}')
  [ "$ready" = "1" ] || unverified "observability: the ready probe is ${ready:-missing}, expected 1"
  [ "$health" = "1" ] || unverified "observability: the health probe is ${health:-missing}, expected 1"
  down=$(obs_query 'up{job=~"app|ingress|patroni|etcd|haproxy|rabbitmq|minio|node|promtail|blackbox|prometheus|alertmanager|loki|grafana"} == 0' \
    | jq -r '.data.result | length')
  [ "$down" = "0" ] || unverified "observability: $down required scrape target(s) are down"
  jobs=$(obs_query 'count by (job) (up)' | jq -r '.data.result[].metric.job' | sort)
  for job in app ingress patroni etcd haproxy rabbitmq minio node promtail blackbox prometheus alertmanager loki grafana; do
    printf '%s\n' "$jobs" | grep -qx "$job" \
      || unverified "observability: the $job job has no target"
  done
  for attempt in $(seq 1 "$obs_settle_attempts"); do
    firing=$(obs_prom_get "/api/v1/alerts" \
      | jq -r '.data.alerts[] | select(.state == "firing") | .labels.alertname' \
      | sort -u)
    unexpected=$(printf '%s\n' "$firing" \
      | grep -vE '^(BackupAgeHigh|VerifyAgeHigh|GatewayTokenOld)$' || true)
    if [ -z "$unexpected" ]; then
      say "observability: baseline: health/ready probes 1, every job has a target, no unexpected alert"
      return 0
    fi
    sleep "$obs_settle_interval"
  done
  unverified "observability: unexpected firing alerts: $unexpected"
}

# A deterministic rule-unit proof of GatewaySyntheticDown. The fixture is run in
# the pinned Prometheus image with the rendered rules mounted read-only; this is
# not a live ingress failure and it is not `Simulated HA`.
obs_rule_unit() {
  local image remote_dir output host
  image=$(jq -r '.images.prometheus.reference + "@" + .images.prometheus.digest' "$descriptor") \
    || unverified "observability: cannot read images.prometheus"
  host=$(host_address "$obs_prometheus_host")
  remote_dir="/tmp/quantrule-promtool-$$"
  # shellcheck disable=SC2086
  $ssh_bin "$host" "mkdir -p $remote_dir" \
    || unverified "observability: cannot create $remote_dir on $obs_prometheus_host"
  # shellcheck disable=SC2086
  $ssh_bin "$host" "cat > $remote_dir/gateway-synthetic-down.yml" \
    < deploy/v3/observability/promtool/gateway-synthetic-down.yml \
    || unverified "observability: cannot copy the rule-unit fixture"
  # shellcheck disable=SC2086
  output=$($ssh_bin "$host" "$obs_sudo $docker_bin run --rm --network none \
    --entrypoint promtool \
    -v /etc/quantrule/prometheus:/etc/prometheus:ro \
    -v $remote_dir:/tests:ro \
    $image test rules /tests/gateway-synthetic-down.yml" 2>&1) \
    || unverified "observability: promtool refused the rule-unit proof"
  printf '%s\n' "$output" | grep -q 'SUCCESS' \
    || unverified "observability: the rule-unit proof did not report SUCCESS"
  say "observability: rule-unit proof: GatewaySyntheticDown fires for check=ready (not Simulated HA)"
  obs_rule_unit_ok=1
}

obs_start() {
  obs_prometheus_host=$(jq -r '.observability.prometheus.host' "$descriptor") \
    || unverified "observability: cannot read observability.prometheus.host"
  obs_alertmanager_host=$(jq -r '.observability.alertmanager.host' "$descriptor") \
    || unverified "observability: cannot read observability.alertmanager.host"
  obs_manager=$(jq -r '.swarm.managers[0]' "$descriptor") \
    || unverified "observability: cannot read swarm.managers"
  obs_manager=$(host_address "$obs_manager")
  say "observability: copy, start, wait, verify"
  obs_tool copy || unverified "observability: copy failed"
  obs_tool start || unverified "observability: start failed"
  obs_tool wait || unverified "observability: wait failed"
  obs_tool verify || unverified "observability: verify failed"
  obs_baseline
  obs_rule_unit
}

# The caller has stopped one manager. Write the quorum textfile from a survivor
# and hold until the AtRisk rule fires and reaches Alertmanager.
obs_manager_loss() {
  local stopped=$1 ready attempt
  obs_manager=$(obs_live_manager "$stopped") \
    || unverified "observability: no live manager to write the quorum textfile"
  say "observability: manager $stopped is down; write the quorum textfile from $obs_manager"
  obs_tool manager-quorum || unverified "observability: manager-quorum failed"
  ready=""
  for attempt in $(seq 1 12); do
    ready=$(obs_value 'quantrule_swarm_managers_ready')
    if [ -n "$ready" ] && [ "$ready" -lt 3 ]; then
      break
    fi
    sleep 5
  done
  [ -n "$ready" ] && [ "$ready" -lt 3 ] \
    || unverified "observability: the stopped manager still counts as ready (${ready:-missing})"
  obs_wait_alert SwarmManagerQuorumAtRisk "$obs_alert_timeout" \
    || unverified "observability: SwarmManagerQuorumAtRisk did not fire within ${obs_alert_timeout}s"
  obs_wait_alertmanager SwarmManagerQuorumAtRisk 120 \
    || unverified "observability: SwarmManagerQuorumAtRisk did not reach Alertmanager"
  obs_manager_down=1
}

obs_manager_restore() {
  obs_tool manager-quorum || unverified "observability: manager-quorum restore failed"
  obs_wait_resolved SwarmManagerQuorumAtRisk "$obs_resolve_timeout" \
    || unverified "observability: SwarmManagerQuorumAtRisk did not resolve"
  obs_manager_resolved=1
}

# The caller has drained the VIP holder. The gateway may reschedule to the
# surviving eligible host, so this only proves the VIP stays up; the
# deterministic replica loss below is a pause on the non-VIP gateway.
obs_ingress_loss() {
  local ready
  # Let the 15 s blackbox scrape observe the VIP on its new holder.
  sleep 20
  ready=$(obs_value 'probe_success{job="blackbox",check="ready"}')
  [ "$ready" = "1" ] \
    || unverified "observability: the VIP ready probe is ${ready:-missing} during the ingress drain"
  say "observability: the VIP ready probe stays 1 during the ingress drain"
}

# Pause one non-VIP gateway replica for the full `for` window. The target stays
# in DNS SD, so `up{job="app", service="gateway"} == 0` is the live signal while
# the VIP continues to answer through the surviving gateway.
obs_gateway_replica_loss() {
  local holder host container state old_exit
  holder=$(ingress_holder)
  [ -n "$holder" ] || unverified "observability: no VIP holder for the gateway replica proof"
  host=$(jq -r '.services.gateway.hosts[]' "$descriptor" | grep -vx "$holder" | head -n 1)
  [ -n "$host" ] \
    || unverified "observability: no non-VIP gateway host for the replica proof"
  container="quantrule-gateway-$host"
  say "observability: pause $container on $host; the VIP stays on $holder"
  # shellcheck disable=SC2086
  $ssh_bin "$(host_address "$host")" "$obs_sudo $docker_bin pause $container" \
    || unverified "observability: cannot pause $container on $host"
  obs_gateway_host=$host
  obs_gateway_container=$container
  old_exit=$(trap -p EXIT || true)
  trap 'obs_unpause_gateway "$obs_gateway_host" "$obs_gateway_container" || true' EXIT
  if ! obs_wait_alert GatewayReplicaDown "$obs_alert_timeout"; then
    obs_unpause_gateway "$host" "$container"
    eval "$old_exit"
    unverified "observability: GatewayReplicaDown did not fire within ${obs_alert_timeout}s"
  fi
  if ! obs_wait_alertmanager GatewayReplicaDown 120; then
    obs_unpause_gateway "$host" "$container"
    eval "$old_exit"
    unverified "observability: GatewayReplicaDown did not reach Alertmanager"
  fi
  # shellcheck disable=SC2086
  state=$($ssh_bin "$(host_address "$host")" \
    "$obs_sudo $docker_bin inspect --format '{{.State.Paused}}' $container" 2>/dev/null || true)
  obs_unpause_gateway "$host" "$container"
  eval "$old_exit"
  [ "$state" = "true" ] \
    || unverified "observability: $container restarted during the replica pause"
  obs_gateway_down=1
  obs_wait_resolved GatewayReplicaDown "$obs_resolve_timeout" \
    || unverified "observability: GatewayReplicaDown did not resolve within ${obs_resolve_timeout}s"
  obs_gateway_resolved=1
}

obs_unpause_gateway() {
  local host=$1 container=$2
  [ -n "$container" ] || return 0
  # shellcheck disable=SC2086
  $ssh_bin "$(host_address "$host")" "$obs_sudo $docker_bin unpause $container" \
    >/dev/null 2>&1 || true
  obs_gateway_host=""
  obs_gateway_container=""
}

# `Simulated HA` only when every observability assertion fired and resolved.
obs_finish() {
  if [ "$obs_manager_down" = 1 ] && [ "$obs_manager_resolved" = 1 ] \
    && [ "$obs_gateway_down" = 1 ] && [ "$obs_gateway_resolved" = 1 ] \
    && [ "$obs_rule_unit_ok" = 1 ]; then
    observability_status="Simulated HA"
  else
    observability_status="Unverified"
  fi
  say "observability: status $observability_status"
}
