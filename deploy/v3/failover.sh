#!/usr/bin/env bash
# Database failover phase for the v3 `ha` lab tier.
#
# Sourced by `deploy/v3/test.sh` after its preflight. It records the Patroni
# leader and synchronous standby, writes a marker through the new primary,
# pauses the leader's Patroni container with the watchdog off, waits for a new
# leader and sync standby, proves a write through the `postgres`/HAProxy alias,
# compares the canonical digest, pauses one HAProxy, restores, waits for the old
# primary to rejoin, and writes a value-free evidence JSON. It never removes a
# stack, volume, secret, config, node, or service. A missing lab precondition is
# `UNVERIFIED`, never a pass.

db_paused=()

db_say() {
  printf 'ha: database: %s\n' "$*"
}

db_fail() {
  unverified "database: $*"
}

db_required_list() {
  local value
  value=$(jq -r "$1[]" "$descriptor") || db_fail "cannot read $1"
  [ -n "$value" ] || db_fail "the descriptor declares no $1"
  printf '%s\n' "$value"
}

db_patroni_list() {
  local host=$1
  # shellcheck disable=SC2086
  remote "$host" "$docker_bin exec quantrule-patroni-$host sh -c \
    'PATRONI_CTL_PASSWORD=\$(cat /run/secrets/patroni_rest_password) patronictl -c /etc/patroni/patroni.yml list'"
}

# The member name in one role column (`Leader`, `Sync`, `Replica`), or empty.
db_role_member() {
  local list=$1 role=$2
  printf '%s\n' "$list" | awk -F'|' -v role="$role" '
    {
      member = $2
      gsub(/^ +| +$/, "", member)
      gsub(/^ +| +$/, "", $4)
      if (tolower($4) == tolower(role)) { print member; exit }
    }
  '
}

db_member_role() {
  local list=$1 member=$2
  printf '%s\n' "$list" | awk -F'|' -v member="$member" '
    {
      name = $2
      gsub(/^ +| +$/, "", name)
      gsub(/^ +| +$/, "", $4)
      if (name == member) { print $4; exit }
    }
  '
}

db_host_of_member() {
  printf '%s' "${1#patroni-}"
}

db_etcd_health() {
  local host=$1
  # shellcheck disable=SC2086
  remote "$host" "$docker_bin exec -e ETCDCTL_API=3 quantrule-etcd-$host etcdctl \
    --endpoints=https://127.0.0.1:2379 \
    --cacert=/etc/quantrule/pki/ca.crt \
    --cert=/etc/quantrule/pki/etcd/client.crt \
    --key=/etc/quantrule/pki/etcd/client.key \
    --dial-timeout=2s --command-timeout=5s endpoint health" >/dev/null 2>&1
}

db_etcd_leader() {
  local host=$1 endpoints="" member json
  for member in $db_etcd_hosts; do
    endpoints="${endpoints:+$endpoints,}https://etcd-$member:2379"
  done
  # shellcheck disable=SC2086
  json=$(remote "$host" "$docker_bin exec -e ETCDCTL_API=3 quantrule-etcd-$host etcdctl \
    --endpoints=$endpoints \
    --cacert=/etc/quantrule/pki/ca.crt \
    --cert=/etc/quantrule/pki/etcd/client.crt \
    --key=/etc/quantrule/pki/etcd/client.key \
    --dial-timeout=2s --command-timeout=5s endpoint status --cluster -w json" 2>/dev/null) \
    || return 1
  printf '%s' "$json" \
    | jq -r '.[] | select(.Status.header.member_id == .Status.leader) | .Endpoint' \
    | head -n 1
}

db_psql_local() {
  local host=$1 sql=$2
  # shellcheck disable=SC2086
  remote "$host" "$docker_bin exec -u postgres quantrule-patroni-$host \
    psql -d postgres -tAc \"$sql\""
}

# Run psql inside a Patroni container with the mounted superuser file and CA.
# `extra_env` is empty for the application path or `PGHOSTADDR=<overlay-ip>` for
# the HAProxy control; the hostname stays `postgres`, so `verify-full` still
# verifies the Postgres SAN.
db_psql_remote() {
  local exec_host=$1 sql=$2 extra_env=${3:-}
  # shellcheck disable=SC2086
  remote "$exec_host" "$docker_bin exec quantrule-patroni-$exec_host sh -c \
    'PGPASSWORD=\$(cat /run/secrets/postgres_superuser_password); \
     export PGPASSWORD; \
     $extra_env PGSSLMODE=verify-full PGSSLROOTCERT=/etc/quantrule/pki/ca.crt PGCONNECT_TIMEOUT=5 \
     psql -h postgres -U postgres -d postgres -tAc \"$sql\"'"
}

# Write through the application endpoint: the HAProxy `postgres` alias and TLS.
db_psql_through_postgres() {
  db_psql_remote "$1" "$2" ""
}

# Dial one surviving HAProxy overlay address while keeping the `postgres`
# hostname for `verify-full`; this is a lab connectivity check, not the app path.
db_psql_via_hostaddr() {
  db_psql_remote "$1" "$2" "PGHOSTADDR=$3"
}

db_cleanup() {
  local entry host container
  for entry in ${db_paused[@]+"${db_paused[@]}"}; do
    host=${entry%%:*}
    container=${entry#*:}
    # shellcheck disable=SC2086
    remote "$host" "$docker_bin unpause $container" >/dev/null 2>&1 || true
  done
  db_paused=()
}

db_pause() {
  local host=$1 container=$2
  # shellcheck disable=SC2086
  remote "$host" "$docker_bin pause $container" >/dev/null \
    || db_fail "cannot pause $container on $host"
  db_paused+=("$host:$container")
  db_say "paused $container on $host"
}

db_unpause() {
  local host=$1 container=$2 keep=() entry
  # shellcheck disable=SC2086
  remote "$host" "$docker_bin unpause $container" >/dev/null \
    || db_fail "cannot unpause $container on $host"
  for entry in ${db_paused[@]+"${db_paused[@]}"}; do
    [ "$entry" = "$host:$container" ] || keep+=("$entry")
  done
  db_paused=(${keep[@]+"${keep[@]}"})
  db_say "unpaused $container on $host"
}

db_preflight() {
  db_postgres_hosts=$(db_required_list '.stateful.postgres.nodes')
  db_etcd_hosts=$(db_required_list '.stateful.etcd.nodes')
  db_haproxy_hosts=$(db_required_list '.stateful.haproxy.nodes')
  db_watchdog=$(jq -r '.stateful.postgres.watchdog // "off"' "$descriptor")
  db_overlay=$(jq -r '.network.overlay' "$descriptor") \
    || db_fail "cannot read network.overlay"
  [ -n "$db_overlay" ] || db_fail "network.overlay is empty"
  [ "$db_watchdog" = "off" ] \
    || db_fail "the pause phase requires stateful.postgres.watchdog=off; found $db_watchdog"
  [ "$profile" = "lab-single-host" ] \
    || db_fail "the database failover phase is lab-only; use a lab-single-host descriptor or --skip-database"

  local host
  for host in $db_postgres_hosts $db_etcd_hosts $db_haproxy_hosts; do
    # shellcheck disable=SC2086
    $ssh_bin "$(host_address "$host")" true >/dev/null 2>&1 \
      || db_fail "cannot SSH to $host ($(host_address "$host")); set QR_SWARM_SSH"
  done
  for host in $db_etcd_hosts; do
    db_etcd_health "$host" || db_fail "etcd $host is not healthy with the client TLS leaves"
  done
  db_etcd_leader_endpoint=$(db_etcd_leader "$(printf '%s\n' "$db_etcd_hosts" | head -n 1)") \
    || db_fail "cannot read the etcd leader with the client TLS leaves"
  [ -n "$db_etcd_leader_endpoint" ] || db_fail "etcd reports no leader"

  local first_postgres first_haproxy output running
  first_postgres=$(printf '%s\n' "$db_postgres_hosts" | head -n 1)
  first_haproxy=$(printf '%s\n' "$db_haproxy_hosts" | head -n 1)
  output=$(db_patroni_list "$first_postgres") \
    || db_fail "cannot read patronictl list on $first_postgres"
  db_primary=$(db_role_member "$output" Leader)
  db_sync=$(db_role_member "$output" Sync)
  [ -n "$db_primary" ] || db_fail "patronictl reports no leader"
  [ -n "$db_sync" ] || db_fail "patronictl reports no synchronous standby; writes would block"
  case "$db_primary" in patroni-*) ;; *) db_fail "unexpected leader member $db_primary" ;; esac
  case "$db_sync" in patroni-*) ;; *) db_fail "unexpected sync member $db_sync" ;; esac
  for host in $db_haproxy_hosts; do
    # shellcheck disable=SC2086
    running=$(remote "$host" "$docker_bin inspect --format '{{.State.Running}}' quantrule-haproxy-$host" 2>/dev/null || true)
    [ "$running" = "true" ] || db_fail "HAProxy quantrule-haproxy-$host is not running on $host"
  done
  local through
  through=$(db_psql_through_postgres "$first_postgres" "SELECT NOT pg_is_in_recovery();" \
    | tr -d '[:space:]') \
    || db_fail "psql through the postgres alias does not answer"
  [ "$through" = "t" ] || db_fail "the postgres alias does not reach a primary"
  db_probe_haproxy=$first_haproxy
  db_probe_postgres=$(printf '%s\n' "$db_postgres_hosts" | grep -Fxv "$(db_host_of_member "$db_primary")" | head -n 1)
  [ -n "$db_probe_postgres" ] || db_fail "no second Patroni node is available to observe failover"

  db_say "preflight: leader $db_primary, sync $db_sync, etcd leader $db_etcd_leader_endpoint"
  db_say "HAProxy $first_haproxy answers psql through postgres"
}

db_baseline() {
  local primary_host
  primary_host=$(db_host_of_member "$db_primary")
  db_baseline_host=$primary_host
  db_psql_local "$primary_host" \
    "CREATE TABLE IF NOT EXISTS qr_ha_marker (marker text primary key, recorded_at timestamptz not null default now());" \
    >/dev/null || db_fail "cannot create the marker table on the primary"
  db_psql_local "$primary_host" \
    "INSERT INTO qr_ha_marker (marker) VALUES ('baseline') ON CONFLICT (marker) DO NOTHING;" \
    >/dev/null || db_fail "cannot insert the baseline marker"
  db_marker_baseline=$(db_psql_local "$primary_host" \
    "SELECT count(*) FROM qr_ha_marker WHERE marker = 'baseline';" | tr -d '[:space:]')
  [ "$db_marker_baseline" -ge 1 ] || db_fail "the baseline marker was not persisted"
  db_lsn_before=$(db_psql_local "$primary_host" "SELECT pg_current_wal_lsn();" | tr -d '[:space:]')
  [ -n "$db_lsn_before" ] || db_fail "cannot read pg_current_wal_lsn() on the primary"
  verify_run "database baseline"
  db_digest_baseline=$last_digest
  [ -n "$db_digest_baseline" ] || db_fail "the database baseline printed no canonical digest"
  db_say "baseline: marker $db_marker_baseline, LSN $db_lsn_before, digest sha256:$db_digest_baseline"
}

db_loss() {
  local old_host old_member deadline output leader sync through
  old_host=$(db_host_of_member "$db_primary")
  old_member=$db_primary
  db_started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  db_loss_start=$(date +%s)
  db_pause "$old_host" "quantrule-patroni-$old_host"
  deadline=$(( $(date +%s) + 180 ))
  while [ "$(date +%s)" -lt "$deadline" ]; do
    output=$(db_patroni_list "$db_probe_postgres" 2>/dev/null || true)
    leader=$(db_role_member "$output" Leader)
    sync=$(db_role_member "$output" Sync)
    through=$(db_psql_through_postgres "$db_probe_postgres" \
      "SELECT NOT pg_is_in_recovery();" 2>/dev/null | tr -d '[:space:]' || true)
    if [ -n "$leader" ] && [ -n "$sync" ] && [ "$leader" != "$old_member" ] \
      && [ "$through" = "t" ]; then
      db_new_leader=$leader
      db_new_sync=$sync
      break
    fi
    sleep 1
  done
  db_rto=$(( $(date +%s) - db_loss_start ))
  [ -n "$db_new_leader" ] \
    || db_fail "no new leader and synchronous standby within 180 s"
  [ -n "$db_new_sync" ] || db_fail "the new leader has no synchronous standby"
  db_new_leader_host=$(db_host_of_member "$db_new_leader")
  [ "$db_new_leader" != "$old_member" ] || db_fail "the paused member is still the leader"
  db_say "failover: $old_member paused; leader $db_new_leader, sync $db_new_sync, RTO ${db_rto}s"
}

db_verify_marker() {
  db_marker_after=$(db_psql_local "$db_new_leader_host" \
    "SELECT count(*) FROM qr_ha_marker WHERE marker = 'baseline';" | tr -d '[:space:]')
  [ "$db_marker_after" -ge 1 ] || db_fail "the baseline marker did not survive the failover"
  db_psql_through_postgres "$db_probe_postgres" \
    "INSERT INTO qr_ha_marker (marker) VALUES ('after-failover') ON CONFLICT (marker) DO NOTHING;" \
    >/dev/null || db_fail "cannot write the second marker through the postgres alias"
  db_marker_total=$(db_psql_local "$db_new_leader_host" \
    "SELECT count(*) FROM qr_ha_marker;" | tr -d '[:space:]')
  [ "$db_marker_total" -ge 2 ] || db_fail "the second marker was not persisted"
  db_lsn_after=$(db_psql_local "$db_new_leader_host" \
    "SELECT pg_current_wal_lsn();" | tr -d '[:space:]')
  local advanced
  advanced=$(db_psql_local "$db_new_leader_host" \
    "SELECT pg_current_wal_lsn() > '$db_lsn_before'::pg_lsn;" | tr -d '[:space:]')
  [ "$advanced" = "t" ] || db_fail "the WAL LSN did not advance after the failover write"
  verify_run "after database failover"
  [ "$last_digest" = "$db_digest_baseline" ] \
    || db_fail "the canonical digest changed after the database failover"
  db_digest_after=$last_digest
  db_say "failover: marker $db_marker_after/$db_marker_total, LSN $db_lsn_after, digest sha256:$db_digest_after"
}

db_haproxy_control() {
  local host container survivor survivor_container survivor_ip through
  host=$db_probe_haproxy
  container="quantrule-haproxy-$host"
  survivor=$(printf '%s\n' "$db_haproxy_hosts" | grep -Fxv "$host" | head -n 1)
  [ -n "$survivor" ] || db_fail "the lab needs a second HAProxy host for the control"
  survivor_container="quantrule-haproxy-$survivor"
  survivor_ip=$(remote "$survivor" "$docker_bin inspect --format '{{json .NetworkSettings.Networks}}' $survivor_container" 2>/dev/null \
    | jq -r --arg net "$db_overlay" '.[$net].IPAddress // empty') \
    || db_fail "cannot read the surviving HAProxy network on $survivor"
  case "$survivor_ip" in
    "" | *[!0-9.]*) db_fail "cannot read the surviving HAProxy overlay address on $survivor" ;;
  esac
  db_pause "$host" "$container"
  sleep 1
  through=$(db_psql_via_hostaddr "$db_probe_postgres" "SELECT 1;" "$survivor_ip" 2>/dev/null \
    | tr -d '[:space:]' || true)
  db_unpause "$host" "$container"
  [ "$through" = "1" ] \
    || db_fail "psql via the surviving $survivor_container failed while $container was paused"
  db_haproxy_control_paused=$container
  db_haproxy_control_survivor=$survivor_container
  db_haproxy_control_path="postgres hostname via PGHOSTADDR=$survivor_ip (sslmode=verify-full)"
  db_haproxy_control_result="paused $container; $survivor_container answered over TLS"
  db_say "HAProxy control: $db_haproxy_control_result; path $db_haproxy_control_path"
}

db_restore() {
  local old_host old_member deadline output role streaming logs
  old_host=$(db_baseline_host)
  old_member=$db_primary
  db_unpause "$old_host" "quantrule-patroni-$old_host"
  deadline=$(( $(date +%s) + 180 ))
  role=""
  while [ "$(date +%s)" -lt "$deadline" ]; do
    output=$(db_patroni_list "$db_probe_postgres" 2>/dev/null || true)
    role=$(db_member_role "$output" "$old_member")
    streaming=$(db_psql_local "$db_new_leader_host" \
      "SELECT count(*) FROM pg_stat_replication WHERE state = 'streaming';" \
      2>/dev/null | tr -d '[:space:]' || true)
    case "$role" in
      Replica | Sync)
        if [ -n "$streaming" ] && [ "$streaming" -ge 2 ]; then
          break
        fi
        ;;
    esac
    sleep 1
  done
  case "$role" in
    Replica | Sync) ;;
    *) db_fail "$old_member did not rejoin as a replica" ;;
  esac
  [ -n "$streaming" ] && [ "$streaming" -ge 2 ] \
    || db_fail "the new primary has only ${streaming:-0} streaming standby(s); expected 2"
  logs=$(remote "$old_host" "$docker_bin logs --since $db_started_at quantrule-patroni-$old_host" 2>&1 || true)
  case "$logs" in
    *pg_rewind* | *"rewind"*) db_rejoin="pg_rewind" ;;
    *pg_basebackup* | *reinitialize* | *reinit*) db_rejoin="reinit" ;;
    *) db_rejoin="streaming" ;;
  esac
  if [ -f "$out/stateful-manifest.json" ]; then
    qr_deploy check "$descriptor" --remote docker --overlay --out "$out" \
      || db_fail "the full remote/overlay check failed after the database restore"
  else
    qr_deploy check "$descriptor" --remote docker --overlay \
      || db_fail "the full remote/overlay check failed after the database restore"
  fi
  verify_run "after database restore"
  [ "$last_digest" = "$db_digest_baseline" ] \
    || db_fail "the canonical digest changed after the database restore"
  db_say "restore: $old_member rejoined as $role ($db_rejoin); digest sha256:$last_digest"
}

db_evidence() {
  local path
  path=${QR_SWARM_EVIDENCE:-deploy/v3/evidence/$(date -u +%Y-%m-%dT%H%M%SZ).json}
  mkdir -p "$(dirname "$path")"
  jq -n \
    --arg evidence "Simulated HA" \
    --arg profile "$profile" \
    --arg recorded_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --arg primary_before "$db_primary" \
    --arg primary_after "$db_new_leader" \
    --arg sync_before "$db_sync" \
    --arg sync_after "$db_new_sync" \
    --arg etcd_leader "$db_etcd_leader_endpoint" \
    --arg method "docker pause of the primary Patroni container with the watchdog off" \
    --argjson rto_seconds "$db_rto" \
    --arg marker_baseline "$db_marker_baseline" \
    --arg marker_after "$db_marker_after" \
    --arg marker_total "$db_marker_total" \
    --arg lsn_before "$db_lsn_before" \
    --arg lsn_after "$db_lsn_after" \
    --arg digest_before "sha256:$db_digest_baseline" \
    --arg digest_failover "sha256:$db_digest_after" \
    --arg digest_restore "sha256:$last_digest" \
    --arg rejoin "$db_rejoin" \
    --arg haproxy_paused "$db_haproxy_control_paused" \
    --arg haproxy_survivor "$db_haproxy_control_survivor" \
    --arg haproxy_path "$db_haproxy_control_path" \
    --arg haproxy_result "$db_haproxy_control_result" \
    --arg rpo "RPO 0 for acknowledged commits; no disk, power, or AZ claim" \
    --arg rto "RTO is measured, not the 15-minute SLO" \
    '{
      evidence: $evidence,
      profile: $profile,
      recorded_at: $recorded_at,
      topology: {primary_before: $primary_before, primary_after: $primary_after,
                 sync_before: $sync_before, sync_after: $sync_after,
                 etcd_leader: $etcd_leader},
      failover: {method: $method, rto_seconds: $rto_seconds},
      markers: {baseline: $marker_baseline, after: $marker_after, total: $marker_total},
      wal: {lsn_before: $lsn_before, lsn_after: $lsn_after},
      digests: {before: $digest_before, after_failover: $digest_failover, after_restore: $digest_restore},
      rejoin: $rejoin,
      haproxy_control: {
        paused: $haproxy_paused,
        survivor: $haproxy_survivor,
        path: $haproxy_path,
        tls: "sslmode=verify-full",
        lab_check: true,
        app_path: "postgres alias, sslmode=verify-full",
        result: $haproxy_result
      },
      claims: {rpo: $rpo, rto: $rto}
    }' > "$path"
  db_say "evidence: $path (Simulated HA)"
}

db_failover() {
  trap db_cleanup EXIT
  db_preflight
  db_baseline
  db_loss
  db_verify_marker
  db_haproxy_control
  negative_controls
  db_restore
  db_evidence
  db_cleanup
  trap - EXIT
  db_say "PASS: Simulated HA; RTO ${db_rto}s; leader $db_new_leader; sync $db_new_sync; rejoin $db_rejoin"
}
