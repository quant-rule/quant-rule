#!/usr/bin/env bash
# Opt-in HA lab harness for the v3 Swarm profile.
#
# This is the `ha` test tier. It is not part of the fast gate: it deploys a real
# stack, drains one worker-capacity node, stops one manager on a distinct host,
# and proves that the surviving gateway and ingress still deliver the pinned
# digest. It never removes a stack, volume, secret, config, or node, and never
# moves a stateful unit. `--plan` prints the sequence without contacting Docker.
#
# Usage:
#   deploy/v3/test.sh --plan
#   deploy/v3/test.sh --confirm
#   deploy/v3/test.sh --confirm --skip-deploy
#
# Environment:
#   DESCRIPTOR             descriptor path (default deploy/v3/deploy.example.json)
#   OUT                    render directory (default deploy/v3/out)
#   QR_SWARM_DOCKER        docker executable (default docker)
#   QR_SWARM_SSH           ssh command and options (default "ssh -o BatchMode=yes -o ConnectTimeout=5")
#   QR_SWARM_STOP_CMD      remote manager stop command (default "sudo systemctl stop docker")
#   QR_SWARM_START_CMD     remote manager start command (default "sudo systemctl start docker")
#   QR_SWARM_VERIFY        verification script (default tools/verify.sh)
#   QR_SWARM_API_URL       gateway API base (default https://<ingress.dns>/api/v1)
#   QR_SWARM_VERIFY_TIMEOUT seconds per verification run (default 300)
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"

descriptor=${DESCRIPTOR:-deploy/v3/deploy.example.json}
out=${OUT:-deploy/v3/out}
docker_bin=${QR_SWARM_DOCKER:-docker}
ssh_bin=${QR_SWARM_SSH:-ssh -o BatchMode=yes -o ConnectTimeout=5}
stop_cmd=${QR_SWARM_STOP_CMD:-sudo systemctl stop docker}
start_cmd=${QR_SWARM_START_CMD:-sudo systemctl start docker}
verify=${QR_SWARM_VERIFY:-tools/verify.sh}
verify_timeout=${QR_SWARM_VERIFY_TIMEOUT:-300}
api_url=${QR_SWARM_API_URL:-}

plan=0
confirm=0
skip_deploy=0

usage() {
  awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
}

usage_error() {
  printf 'ha: %s\n\n' "$*" >&2
  usage >&2
  exit 2
}

say() {
  printf 'ha: %s\n' "$*"
}

unverified() {
  printf 'ha: UNVERIFIED: %s\n' "$*" >&2
  exit 1
}

while [ $# -gt 0 ]; do
  case "$1" in
    --plan | --dry-run)
      plan=1
      shift
      ;;
    --confirm)
      confirm=1
      shift
      ;;
    --skip-deploy)
      skip_deploy=1
      shift
      ;;
    --descriptor)
      [ $# -ge 2 ] || usage_error "--descriptor needs a value"
      descriptor=$2
      shift 2
      ;;
    --out)
      [ $# -ge 2 ] || usage_error "--out needs a value"
      out=$2
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      usage_error "unknown argument $1"
      ;;
  esac
done

if [ "$plan" -eq 1 ]; then
  cat <<'PLAN'
ha: plan (no Docker is contacted)
ha: 1. preflight: docker info, active Swarm manager, 3 odd managers, descriptor hosts resolve
ha: 2. deploy: tools/swarm.sh bootstrap (render, labels, jobs, stack)
ha: 3. check: qr-deploy check --remote docker --overlay; ingress /health /ready /version
ha: 4. loss: drain one worker-capacity node; stop one manager on a distinct host
ha: 5. verify: manager quorum, gateway/ingress, one 443 port, pinned digest, verification run
ha: 6. verify: stateful containers and bind mounts on survivors; no stateful unit moves
ha: 7. restore: start the manager, activate the worker, full check again
ha: evidence: Simulated HA for lab-single-host; Real HA is HA-014 only
ha: this plan is not evidence
PLAN
  exit 0
fi

qr_deploy() {
  if [ -n "${QR_DEPLOY_BIN:-}" ]; then
    "$QR_DEPLOY_BIN" "$@"
  elif command -v qr-deploy >/dev/null 2>&1; then
    qr-deploy "$@"
  else
    cargo run --quiet -p quantrule-deploy -- "$@"
  fi
}

host_address() {
  jq -r --arg host "$1" '.hosts[$host].address' "$descriptor"
}

remote() {
  local host=$1
  shift
  $ssh_bin "$(host_address "$host")" "$@"
}

stateful_containers() {
  jq -r '
    def unit($name; $hosts): $hosts[] | [., "quantrule-\($name)-\(.)"] | @tsv;
    (unit("patroni"; .stateful.postgres.nodes)),
    (unit("etcd"; .stateful.etcd.nodes)),
    (unit("haproxy"; .stateful.haproxy.nodes)),
    (unit("rabbitmq"; .stateful.rabbitmq.nodes)),
    (unit("minio"; .stateful.minio.nodes)),
    ([.observability.prometheus.host, "quantrule-prometheus-\(.observability.prometheus.host)"] | @tsv),
    ([.observability.grafana.host, "quantrule-grafana-\(.observability.grafana.host)"] | @tsv),
    ([.observability.loki.host, "quantrule-loki-\(.observability.loki.host)"] | @tsv),
    ([.observability.alertmanager.host, "quantrule-alertmanager-\(.observability.alertmanager.host)"] | @tsv)
  ' "$descriptor"
}

stateful_check_host() {
  local host=$1 expected container running mounts_json source
  while IFS=$'\t' read -r expected container; do
    [ -n "$container" ] || continue
    [ "$expected" = "$host" ] || continue
    running=$(remote "$host" "$docker_bin inspect --format '{{.State.Running}}' $container" 2>/dev/null || true)
    [ "$running" = "true" ] || unverified "stateful container $container is not running on $host"
    mounts_json=$(remote "$host" "$docker_bin inspect --format '{{json .Mounts}}' $container" 2>/dev/null || true)
    [ -n "$mounts_json" ] || unverified "cannot read the mounts of $container on $host"
    printf '%s' "$mounts_json" | jq -e 'all(.[]; .Type == "bind" and (.Source | length > 0))' >/dev/null \
      || unverified "stateful container $container on $host has a non-bind or empty mount"
    while IFS= read -r source; do
      [ -n "$source" ] || continue
      remote "$host" "test -d '$source'" >/dev/null 2>&1 \
        || unverified "bind mount $source is missing on $host"
    done <<< "$(printf '%s' "$mounts_json" | jq -r '.[].Source')"
  done <<< "$(stateful_containers)"
}

stateful_check_all() {
  local skip=${1:-} host
  for host in $stateful_hosts; do
    [ "$host" = "$skip" ] && continue
    stateful_check_host "$host"
  done
}

stateful_no_move() {
  local skip=${1:-} host container expected names
  declare -A running=()
  for host in $stateful_hosts; do
    [ "$host" = "$skip" ] && continue
    names=$(remote "$host" "$docker_bin ps --format '{{.Names}}'" 2>/dev/null || true)
    while IFS= read -r container; do
      [ -n "$container" ] || continue
      running[$container]=$host
    done <<< "$names"
  done
  while IFS=$'\t' read -r expected container; do
    [ -n "$container" ] || continue
    if [ -n "${running[$container]:-}" ] && [ "${running[$container]}" != "$expected" ]; then
      unverified "stateful container $container moved from $expected to ${running[$container]}"
    fi
  done <<< "$(stateful_containers)"
}

manager_quorum() {
  local reachable
  "$docker_bin" node ls >/dev/null 2>&1 || unverified "the surviving managers have no quorum"
  reachable=$("$docker_bin" node ls --format '{{.ManagerStatus}}' | grep -Ec 'Leader|Reachable' || true)
  [ "${reachable:-0}" -ge 2 ] || unverified "only ${reachable:-0} managers are reachable"
}

wait_ready() {
  local host=$1 state attempt
  for attempt in $(seq 1 60); do
    state=$("$docker_bin" node ls --format '{{.Hostname}} {{.Status}}' | awk -v host="$host" '$1 == host { print $2 }')
    [ "$state" = "Ready" ] && return 0
    sleep 2
  done
  unverified "node $host did not return to Ready"
}

wait_manager_down() {
  local host=$1 status attempt
  for attempt in $(seq 1 30); do
    status=$("$docker_bin" node ls --format '{{.Hostname}} {{.ManagerStatus}}' | awk -v host="$host" '$1 == host { print $2 }')
    case "$status" in
      Unreachable | "") return 0 ;;
    esac
    sleep 2
  done
  unverified "manager $host did not become unreachable"
}

wait_converged() {
  local attempt rows
  for attempt in $(seq 1 120); do
    rows=$("$docker_bin" service ls --filter "label=com.docker.stack.namespace=$stack" --format '{{.Replicas}}')
    if [ -n "$rows" ] && printf '%s\n' "$rows" | awk -F/ '$1 != $2 { bad=1 } END { exit bad }'; then
      return 0
    fi
    sleep 2
  done
  unverified "the stack did not converge after the node was restored"
}

targeted_check() {
  local phase=$1 expected_reference expected_image actual ports body path
  expected_reference=$(jq -r '.image.reference' "$descriptor")
  case "$expected_reference" in
    *@*) expected_reference=${expected_reference%%@*} ;;
  esac
  expected_image="$expected_reference@$(jq -r '.image.digest' "$descriptor")"
  for service in gateway ingress; do
    actual=$("$docker_bin" service inspect --format '{{.ServiceStatus.RunningTasks}}' "${stack}_$service" 2>/dev/null || true)
    [ "${actual:-0}" -ge 1 ] || unverified "$phase: $service has no running task"
  done
  actual=$("$docker_bin" service inspect --format '{{.Spec.TaskTemplate.ContainerSpec.Image}}' "${stack}_gateway")
  [ "$actual" = "$expected_image" ] \
    || unverified "$phase: gateway runs $actual, expected $expected_image"
  ports=$("$docker_bin" service inspect --format '{{json .Endpoint.Ports}}' "${stack}_ingress")
  case "$ports" in
    *'"PublishedPort":443'*) ;;
    *) unverified "$phase: the ingress does not publish 443" ;;
  esac
  case "$ports" in
    *'"PublishMode":"host"'*) ;;
    *) unverified "$phase: the ingress 443 port is not host mode" ;;
  esac
  for service in gateway data model strategy backtest evidence simulation; do
    ports=$("$docker_bin" service inspect --format '{{json .Endpoint.Ports}}' "${stack}_$service")
    case "$ports" in
      null | '[]') ;;
      *) unverified "$phase: $service publishes a port" ;;
    esac
  done
  for path in health ready version; do
    body=$(curl -fsS --max-time 10 "https://$dns/$path" 2>/dev/null) \
      || unverified "$phase: ingress /$path is unreachable"
    case "$path" in
      version)
        printf '%s' "$body" | grep -Eq '"image_digest"[[:space:]]*:[[:space:]]*"'"$(jq -r '.image.digest' "$descriptor")"'"' \
          || unverified "$phase: /version does not report the pinned digest"
        ;;
      *)
        printf '%s' "$body" | grep -Fq '"state":"healthy"' \
          || unverified "$phase: /$path is not healthy"
        ;;
    esac
  done
}

verify_run() {
  local phase=$1 output digest
  [ -x "$verify" ] || unverified "$phase: verification script $verify is not executable"
  output=$("$verify" --api-url "$api_url" --timeout "$verify_timeout") \
    || unverified "$phase: the verification run failed"
  digest=$(printf '%s\n' "$output" | sed -n 's/^canonical digest sha256:\([0-9a-f]*\)$/\1/p')
  [ -n "$digest" ] || unverified "$phase: the verification run printed no canonical digest"
  say "$phase: verification run digest sha256:$digest"
}

preflight() {
  local local_state control nodes manager_count host address found id role
  command -v "$docker_bin" >/dev/null 2>&1 \
    || unverified "docker executable $docker_bin is not available"
  "$docker_bin" info >/dev/null 2>&1 || unverified "cannot reach the Docker daemon"
  command -v jq >/dev/null 2>&1 || unverified "jq is required to read the descriptor"
  command -v curl >/dev/null 2>&1 || unverified "curl is required to probe the ingress"
  [ -f "$descriptor" ] || unverified "descriptor $descriptor is missing"
  local_state=$("$docker_bin" info --format '{{.Swarm.LocalNodeState}}' 2>/dev/null || true)
  control=$("$docker_bin" info --format '{{.Swarm.ControlAvailable}}' 2>/dev/null || true)
  [ "$local_state" = "active" ] \
    || unverified "Swarm is ${local_state:-unavailable}; run this on an active Swarm manager"
  [ "$control" = "true" ] || unverified "this node is not a Swarm manager"
  profile=$(jq -r '.profile' "$descriptor") \
    || unverified "cannot read the descriptor profile"
  dns=$(jq -r '.ingress.dns' "$descriptor") \
    || unverified "cannot read the ingress DNS"
  stack=$(jq -r '.name // "quantrule"' "$descriptor") \
    || unverified "cannot read the stack name"
  nodes=$("$docker_bin" node ls --format '{{.ID}} {{.Hostname}} {{.Role}} {{.Availability}} {{.ManagerStatus}}' 2>/dev/null) \
    || unverified "cannot list Swarm nodes"
  manager_count=$(printf '%s\n' "$nodes" | awk '$3 == "manager" { count++ } END { print count+0 }')
  [ "$manager_count" -ge 3 ] \
    || unverified "manager count $manager_count; three odd managers are required"
  [ $((manager_count % 2)) -eq 1 ] \
    || unverified "manager count $manager_count is even; three odd managers are required"
  node_ids=$("$docker_bin" node ls -q) || unverified "cannot list Swarm node IDs"
  [ -n "$node_ids" ] || unverified "the Swarm reports no nodes"
  node_ids=$(printf '%s\n' "$node_ids" | sort)
  local_id=$("$docker_bin" info --format '{{.Swarm.NodeID}}') \
    || unverified "cannot read the local Swarm node ID"
  for id in $node_ids; do
    node_host[$id]=$("$docker_bin" node inspect --format '{{.Description.Hostname}}' "$id") \
      || unverified "cannot inspect node $id"
    node_addr[$id]=$("$docker_bin" node inspect --format '{{.Status.Addr}}' "$id") \
      || unverified "cannot inspect node $id"
    node_role[$id]=$("$docker_bin" node inspect --format '{{.Spec.Role}}' "$id") \
      || unverified "cannot inspect node $id"
  done
  host_keys=$(jq -r '.hosts | keys[]' "$descriptor") \
    || unverified "cannot read the descriptor hosts"
  [ -n "$host_keys" ] || unverified "the descriptor declares no hosts"
  for host in $host_keys; do
    address=$(jq -r --arg host "$host" '.hosts[$host].address' "$descriptor")
    found=""
    for id in $node_ids; do
      if [ "${node_host[$id]}" = "$host" ] || [ "${node_addr[$id]}" = "$address" ]; then
        found=$id
        break
      fi
    done
    [ -n "$found" ] \
      || unverified "descriptor host $host ($address) does not resolve to a Swarm node"
  done
  ingress_hosts=$(jq -r '.ingress.hosts[]' "$descriptor") \
    || unverified "cannot read the ingress hosts"
  worker_id=""
  manager_id=""
  select_loss_targets() {
    local skip_ingress=$1 id role host
    for id in $node_ids; do
      [ "$id" = "$local_id" ] && continue
      host=${node_host[$id]}
      if [ "$skip_ingress" -eq 1 ] && printf '%s\n' "$ingress_hosts" | grep -Fxq "$host"; then
        continue
      fi
      role=${node_role[$id]}
      if [ "$role" = "worker" ] && [ -z "$worker_id" ]; then
        worker_id=$id
      fi
      if [ "$role" = "manager" ] && [ -z "$manager_id" ]; then
        manager_id=$id
      fi
    done
  }
  select_loss_targets 1
  select_loss_targets 0
  if [ -z "$worker_id" ]; then
    for id in $node_ids; do
      [ "$id" = "$local_id" ] && continue
      [ "$id" = "$manager_id" ] && continue
      worker_id=$id
      break
    done
  fi
  [ -n "$worker_id" ] || unverified "no worker-capacity node distinct from the local manager"
  [ -n "$manager_id" ] || unverified "no second manager to stop; three managers are required"
  [ "$worker_id" != "$manager_id" ] || unverified "worker and manager targets are the same host"
  worker_host=${node_host[$worker_id]}
  manager_host=${node_host[$manager_id]}
  manager_addr=${node_addr[$manager_id]}
  $ssh_bin "$manager_addr" true >/dev/null 2>&1 \
    || unverified "cannot SSH to manager $manager_host ($manager_addr); set QR_SWARM_SSH"
  stateful_hosts=$(jq -r '
    [.stateful.postgres.nodes[], .stateful.etcd.nodes[], .stateful.haproxy.nodes[],
     .stateful.rabbitmq.nodes[], .stateful.minio.nodes[],
     .observability.prometheus.host, .observability.grafana.host,
     .observability.loki.host, .observability.alertmanager.host] | unique | .[]
  ' "$descriptor") || unverified "cannot read the stateful host list"
  if [ -z "$api_url" ]; then
    api_url="https://$dns/api/v1"
  fi
  case "$profile" in
    lab-single-host) evidence="Simulated HA" ;;
    *) evidence="HA profile (unverified)" ;;
  esac
  say "preflight: $manager_count managers, $profile, evidence $evidence"
  say "loss targets: worker-capacity $worker_host, manager $manager_host"
}

declare -A node_host=() node_addr=() node_role=()
node_ids=""
local_id=""
profile=""
dns=""
stack=""
evidence=""
stateful_hosts=""
ingress_hosts=""
worker_id=""
worker_host=""
manager_id=""
manager_host=""
manager_addr=""

preflight

if [ "$skip_deploy" -eq 0 ]; then
  say "deploy: tools/swarm.sh bootstrap ($descriptor)"
  DESCRIPTOR="$descriptor" OUT="$out" ./tools/swarm.sh bootstrap \
    || unverified "bootstrap failed"
  say "check: qr-deploy check --remote docker --overlay"
  qr_deploy check "$descriptor" --remote docker --overlay \
    || unverified "the remote/overlay check failed"
else
  say "deploy: skipped (--skip-deploy)"
fi

if [ "$confirm" -eq 0 ]; then
  say "loss checks not run: pass --confirm"
  say "exact commands:"
  say "  $docker_bin node update --availability drain $worker_id   # $worker_host"
  say "  $ssh_bin $manager_addr '$stop_cmd'   # $manager_host"
  say "  $ssh_bin $manager_addr '$start_cmd'"
  say "  $docker_bin node update --availability active $worker_id"
  exit 2
fi

say "loss: drain worker-capacity node $worker_host ($worker_id)"
"$docker_bin" node update --availability drain "$worker_id" >/dev/null
sleep 5
manager_quorum
targeted_check "after worker drain"
stateful_check_all
stateful_no_move
verify_run "after worker drain"
say "restore: activate $worker_host"
"$docker_bin" node update --availability active "$worker_id" >/dev/null
wait_converged
targeted_check "after worker restore"

say "loss: stop manager $manager_host ($manager_addr)"
$ssh_bin "$manager_addr" "$stop_cmd" \
  || unverified "cannot stop the manager on $manager_host"
wait_manager_down "$manager_host"
manager_quorum
targeted_check "after manager stop"
stateful_check_all "$manager_host"
stateful_no_move "$manager_host"
verify_run "after manager stop"
say "restore: start manager $manager_host"
$ssh_bin "$manager_addr" "$start_cmd" \
  || unverified "cannot start the manager on $manager_host"
wait_ready "$manager_host"
manager_quorum
stateful_check_all
stateful_no_move
targeted_check "after manager restore"
verify_run "after manager restore"

say "check: full remote/overlay after restore"
qr_deploy check "$descriptor" --remote docker --overlay \
  || unverified "the full check failed after restore"
say "PASS: $evidence"
