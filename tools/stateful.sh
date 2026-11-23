#!/usr/bin/env bash
# Operate the v3 MinIO/Patroni/etc/HAProxy/RabbitMQ tier in the approved boot order.
#
# The wrapper renders the descriptor, creates the encrypted overlay with
# `bootstrap.sh --network-only`, copies the stateful manifest files, configs,
# PKI, secrets, and host Compose files to each host over SSH, starts MinIO ->
# etcd -> Patroni -> HAProxy -> the first RabbitMQ broker, waits for that broker,
# starts the other two, and waits for MinIO nodes/drives with zero healing, etcd
# quorum, a Patroni leader with a sync standby, a three-node RabbitMQ cluster with
# no alarms, and a psql connection through the `postgres` alias. It never removes
# a stack, volume, secret, config, node, or service. Secret values travel on
# standard input and are never printed or passed as an argument on the operator
# host.
#
# Usage: tools/stateful.sh [--plan|--dry-run] <command>
#
#   network         Render, then create or validate the encrypted attachable overlay.
#   copy            Render, then copy the manifest, configs, PKI, secrets, and compose files.
#   start           Start MinIO, then etcd, then Patroni, then HAProxy, seed the first
#                   RabbitMQ broker, then start the other two brokers.
#   wait            Wait for every MinIO node/drive with zero healing, etcd health,
#                   Patroni leader + sync standby, a three-node RabbitMQ cluster with
#                   no alarms, and a primary psql.
#   queue-bootstrap Create the app user, vhost, and permissions from queue_url.
#                   Run this after `wait` and before `swarm.sh bootstrap`.
#   object-bootstrap Create the quantrule bucket, enable versioning, create the
#                   qr_app user, attach the least-privilege policy, and run a
#                   put/get/delete probe. Run after `wait`, before `swarm.sh bootstrap`.
#   status          Show compose ps, MinIO admin info, Patroni list, etcd health,
#                   and RabbitMQ cluster/alarm state on every stateful host.
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
#   QR_STATEFUL_DATA_OWNER optional owner[:group] for the Patroni, etcd, RabbitMQ,
#                          and MinIO data dirs; empty keeps the image's own ownership
#   QR_STATEFUL_QUEUE_URL  queue_url file for queue-bootstrap (default: the
#                          descriptor secrets.files.queue_url path)
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
    network | copy | start | wait | queue-bootstrap | object-bootstrap | status)
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
stateful: 2 copy: stateful-manifest.json + infra/<host>.yaml + per-host configs/PKI/secrets over SSH stdin
stateful: 3 start: MinIO nodes -> etcd -> Patroni -> HAProxy -> seed the first RabbitMQ broker -> start the other two
stateful: 4 wait: MinIO mc admin info (all nodes/drives, 0 offline, 0 healing); etcdctl endpoint health; patronictl list (Leader + Sync); RabbitMQ cluster_status (3 running, no alarms); psql -h postgres (primary)
stateful: 5 queue-bootstrap: create the app user/vhost/permissions from queue_url (password on stdin, before swarm bootstrap)
stateful: 6 object-bootstrap: pinned mc; create quantrule, enable versioning, create qr_app, attach quantrule-app, put/get/delete probe
stateful: 7 status: compose ps, MinIO admin info/healing, Patroni list, etcd health, RabbitMQ cluster/alarms
stateful: no stack, volume, secret, config, node, service, or bucket object outside the bootstrap probe is removed
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
  jq -r '[.stateful.postgres.nodes[], .stateful.etcd.nodes[], .stateful.haproxy.nodes[], .stateful.rabbitmq.nodes[], .stateful.minio.nodes[]] | unique[]' \
    "$descriptor"
}

rabbitmq_hosts() {
  descriptor_list '.stateful.rabbitmq.nodes'
}

is_rabbitmq_host() {
  host=$1
  for broker in $(rabbitmq_hosts); do
    [ "$broker" = "$host" ] && return 0
  done
  return 1
}

minio_hosts() {
  descriptor_list '.stateful.minio.nodes'
}

is_minio_host() {
  host=$1
  for node in $(minio_hosts); do
    [ "$node" = "$host" ] && return 0
  done
  return 1
}

minio_drives() {
  jq -r --arg host "$1" '
    .stateful.minio.nodes as $nodes
    | .stateful.minio.drives[$nodes | index($host)]
  ' "$descriptor"
}

minio_total_drives() {
  jq -r '[.stateful.minio.drives[]] | add' "$descriptor"
}

minio_image() {
  jq -r '.images.minio_client.reference + "@" + .images.minio_client.digest' "$descriptor"
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
    [ -f "$out/infra/$host.yaml" ] || die "missing $out/infra/$host.yaml"
    copy_one "$host" "$out/stateful-manifest.json" "/etc/quantrule/stateful-manifest.json" 644
    copy_one "$host" "$out/infra/$host.yaml" "/etc/quantrule/infra/$host.yaml" 644
    while IFS=$'\t' read -r source target mode; do
      [ -n "$source" ] || continue
      copy_one "$host" "$out/$source" "$target" "$mode"
    done < <(
      jq -r --arg host "$host" '
        .files[]
        | select(.hosts | index($host))
        | [.source, .target, .mode] | @tsv
      ' "$out/stateful-manifest.json"
    )
    directories="/srv/quantrule/patroni/data /srv/quantrule/patroni/wal-archive /srv/quantrule/etcd/data"
    owned="/srv/quantrule/patroni /srv/quantrule/patroni/data /srv/quantrule/patroni/wal-archive /srv/quantrule/etcd /srv/quantrule/etcd/data"
    private_dirs=""
    if is_rabbitmq_host "$host"; then
      directories="$directories /srv/quantrule/rabbitmq/data /srv/quantrule/rabbitmq/definitions /srv/quantrule/rabbitmq/conf.d"
      owned="$owned /srv/quantrule/rabbitmq /srv/quantrule/rabbitmq/data /srv/quantrule/rabbitmq/definitions /srv/quantrule/rabbitmq/conf.d"
    fi
    if is_minio_host "$host"; then
      drives=$(minio_drives "$host")
      directories="$directories /srv/quantrule/minio /srv/quantrule/minio/$host /etc/quantrule/minio /etc/quantrule/minio/mc /etc/quantrule/minio/mc-app"
      owned="$owned /srv/quantrule/minio /srv/quantrule/minio/$host"
      private_dirs="/etc/quantrule/minio /etc/quantrule/minio/mc /etc/quantrule/minio/mc-app"
      for drive in $(seq 1 "$drives"); do
        directories="$directories /srv/quantrule/minio/$host/drive$drive"
        owned="$owned /srv/quantrule/minio/$host/drive$drive"
      done
    fi
    # shellcheck disable=SC2086
    $ssh_bin "$host" "$sudo_bin sh -c 'umask 077; mkdir -p $directories; [ -z \"$private_dirs\" ] || chmod 700 $private_dirs'" || \
      die "cannot create the data directories on $host"
    if [ -n "${QR_STATEFUL_DATA_OWNER:-}" ]; then
      # shellcheck disable=SC2086
      $ssh_bin "$host" "$sudo_bin chown $QR_STATEFUL_DATA_OWNER $owned" || \
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

rabbitmq_container() {
  printf 'quantrule-rabbitmq-%s' "$1"
}

rabbitmq_exec() {
  host=$1
  shift
  # shellcheck disable=SC2086
  $ssh_bin "$host" "$sudo_bin $docker_bin exec -u rabbitmq $(rabbitmq_container "$host") $*"
}

rabbitmq_ping() {
  rabbitmq_exec "$1" rabbitmq-diagnostics -q ping >/dev/null
}

rabbitmq_ready() {
  host=$1
  rabbitmq_ping "$host" || return 1
  rabbitmq_exec "$host" rabbitmqctl await_startup >/dev/null
}

rabbitmq_eval() {
  host=$1
  expression=$2
  rabbitmq_exec "$host" rabbitmqctl -q eval "$expression" | tr -d '[:space:]'
}

rabbitmq_cluster_ready() {
  host=$1
  [ "$(rabbitmq_eval "$host" 'length(rabbit_nodes:list_running()).')" = "3" ] || return 1
  [ "$(rabbitmq_eval "$host" 'length(rabbit_alarm:get_alarms()).')" = "0" ] || return 1
  status=$(rabbitmq_exec "$host" rabbitmqctl cluster_status) || return 1
  for broker in $(rabbitmq_hosts); do
    printf '%s\n' "$status" | grep -q "rabbit@rabbitmq-$broker" || return 1
  done
}

wait_rabbitmq_seed() {
  seed=$1
  attempt=1
  while :; do
    rabbitmq_ready "$seed" && return 0
    [ "$attempt" -lt "$attempts" ] || die "RabbitMQ seed $seed did not start after $attempts attempts"
    printf 'stateful: waiting for RabbitMQ seed %s (attempt %s/%s)\n' "$seed" "$attempt" "$attempts"
    attempt=$((attempt + 1))
    sleep "$interval"
  done
}

start() {
  for host in $(minio_hosts); do
    start_phase "minio-$host" "$host"
  done
  start_phase etcd "$(descriptor_list '.stateful.etcd.nodes')"
  start_phase patroni "$(descriptor_list '.stateful.postgres.nodes')"
  start_phase haproxy "$(descriptor_list '.stateful.haproxy.nodes')"
  brokers=$(rabbitmq_hosts)
  seed=$(printf '%s\n' "$brokers" | head -n 1)
  [ -n "$seed" ] || die "stateful.rabbitmq.nodes is empty"
  start_phase "rabbitmq-broker-$seed" "$seed"
  wait_rabbitmq_seed "$seed"
  for host in $brokers; do
    [ "$host" = "$seed" ] && continue
    start_phase "rabbitmq-broker-$host" "$host"
  done
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

minio_mc_dir=/etc/quantrule/minio/mc
minio_mc_app_dir=/etc/quantrule/minio/mc-app
minio_policy_file=/etc/quantrule/minio/quantrule-app-policy.json
minio_bucket=quantrule
minio_app_user=qr_app
minio_app_policy=quantrule-app

minio_container() {
  printf 'quantrule-minio-%s' "$1"
}

# Write an mc config from two mounted secret files. The JSON is built on the
# host and written at mode 600; the secret values never enter the operator's
# command line, the SSH command line, or a log.
minio_write_config() {
  host=$1
  directory=$2
  user_file=$3
  secret_file=$4
  endpoint=$(jq -r '.endpoints.object_store.scheme + "://" + .endpoints.object_store.host + ":" + (.endpoints.object_store.port | tostring)' "$descriptor")
  script='
set -eu
umask 077
directory=$1
user_file=$2
secret_file=$3
endpoint=$4
mkdir -p "$directory"
chmod 700 "$directory"
user=$(cat "$user_file")
secret=$(cat "$secret_file")
printf "{\"version\":\"10\",\"aliases\":{\"local\":{\"url\":\"%s\",\"accessKey\":\"%s\",\"secretKey\":\"%s\",\"api\":\"S3v4\",\"path\":\"auto\"}}}\n" \
  "$endpoint" "$user" "$secret" > "$directory/config.json"
chmod 600 "$directory/config.json"
'
  # shellcheck disable=SC2086
  printf '%s\n' "$script" | $ssh_bin "$host" "$sudo_bin sh -s -- '$directory' '$user_file' '$secret_file' '$endpoint'" || \
    die "cannot write the MinIO client config on $host"
}

# Run the pinned mc client with a config directory mounted at /mc. The private
# MinIO CA is mounted at /certs/ca.crt and SSL_CERT_FILE points mc at it, so a
# self-signed internal leaf verifies without --insecure.
minio_mc() {
  host=$1
  directory=$2
  shift 2
  image=$(minio_image)
  overlay=$(overlay)
  # shellcheck disable=SC2086
  $ssh_bin "$host" "$sudo_bin $docker_bin run --rm --network $overlay \
    -v $directory:/mc:ro \
    -v /etc/quantrule/pki/minio/CAs/ca.crt:/certs/ca.crt:ro \
    -e SSL_CERT_FILE=/certs/ca.crt \
    $image --config-dir /mc $*"
}

# Run a shell script inside the pinned mc client; used only where the mc CLI
# needs a value read from a mounted file. The private CA is mounted and trusted
# here for the same reason as minio_mc.
minio_mc_shell() {
  host=$1
  directory=$2
  mounts=$3
  script=$4
  image=$(minio_image)
  overlay=$(overlay)
  # shellcheck disable=SC2086
  $ssh_bin "$host" "$sudo_bin $docker_bin run --rm --network $overlay \
    --entrypoint /bin/sh $mounts \
    -v $directory:/mc:ro \
    -v /etc/quantrule/pki/minio/CAs/ca.crt:/certs/ca.crt:ro \
    -e SSL_CERT_FILE=/certs/ca.crt \
    $image -c '$script'"
}

minio_ready() {
  host=$1
  total=$(minio_total_drives)
  info=$(minio_mc "$host" "$minio_mc_dir" admin info local) || return 1
  for node in $(minio_hosts); do
    printf '%s\n' "$info" | grep -q "minio-$node:9000" || return 1
  done
  printf '%s\n' "$info" | grep -q "0 drives offline" || return 1
  printf '%s\n' "$info" | grep -q "0 drives healing" || return 1
  printf '%s\n' "$info" | grep -Eq "$total drives online" || return 1
}

wait_ready() {
  image=$(postgres_image)
  name=$(overlay)
  minio_host=$(minio_hosts | head -n 1)
  [ -n "$minio_host" ] || die "stateful.minio.nodes is empty"
  minio_write_config "$minio_host" "$minio_mc_dir" \
    /etc/quantrule/secrets/minio_root_user \
    /etc/quantrule/secrets/minio_root_password
  psql_host=$(descriptor_list '.stateful.haproxy.nodes' | head -n 1)
  [ -n "$psql_host" ] || psql_host=$(descriptor_list '.stateful.postgres.nodes' | head -n 1)
  attempt=1
  while :; do
    failed=""
    minio_ready "$minio_host" || failed="MinIO $minio_host (nodes/drives/healing)"
    if [ -z "$failed" ]; then
      for host in $(descriptor_list '.stateful.etcd.nodes'); do
        etcd_health "$host" || { failed="etcd $host"; break; }
      done
    fi
    if [ -z "$failed" ]; then
      for host in $(descriptor_list '.stateful.postgres.nodes'); do
        patroni_ready "$host" || { failed="Patroni $host (leader/sync)"; break; }
      done
    fi
    if [ -z "$failed" ]; then
      for host in $(rabbitmq_hosts); do
        rabbitmq_cluster_ready "$host" || { failed="RabbitMQ $host (cluster/alarms)"; break; }
      done
    fi
    if [ -z "$failed" ]; then
      primary=$(primary_psql "$psql_host" "$image" "$name" 2>/dev/null || true)
      [ "$primary" = "t" ] || failed="psql through postgres"
    fi
    if [ -z "$failed" ]; then
      printf 'stateful: ready: MinIO nodes/drives with no healing, etcd health, Patroni leader + sync standby, three RabbitMQ nodes with no alarms, primary psql through postgres\n'
      return 0
    fi
    [ "$attempt" -lt "$attempts" ] || die "not ready after $attempts attempts: $failed"
    printf 'stateful: waiting for %s (attempt %s/%s)\n' "$failed" "$attempt" "$attempts"
    attempt=$((attempt + 1))
    sleep "$interval"
  done
}

queue_bootstrap() {
  url_file=${QR_STATEFUL_QUEUE_URL:-}
  if [ -z "$url_file" ]; then
    url_file=$(jq -r '.secrets.files.queue_url // empty' "$descriptor")
  fi
  [ -n "$url_file" ] || die "queue_url is not declared; set QR_STATEFUL_QUEUE_URL"
  [ -f "$url_file" ] || die "cannot read the queue_url file $url_file"
  host=$(rabbitmq_hosts | head -n 1)
  [ -n "$host" ] || die "stateful.rabbitmq.nodes is empty"
  url=$(cat "$url_file")
  [ -n "$url" ] || die "the queue_url file is empty"
  printf 'stateful: bootstrapping the queue application user on %s\n' "$host"
  script='
set -eu
IFS= read -r url
[ -n "$url" ] || exit 2
rest=${url#amqps://}
userinfo=${rest%%@*}
hostpart=${rest#*@}
user=${userinfo%%:*}
password=${userinfo#*:}
vhost=${hostpart#*/}
vhost=${vhost%%/*}
case "$vhost" in
  %2f|%2F) vhost=/ ;;
  *) vhost=$(printf "%s" "$vhost" | sed "s/%2[fF]/\\//g") ;;
esac
[ -n "$user" ] && [ -n "$password" ] && [ -n "$vhost" ] || exit 2
rabbitmqctl -q add_vhost "$vhost" >/dev/null 2>&1 || true
hash=$(printf "%s" "$password" | rabbitmqctl -q hash_password 2>/dev/null)
[ -n "$hash" ] || { printf "stateful: cannot hash the queue password\n" >&2; exit 1; }
result=$(rabbitmqctl -q eval "case rabbit_auth_backend_internal:add_user(<<\"$user\">>, <<\"$hash\">>) of ok -> ok; {error, user_already_exists} -> rabbit_auth_backend_internal:change_password(<<\"$user\">>, <<\"$hash\">>); Error -> Error end.")
[ "$result" = "ok" ] || { printf "stateful: cannot create or update the queue user: %s\n" "$result" >&2; exit 1; }
rabbitmqctl -q set_permissions -p "$vhost" "$user" ".*" ".*" ".*" >/dev/null
printf "stateful: queue user %s is ready on vhost %s\n" "$user" "$vhost"
'
  # shellcheck disable=SC2086
  printf '%s\n' "$url" | $ssh_bin "$host" "$sudo_bin $docker_bin exec -i -u rabbitmq $(rabbitmq_container "$host") sh -c '$script'" || \
    die "cannot bootstrap the queue application user on $host"
}

object_bootstrap() {
  host=$(minio_hosts | head -n 1)
  [ -n "$host" ] || die "stateful.minio.nodes is empty"
  minio_write_config "$host" "$minio_mc_dir" \
    /etc/quantrule/secrets/minio_root_user \
    /etc/quantrule/secrets/minio_root_password
  minio_write_config "$host" "$minio_mc_app_dir" \
    /etc/quantrule/secrets/object_store_access_key \
    /etc/quantrule/secrets/object_store_secret_key
  printf 'stateful: bootstrapping the object store on %s\n' "$host"
  minio_mc "$host" "$minio_mc_dir" mb --ignore-existing "local/$minio_bucket" >/dev/null || \
    die "cannot create the $minio_bucket bucket"
  minio_mc "$host" "$minio_mc_dir" version enable "local/$minio_bucket" >/dev/null || \
    die "cannot enable versioning on $minio_bucket"
  admin_script='
set -eu
secret=$(cat /secrets/app_secret)
mc --config-dir /mc admin policy detach local __POLICY__ --user __USER__ >/dev/null 2>&1 || true
mc --config-dir /mc admin policy remove local __POLICY__ >/dev/null 2>&1 || true
mc --config-dir /mc admin policy create local __POLICY__ /policy.json >/dev/null
mc --config-dir /mc admin policy attach local __POLICY__ --user __USER__ >/dev/null
if ! printf "%s\n%s\n" __USER__ "$secret" | mc --config-dir /mc admin user add local >/dev/null 2>&1; then
  printf "stateful: %s exists; verifying the pre-provisioned secret\n" __USER__ >&2
fi
'
  admin_script=${admin_script//__POLICY__/$minio_app_policy}
  admin_script=${admin_script//__USER__/$minio_app_user}
  minio_mc_shell "$host" "$minio_mc_dir" \
    "-v /etc/quantrule/secrets/object_store_secret_key:/secrets/app_secret:ro -v $minio_policy_file:/policy.json:ro" \
    "$admin_script" || die "cannot create the $minio_app_user user or attach the $minio_app_policy policy"
  probe_script='
set -eu
mc --config-dir /mc ls local/__BUCKET__ >/dev/null
printf "bootstrap-probe\n" | mc --config-dir /mc pipe local/__BUCKET__/.bootstrap-probe >/dev/null
[ "$(mc --config-dir /mc cat local/__BUCKET__/.bootstrap-probe)" = "bootstrap-probe" ]
mc --config-dir /mc rm --force --versions local/__BUCKET__/.bootstrap-probe >/dev/null
versioning=$(mc --config-dir /mc version info local/__BUCKET__ 2>&1)
printf "%s\n" "$versioning" | grep -Eiq "enabled"
! printf "%s\n" "$versioning" | grep -Eiq "not enabled|disabled"
printf "stateful: object bootstrap verified: bucket, versioning, %s, %s\n" __USER__ __POLICY__
'
  probe_script=${probe_script//__BUCKET__/$minio_bucket}
  probe_script=${probe_script//__USER__/$minio_app_user}
  probe_script=${probe_script//__POLICY__/$minio_app_policy}
  minio_mc_shell "$host" "$minio_mc_app_dir" "" "$probe_script" || \
    die "the $minio_app_user probe failed; the existing user may have a different secret. Rotate the object_store_secret_key file, update the app secret, and rerun object-bootstrap"
  printf 'stateful: object bootstrap complete: bucket %s, versioning enabled, %s with %s, put/get/delete verified\n' \
    "$minio_bucket" "$minio_app_user" "$minio_app_policy"
}

status() {
  for host in $(all_hosts); do
    printf '\n== %s ==\n' "$host"
    # shellcheck disable=SC2086
    $ssh_bin "$host" "cd /etc/quantrule/infra && $sudo_bin $docker_bin compose -f $host.yaml ps" || true
  done
  printf '\n== MinIO ==\n'
  minio_host=$(minio_hosts | head -n 1)
  if [ -n "$minio_host" ]; then
    if minio_write_config "$minio_host" "$minio_mc_dir" \
      /etc/quantrule/secrets/minio_root_user \
      /etc/quantrule/secrets/minio_root_password 2>/dev/null; then
      minio_mc "$minio_host" "$minio_mc_dir" admin info local || true
    else
      printf 'MinIO: cannot read the root secret files on %s\n' "$minio_host"
    fi
  fi
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
  printf '\n== RabbitMQ ==\n'
  for host in $(rabbitmq_hosts); do
    printf '%s:\n' "$host"
    rabbitmq_exec "$host" rabbitmqctl cluster_status || true
    rabbitmq_exec "$host" rabbitmq-diagnostics -q alarms || true
  done
}

case "$command" in
  network) network ;;
  copy) copy ;;
  start) start ;;
  wait) wait_ready ;;
  queue-bootstrap) queue_bootstrap ;;
  object-bootstrap) object_bootstrap ;;
  status) status ;;
  *) die "unknown command $command" ;;
esac
