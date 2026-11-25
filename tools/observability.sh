#!/usr/bin/env bash
# Operate the v3 Prometheus/Grafana/Loki/Alertmanager tier.
#
# The wrapper renders the descriptor, copies the observability manifest, the
# per-host Compose files, configs, dashboards, the private CA, and the Patroni
# REST password to the hosts that need them over SSH standard input, starts
# Alertmanager -> exporters -> Prometheus -> Loki -> promtail -> Grafana, waits
# for readiness, targets, rules, and the log pipeline, runs promtool/amtool when
# the pinned images provide them, and writes the manager-quorum textfile that
# node_exporter already mounts.
#
# Usage: tools/observability.sh [--plan|--dry-run] <command>
#
#   copy             Render, then copy the manifest, units, configs, dashboards,
#                    CA, and Patroni password to the right hosts.
#   start            Start Alertmanager, node_exporter/blackbox, Prometheus, Loki,
#                    promtail on every host, then Grafana.
#   wait             Wait for Prometheus /-/ready, every target up, the 17 rule
#                    rows, Alertmanager, Loki, promtail, and Grafana.
#   status           Show compose ps and readiness on every observability host.
#   verify           Run wait, promtool/amtool where present, and the manager
#                    quorum textfile; optionally stamp the verify-age textfile.
#   manager-quorum   Write quantrule_swarm_managers_ready from `docker node ls`
#                    into the node_exporter textfile directory. No Docker socket
#                    is added to a container.
#
# Environment:
#   DESCRIPTOR                  descriptor path (default deploy/v3/deploy.example.json)
#   OUT                         render directory (default deploy/v3/out)
#   QR_DEPLOY_BIN               qr-deploy binary; otherwise qr-deploy on PATH, then cargo
#   QR_OBSERVABILITY_SSH        ssh command and options
#   QR_OBSERVABILITY_SUDO       remote privilege command (default "sudo -n"; empty for root)
#   QR_OBSERVABILITY_DOCKER     remote docker executable (default docker)
#   QR_OBSERVABILITY_ATTEMPTS   wait attempts (default 60)
#   QR_OBSERVABILITY_INTERVAL   seconds between wait attempts (default 2)
#   QR_OBSERVABILITY_MANAGER    manager host for the quorum textfile (default the
#                               first declared manager)
#   QR_OBSERVABILITY_STAMP      set 1 to write the verify-age timestamp textfile
#                               after a successful verify (default off; the F16
#                               verify-age rule stays dormant without it)
#
# No stack, volume, secret, config, node, or service is removed. No secret value
# is printed or passed as an argument. The manager-quorum path reads `docker
# node ls` on the host, never through a container socket. The lab remains
# `Unverified`; `Real HA` is HA-014.
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "$script_dir/.." && pwd)
cd "$root"

descriptor=${DESCRIPTOR:-deploy/v3/deploy.example.json}
out=${OUT:-deploy/v3/out}
ssh_bin=${QR_OBSERVABILITY_SSH:-ssh -o BatchMode=yes -o ConnectTimeout=5}
sudo_bin=${QR_OBSERVABILITY_SUDO-sudo -n}
docker_bin=${QR_OBSERVABILITY_DOCKER:-docker}
attempts=${QR_OBSERVABILITY_ATTEMPTS:-60}
interval=${QR_OBSERVABILITY_INTERVAL:-2}
manager=${QR_OBSERVABILITY_MANAGER:-}
plan=0
command=""

die() {
  printf 'observability: %s\n' "$*" >&2
  exit 1
}

usage() {
  awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --plan | --dry-run)
      plan=1
      shift
      ;;
    -h | --help | help)
      usage
      exit 0
      ;;
    copy | start | wait | status | verify | manager-quorum)
      [ -z "$command" ] || { printf 'observability: one command at a time\n' >&2; exit 2; }
      command=$1
      shift
      ;;
    *)
      printf 'observability: unknown argument %s; run tools/observability.sh --help\n' "$1" >&2
      exit 2
      ;;
  esac
done

if [ "$plan" -eq 1 ]; then
  cat <<PLAN
observability: plan (no Docker, SSH, jq, or secret file is read)
observability: descriptor: $descriptor
observability: render: qr-deploy render $descriptor --out $out --target all
observability: 1 copy: observability-manifest.json + infra/<host>.yaml + configs, dashboards, CA, and Patroni password over SSH stdin
observability: 2 start: Alertmanager -> node_exporter + blackbox -> Prometheus -> Loki -> promtail (every host) -> Grafana
observability: 3 wait: Prometheus /-/ready, every target up, 38 rules across 17 failure_row values, Alertmanager /-/ready, Loki /-/ready, promtail /ready, Grafana /api/health
observability: 4 verify: promtool check rules, amtool check-config, manager-quorum textfile
observability: 5 manager-quorum: docker node ls -> quantrule_swarm_managers_ready in the node_exporter textfile directory
observability: no stack, volume, secret, config, node, or service is removed
PLAN
  exit 0
fi

[ -n "$command" ] || { printf 'observability: a command is required; run tools/observability.sh --help\n' >&2; exit 2; }
command -v jq >/dev/null 2>&1 || die "jq is required for $command"

qr_deploy() {
  if [ -n "${QR_DEPLOY_BIN:-}" ]; then
    "$QR_DEPLOY_BIN" "$@"
  elif command -v qr-deploy >/dev/null 2>&1; then
    qr-deploy "$@"
  else
    cargo run --quiet -p quantrule-deploy -- "$@"
  fi
}

render() {
  qr_deploy render "$descriptor" --out "$out" --target all
}

unit_host() {
  jq -r --arg unit "$1" '.observability[$unit].host' "$descriptor"
}

all_hosts() {
  jq -r '.hosts | keys[]' "$descriptor"
}

manager_host() {
  if [ -n "$manager" ]; then
    printf '%s\n' "$manager"
  else
    jq -r '.swarm.managers[0]' "$descriptor"
  fi
}

copy_one() {
  host=$1
  source=$2
  target=$3
  mode=$4
  [ -f "$source" ] || die "missing $source"
  directory=$(dirname "$target")
  remote="umask 077; mkdir -p $directory; cat > $target; chmod $mode $target"
  # shellcheck disable=SC2086
  $ssh_bin "$host" "$sudo_bin sh -c '$remote'" < "$source" || \
    die "cannot copy $source to $host:$target"
  printf 'observability: copied %s -> %s:%s\n' "$source" "$host" "$target"
}

copy() {
  render
  [ -f "$out/observability-manifest.json" ] || die "missing $out/observability-manifest.json"
  prom_host=$(unit_host prometheus)
  [ -n "$prom_host" ] || die "observability.prometheus.host is empty"
  for host in $(all_hosts); do
    [ -f "$out/infra/$host.yaml" ] || die "missing $out/infra/$host.yaml"
    copy_one "$host" "$out/observability-manifest.json" \
      /etc/quantrule/observability-manifest.json 644
    copy_one "$host" "$out/infra/$host.yaml" "/etc/quantrule/infra/$host.yaml" 644
    while IFS=$'\t' read -r source target mode; do
      [ -n "$source" ] || continue
      # The Patroni REST password is a stateful secret owned by the stateful
      # copy. Do not reset its mode here: the Patroni image runs as `postgres`
      # and must still read it.
      if [ "$target" = "/etc/quantrule/secrets/patroni_rest_password" ] \
        && $ssh_bin "$host" "test -e '$target'" >/dev/null 2>&1; then
        continue
      fi
      copy_one "$host" "$out/$source" "$target" "$mode"
    done < <(
      jq -r --arg host "$host" '
        .files[]
        | select(.hosts | index($host))
        | [.source, .target, .mode] | @tsv
      ' "$out/observability-manifest.json"
    )
    # Bind-mount data directories. The images run as their documented users:
    # Prometheus, Alertmanager, node_exporter, and blackbox as nobody; Grafana
    # as 472; Loki as 10001; promtail as root for the Docker socket.
    # shellcheck disable=SC2086
    $ssh_bin "$host" "$sudo_bin sh -c '
      umask 022
      install -d -m 0750 /srv/quantrule/prometheus /srv/quantrule/prometheus/data /srv/quantrule/alertmanager
      install -d -m 0750 /srv/quantrule/grafana /srv/quantrule/grafana/data
      install -d -m 0750 /srv/quantrule/loki /srv/quantrule/loki/data
      install -d -m 0750 /srv/quantrule/promtail
      install -d -m 0755 /srv/quantrule/node_exporter /srv/quantrule/node_exporter/textfile
      for d in /etc/quantrule/prometheus /etc/quantrule/blackbox /etc/quantrule/grafana /etc/quantrule/loki /etc/quantrule/promtail; do
        [ ! -d "\$d" ] || chmod 0755 "\$d"
      done
      chown -R 65534:65534 /srv/quantrule/alertmanager /srv/quantrule/prometheus
      chown -R 472:472 /srv/quantrule/grafana
      chown -R 10001:10001 /srv/quantrule/loki
    '" || die "cannot create the observability data directories on $host"
  done
  printf 'observability: copied the configs, dashboards, CA, and Patroni password to %s\n' "$prom_host"
}

start_unit() {
  host=$1
  service=$2
  [ -n "$host" ] || die "cannot start $service without a host"
  printf 'observability: starting %s on %s\n' "$service" "$host"
  # shellcheck disable=SC2086
  # Bind-mounted config files are read at process start; `compose up` alone does
  # not recreate a running unit when only their bytes changed. Force one
  # recreate so a copied Prometheus/Alertmanager/Loki/Grafana config takes effect.
  $ssh_bin "$host" "cd /etc/quantrule/infra && $sudo_bin $docker_bin compose -f $host.yaml up -d --force-recreate $service" || \
    die "cannot start $service on $host"
}

start() {
  start_unit "$(unit_host alertmanager)" alertmanager
  for host in $(all_hosts); do
    start_unit "$host" "node-exporter-$host"
  done
  start_unit "$(unit_host prometheus)" blackbox
  start_unit "$(unit_host prometheus)" prometheus
  start_unit "$(unit_host loki)" loki
  for host in $(all_hosts); do
    start_unit "$host" "promtail-$host"
  done
  start_unit "$(unit_host grafana)" grafana
}

container_name() {
  case "$1" in
    prometheus) printf 'quantrule-prometheus' ;;
    alertmanager) printf 'quantrule-alertmanager' ;;
    loki) printf 'quantrule-loki' ;;
    grafana) printf 'quantrule-grafana' ;;
    blackbox) printf 'quantrule-blackbox' ;;
    node-exporter-*) printf 'quantrule-%s' "$1" ;;
    promtail-*) printf 'quantrule-%s' "$1" ;;
    *) printf 'quantrule-%s' "$1" ;;
  esac
}

remote_get() {
  host=$1
  service=$2
  url=$3
  # shellcheck disable=SC2086
  $ssh_bin "$host" "$sudo_bin $docker_bin exec $(container_name "$service") wget -q -O - $url"
}

remote_ready() {
  host=$1
  service=$2
  url=$3
  remote_get "$host" "$service" "$url" >/dev/null
}

# Probe a service locally through the shipped app image, sharing the target
# container's network namespace. Loki, Grafana, and promtail images do not all
# include a shell HTTP client, and cross-host overlay DNS is not reliable in the
# single-host lab.
probe_ready() {
  local host=$1 service=$2 path=$3 image port
  case "$service" in
    loki) port=3100 ;;
    grafana) port=3000 ;;
    promtail-*) port=9080 ;;
    *) port=80 ;;
  esac
  image=$(jq -r '.image.reference + "@" + .image.digest' "$descriptor")
  # shellcheck disable=SC2086
  $ssh_bin "$host" "$sudo_bin $docker_bin run --rm --network container:$(container_name "$service") --entrypoint qr-gateway $image --healthcheck http://127.0.0.1:$port$path" >/dev/null
}

wait_ready() {
  prom_host=$(unit_host prometheus)
  alert_host=$(unit_host alertmanager)
  attempt=1
  while :; do
    failed=""
    remote_ready "$prom_host" prometheus "http://127.0.0.1:9090/-/ready" || failed="Prometheus /-/ready"
    if [ -z "$failed" ]; then
      targets=$(remote_get "$prom_host" prometheus "http://127.0.0.1:9090/api/v1/targets?state=active" || true)
      printf '%s\n' "$targets" | grep -q '"health":"up"' || failed="Prometheus targets"
      printf '%s\n' "$targets" | grep -q '"health":"down"' && failed="Prometheus target down"
    fi
    if [ -z "$failed" ]; then
      rules=$(remote_get "$prom_host" prometheus "http://127.0.0.1:9090/api/v1/rules?type=alert" || true)
      rows=$(printf '%s\n' "$rules" | grep -o '"failure_row":"F[0-9][0-9]"' | sort -u | wc -l | tr -d ' ')
      [ "$rows" = "17" ] || failed="alert rules (found $rows failure rows)"
    fi
    if [ -z "$failed" ]; then
      remote_ready "$alert_host" alertmanager "http://127.0.0.1:9093/-/ready" || failed="Alertmanager /-/ready"
    fi
    if [ -z "$failed" ]; then
      probe_ready "$(unit_host loki)" loki "/ready" || failed="Loki /ready"
    fi
    if [ -z "$failed" ]; then
      for host in $(all_hosts); do
        probe_ready "$host" "promtail-$host" "/ready" || { failed="promtail $host"; break; }
      done
    fi
    if [ -z "$failed" ]; then
      probe_ready "$(unit_host grafana)" grafana "/api/health" || failed="Grafana /api/health"
    fi
    if [ -z "$failed" ]; then
      printf 'observability: ready: Prometheus, every target, 17 rule rows, Alertmanager, Loki, promtail, Grafana\n'
      return 0
    fi
    [ "$attempt" -lt "$attempts" ] || die "not ready after $attempts attempts: $failed"
    printf 'observability: waiting for %s (attempt %s/%s)\n' "$failed" "$attempt" "$attempts"
    attempt=$((attempt + 1))
    sleep "$interval"
  done
}

write_metric() {
  host=$1
  name=$2
  value=$3
  help=$4
  path=/srv/quantrule/node_exporter/textfile/$name.prom
  # shellcheck disable=SC2086
  printf '# HELP %s %s\n# TYPE %s gauge\n%s %s\n' "$name" "$help" "$name" "$name" "$value" | \
    $ssh_bin "$host" "$sudo_bin sh -c 'umask 022; cat > $path; chmod 644 $path'" || \
    die "cannot write the $name textfile on $host"
  printf 'observability: wrote %s=%s on %s:%s\n' "$name" "$value" "$host" "$path"
}

manager_quorum() {
  host=$(manager_host)
  [ -n "$host" ] || die "swarm.managers is empty"
  count=$(
    # shellcheck disable=SC2086
    $ssh_bin "$host" "$sudo_bin $docker_bin node ls --format '{{.Status}}'" 2>/dev/null | \
      grep -c '^Ready' || true
  )
  count=${count:-0}
  printf 'observability: %s reports %s Ready Swarm managers\n' "$host" "$count"
  write_metric "$host" quantrule_swarm_managers_ready "$count" \
    "Number of Swarm managers reporting Ready."
}

verify() {
  wait_ready
  prom_host=$(unit_host prometheus)
  alert_host=$(unit_host alertmanager)
  # shellcheck disable=SC2086
  if $ssh_bin "$prom_host" "$sudo_bin $docker_bin exec quantrule-prometheus promtool check rules /etc/prometheus/alert-rules.yml" >/dev/null 2>&1; then
    printf 'observability: promtool accepted the alert rules\n'
  else
    printf 'observability: promtool is unavailable; the rule API check above is the fallback\n'
  fi
  # shellcheck disable=SC2086
  if $ssh_bin "$alert_host" "$sudo_bin $docker_bin exec quantrule-alertmanager amtool check-config /etc/alertmanager/alertmanager.yml" >/dev/null 2>&1; then
    printf 'observability: amtool accepted the Alertmanager config\n'
  else
    printf 'observability: amtool is unavailable; readiness above is the fallback\n'
  fi
  manager_quorum
  if [ "${QR_OBSERVABILITY_STAMP:-0}" = "1" ]; then
    write_metric "$(manager_host)" quantrule_verify_last_success_timestamp_seconds \
      "$(date +%s)" "Unix time of the last successful observability verification."
  else
    printf 'observability: verify-age textfile not written (set QR_OBSERVABILITY_STAMP=1); the F16 verify-age rule is dormant\n'
  fi
  printf 'observability: verified\n'
}

status() {
  prom_host=$(unit_host prometheus)
  for host in $(printf '%s\n' \
    "$(unit_host prometheus)" "$(unit_host grafana)" \
    "$(unit_host loki)" "$(unit_host alertmanager)" | sort -u); do
    printf '\n== %s ==\n' "$host"
    # shellcheck disable=SC2086
    $ssh_bin "$host" "cd /etc/quantrule/infra && $sudo_bin $docker_bin compose -f $host.yaml ps" || true
  done
  printf '\n== readiness ==\n'
  printf 'Prometheus: '
  remote_ready "$(unit_host prometheus)" prometheus "http://127.0.0.1:9090/-/ready" && printf 'ready\n' || printf 'not ready\n'
  printf 'Alertmanager: '
  remote_ready "$(unit_host alertmanager)" alertmanager "http://127.0.0.1:9093/-/ready" && printf 'ready\n' || printf 'not ready\n'
  printf 'Loki: '
  probe_ready "$(unit_host loki)" loki "/ready" && printf 'ready\n' || printf 'not ready\n'
  printf 'Grafana: '
  probe_ready "$(unit_host grafana)" grafana "/api/health" && printf 'ready\n' || printf 'not ready\n'
}

case "$command" in
  copy) copy ;;
  start) start ;;
  wait) wait_ready ;;
  status) status ;;
  verify) verify ;;
  manager-quorum) manager_quorum ;;
  *) die "unknown command $command" ;;
esac
