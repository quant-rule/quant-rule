#!/usr/bin/env bash
# Descriptor, pgBackRest, psql, marker, and RPO helpers. Sourced by dr.sh.

dr_read_descriptor() {
  [ -f "$DR_DESCRIPTOR" ] || dr_die "descriptor $DR_DESCRIPTOR is missing"
  jq -e . "$DR_DESCRIPTOR" >/dev/null 2>&1 || dr_die "descriptor $DR_DESCRIPTOR is not JSON"
  DR_PROFILE=$(jq -r '.profile' "$DR_DESCRIPTOR")
  DR_STACK=$(jq -r '.name // "quantrule"' "$DR_DESCRIPTOR")
  DR_OVERLAY=$(jq -r '.network.overlay' "$DR_DESCRIPTOR")
  DR_NODE=$(jq -r '.stateful.postgres.nodes[0]' "$DR_DESCRIPTOR")
  DR_PGDATA=$(jq -r '.stateful.postgres.pgdata // "/srv/quantrule/patroni/data"' "$DR_DESCRIPTOR" 2>/dev/null || true)
  [ -n "$DR_PGDATA" ] || DR_PGDATA=/srv/quantrule/patroni/data
  DR_REPO_KIND=$(jq -r '.backup.repo.kind // "posix"' "$DR_DESCRIPTOR")
  DR_REPO_ENDPOINT=$(jq -r '.backup.repo.endpoint // empty' "$DR_DESCRIPTOR")
  DR_REPO_BUCKET=$(jq -r '.backup.repo.bucket // empty' "$DR_DESCRIPTOR")
  DR_REPO_PATH=$(jq -r '.backup.repo.path // "/srv/quantrule/backup/pgbackrest"' "$DR_DESCRIPTOR")
  DR_REPO_CA=$(jq -r '.backup.repo.tls_ca // empty' "$DR_DESCRIPTOR")
  DR_CIPHER=$(jq -r '.backup.cipher // "aes-256-cbc"' "$DR_DESCRIPTOR")
  DR_BACKUP_HOST=$(jq -r '.backup.host // .swarm.managers[0]' "$DR_DESCRIPTOR")
  DR_PG_IMAGE=$(jq -r '.images.postgres.reference + "@" + .images.postgres.digest' "$DR_DESCRIPTOR")
  DR_APP_IMAGE=$(jq -r '.image.reference + "@" + .image.digest' "$DR_DESCRIPTOR")
  DR_INGRESS_DNS=$(jq -r '.ingress.dns' "$DR_DESCRIPTOR")
  DR_DATABASE_CA=$(jq -r '.endpoints.database.tls_ca' "$DR_DESCRIPTOR")
  DR_OBJECT_CA=$(jq -r '.endpoints.object_store.tls_ca' "$DR_DESCRIPTOR")
  DR_SUPERUSER_FILE=$(jq -r '.secrets.files.postgres_superuser_password // empty' "$DR_DESCRIPTOR")
  DR_ARCHIVE_TARGET=$(jq -r '.backup.object_archive.target // empty' "$DR_DESCRIPTOR")
  DR_ARCHIVE_INTERVAL=${QR_DR_ARCHIVE_INTERVAL_SECONDS:-86400}
  DR_PATRONI_IMAGE=$(jq -r '.images.patroni.reference + "@" + .images.patroni.digest' "$DR_DESCRIPTOR")
  DR_RENDER=${DR_RENDER:-${OUT:-deploy/v3/out}}
}

dr_repo_facts() {
  dr_fact "schema" "quantrule.dr/v1"
  dr_fact "drill_id" "$DR_DRILL_ID"
  dr_fact "mode" "$DR_MODE"
  dr_fact "started_utc" "$DR_STARTED_UTC"
  dr_fact "descriptor" "$DR_DESCRIPTOR"
  dr_fact "descriptor_sha256" "$(sha256sum "$DR_DESCRIPTOR" | cut -d' ' -f1)"
  dr_fact "profile" "$DR_PROFILE"
  dr_fact "stack" "$DR_STACK"
  dr_fact "image_digest" "${DR_APP_IMAGE##*@}"
  dr_fact "repo_kind" "$DR_REPO_KIND"
  if [ "$DR_REPO_KIND" = "posix" ]; then
    dr_fact "repo_endpoint" "posix"
  else
    dr_fact "repo_endpoint" "$(dr_redact_endpoint "${DR_REPO_ENDPOINT:-$DR_REPO_PATH}")"
  fi
  dr_fact "repo_bucket" "$DR_REPO_BUCKET"
  dr_fact "repo_path" "$DR_REPO_PATH"
  dr_fact "cipher" "$DR_CIPHER"
  dr_fact "stanza" "quantrule"
  dr_fact "backup_host" "$DR_BACKUP_HOST"
  dr_fact "archive_target" "$DR_ARCHIVE_TARGET"
}

# Read one marker (`epoch|lsn`) from the source primary through the shared
# `postgres` alias. The superuser password is read from a file inside the
# container; it never appears on the operator command line.
dr_source_psql() {
  local sql=$1
  [ -n "$DR_SUPERUSER_FILE" ] && [ -r "$DR_SUPERUSER_FILE" ] \
    || dr_die "the postgres superuser password file is not readable"
  [ -r "$DR_DATABASE_CA" ] || dr_die "the database CA $DR_DATABASE_CA is not readable"
  docker run --rm --network "$DR_OVERLAY" \
    -v "$DR_SUPERUSER_FILE:/run/secrets/pg:ro" \
    -v "$DR_DATABASE_CA:/run/pki/ca.crt:ro" \
    --entrypoint sh "$DR_PG_IMAGE" -c '
      set -eu
      PGPASSWORD=$(cat /run/secrets/pg)
      export PGPASSWORD PGSSLMODE=verify-full PGSSLROOTCERT=/run/pki/ca.crt
      exec psql -h postgres -U postgres -d postgres -tAc "$1"
    ' -- "$sql"
}

dr_marker_window() {
  local interval=${1:-10} window=${2:-60}
  case "$interval$window" in
    *[!0-9]*) dr_die "the marker interval and window must be whole seconds" ;;
  esac
  [ "$interval" -gt 0 ] && [ "$window" -ge "$interval" ] \
    || dr_die "the marker window must be at least one interval"
  mkdir -p "$DR_WORK/markers"
  : > "$DR_WORK/markers/rows.txt"
  local started now count=0
  started=$(dr_now_epoch)
  while :; do
    local row
    row=$(dr_source_psql "SELECT extract(epoch FROM clock_timestamp())::bigint || '|' || pg_current_wal_lsn()::text") \
      || dr_die "cannot read a source marker"
    [ -n "$row" ] || dr_die "the source returned an empty marker"
    printf '%s\n' "$row" >> "$DR_WORK/markers/rows.txt"
    count=$((count + 1))
    now=$(dr_now_epoch)
    [ $((now - started)) -lt "$window" ] || break
    sleep "$interval"
  done
  DR_MARKER_ROWS=$count
  dr_fact_num "marker_rows" "$count"
  dr_fact_num "marker_window_seconds" "$window"
  dr_fact_num "marker_interval_seconds" "$interval"
}

# The target time is the recovery point. A marker at or before it is covered by
# the restore; the newest covered marker bounds the measured row RPO. This is a
# read-only marker: no schema or row is written to the source to prove RPO.
dr_compute_rpo() {
  local target_epoch=$1 restored_lsn=${2:-}
  local covered=0 newest=0 line epoch lsn compressed
  while IFS='|' read -r epoch lsn; do
    [ -n "$epoch" ] || continue
    case "$epoch" in *[!0-9]*) continue ;; esac
    [ "$epoch" -le "$target_epoch" ] || continue
    covered=$((covered + 1))
    [ "$epoch" -gt "$newest" ] && newest=$epoch
  done < "$DR_WORK/markers/rows.txt"
  DR_MARKER_COVERED=$covered
  dr_fact_num "marker_covered_rows" "$covered"
  [ -n "$restored_lsn" ] && dr_fact "restored_lsn" "$restored_lsn"
  if [ "$covered" -gt 0 ]; then
    DR_RPO_SECONDS=$((target_epoch - newest))
    dr_fact_num "rpo_seconds" "$DR_RPO_SECONDS"
    dr_fact "rpo_basis" "target_epoch - max(covered marker_epoch)"
  else
    DR_RPO_SECONDS=-1
    dr_fact_num "rpo_seconds" -1
    dr_fact "rpo_basis" "no marker at or before the target"
  fi
}

dr_object_rpo() {
  local target_epoch=$1
  local file archive_epoch
  file=$(dr_last_archive_evidence)
  if [ -z "$file" ] || [ ! -f "$file" ]; then
    DR_OBJECT_RPO=-1
    dr_fact_num "object_rpo_seconds" -1
    dr_fact "object_rpo_basis" "no object-archive evidence"
    return 1
  fi
  archive_epoch=$(dr_archive_epoch "$file") || {
    DR_OBJECT_RPO=-1
    dr_fact_num "object_rpo_seconds" -1
    dr_fact "object_rpo_basis" "unreadable object-archive evidence"
    return 1
  }
  DR_ARCHIVE_EPOCH=$archive_epoch
  DR_ARCHIVE_EVIDENCE_FILE=$file
  DR_OBJECT_RPO=$((target_epoch - archive_epoch))
  dr_fact_num "object_rpo_seconds" "$DR_OBJECT_RPO"
  dr_fact_num "object_archive_interval_seconds" "$DR_ARCHIVE_INTERVAL"
  dr_fact "object_archive_evidence" "$file"
  dr_fact "object_rpo_basis" "last archive finished_utc minus target"
  [ "$DR_OBJECT_RPO" -le 0 ]
}

dr_epoch_value() {
  case "$1" in
    '' ) return 1 ;;
    *[!0-9]*) date -u -d "$1" +%s 2>/dev/null ;;
    *) printf '%s' "$1" ;;
  esac
}

# pgBackRest info --output=json is an array of stanzas; accept an object form
# too so a future schema change does not turn a successful backup into a pass
# with no label.
dr_info_backups() {
  jq -c '(if type == "array" then [.[].backups[]?] else [.backups[]?] end)'
}

# Resolve a backup label and a recovery time. `pgbackrest info --output=json`
# runs in a one-shot container on the host; the returned label/start time is the
# safe default when no --target-time is supplied. The start is the base backup's
# consistency point; targeting it keeps the marker window bounded.
dr_resolve_target() {
  DR_TARGET_TYPE=${DR_TARGET_TYPE:-time}
  DR_TARGET_LSN=${DR_TARGET_LSN:-}
  DR_TARGET_TIME=${DR_TARGET_TIME:-}
  DR_BACKUP_LABEL=${DR_BACKUP_LABEL:-}
  if [ -n "$DR_TARGET_TIME" ]; then
    DR_TARGET_EPOCH=$(dr_epoch_value "$DR_TARGET_TIME") \
      || dr_die "--target-time $DR_TARGET_TIME is not a UTC timestamp"
    dr_fact "target_time_utc" "$DR_TARGET_TIME"
  fi
  if [ "$DR_TARGET_TYPE" = "lsn" ] && [ -n "$DR_TARGET_LSN" ]; then
    [ -n "$DR_TARGET_EPOCH" ] || DR_TARGET_EPOCH=$(dr_now_epoch)
    dr_fact "target_lsn" "$DR_TARGET_LSN"
  fi
  if [ -z "$DR_TARGET_EPOCH" ]; then
    local info
    info=$(dr_pgbackrest_info) || dr_unverified "no readable pgBackRest info; pass --target-time or --backup-label"
    if [ -z "$DR_BACKUP_LABEL" ]; then
      DR_BACKUP_LABEL=$(printf '%s\n' "$info" | dr_info_backups | jq -r '
        [.[] | select(.error == false)] | sort_by(.timestamp.start) | last | .label // empty
      ') || true
    fi
    [ -n "$DR_BACKUP_LABEL" ] \
      || dr_unverified "no successful backup label; pass --target-time or --backup-label"
    DR_TARGET_EPOCH=$(printf '%s\n' "$info" | dr_info_backups | jq -r --arg label "$DR_BACKUP_LABEL" '
      .[] | select(.label == $label) | .timestamp.start
    ' | head -n 1)
    DR_TARGET_EPOCH=$(dr_epoch_value "$DR_TARGET_EPOCH") \
      || dr_unverified "backup $DR_BACKUP_LABEL has no start time"
    DR_TARGET_TIME=$(date -u -d "@$DR_TARGET_EPOCH" +%Y-%m-%dT%H:%M:%SZ)
    DR_TARGET_TYPE=time
    dr_fact "target_time_utc" "$DR_TARGET_TIME"
  fi
  [ -n "$DR_BACKUP_LABEL" ] || {
    if [ -n "$DR_TARGET_TIME$DR_TARGET_LSN" ]; then
      DR_BACKUP_LABEL=target-time
    else
      DR_BACKUP_LABEL=latest
    fi
  }
  dr_fact "backup_label" "$DR_BACKUP_LABEL"
  dr_fact "backup_type" "${DR_BACKUP_TYPE:-selected}"
  dr_fact "target_type" "$DR_TARGET_TYPE"
  dr_fact_num "target_epoch" "$DR_TARGET_EPOCH"
}

# A fresh full backup is the base for the drill and, in-place, the pre-restore
# safety copy. A full success is required; a dry-run or a failed backup refuses.
# With keep_target=1 the caller supplied its own PITR target, so the fresh
# backup is recorded as the pre-restore safety copy and does not replace it.
dr_take_backup() {
  local keep_target=${1:-0}
  local script=${QR_DR_BACKUP_SCRIPT:-$DR_RENDER/pgbackrest-backup.sh}
  [ -x "$script" ] || dr_unverified "the rendered backup script $script is missing or not executable"
  local start end info label epoch
  dr_say "backup: full backup before the restore"
  start=$(dr_now_epoch)
  if [ "${QR_DR_BACKUP_LOCAL:-0}" = "1" ]; then
    "$script" full || dr_unverified "the pre-restore full backup failed"
  else
    PGBACKREST_NODE="$DR_NODE" PGBACKREST_LOCAL=auto \
      "$script" full || dr_unverified "the pre-restore full backup failed"
  fi
  end=$(dr_now_epoch)
  DR_BACKUP_SECONDS=$(dr_elapsed "$start" "$end")
  dr_fact_num "backup_seconds" "$DR_BACKUP_SECONDS"
  info=$(dr_pgbackrest_info) || dr_unverified "the backup succeeded but pgBackRest info is unreadable"
  label=$(printf '%s\n' "$info" | dr_info_backups | jq -r '
    [.[] | select(.error == false)] | sort_by(.timestamp.start) | last | .label // empty
  ') || true
  [ -n "$label" ] || dr_unverified "the fresh backup has no successful label"
  epoch=$(printf '%s\n' "$info" | dr_info_backups | jq -r --arg label "$label" '
    .[] | select(.label == $label) | .timestamp.start
  ' | head -n 1)
  epoch=$(dr_epoch_value "$epoch") \
    || dr_unverified "the fresh backup $label has no start time"
  if [ "$keep_target" -eq 1 ]; then
    DR_PRE_RESTORE_BACKUP_LABEL=$label
    DR_PRE_RESTORE_BACKUP_EPOCH=$epoch
    dr_fact "pre_restore_backup_label" "$label"
    dr_fact_num "pre_restore_backup_epoch" "$epoch"
    dr_fact "pre_restore_backup_basis" "fresh full backup; the supplied target is unchanged"
    return 0
  fi
  DR_BACKUP_LABEL=$label
  DR_TARGET_EPOCH=$epoch
  DR_TARGET_TIME=$(date -u -d "@$epoch" +%Y-%m-%dT%H:%M:%SZ)
  DR_TARGET_TYPE=time
  DR_BACKUP_TYPE=full
  dr_fact "backup_label" "$label"
  dr_fact "backup_type" "full"
  dr_fact "target_time_utc" "$DR_TARGET_TIME"
  dr_fact "target_type" "time"
  dr_fact_num "target_epoch" "$epoch"
  dr_fact "backup_basis" "fresh full backup; target is its start (base consistency point)"
}

# The pgBackRest info invocation. The secret files are mounted read-only; the
# values are read into the container environment, never an argument or a log.
dr_pgbackrest_env() {
  local env_file=$1
  umask 077
  : > "$env_file"
  [ -r "$DR_RENDER/secrets/pgbackrest_cipher_pass" ] \
    || dr_die "the rendered pgbackrest_cipher_pass is missing"
  printf 'PGBACKREST_REPO1_CIPHER_PASS=%s\n' \
    "$(cat "$DR_RENDER/secrets/pgbackrest_cipher_pass")" >> "$env_file"
  if [ "$DR_REPO_KIND" = "s3" ]; then
    [ -r "$DR_RENDER/secrets/pgbackrest_s3_access_key" ] \
      || dr_die "the rendered pgbackrest_s3_access_key is missing"
    [ -r "$DR_RENDER/secrets/pgbackrest_s3_secret_key" ] \
      || dr_die "the rendered pgbackrest_s3_secret_key is missing"
    printf 'PGBACKREST_REPO1_S3_KEY=%s\n' \
      "$(cat "$DR_RENDER/secrets/pgbackrest_s3_access_key")" >> "$env_file"
    printf 'PGBACKREST_REPO1_S3_KEY_SECRET=%s\n' \
      "$(cat "$DR_RENDER/secrets/pgbackrest_s3_secret_key")" >> "$env_file"
  fi
  chmod 600 "$env_file"
}

dr_pgbackrest_run() {
  local env_file=$1
  shift
  local args=(
    run --rm
    --env-file "$env_file"
    -v "$DR_RENDER/pgbackrest-$DR_NODE.conf:/etc/pgbackrest/pgbackrest.conf:ro"
    --entrypoint pgbackrest
    "$DR_PATRONI_IMAGE" --stanza=quantrule
  )
  if [ "$DR_REPO_KIND" = "posix" ]; then
    args+=(-v "$DR_REPO_PATH:$DR_REPO_PATH")
  elif [ -n "$DR_REPO_CA" ] && [ -r "$DR_REPO_CA" ]; then
    args+=(-v "$DR_REPO_CA:$DR_REPO_CA:ro")
  fi
  docker "${args[@]}" "$@"
}

dr_pgbackrest_info() {
  local env_file="$DR_WORK/pgbackrest.env"
  dr_pgbackrest_env "$env_file"
  dr_pgbackrest_run "$env_file" info --output=json
}

dr_restore_pgdata() {
  local pgdata=$1 mode=$2 start end
  local env_file="$DR_WORK/pgbackrest.env"
  dr_pgbackrest_env "$env_file"
  local args=(restore)
  [ "$mode" = "in-place" ] && args+=(--delta)
  case "$DR_TARGET_TYPE" in
    lsn)
      args+=(--type=lsn --target="$DR_TARGET_LSN" --target-action=promote)
      ;;
    time)
      args+=(--type=time --target="$DR_TARGET_TIME" --target-action=promote)
      ;;
    *)
      dr_die "unsupported target type $DR_TARGET_TYPE"
      ;;
  esac
  start=$(dr_now_epoch)
  local user_args=()
  [ "$mode" = "scratch" ] && user_args=(--user 0)
  local mounts=(
    -v "$pgdata:/var/lib/postgresql/data"
    -v "$DR_RENDER/pgbackrest-$DR_NODE.conf:/etc/pgbackrest/pgbackrest.conf:ro"
  )
  if [ "$DR_REPO_KIND" = "posix" ]; then
    mounts+=(-v "$DR_REPO_PATH:$DR_REPO_PATH")
  elif [ -n "$DR_REPO_CA" ] && [ -r "$DR_REPO_CA" ]; then
    mounts+=(-v "$DR_REPO_CA:$DR_REPO_CA:ro")
  fi
  docker run --rm --env-file "$env_file" "${user_args[@]}" "${mounts[@]}" \
    --entrypoint pgbackrest "$DR_PATRONI_IMAGE" --stanza=quantrule "${args[@]}" \
    || dr_unverified "pgBackRest restore failed; no artifact was published"
  end=$(dr_now_epoch)
  DR_RESTORE_SECONDS=$(dr_elapsed "$start" "$end")
  dr_fact_num "restore_seconds" "$DR_RESTORE_SECONDS"
}

dr_startup_timing() {
  local start=$1
  local end
  end=$(dr_now_epoch)
  DR_STARTUP_SECONDS=$(dr_elapsed "$start" "$end")
  dr_fact_num "startup_seconds" "$DR_STARTUP_SECONDS"
}

dr_digest_timing() {
  local start=$1 end
  end=$(dr_now_epoch)
  DR_DIGEST_SECONDS=$(dr_elapsed "$start" "$end")
  dr_fact_num "digest_seconds" "$DR_DIGEST_SECONDS"
}

dr_total_timing() {
  local end
  end=$(dr_now_epoch)
  DR_FINISHED_UTC=$(dr_now_utc)
  DR_TOTAL_SECONDS=$((end - DR_STARTED_EPOCH))
  dr_fact "finished_utc" "$DR_FINISHED_UTC"
  dr_fact_num "total_seconds" "$DR_TOTAL_SECONDS"
}
