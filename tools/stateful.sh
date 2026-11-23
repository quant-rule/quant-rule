#!/usr/bin/env bash
# Operate the v3 Patroni/etcd/HAProxy tier in the approved boot order.
#
# The wrapper renders the descriptor, creates the encrypted overlay with
# `bootstrap.sh --network-only`, copies the stateful manifest files, configs,
# PKI, secrets, and host Compose files to each host over SSH, starts etcd ->
# Patroni -> HAProxy, and waits for etcd quorum, a Patroni leader with a sync
# standby, and a psql connection through the `postgres` alias. It never removes
# a stack, volume, secret, config, node, or service. Secret values travel on
# standard input and are never printed or passed as an argument.
#
# Usage: tools/stateful.sh [--plan|--dry-run] <command>
#
#   network  Render, then create or validate the encrypted attachable overlay.
#   copy     Render, then copy the manifest, configs, PKI, secrets, and compose files.
#   start    Start etcd on every etcd host, then Patroni, then HAProxy.
#   wait     Wait for etcd health, Patroni leader + sync standby, and a primary psql.
#   status   Show compose ps, Patroni list, and etcd health on every stateful host.
#
# Environment:
#   DESCRIPTOR             descriptor path (default deploy/v3/deploy.example.json)
#   OUT                    render directory (default deploy/v3/out)
#   QR_DEPLOY_BIN          qr-deploy binary; otherwise qr-deploy on PATH, then cargo
#   QR_STATEFUL_SSH        ssh command and options (default "ssh -o BatchMode=yes -o ConnectTimeout=5")
#   QR_STATEFUL_SUDO       remote privilege command (default "sudo -n"; set empty for root)
#   QR_STATEFUL_DOCKER     remote docker executable (default docker)
#   QR_STATEFUL_ATTEMPTS   wait attempts (default 90)
#   QR_STATEFUL_INTERVAL   seconds between wait attempts (default 2)
#   QR_STATEFUL_DATA_OWNER optional owner[:group] for the Patroni etcd data dirs;
#                          empty keeps the image's own ownership
#
# The overlay step must run on a Swarm manager. A three-VM lab is `Simulated HA`;
# `Real HA` remains HA-014. A missing lab or precondition is `UNVERIFIED`, never
# a pass.
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "$script_dir/.." && pwd)
cd "$root"

descriptor=${DESCRIPTOR:-deploy/v3/deploy.example.json}
out=${OUT:-deploy/v3/out}
ssh_bin=${QR_STATEFUL_SSH:-ssh -o BatchMode=yes -o ConnectTimeout=5}
sudo_bin=${QR_STATEFUL_SUDO-sudo -n}
docker_bin=${QR_STATEFUL_DOCKER:-docker}
attempts=${QR_STATEFUL_ATTEMPTS:-90}
interval=${QR_STATEFUL_INTERVAL:-2}
plan=0
command=""

die() {
  printf 'stateful: %s\n' "$*" >&2
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
    network | copy | start | wait | status)
      [ -z "$command" ] || { printf 'stateful: one command at a time\n' >&2; exit 2; }
      command=$1
      shift
      ;;
    *)
      printf 'stateful: unknown argument %s; run tools/stateful.sh --help\n' "$1" >&2
      exit 2
      ;;
  esac
done

if [ "$plan" -eq 1 ]; then
  cat <<PLAN
stateful: plan (no Docker, SSH, jq, or secret file is read)
stateful: descriptor: $descriptor
stateful: render: qr-deploy render $descriptor --out $out --target all
stateful: 1 network: sh $out/bootstrap.sh --network-only on a Swarm manager
stateful: 2 copy: stateful-manifest.json + infra/<host>.yaml + configs + PKI + secrets over SSH stdin
stateful: 3 start: etcd -> Patroni -> HAProxy on each host
stateful: 4 wait: etcdctl endpoint health; patronictl list (Leader + Sync); psql -h postgres (primary)
stateful: 5 status: compose ps, Patroni list, etcd health
stateful: no stack, volume, secret, config, node, or service is removed
PLAN
  exit 0
fi

[ -n "$command" ] || { printf 'stateful: a command is required; run tools/stateful.sh --help\n' >&2; exit 2; }
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

descriptor_list() {
  jq -r "$1[]" "$descriptor"
}

all_hosts() {
  jq -r '[.stateful.postgres.nodes[], .stateful.etcd.nodes[], .stateful.haproxy.nodes[]] | unique[]' \
    "$descriptor"
}

overlay() {
  jq -r '.network.overlay' "$descriptor"
}

postgres_image() {
  jq -r '.images.postgres.reference + "@" + .images.postgres.digest' "$descriptor"
}

network() {
  render
  [ -f "$out/bootstrap.sh" ] || die "missing $out/bootstrap.sh"
  sh "$out/bootstrap.sh" --network-only
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
  printf 'stateful: copied %s -> %s:%s\n' "$source" "$host" "$target"
}

copy() {
  render
  [ -f "$out/stateful-manifest.json" ] || die "missing $out/stateful-manifest.json"
  for host in $(all_hosts); do
    copy_one "$host" "$out/stateful-manifest.json" "/etc/quantrule/stateful-manifest.json" 644
    copy_one "$host" "$out/infra/$host.yaml" "/etc/quantrule/infra/$host.yaml" 644
    while IFS=$'\t' read -r source target mode; do
      [ -n "$source" ] || continue
      copy_one "$host" "$out/$source" "$target" "$mode"
    done < <(
      jq -r --arg host "$host" '
        .files[]
        | select(
            .source == ("patroni-" + $host + ".yml")
            or .source == ("etcd-" + $host + ".conf.yml")
            or .source == ("haproxy-" + $host + ".cfg")
            or .scope == "secret"
            or .scope == "pki"
          )
        | [.source, .target, .mode] | @tsv
      ' "$out/stateful-manifest.json"
    )
    # shellcheck disable=SC2086
    $ssh_bin "$host" "$sudo_bin sh -c 'mkdir -p /srv/quantrule/patroni/data /srv/quantrule/patroni/wal-archive /srv/quantrule/etcd/data'" || \
      die "cannot create the data directories on $host"
    if [ -n "${QR_STATEFUL_DATA_OWNER:-}" ]; then
      # shellcheck disable=SC2086
      $ssh_bin "$host" "$sudo_bin chown $QR_STATEFUL_DATA_OWNER /srv/quantrule/patroni /srv/quantrule/patroni/data /srv/quantrule/patroni/wal-archive /srv/quantrule/etcd /srv/quantrule/etcd/data" || \
        die "cannot set the data directory owner on $host"
    fi
  done
}

start_phase() {
  phase=$1
  hosts=$2
  for host in $hosts; do
    printf 'stateful: starting %s on %s\n' "$phase" "$host"
    # shellcheck disable=SC2086
    $ssh_bin "$host" "cd /etc/quantrule/infra && $sudo_bin $docker_bin compose -f $host.yaml up -d $phase" || \
      die "cannot start $phase on $host"
  done
}

start() {
  start_phase etcd "$(descriptor_list '.stateful.etcd.nodes')"
  start_phase patroni "$(descriptor_list '.stateful.postgres.nodes')"
  start_phase haproxy "$(descriptor_list '.stateful.haproxy.nodes')"
}

etcd_health() {
  host=$1
  # shellcheck disable=SC2086
  $ssh_bin "$host" "$sudo_bin $docker_bin exec -e ETCDCTL_API=3 quantrule-etcd-$host etcdctl \
    --endpoints=https://127.0.0.1:2379 \
    --cacert=/etc/quantrule/pki/ca.crt \
    --cert=/etc/quantrule/pki/etcd/client.crt \
    --key=/etc/quantrule/pki/etcd/client.key \
    --dial-timeout=2s --command-timeout=5s endpoint health" >/dev/null
}

patroni_list() {
  host=$1
  # shellcheck disable=SC2086
  $ssh_bin "$host" "$sudo_bin $docker_bin exec quantrule-patroni-$host sh -c \
    'PATRONI_CTL_PASSWORD=\$(cat /run/secrets/patroni_rest_password) patronictl -c /etc/patroni/patroni.yml list'"
}

patroni_ready() {
  output=$(patroni_list "$1") || return 1
  printf '%s\n' "$output" | grep -q 'Leader' || return 1
  printf '%s\n' "$output" | grep -q 'Sync' || return 1
}

primary_psql() {
  host=$1
  image=$2
  name=$3
  # shellcheck disable=SC2086
  $ssh_bin "$host" "PGPASSWORD=\$(cat /etc/quantrule/secrets/postgres_superuser_password); export PGPASSWORD; \
    $sudo_bin $docker_bin run --rm --network $name \
    -e PGPASSWORD -e PGSSLMODE=verify-full -e PGSSLROOTCERT=/run/pki/ca.crt \
    -v /etc/quantrule/pki/ca.crt:/run/pki/ca.crt:ro \
    $image psql -h postgres -U postgres -d postgres -tAc 'SELECT NOT pg_is_in_recovery()'"
}

wait_ready() {
  image=$(postgres_image)
  name=$(overlay)
  psql_host=$(descriptor_list '.stateful.haproxy.nodes' | head -n 1)
  [ -n "$psql_host" ] || psql_host=$(descriptor_list '.stateful.postgres.nodes' | head -n 1)
  attempt=1
  while :; do
    failed=""
    for host in $(descriptor_list '.stateful.etcd.nodes'); do
      etcd_health "$host" || { failed="etcd $host"; break; }
    done
    if [ -z "$failed" ]; then
      for host in $(descriptor_list '.stateful.postgres.nodes'); do
        patroni_ready "$host" || { failed="Patroni $host (leader/sync)"; break; }
      done
    fi
    if [ -z "$failed" ]; then
      primary=$(primary_psql "$psql_host" "$image" "$name" 2>/dev/null || true)
      [ "$primary" = "t" ] || failed="psql through postgres"
    fi
    if [ -z "$failed" ]; then
      printf 'stateful: ready: etcd health, Patroni leader + sync standby, primary psql through postgres\n'
      return 0
    fi
    [ "$attempt" -lt "$attempts" ] || die "not ready after $attempts attempts: $failed"
    printf 'stateful: waiting for %s (attempt %s/%s)\n' "$failed" "$attempt" "$attempts"
    attempt=$((attempt + 1))
    sleep "$interval"
  done
}

status() {
  for host in $(all_hosts); do
    printf '\n== %s ==\n' "$host"
    # shellcheck disable=SC2086
    $ssh_bin "$host" "cd /etc/quantrule/infra && $sudo_bin $docker_bin compose -f $host.yaml ps" || true
  done
  printf '\n== Patroni ==\n'
  for host in $(descriptor_list '.stateful.postgres.nodes'); do
    printf '%s: ' "$host"
    patroni_list "$host" | tail -n +2 || true
  done
  printf '\n== etcd ==\n'
  for host in $(descriptor_list '.stateful.etcd.nodes'); do
    printf '%s: ' "$host"
    etcd_health "$host" && printf 'healthy\n' || printf 'unhealthy\n'
  done
}

case "$command" in
  network) network ;;
  copy) copy ;;
  start) start ;;
  wait) wait_ready ;;
  status) status ;;
  *) die "unknown command $command" ;;
esac
