#!/usr/bin/env bash
# MinIO node-loss phase for the v3 `ha` lab tier.
#
# Sourced by `deploy/v3/test.sh` after the queue phase. It proves the approved
# lab layout (three nodes, two drives each, EC:3) keeps the write quorum after
# one node loss: it writes a probe through `https://minio:9000` with the `qr_app`
# credentials, records its SHA-256, stops one MinIO node with `docker stop`,
# records the reduced `mc admin info`, reads the same probe through the HAProxy
# front door, and compares the hash. It then starts the node with `docker start`,
# waits for every node/drive, runs a bounded `mc admin heal -r qr/quantrule`,
# waits for zero healing, reads the probe again, and compares the hash. The
# end-to-end digest is `Measured` only when `tools/verify.sh` succeeds;
# otherwise the phase records `Unverified` or `Blocked`. `--skip-object` leaves
# the object row `Unverified`. A missing precondition is `UNVERIFIED`, never a
# pass. The phase never removes a container, volume, stack, secret, bucket, or
# object, and the EXIT trap only starts a node it stopped.

o_stopped_host=""
o_client_host=""
o_config_dir="/etc/quantrule/minio/mc"
o_app_config_dir="/etc/quantrule/minio/mc-app"
o_ca_host="/etc/quantrule/pki/minio/CAs/ca.crt"
o_bucket="quantrule"
o_probe_key="lab/minio-node-loss-probe"
o_probe_content="quantrule lab minio node-loss probe"
o_probe_hash=""
o_probe_hash_loss=""
o_probe_hash_restore=""
o_nodes=""
o_total=0
o_max=0
o_parity=0
o_quorum=0
o_remaining=0
o_loss_host=""
o_loss_drives=0
o_drives_online_loss=0
o_drives_offline_loss=0
o_rto=0
o_client_image=""
o_app_user="qr_app"
o_sudo=${QR_SWARM_SUDO:-}
o_digest_status="Unverified"
o_digest_before=""
o_digest_after=""
o_digest_restore=""

o_say() {
  printf 'ha: object: %s\n' "$*"
}

o_fail() {
  unverified "object: $*"
}

o_hosts() {
  local value
  value=$(jq -r '.stateful.minio.nodes[]' "$descriptor") \
    || o_fail "cannot read stateful.minio.nodes"
  [ -n "$value" ] || o_fail "the descriptor declares no MinIO nodes"
  printf '%s\n' "$value"
}

o_container() {
  printf 'quantrule-minio-%s' "$1"
}

o_overlay() {
  jq -r '.network.overlay' "$descriptor" || o_fail "cannot read network.overlay"
}

o_client_image() {
  printf '%s@%s' \
    "$(jq -r '.images.minio_client.reference' "$descriptor")" \
    "$(jq -r '.images.minio_client.digest' "$descriptor")"
}

# The layout and quorum rule from the descriptor. `auto` is total / 2; an
# explicit EC:N must keep the write quorum after the largest node is lost and
# must not leave fewer data drives than parity drives.
o_layout() {
  o_nodes=$(o_hosts)
  local count
  count=$(printf '%s\n' "$o_nodes" | wc -l)
  [ "$count" -eq 3 ] || o_fail "the node-loss phase needs exactly three MinIO nodes; found $count"

  o_total=0
  o_max=0
  local node drives
  while IFS=$'\t' read -r node drives; do
    [ -n "$node" ] || continue
    case "$drives" in
      '' | *[!0-9]*) o_fail "stateful.minio.drives has a non-integer entry for $node" ;;
    esac
    [ "$drives" -gt 0 ] || o_fail "stateful.minio.drives has zero drives for $node"
    o_total=$((o_total + drives))
    [ "$drives" -gt "$o_max" ] && o_max=$drives
  done <<< "$(jq -r '
    .stateful.minio.nodes as $nodes
    | .stateful.minio.drives as $drives
    | range(0; $nodes | length) | [$nodes[.], $drives[.]] | @tsv
  ' "$descriptor")"

  local declared
  declared=$(jq -r '.stateful.minio.parity' "$descriptor") \
    || o_fail "cannot read stateful.minio.parity"
  case "$declared" in
    auto) o_parity=$((o_total / 2)) ;;
    EC:*)
      o_parity=${declared#EC:}
      case "$o_parity" in
        '' | *[!0-9]*) o_fail "stateful.minio.parity $declared is not EC:<n>" ;;
      esac
      ;;
    *) o_fail "stateful.minio.parity $declared is neither auto nor EC:<n>" ;;
  esac
  [ "$o_parity" -gt 0 ] || o_fail "EC:$o_parity has no parity drives"
  [ $((o_parity * 2)) -le "$o_total" ] \
    || o_fail "EC:$o_parity needs at least $((o_parity * 2)) drives; found $o_total"
  [ "$o_parity" -gt "$o_max" ] \
    || o_fail "EC:$o_parity does not keep the write quorum after one node loss; a node with $o_max drives needs parity greater than $o_max"

  o_quorum=$((o_total - o_parity + 1))
  o_remaining=$((o_total - o_max))
  [ "$o_remaining" -ge "$o_quorum" ] \
    || o_fail "one node loss leaves $o_remaining drives; the write quorum is $o_quorum"
  [ $((o_total - o_parity)) -ge "$o_parity" ] \
    || o_fail "EC:$o_parity needs data drives at least parity drives"

  # Prefer the largest node that still leaves the write quorum, so the lab
  # exercises the worst safe loss.
  local best=-1
  while IFS=$'\t' read -r node drives; do
    [ -n "$node" ] || continue
    if [ $((o_total - drives)) -ge "$o_quorum" ] && [ "$drives" -gt "$best" ]; then
      best=$drives
      o_loss_host=$node
      o_loss_drives=$drives
    fi
  done <<< "$(jq -r '
    .stateful.minio.nodes as $nodes
    | .stateful.minio.drives as $drives
    | range(0; $nodes | length) | [$nodes[.], $drives[.]] | @tsv
  ' "$descriptor")"
  [ -n "$o_loss_host" ] \
    || o_fail "no single MinIO node loss leaves the write quorum"
  o_say "layout: $o_total drives, EC:$o_parity, write quorum $o_quorum; loss target $o_loss_host ($o_loss_drives drives)"
}

o_client_environment() {
  o_client_image=$(o_client_image) || o_fail "cannot read images.minio_client"
  [ -n "$o_client_image" ] || o_fail "images.minio_client is missing"
  command -v sha256sum >/dev/null 2>&1 \
    || o_fail "sha256sum is required to hash the probe object"
  if [ -n "${QR_SWARM_MC:-}" ]; then
    [ -x "$QR_SWARM_MC" ] \
      || o_fail "the mc client $QR_SWARM_MC is not executable"
  fi
}
# Write an mc config from two mounted secret files. The JSON is built on the
# host at mode 600; the secret values never enter the operator's command line,
# the SSH command line, or a log.
o_write_config() {
  local host=$1 directory=$2 user_file=$3 secret_file=$4 alias=$5
  local address endpoint script
  address=$(host_address "$host")
  endpoint=$(jq -r '.endpoints.object_store.scheme + "://" + .endpoints.object_store.host + ":" + (.endpoints.object_store.port | tostring)' "$descriptor")
  script='
set -eu
umask 077
directory=$1
user_file=$2
secret_file=$3
endpoint=$4
alias=$5
mkdir -p "$directory"
chmod 700 "$directory"
user=$(cat "$user_file")
secret=$(cat "$secret_file")
printf "{\"version\":\"10\",\"aliases\":{\"%s\":{\"url\":\"%s\",\"accessKey\":\"%s\",\"secretKey\":\"%s\",\"api\":\"S3v4\",\"path\":\"auto\"}}}\n" \
  "$alias" "$endpoint" "$user" "$secret" > "$directory/config.json"
chmod 600 "$directory/config.json"
'
  # shellcheck disable=SC2086
  printf '%s\n' "$script" | $ssh_bin "$address" "$o_sudo sh -s -- '$directory' '$user_file' '$secret_file' '$endpoint' '$alias'" \
    || o_fail "cannot write the MinIO client config on $host"
}

# Run the pinned mc client on the overlay with the private CA. A host mc binary
# is accepted only through QR_SWARM_MC; the default is the pinned image.
o_mc() {
  local host=$1 directory=$2
  shift 2
  if [ -n "${QR_SWARM_MC:-}" ]; then
    "$QR_SWARM_MC" --config-dir "$directory" "$@"
    return
  fi
  local image overlay address
  image=$o_client_image
  overlay=$(o_overlay)
  address=$(host_address "$host")
  # shellcheck disable=SC2086
  $ssh_bin "$address" "$o_sudo $docker_bin run --rm --network $overlay \
    -v $directory:/mc:ro -v $o_ca_host:/certs/ca.crt:ro -e SSL_CERT_FILE=/certs/ca.crt \
    $image --config-dir /mc $*"
}

# The same container with stdin piped to mc (for `mc pipe`).
o_mc_stdin() {
  local host=$1 directory=$2
  shift 2
  if [ -n "${QR_SWARM_MC:-}" ]; then
    "$QR_SWARM_MC" --config-dir "$directory" "$@"
    return
  fi
  local image overlay address
  image=$o_client_image
  overlay=$(o_overlay)
  address=$(host_address "$host")
  # shellcheck disable=SC2086
  $ssh_bin "$address" "$o_sudo $docker_bin run --rm -i --network $overlay \
    -v $directory:/mc:ro -v $o_ca_host:/certs/ca.crt:ro -e SSL_CERT_FILE=/certs/ca.crt \
    $image --config-dir /mc $*"
}

o_admin_info() {
  o_mc "$o_client_host" "$o_config_dir" admin info qr
}

# One count from the `mc admin info` summary (`N drives online|offline|healing`).
o_drive_count() {
  local text=$1 state=$2
  printf '%s\n' "$text" \
    | sed -n "s/.* \([0-9][0-9]*\) drives $state.*/\1/p" \
    | tail -n 1
}

o_ready() {
  local online=$1 offline=$2 healing=$3 info node
  info=$(o_admin_info 2>/dev/null) || return 1
  for node in $o_nodes; do
    printf '%s\n' "$info" | grep -q "minio-$node:9000" || return 1
  done
  [ "$(o_drive_count "$info" online)" = "$online" ] || return 1
  [ "$(o_drive_count "$info" offline)" = "$offline" ] || return 1
  [ "$(o_drive_count "$info" healing)" = "$healing" ] || return 1
  return 0
}

o_all_drives_online() {
  local info
  info=$(o_admin_info 2>/dev/null) || return 1
  [ "$(o_drive_count "$info" online)" = "$o_total" ] || return 1
  [ "$(o_drive_count "$info" offline)" = "0" ] || return 1
  return 0
}

o_wait_ready() {
  local online=$1 offline=$2 healing=$3 seconds=$4 deadline
  deadline=$(( $(date +%s) + seconds ))
  while [ "$(date +%s)" -lt "$deadline" ]; do
    if o_ready "$online" "$offline" "$healing"; then
      return 0
    fi
    sleep 2
  done
  return 1
}

o_wait_all_online() {
  local seconds=$1 deadline
  deadline=$(( $(date +%s) + seconds ))
  while [ "$(date +%s)" -lt "$deadline" ]; do
    if o_all_drives_online; then
      return 0
    fi
    sleep 2
  done
  return 1
}

o_bucket_check() {
  o_mc "$o_client_host" "$o_app_config_dir" ls "qr/$o_bucket" >/dev/null 2>&1 \
    || o_fail "the $o_bucket bucket is not readable with the $o_app_user credentials"
  local versioning
  versioning=$(o_mc "$o_client_host" "$o_app_config_dir" version info "qr/$o_bucket" 2>&1) \
    || o_fail "cannot read the bucket versioning state"
  printf '%s\n' "$versioning" | grep -Eiq 'enabled' \
    || o_fail "versioning is not enabled on $o_bucket"
  printf '%s\n' "$versioning" | grep -Eiq 'not enabled|disabled' \
    && o_fail "versioning is disabled on $o_bucket"
  return 0
}

# The app path is checked only through the public ingress, not a node. A
# transient reconnect is retried before the phase fails.
o_app_health() {
  local phase=$1 attempt body
  for attempt in $(seq 1 30); do
    for path in health ready; do
      body=$(curl_ingress -fsS --max-time 10 "https://$dns/$path" 2>/dev/null || true)
      if ! printf '%s' "$body" | grep -Fq '"state":"healthy"'; then
        body=""
        break
      fi
    done
    if [ -n "$body" ]; then
      return 0
    fi
    sleep 2
  done
  o_fail "$phase: the app /health or /ready is not healthy"
}

o_preflight() {
  [ "$profile" = "lab-single-host" ] \
    || o_fail "the MinIO node-loss phase is lab-only; use a lab-single-host descriptor or --skip-object"
  o_layout
  o_client_environment

  local host
  for host in $o_nodes; do
    # shellcheck disable=SC2086
    $ssh_bin "$(host_address "$host")" true >/dev/null 2>&1 \
      || o_fail "cannot SSH to $host ($(host_address "$host")); set QR_SWARM_SSH"
    local running
    running=$(remote "$host" "$docker_bin inspect --format '{{.State.Running}}' $(o_container "$host")" 2>/dev/null || true)
    [ "$running" = "true" ] || o_fail "MinIO $(o_container "$host") is not running on $host"
    remote "$host" "test -r $o_ca_host" >/dev/null 2>&1 \
      || o_fail "the MinIO CA $o_ca_host is not readable on $host"
  done
  o_client_host=$(printf '%s\n' "$o_nodes" | head -n 1)
  [ -n "$o_client_host" ] || o_fail "no MinIO client host"
  o_write_config "$o_client_host" "$o_config_dir" \
    /etc/quantrule/secrets/minio_root_user \
    /etc/quantrule/secrets/minio_root_password qr
  o_write_config "$o_client_host" "$o_app_config_dir" \
    /etc/quantrule/secrets/object_store_access_key \
    /etc/quantrule/secrets/object_store_secret_key qr
  o_wait_ready "$o_total" 0 0 60 \
    || o_fail "mc admin info does not report $o_total online drives, 0 offline, and 0 healing on every node"
  o_bucket_check
  o_app_health "object preflight"
  o_say "preflight: $o_total drives online, EC:$o_parity, write quorum $o_quorum; bucket/versioning and app health verified"
}

o_probe_read() {
  local hash
  hash=$(o_mc "$o_client_host" "$o_app_config_dir" cat "qr/$o_bucket/$o_probe_key" 2>/dev/null \
    | sha256sum | awk '{print $1}')
  [ -n "$hash" ] || return 1
  printf '%s' "$hash"
}

o_probe_read_retry() {
  local attempt hash
  for attempt in $(seq 1 30); do
    if hash=$(o_probe_read); then
      printf '%s' "$hash"
      return 0
    fi
    sleep 2
  done
  return 1
}

o_verify() {
  local phase=$1 tag=$2 output digest args
  if [ ! -x "$verify" ]; then
    o_digest_status="Unverified"
    o_say "$phase: verification script $verify is not executable; end-to-end digest Unverified"
    return 0
  fi
  args=(--api-url "$api_url" --timeout "$verify_timeout")
  [ -n "$cacert" ] && args+=(--cacert "$cacert")
  if output=$("$verify" "${args[@]}" 2>&1); then
    digest=$(printf '%s\n' "$output" | sed -n 's/^canonical digest sha256:\([0-9a-f]*\)$/\1/p')
    if [ -n "$digest" ]; then
      o_digest_status="Measured"
      case "$tag" in
        baseline) o_digest_before=$digest ;;
        loss) o_digest_after=$digest ;;
        restore) o_digest_restore=$digest ;;
      esac
      o_say "$phase: canonical digest sha256:$digest"
    else
      o_digest_status="Unverified"
      o_say "$phase: the verification run printed no digest; end-to-end Unverified"
    fi
  else
    o_digest_status="Blocked"
    o_say "$phase: tools/verify.sh failed; end-to-end digest Blocked (the object evidence stays object-level)"
  fi
}

o_baseline() {
  o_probe_hash=$(printf '%s' "$o_probe_content" | sha256sum | awk '{print $1}')
  printf '%s' "$o_probe_content" | o_mc_stdin "$o_client_host" "$o_app_config_dir" \
    pipe "qr/$o_bucket/$o_probe_key" >/dev/null \
    || o_fail "cannot write the probe through https://minio:9000"
  local read_back
  read_back=$(o_probe_read_retry) \
    || o_fail "cannot read the probe back through https://minio:9000"
  [ "$read_back" = "$o_probe_hash" ] \
    || o_fail "the probe hash changed immediately after the write"
  o_say "baseline: wrote qr/$o_bucket/$o_probe_key through https://minio:9000; sha256:$o_probe_hash read back"
  o_verify "object baseline" baseline
}

o_loss() {
  local host=$o_loss_host start deadline ready info
  [ -n "$host" ] || o_fail "no MinIO loss target"
  o_stopped_host=$host
  remote "$host" "$docker_bin stop $(o_container "$host")" >/dev/null \
    || o_fail "cannot stop the MinIO node on $host"
  o_say "loss: stopped $host ($(o_container "$host"))"
  o_drives_online_loss=$((o_total - o_loss_drives))
  o_drives_offline_loss=$o_loss_drives
  start=$(date +%s)
  deadline=$((start + 120))
  ready=0
  while [ "$(date +%s)" -lt "$deadline" ]; do
    if o_ready "$o_drives_online_loss" "$o_drives_offline_loss" 0; then
      o_rto=$(( $(date +%s) - start ))
      ready=1
      break
    fi
    sleep 1
  done
  [ "$ready" -eq 1 ] \
    || o_fail "mc admin info did not report $o_drives_online_loss online, $o_drives_offline_loss offline, and 0 healing within 120 s"
  info=$(o_admin_info 2>/dev/null || true)
  o_say "loss: mc admin info reports $(o_drive_count "$info" online) online, $(o_drive_count "$info" offline) offline, $(o_drive_count "$info" healing) healing; measured gap ${o_rto}s"
  o_probe_hash_loss=$(o_probe_read_retry) \
    || o_fail "the probe is not readable through https://minio:9000 after one node loss"
  [ "$o_probe_hash_loss" = "$o_probe_hash" ] \
    || o_fail "the probe hash changed after one node loss"
  o_say "loss: probe sha256:$o_probe_hash_loss read through minio:9000 matches the baseline"
  o_app_health "after object node loss"
  o_verify "after object node loss" loss
  if [ -n "$o_digest_before" ] && [ -n "$o_digest_after" ] \
    && [ "$o_digest_before" != "$o_digest_after" ]; then
    o_fail "the canonical digest changed after one MinIO node loss"
  fi
}

o_heal() {
  local pid deadline
  o_say "restore: bounded mc admin heal -r qr/$o_bucket"
  o_mc "$o_client_host" "$o_config_dir" admin heal -r "qr/$o_bucket" >/dev/null 2>&1 &
  pid=$!
  deadline=$(( $(date +%s) + 600 ))
  while kill -0 "$pid" 2>/dev/null; do
    if [ "$(date +%s)" -ge "$deadline" ]; then
      o_say "restore: heal still running after 600 s; polling the healing count"
      kill "$pid" 2>/dev/null || true
      break
    fi
    sleep 2
  done
  wait "$pid" 2>/dev/null || true
}

o_restore() {
  local host=$o_stopped_host read_back
  [ -n "$host" ] || o_fail "no stopped MinIO node to restore"
  remote "$host" "$docker_bin start $(o_container "$host")" >/dev/null \
    || o_fail "cannot start the MinIO node on $host"
  o_stopped_host=""
  o_say "restore: starting $host"
  o_wait_all_online 240 \
    || o_fail "the restored node did not report all $o_total drives online within 240 s"
  o_heal
  o_wait_ready "$o_total" 0 0 600 \
    || o_fail "healing did not reach zero within 600 s"
  read_back=$(o_probe_read_retry) \
    || o_fail "the probe is not readable after the node restore"
  [ "$read_back" = "$o_probe_hash" ] \
    || o_fail "the probe hash changed after the node restore"
  o_probe_hash_restore=$read_back
  o_say "restore: $o_total drives online, 0 healing; probe sha256:$o_probe_hash_restore matches"
  o_app_health "after object node restore"
  o_verify "after object node restore" restore
  if [ -n "$o_digest_before" ] && [ -n "$o_digest_restore" ] \
    && [ "$o_digest_before" != "$o_digest_restore" ]; then
    o_fail "the canonical digest changed across the MinIO restore"
  fi
  stateful_check_all
  stateful_no_move
  if [ -f "$out/stateful-manifest.json" ]; then
    qr_deploy check "$descriptor" --remote docker --overlay --out "$out" \
      || o_fail "the full remote/overlay check failed after the MinIO restore"
  else
    qr_deploy check "$descriptor" --remote docker --overlay \
      || o_fail "the full remote/overlay check failed after the MinIO restore"
  fi
}

o_evidence() {
  local base path status nodes drives
  status=$o_digest_status
  case "$status" in
    Measured) status="Measured" ;;
    Blocked) status="Blocked" ;;
    *) status="Unverified" ;;
  esac
  base=${QR_SWARM_EVIDENCE:-deploy/v3/evidence/$(date -u +%Y-%m-%dT%H%M%SZ).json}
  case "$base" in
    *.json) path="${base%.json}.minio.json" ;;
    *) path="$base.minio.json" ;;
  esac
  mkdir -p "$(dirname "$path")"
  nodes=$(jq -c '[.stateful.minio.nodes[]]' "$descriptor")
  drives=$(jq -c '[.stateful.minio.drives[]]' "$descriptor")
  jq -n \
    --arg evidence "Simulated HA" \
    --arg profile "$profile" \
    --arg recorded_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --argjson nodes "$nodes" \
    --argjson drives "$drives" \
    --argjson total "$o_total" \
    --arg parity "EC:$o_parity" \
    --argjson quorum "$o_quorum" \
    --argjson remaining "$o_remaining" \
    --argjson max "$o_max" \
    --arg stopped "$o_loss_host" \
    --argjson online "$o_drives_online_loss" \
    --argjson offline "$o_drives_offline_loss" \
    --argjson rto "$o_rto" \
    --arg bucket "$o_bucket" \
    --arg key "$o_probe_key" \
    --arg hash "$o_probe_hash" \
    --arg hash_loss "$o_probe_hash_loss" \
    --arg hash_restore "$o_probe_hash_restore" \
    --arg status "$status" \
    --arg before "$o_digest_before" \
    --arg after "$o_digest_after" \
    --arg restore "$o_digest_restore" \
    '{
      evidence: $evidence,
      scope: "object_store",
      profile: $profile,
      recorded_at: $recorded_at,
      layout: {
        nodes: $nodes,
        drives: $drives,
        total_drives: $total,
        parity: $parity,
        write_quorum: $quorum,
        drives_after_node_loss: $remaining,
        max_node_drives: $max
      },
      node_loss: {
        stopped: $stopped,
        drives_online: $online,
        drives_offline: $offline,
        drives_healing: 0,
        write_quorum_survived: true,
        rto_seconds: $rto
      },
      probe: {
        bucket: $bucket,
        key: $key,
        sha256: $hash,
        read_after_loss: $hash_loss,
        read_after_restore: $hash_restore,
        hash_unchanged: true
      },
      heal: {
        command: "mc admin heal -r qr/quantrule",
        bounded_seconds: 600,
        drives_healing_after: 0
      },
      app: {health: "healthy", ready: "healthy", path: "https://<dns>/health and /ready"},
      digests: {
        status: $status,
        before: (if $before == "" then "Unverified" else ("sha256:" + $before) end),
        after_loss: (if $after == "" then "Unverified" else ("sha256:" + $after) end),
        after_restore: (if $restore == "" then "Unverified" else ("sha256:" + $restore) end)
      },
      claims: {
        object_store: "Simulated HA",
        end_to_end: $status,
        real_ha: "HA-014 only"
      }
    }' > "$path"
  o_say "evidence: $path (object store Simulated HA; end-to-end $status)"
}

o_cleanup() {
  if [ -n "$o_stopped_host" ]; then
    remote "$o_stopped_host" "$docker_bin start $(o_container "$o_stopped_host")" >/dev/null 2>&1 || true
    o_stopped_host=""
  fi
}

object_failover() {
  trap o_cleanup EXIT
  o_preflight
  o_baseline
  o_loss
  o_restore
  o_evidence
  o_cleanup
  trap - EXIT
  o_say "result: Simulated HA (object store); end-to-end digest $o_digest_status; Real HA is HA-014"
}
