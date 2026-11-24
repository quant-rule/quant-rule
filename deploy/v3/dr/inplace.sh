#!/usr/bin/env bash
# In-place DR for the disposable lab only. Sourced by dr.sh.
#
# It stops the app services and every Patroni node, restores all three lab
# PGDATA directories to one target with `pgbackrest restore --delta`, restores
# the objects before rows, rebuilds the queue user, starts Patroni and the app,
# and leaves the result running for the digest. It never runs on a `swarm-ha` or
# `single-az` descriptor, never removes a stack/volume/secret/config/service,
# and never runs `DROP DATABASE`, `DROP SCHEMA`, or `rm -rf`.

dr_inplace_preflight() {
  [ "$DR_CONFIRM" -eq 1 ] || dr_die "--target in-place is destructive; pass --confirm"
  [ "$DR_PROFILE" = "lab-single-host" ] \
    || dr_unverified "the in-place lab restore requires lab-single-host; $DR_PROFILE is not disposable"
  DR_NODES=$(jq -r '.stateful.postgres.nodes[]' "$DR_DESCRIPTOR")
  [ -n "$DR_NODES" ] || dr_unverified "the descriptor declares no Patroni nodes"
  DR_BACKUP_SSH=${QR_DR_BACKUP_SSH:-ssh -o BatchMode=yes -o ConnectTimeout=5}
  DR_BACKUP_SUDO=${QR_DR_BACKUP_SUDO-sudo -n}
  DR_NODE_ADDR=$(jq -r --arg node "$DR_NODE" '.hosts[$node].address' "$DR_DESCRIPTOR")
  [ -n "$DR_NODE_ADDR" ] && [ "$DR_NODE_ADDR" != "null" ] \
    || dr_unverified "the Patroni node $DR_NODE has no declared address"
  command -v ssh >/dev/null 2>&1 || dr_unverified "SSH is required for the in-place lab restore"
  command -v docker >/dev/null 2>&1 || dr_unverified "Docker is required for the in-place lab restore"
  dr_qr_deploy validate "$DR_DESCRIPTOR" --release >/dev/null 2>&1 \
    || dr_unverified "qr-deploy validate --release refused $DR_DESCRIPTOR; resolve the app, MinIO, and derived Patroni image blockers"
  if ! "$DR_BACKUP_SSH" "$DR_NODE_ADDR" true >/dev/null 2>&1; then
    dr_unverified "cannot SSH to $DR_NODE ($DR_NODE_ADDR); set QR_DR_BACKUP_SSH"
  fi
  local node address
  while IFS= read -r node; do
    [ -n "$node" ] || continue
    address=$(jq -r --arg node "$node" '.hosts[$node].address' "$DR_DESCRIPTOR")
    [ -n "$address" ] && [ "$address" != "null" ] \
      || dr_unverified "the Patroni node $node has no declared address"
    "$DR_BACKUP_SSH" "$address" true >/dev/null 2>&1 \
      || dr_unverified "cannot SSH to $node ($address); no app or Patroni service was stopped"
  done <<< "$DR_NODES"
  DR_IN_PLACE_REPLICAS=0
  : > "$DR_WORK/replicas.txt"
  DR_IN_PLACE_MOUNTS=""
  if [ "$DR_REPO_KIND" = "posix" ]; then
    DR_IN_PLACE_MOUNTS="-v $DR_REPO_PATH:$DR_REPO_PATH"
  elif [ -n "$DR_REPO_CA" ] && [ "$DR_REPO_CA" = "/etc/quantrule/pki/ca.crt" ]; then
    DR_IN_PLACE_MOUNTS="-v /etc/quantrule/pki/ca.crt:/etc/quantrule/pki/ca.crt:ro"
  elif [ -n "$DR_REPO_CA" ]; then
    DR_IN_PLACE_MOUNTS="-v $DR_REPO_CA:$DR_REPO_CA:ro"
  fi
  dr_fact "in_place_nodes" "$(printf '%s\n' "$DR_NODES" | jq -R . | jq -s .)"
  dr_fact_num "in_place_nodes_count" "$(printf '%s\n' "$DR_NODES" | wc -l | tr -d ' ')"
}

dr_node_run() {
  local node=$1 address=$2
  shift 2
  "$DR_BACKUP_SSH" "$address" "$DR_BACKUP_SUDO $*"
}

dr_inplace_stop_apps() {
  local service replicas stopped=0
  dr_say "in-place: stopping the seven app services"
  for service in gateway data model strategy backtest evidence simulation; do
    replicas=$(jq -r --arg service "$service" '.services[$service].replicas' "$DR_DESCRIPTOR")
    if dr_node_run "$DR_NODE" "$DR_NODE_ADDR" "docker service inspect ${DR_STACK}_${service}" >/dev/null 2>&1; then
      dr_node_run "$DR_NODE" "$DR_NODE_ADDR" "docker service scale ${DR_STACK}_${service}=0" >/dev/null \
        || dr_unverified "cannot scale ${DR_STACK}_${service} to zero"
      printf '%s:%s\n' "$service" "$replicas" >> "$DR_WORK/replicas.txt"
      stopped=$((stopped + 1))
    fi
  done
  [ "$stopped" -gt 0 ] || dr_unverified "no live ${DR_STACK} app services were found"
  dr_fact "app_services_stopped" "true"
  dr_fact_num "app_services_stopped_count" "$stopped"
}

dr_inplace_stop_patroni() {
  local node=$1 address=$2
  dr_node_run "$node" "$address" "docker stop quantrule-patroni-$node" >/dev/null \
    || dr_unverified "cannot stop Patroni on $node"
}

dr_inplace_restore_node() {
  local node=$1 address=$2 env_file=$3
  local remote_env=/run/quantrule-dr-pgbackrest.env
  dr_say "in-place: restoring PGDATA on $node"
  # The secret env file travels over SSH standard input and lives under /run
  # (tmpfs) for the one-shot. It is removed immediately after the restore.
  "$DR_BACKUP_SSH" "$address" "$DR_BACKUP_SUDO sh -c 'umask 077; cat > $remote_env && chmod 600 $remote_env'" < "$env_file" \
    || dr_unverified "cannot stage the pgBackRest environment on $node"
  local args="restore --delta"
  case "$DR_TARGET_TYPE" in
    lsn) args="$args --type=lsn --target=$DR_TARGET_LSN --target-action=promote" ;;
    time) args="$args --type=time --target=$DR_TARGET_TIME --target-action=promote" ;;
    *) dr_unverified "unsupported target type $DR_TARGET_TYPE" ;;
  esac
  if ! dr_node_run "$node" "$address" \
    "docker run --rm --env-file $remote_env -v /srv/quantrule/patroni/data:/var/lib/postgresql/data -v /etc/pgbackrest/pgbackrest.conf:/etc/pgbackrest/pgbackrest.conf:ro $DR_IN_PLACE_MOUNTS --entrypoint pgbackrest $DR_PATRONI_IMAGE --stanza=quantrule $args"; then
    dr_node_run "$node" "$address" "rm -f $remote_env" >/dev/null 2>&1 || true
    dr_unverified "pgBackRest restore failed on $node"
  fi
  dr_node_run "$node" "$address" "rm -f $remote_env" >/dev/null 2>&1 || true
}

dr_inplace_restore() {
  local env_file="$DR_WORK/pgbackrest.env"
  dr_pgbackrest_env "$env_file"
  local start end node address
  start=$(dr_now_epoch)
  while IFS= read -r node; do
    [ -n "$node" ] || continue
    address=$(jq -r --arg node "$node" '.hosts[$node].address' "$DR_DESCRIPTOR")
    [ -n "$address" ] && [ "$address" != "null" ] \
      || dr_unverified "the Patroni node $node has no declared address"
    dr_inplace_restore_node "$node" "$address" "$env_file"
  done <<< "$DR_NODES"
  end=$(dr_now_epoch)
  DR_RESTORE_SECONDS=$(dr_elapsed "$start" "$end")
  dr_fact_num "restore_seconds" "$DR_RESTORE_SECONDS"
  dr_fact "restore_nodes" "$(printf '%s\n' "$DR_NODES" | wc -l | tr -d ' ')"
}

dr_inplace_start_patroni() {
  local node address first=1
  dr_say "in-place: starting Patroni nodes"
  while IFS= read -r node; do
    [ -n "$node" ] || continue
    address=$(jq -r --arg node "$node" '.hosts[$node].address' "$DR_DESCRIPTOR")
    dr_node_run "$node" "$address" "docker start quantrule-patroni-$node" >/dev/null \
      || dr_unverified "cannot start Patroni on $node"
    [ "$first" -eq 1 ] && sleep 10
    first=0
  done <<< "$DR_NODES"
  local attempt=0 output
  while [ "$attempt" -lt 90 ]; do
    attempt=$((attempt + 1))
    output=$("$DR_BACKUP_SSH" "$DR_NODE_ADDR" \
      "$DR_BACKUP_SUDO docker exec quantrule-patroni-$DR_NODE sh -c 'PATRONI_CTL_PASSWORD=\$(cat /run/secrets/patroni_rest_password) patronictl -c /etc/patroni/patroni.yml list'" 2>/dev/null || true)
    if printf '%s\n' "$output" | grep -q 'Leader' \
      && printf '%s\n' "$output" | grep -q 'Sync'; then
      return 0
    fi
    sleep 2
  done
  dr_unverified "Patroni did not report a leader and a synchronous standby after the restore"
}

dr_inplace_objects() {
  if [ "$DR_SKIP_OBJECTS" -eq 1 ]; then
    dr_fact "object_status" "Unverified"
    DR_ROWS="${DR_ROWS}object=Unverified
"
    dr_say "objects: skipped (--skip-objects); row Unverified"
    return 0
  fi
  [ -n "$DR_ARCHIVE_TARGET" ] || dr_unverified "backup.object_archive.target is empty; no archive to restore"
  local source_url=${QR_DR_ARCHIVE_SOURCE_URL:-https://minio:9000}
  QR_OBJECT_ARCHIVE_SOURCE_URL="$source_url" \
  QR_OBJECT_ARCHIVE_SOURCE_BUCKET="$DR_ARCHIVE_TARGET" \
  QR_OBJECT_ARCHIVE_SOURCE_ACCESS_KEY_FILE="$DR_RENDER/secrets/minio_root_user" \
  QR_OBJECT_ARCHIVE_SOURCE_SECRET_KEY_FILE="$DR_RENDER/secrets/minio_root_password" \
  QR_OBJECT_ARCHIVE_TARGET_URL="https://minio:9000" \
  QR_OBJECT_ARCHIVE_TARGET_BUCKET="quantrule" \
  QR_OBJECT_ARCHIVE_TARGET_ACCESS_KEY_FILE="$DR_RENDER/secrets/minio_root_user" \
  QR_OBJECT_ARCHIVE_TARGET_SECRET_KEY_FILE="$DR_RENDER/secrets/minio_root_password" \
  QR_OBJECT_ARCHIVE_IMAGE="$DR_MINIO_CLIENT_IMAGE" \
  QR_OBJECT_ARCHIVE_NETWORK="$DR_OVERLAY" \
  QR_OBJECT_ARCHIVE_CA="$DR_OBJECT_CA" \
  QR_OBJECT_ARCHIVE_EVIDENCE="$DR_WORK/object-restore.json" \
    ./tools/object-archive.sh --restore --confirm \
    || dr_unverified "the archived objects were not restored into quantrule"
  dr_fact "object_status" "Measured"
  DR_ROWS="${DR_ROWS}object=Measured
"
  if [ -f "$DR_WORK/object-restore.json" ]; then
    dr_fact_num "object_count" "$(jq -r '.objects.count // 0' "$DR_WORK/object-restore.json")"
    dr_fact_num "object_bytes" "$(jq -r '.objects.bytes // 0' "$DR_WORK/object-restore.json")"
    dr_fact "object_manifest_sha256" "$(jq -r '.manifest.sha256 // ""' "$DR_WORK/object-restore.json")"
  fi
}

dr_inplace_queue() {
  if [ "$DR_SKIP_QUEUE" -eq 1 ]; then
    dr_fact "queue_status" "Unverified"
    DR_ROWS="${DR_ROWS}queue=Unverified
"
    dr_say "queue: skipped (--skip-queue); row Unverified"
    return 0
  fi
  DESCRIPTOR="$DR_DESCRIPTOR" OUT="$DR_RENDER" QR_STATEFUL_QUEUE_URL="${QR_STATEFUL_QUEUE_URL:-$DR_RENDER/secrets/queue_url}" \
    ./tools/stateful.sh queue-bootstrap \
    || dr_unverified "cannot recreate the queue application user"
  dr_fact "queue_user" "ready"
  dr_say "queue: application user ready; the evidence outbox re-arms after the service starts"
}

dr_inplace_queue_counts() {
  if [ "$DR_SKIP_QUEUE" -eq 1 ]; then
    return 0
  fi
  local logs rearmed runs stages
  logs=$(dr_node_run "$DR_NODE" "$DR_NODE_ADDR" \
    "docker service logs --no-trunc --tail 2000 ${DR_STACK}_evidence" 2>&1 || true)
  rearmed=$(printf '%s\n' "$logs" | sed -n 's/.*rearmed["]*[=:][[:space:]]*\([0-9][0-9]*\).*/\1/p' | awk '{ sum += $1 } END { print sum + 0 }')
  runs=$(printf '%s\n' "$logs" | grep -c 'run notifications reconciled' || true)
  stages=$(printf '%s\n' "$logs" | grep -c 'stage notifications reconciled' || true)
  if [ "$runs" -eq 0 ] && [ "$stages" -eq 0 ]; then
    dr_fact "queue_status" "Unverified"
    dr_fact_num "queue_rearmed" -1
    dr_fact_num "queue_replayed_runs" -1
    dr_fact_num "queue_replayed_stages" -1
    DR_ROWS="${DR_ROWS}queue=Unverified
"
    dr_say "queue: no re-arm log line observed; row Unverified"
    return 0
  fi
  dr_fact "queue_status" "Measured"
  dr_fact_num "queue_rearmed" "$rearmed"
  dr_fact_num "queue_replayed_runs" "$runs"
  dr_fact_num "queue_replayed_stages" "$stages"
  DR_ROWS="${DR_ROWS}queue=Measured
"
  dr_say "queue: run reconcile lines $runs, stage reconcile lines $stages, rearmed $rearmed"
}

dr_inplace_start_apps() {
  local start=$1 service replicas
  while IFS=: read -r service replicas; do
    [ -n "$service" ] || continue
    dr_node_run "$DR_NODE" "$DR_NODE_ADDR" "docker service scale ${DR_STACK}_${service}=${replicas}" >/dev/null \
      || dr_unverified "cannot scale ${DR_STACK}_${service} to $replicas"
  done < "$DR_WORK/replicas.txt"
  local attempt=0
  while [ "$attempt" -lt 90 ]; do
    attempt=$((attempt + 1))
    if curl --cacert "$DR_CACERT" -fsS --max-time 2 "https://$DR_INGRESS_DNS/health" 2>/dev/null \
      | grep -q '"state":"healthy"'; then
      dr_startup_timing "$start"
      return 0
    fi
    sleep 2
  done
  dr_unverified "the lab ingress did not become healthy after the restore"
}

dr_inplace_run() {
  dr_inplace_preflight
  local start
  DR_MINIO_CLIENT_IMAGE=$(jq -r '.images.minio_client.reference + "@" + .images.minio_client.digest' "$DR_DESCRIPTOR")
  DR_CACERT=${QR_DR_CACERT:-$DR_DATABASE_CA}
  [ -r "$DR_CACERT" ] || dr_unverified "the ingress CA $DR_CACERT is not readable"
  DR_API_URL=${QR_DR_API_URL:-https://$DR_INGRESS_DNS/api/v1}
  dr_inplace_stop_apps
  local node address
  while IFS= read -r node; do
    [ -n "$node" ] || continue
    address=$(jq -r --arg node "$node" '.hosts[$node].address' "$DR_DESCRIPTOR")
    dr_inplace_stop_patroni "$node" "$address"
  done <<< "$DR_NODES"
  dr_inplace_objects
  dr_inplace_restore
  dr_inplace_start_patroni
  dr_inplace_queue
  start=$(dr_now_epoch)
  dr_inplace_start_apps "$start"
  dr_inplace_queue_counts
  DR_ROWS="${DR_ROWS}restore=Measured
startup=Measured
"
}
