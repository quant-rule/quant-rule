#!/usr/bin/env bash
# Opt-in HA lab harness for the v3 Swarm profile.
#
# This is the `ha` test tier. It is not part of the fast gate: it deploys a real
# stack, drains one worker-capacity node, stops one manager on a distinct host,
# moves the ingress VIP, pauses the Patroni primary and one HAProxy with the
# watchdog off, and stops the RabbitMQ quorum leader; it proves that the
# surviving gateway, ingress, database, and queue still deliver the pinned
# digest and a confirmed quorum message. It never removes a stack, volume,
# secret, config, or node. `--plan` prints the sequence without contacting
# Docker. `tools/test_tiers.sh ha` defaults to `deploy/v3/deploy.lab.json`; an
# explicit descriptor still wins.
#
# Usage:
#   deploy/v3/test.sh --plan
#   deploy/v3/test.sh --confirm
#   deploy/v3/test.sh --confirm --skip-deploy
#   deploy/v3/test.sh --confirm --skip-database
#   deploy/v3/test.sh --confirm --skip-queue
#   deploy/v3/test.sh --confirm --skip-object
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
#   QR_SWARM_CACERT        CA bundle for HTTPS (default endpoints.database.tls_ca,
#                          then the rendered out/pki/database-ca.crt)
#   QR_SWARM_EVIDENCE      database-failover evidence JSON (default
#                          deploy/v3/evidence/<UTC>.json)
#   QR_SWARM_SUDO          reserved for a guarded remote command; empty by default
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd -- "$script_dir/../.." && pwd)"
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
cacert=${QR_SWARM_CACERT:-}

plan=0
confirm=0
skip_deploy=0
skip_database=0
skip_queue=0
skip_object=0
database_status="Unverified"
queue_status="Unverified"
object_status="Unverified"

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

# shellcheck source=deploy/v3/failover.sh
. "$script_dir/failover.sh"
# shellcheck source=deploy/v3/queue-failover.sh
. "$script_dir/queue-failover.sh"
# shellcheck source=deploy/v3/minio-failover.sh
. "$script_dir/minio-failover.sh"

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
    --skip-database)
      skip_database=1
      shift
      ;;
    --skip-queue)
      skip_queue=1
      shift
      ;;
    --skip-object)
      skip_object=1
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
  cat <<PLAN
ha: plan (no Docker is contacted)
ha: descriptor: $descriptor
ha: 1. preflight: docker info, active Swarm manager, 3 odd managers, descriptor hosts resolve, VIP and CA present
ha: 2. deploy: tools/swarm.sh bootstrap (render, labels, jobs, stack)
ha: 3. check: qr-deploy check --remote docker --overlay; ingress /health /ready /version over the private CA
ha: 4. ingress baseline: VIP holder, 2/2 ingress, targeted check and tools/verify.sh digest
ha: 5. ingress loss: drain the VIP holder; bounded 1 Hz HTTPS /health loop; the VIP moves to the survivor
ha: 6. negative controls: no port 80, each direct edge public 443 refused, VIP 443 answers, 8404/5432/8080/9000/5671/15692/15672 refused
ha: 7. loss: drain one worker-capacity node; stop one manager on a distinct host
ha: 8. verify: manager quorum, gateway/ingress, one 443 port, pinned digest, verification run
ha: 9. verify: stateful containers and bind mounts on survivors; no stateful unit moves
ha: 10. restore: activate the ingress holder and worker, start the manager, full check again
ha: 11. database preflight: etcd TLS health, patronictl leader + sync, psql through postgres
ha: 12. database baseline: marker row, pg_current_wal_lsn(), tools/verify.sh digest
ha: 13. database loss: docker pause the primary Patroni container (watchdog off); wait for a new leader + sync; measure RTO; read the marker; write a second marker through postgres; compare the digest and LSN
ha: 14. database control: pause one HAProxy; dial the surviving edge with PGHOSTADDR; psql still works; unpause
ha: 15. database negative controls: public edge/VIP 8404, 5432, 8080, 9000, 5671, 15692 and 15672 refuse; only 443 answers
ha: 16. database restore: unpause the old primary; wait for replica + pg_stat_replication; record pg_rewind/reinit; full check; digest equal
ha: 17. database evidence: value-free JSON under QR_SWARM_EVIDENCE; Simulated HA only on success
ha: 18. --skip-database leaves the failover row Unverified, never a pass
ha: 19. queue preflight: lab-single-host, SSH all brokers, three running nodes, no alarms, four .g queues quorum, no runs.*/stages.* policy, HAProxy rabbitmq:5671
ha: 20. queue baseline: app /health /ready, optional digest, live definitions verify, confirmed publish on qr.lab.queue-probe
ha: 21. queue loss: docker stop the quorum leader; poll two running nodes, a new leader, all queues quorum, no alarms; consume the acknowledged publish and recheck the app
ha: 22. queue restore: docker start; three running nodes, quorum, second confirmed publish; digest equal when measured; full check; value-free evidence
ha: 23. --skip-queue leaves the queue row Unverified, never a pass; the end-to-end digest is Measured only when tools/verify.sh succeeds, otherwise Unverified or Blocked
ha: 24. object preflight: lab-single-host, three MinIO nodes, six drives, EC:3, one-node-loss write quorum, mc admin info all nodes/drives with zero healing, bucket/versioning, app /health /ready
ha: 25. object baseline: write the lab probe through https://minio:9000 with qr_app, record its SHA-256, read it back; optional tools/verify.sh digest
ha: 26. object loss: docker stop the chosen MinIO node; reduced mc admin info; read the probe through minio:9000 and compare; app check; digest comparison
ha: 27. object restore: docker start; wait for three nodes/all drives; bounded mc admin heal -r qr/quantrule; zero healing; probe hash; full check; value-free Simulated HA evidence
ha: 28. --skip-object leaves the object row Unverified; the end-to-end digest is Measured only after tools/verify.sh succeeds; Real HA is HA-014
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

public_address() {
  jq -r --arg host "$1" '.hosts[$host].public_address // empty' "$descriptor"
}

remote() {
  local host=$1
  shift
  $ssh_bin "$(host_address "$host")" "$@"
}

curl_ingress() {
  if [ -n "$cacert" ]; then
    curl --cacert "$cacert" "$@"
  else
    curl "$@"
  fi
}

resolve_cacert() {
  local candidate
  if [ -n "$cacert" ]; then
    [ -r "$cacert" ] || unverified "QR_SWARM_CACERT $cacert is not readable"
    return 0
  fi
  candidate=$(jq -r '.endpoints.database.tls_ca // empty' "$descriptor" 2>/dev/null || true)
  if [ -n "$candidate" ] && [ -r "$candidate" ]; then
    cacert=$candidate
    say "CA: $cacert"
    return 0
  fi
  candidate="$out/pki/database-ca.crt"
  if [ -r "$candidate" ]; then
    cacert=$candidate
    say "CA: $cacert"
    return 0
  fi
  unverified "no readable CA bundle; set QR_SWARM_CACERT"
}

# The one ingress host that currently holds the VIP, or empty.
ingress_holder() {
  local host address output
  for host in $ingress_hosts; do
    address=$(host_address "$host")
    output=$(remote "$host" "ip -4 -o addr show dev $ingress_iface" 2>/dev/null || true)
    if printf '%s\n' "$output" | grep -Fq "inet $vip/"; then
      printf '%s' "$host"
      return 0
    fi
  done
  return 0
}

node_id_for_host() {
  local host=$1 id
  for id in $node_ids; do
    if [ "${node_host[$id]}" = "$host" ]; then
      printf '%s' "$id"
      return 0
    fi
  done
  return 1
}

# Wait for any VIP holder, or for a holder different from $1 when one is given.
wait_vip() {
  local old=${1:-} attempt holder
  for attempt in $(seq 1 90); do
    holder=$(ingress_holder)
    if [ -n "$holder" ] && { [ -z "$old" ] || [ "$holder" != "$old" ]; }; then
      printf '%s' "$holder"
      return 0
    fi
    sleep 1
  done
  unverified "the VIP did not appear${old:+ on a host other than $old} within 90 s"
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
    body=$(curl_ingress -fsS --max-time 10 "https://$dns/$path" 2>/dev/null) \
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
  local phase=$1 output digest args
  [ -x "$verify" ] || unverified "$phase: verification script $verify is not executable"
  args=(--api-url "$api_url" --timeout "$verify_timeout")
  [ -n "$cacert" ] && args+=(--cacert "$cacert")
  output=$("$verify" "${args[@]}") \
    || unverified "$phase: the verification run failed"
  digest=$(printf '%s\n' "$output" | sed -n 's/^canonical digest sha256:\([0-9a-f]*\)$/\1/p')
  [ -n "$digest" ] || unverified "$phase: the verification run printed no canonical digest"
  last_digest=$digest
  say "$phase: verification run digest sha256:$digest"
}

ingress_preflight() {
  local holder count host address expected replicas
  resolve_cacert
  ingress_hosts=$(jq -r '.ingress.hosts[]' "$descriptor") \
    || unverified "cannot read the ingress hosts"
  [ -n "$ingress_hosts" ] || unverified "the descriptor declares no ingress hosts"
  for host in $ingress_hosts; do
    address=$(host_address "$host")
    $ssh_bin "$address" true >/dev/null 2>&1 \
      || unverified "cannot SSH to ingress host $host ($address); set QR_SWARM_SSH"
    remote "$host" "command -v nc" >/dev/null 2>&1 \
      || unverified "nc is not available on ingress host $host"
    remote "$host" "command -v curl" >/dev/null 2>&1 \
      || unverified "curl is not available on ingress host $host"
  done
  "$docker_bin" config inspect quantrule_traefik >/dev/null 2>&1 \
    || unverified "config quantrule_traefik is missing; bootstrap the ingress first"
  wait_converged
  holder=$(ingress_holder)
  [ -n "$holder" ] || unverified "no ingress host holds the VIP $vip"
  count=0
  for host in $ingress_hosts; do
    if [ -n "$(vip_on_host "$host")" ]; then
      count=$((count + 1))
    fi
  done
  [ "$count" -eq 1 ] \
    || unverified "the VIP $vip is held by $count ingress hosts; expected exactly one"
  expected=$(jq -r '.ingress.replicas' "$descriptor")
  replicas=$("$docker_bin" service inspect --format \
    '{{.ServiceStatus.RunningTasks}}/{{.ServiceStatus.DesiredTasks}}' "${stack}_ingress" 2>/dev/null || true)
  [ "$replicas" = "$expected/$expected" ] \
    || unverified "the ingress is $replicas; expected $expected/$expected"
  vip_holder=$holder
  say "ingress preflight: VIP $vip on $vip_holder; ingress $replicas; CA $cacert"
}

vip_on_host() {
  local host=$1 address output
  address=$(host_address "$host")
  output=$(remote "$host" "ip -4 -o addr show dev $ingress_iface" 2>/dev/null || true)
  if printf '%s\n' "$output" | grep -Fq "inet $vip/"; then
    printf '%s' "$host"
  fi
}

negative_closed_http() {
  local label=$1 url=$2 code
  code=$(curl -sS -o /dev/null -w '%{http_code}' --max-time 10 "$url" 2>/dev/null || true)
  [ -n "$code" ] || code=000
  [ "$code" = "000" ] || unverified "$label answered HTTP $code; it must not answer"
  say "negative control: $label closed"
}

negative_closed_tcp() {
  local label=$1 host=$2 port=$3
  if nc -z -w 5 "$host" "$port" >/dev/null 2>&1; then
    unverified "$label is reachable from outside"
  fi
  say "negative control: $label closed"
}

negative_controls() {
  local host address port
  negative_closed_http "port 80" "http://$dns/"
  for host in $ingress_hosts; do
    address=$(public_address "$host")
    [ -n "$address" ] \
      || unverified "ingress host $host has no public_address; the direct-edge control cannot run"
    negative_closed_tcp "direct edge public $host:443" "$address" 443
    for port in 8404 5432 8080 9000 5671 15692 15672; do
      negative_closed_tcp "direct edge public $host:$port" "$address" "$port"
    done
  done
  nc -z -w 5 "$vip" 443 >/dev/null 2>&1 \
    || unverified "the VIP $vip does not answer 443"
  curl_ingress -fsS --max-time 10 --resolve "$dns:443:$vip" "https://$dns/health" >/dev/null 2>&1 \
    || unverified "https://$dns/health does not answer through VIP $vip"
  say "negative control: VIP $vip:443 answers"
  for port in 8404 5432 8080 9000 5671 15692 15672; do
    negative_closed_tcp "VIP $port" "$vip" "$port"
  done
}

ingress_loss() {
  local baseline holder other holder_id
  local failures server_errors unexpected window probe_dir probe_log probe_stop probe_pid
  targeted_check "ingress baseline"
  verify_run "ingress baseline"
  baseline=$last_digest
  holder=$(ingress_holder)
  [ -n "$holder" ] || unverified "the ingress baseline has no VIP holder"
  vip_holder=$holder
  say "ingress baseline: VIP $vip on $vip_holder; digest sha256:$baseline"

  probe_dir=$(mktemp -d "${TMPDIR:-/tmp}/quantrule-ingress-probe.XXXXXX")
  probe_log=$probe_dir/health.log
  probe_stop=$probe_dir/stop
  probe_pid=""
  cleanup_probe() {
    : > "${probe_stop:-/dev/null}"
    if [ -n "${probe_pid:-}" ]; then
      wait "$probe_pid" 2>/dev/null || true
    fi
    if [ -n "${probe_dir:-}" ]; then
      rm -rf "$probe_dir"
    fi
  }
  trap cleanup_probe EXIT
  : > "$probe_log"
  (
    tick=0
    while [ ! -e "$probe_stop" ]; do
      tick=$((tick + 1))
      code=$(curl_ingress -sS -o /dev/null -w '%{http_code}' --max-time 2 \
        "https://$dns/health" 2>/dev/null || true)
      [ -n "$code" ] || code=000
      printf '%s %s\n' "$tick" "$code" >> "$probe_log"
      sleep 1
    done
  ) &
  probe_pid=$!

  holder_id=$(node_id_for_host "$holder") \
    || unverified "cannot map the VIP holder $holder to a Swarm node"
  say "ingress loss: drain the VIP holder $holder ($holder_id)"
  "$docker_bin" node update --availability drain "$holder_id" >/dev/null
  other=$(wait_vip "$holder")
  : > "$probe_stop"
  wait "$probe_pid" || true
  [ -n "$other" ] && [ "$other" != "$holder" ] \
    || unverified "the VIP did not move from $holder"
  say "ingress loss: VIP moved from $holder to $other"

  failures=$(awk '$2 == "000" { n++ } END { print n + 0 }' "$probe_log")
  server_errors=$(awk '$2 ~ /^5/ { n++ } END { print n + 0 }' "$probe_log")
  unexpected=$(awk '$2 != "000" && $2 != "200" { n++ } END { print n + 0 }' "$probe_log")
  [ "$server_errors" -eq 0 ] \
    || unverified "the move produced $server_errors HTTP 5xx response(s)"
  [ "$unexpected" -eq 0 ] \
    || unverified "the move produced $unexpected unexpected HTTP response(s)"
  window=$(awk '
    $2 == "000" && !failed { first = $1; failed = 1 }
    failed && $2 == "200" && !done { window = $1 - first; done = 1 }
    END {
      if (!failed) print 0
      else if (!done) print -1
      else print window
    }
  ' "$probe_log")
  [ "$window" -ge 0 ] \
    || unverified "the 1 Hz HTTPS probe never recovered after the VIP moved"
  say "ingress loss: $failures failed probe(s), outage window ${window}s, no 5xx"

  targeted_check "after ingress move"
  verify_run "after ingress move"
  [ "$last_digest" = "$baseline" ] \
    || unverified "the digest changed after the ingress move"
  stateful_check_all
  stateful_no_move
  negative_controls

  say "restore: activate the ingress holder $holder"
  "$docker_bin" node update --availability active "$holder_id" >/dev/null
  wait_converged
  wait_vip >/dev/null
  targeted_check "after ingress restore"
  verify_run "after ingress restore"
  [ "$last_digest" = "$baseline" ] \
    || unverified "the digest changed after the ingress restore"
  cleanup_probe
  probe_dir=""
  probe_pid=""
  trap - EXIT
}

preflight() {
  local local_state control nodes manager_count host address found id role
  command -v "$docker_bin" >/dev/null 2>&1 \
    || unverified "docker executable $docker_bin is not available"
  "$docker_bin" info >/dev/null 2>&1 || unverified "cannot reach the Docker daemon"
  command -v jq >/dev/null 2>&1 || unverified "jq is required to read the descriptor"
  command -v curl >/dev/null 2>&1 || unverified "curl is required to probe the ingress"
  command -v nc >/dev/null 2>&1 || unverified "nc is required for the ingress negative controls"
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
  vip=$(jq -r '.ingress.vip // empty' "$descriptor") \
    || unverified "cannot read the ingress VIP"
  [ -n "$vip" ] \
    || unverified "ingress.vip is missing; the ingress-loss phase needs the floating VIP"
  ingress_iface=$(jq -r '.ingress.interface // "eth0"' "$descriptor") \
    || unverified "cannot read the ingress interface"
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
vip=""
ingress_iface="eth0"
evidence=""
stateful_hosts=""
ingress_hosts=""
worker_id=""
worker_host=""
manager_id=""
manager_host=""
manager_addr=""
vip_holder=""
last_digest=""

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
  say "  $docker_bin node update --availability drain <vip-holder-id>   # ingress loss"
  say "  $docker_bin node update --availability active <vip-holder-id>  # ingress restore"
  say "  $docker_bin pause quantrule-patroni-<leader>   # database failover, watchdog off"
  say "  $docker_bin unpause quantrule-patroni-<leader> # database restore"
  say "  $docker_bin stop quantrule-rabbitmq-<leader>   # queue leader loss"
  say "  $docker_bin start quantrule-rabbitmq-<leader>  # queue restore"
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

say "ingress: preflight"
ingress_preflight
say "ingress: VIP loss and negative controls"
ingress_loss

if [ "$skip_database" -eq 1 ]; then
  say "database: skipped (--skip-database); failover row Unverified"
else
  say "database: preflight, baseline, pause, write, HAProxy control, restore, evidence"
  db_failover
  database_status="Simulated HA"
fi

if [ "$skip_queue" -eq 1 ]; then
  say "queue: skipped (--skip-queue); broker-loss row Unverified"
else
  say "queue: preflight, baseline, leader stop, publish survival, restore, evidence"
  queue_failover
  queue_status="Simulated HA"
fi

if [ "$skip_object" -eq 1 ]; then
  say "object: skipped (--skip-object); object-store row Unverified"
else
  say "object: preflight, baseline probe, node stop, probe survival, restore, heal, evidence"
  object_failover
  object_status="Simulated HA"
fi

say "check: full remote/overlay after restore"
if [ -f "$out/stateful-manifest.json" ]; then
  qr_deploy check "$descriptor" --remote docker --overlay --out "$out" \
    || unverified "the full check failed after restore"
else
  qr_deploy check "$descriptor" --remote docker --overlay \
    || unverified "the full check failed after restore"
fi
digest_status=${o_digest_status:-Unverified}
if [ "$skip_object" -eq 1 ]; then
  digest_status="Unverified"
fi
say "summary: $evidence; worker/manager/ingress; database $database_status; queue $queue_status; object_store $object_status"
say "end-to-end canonical digest: $digest_status; Real HA is HA-014"
