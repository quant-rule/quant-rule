#!/usr/bin/env bash
# Scratch DR target: restore into a fresh PGDATA and a fresh object bucket in
# its own Compose project. Sourced by dr.sh.

dr_scratch_preflight() {
  [ "$DR_CONFIRM" -eq 1 ] || dr_die "--target scratch performs a restore; pass --confirm"
  [ "$DR_PROFILE" = "lab-single-host" ] \
    || dr_unverified "the scratch drill requires the lab-single-host profile; $DR_PROFILE is a production shape"
  DR_SCRATCH_ROOT=${QR_DR_SCRATCH_ROOT:-/srv/quantrule/dr/$DR_DRILL_ID}
  DR_SCRATCH_PROJECT=${QR_DR_SCRATCH_PROJECT:-quantrule-dr-$DR_DRILL_ID}
  DR_SCRATCH_PORT=${QR_DR_SCRATCH_PORT:-18080}
  case "$DR_SCRATCH_ROOT" in
    /*) ;;
    *) dr_die "QR_DR_SCRATCH_ROOT must be absolute" ;;
  esac
  case "$DR_SCRATCH_ROOT" in
    /srv/quantrule/patroni/data | /srv/quantrule/backup | /srv/quantrule/backup/* | /)
      dr_unverified "the scratch root $DR_SCRATCH_ROOT is a live PGDATA/repository path"
      ;;
  esac
  case "$DR_SCRATCH_PROJECT" in
    "$DR_STACK" | quantrule | '')
      dr_unverified "the scratch project $DR_SCRATCH_PROJECT would reuse the live stack name"
      ;;
    *dr*) ;;
    *) dr_die "QR_DR_SCRATCH_PROJECT must contain 'dr' so it cannot be mistaken for the live stack" ;;
  esac
  case "$DR_SCRATCH_PORT" in
    8080 | 443) dr_unverified "the scratch gateway cannot use the live 8080/443 port" ;;
    '' | *[!0-9]*) dr_die "QR_DR_SCRATCH_PORT must be a whole port number" ;;
  esac
  command -v docker >/dev/null 2>&1 || dr_unverified "Docker is required for the scratch drill"
  docker info >/dev/null 2>&1 || dr_unverified "cannot reach the Docker daemon"
  command -v curl >/dev/null 2>&1 || dr_unverified "curl is required for the scratch health probe"
  dr_qr_deploy validate "$DR_DESCRIPTOR" --release >/dev/null 2>&1 \
    || dr_unverified "qr-deploy validate --release refused $DR_DESCRIPTOR; resolve the app, MinIO, and derived Patroni image blockers"
  DR_SCRATCH_APP_IMAGE=${QR_DR_APP_IMAGE:-}
  DR_SCRATCH_PATRONI_IMAGE=${QR_DR_PATRONI_IMAGE:-$DR_PATRONI_IMAGE}
  case "$DR_SCRATCH_APP_IMAGE" in
    *@sha256:*) ;;
    *) dr_unverified "QR_DR_APP_IMAGE must be a resolved reference@sha256 image; the descriptor app image may be a fixture" ;;
  esac
  case "$DR_SCRATCH_PATRONI_IMAGE" in
    *@sha256:*) ;;
    *) dr_unverified "QR_DR_PATRONI_IMAGE must be a resolved reference@sha256 image; the derived Patroni image is a fixture blocker" ;;
  esac
  DR_PATRONI_IMAGE=$DR_SCRATCH_PATRONI_IMAGE
  DR_MINIO_CLIENT_IMAGE=$(jq -r '.images.minio_client.reference + "@" + .images.minio_client.digest' "$DR_DESCRIPTOR")
  DR_MINIO_IMAGE=$(jq -r '.images.minio.reference + "@" + .images.minio.digest' "$DR_DESCRIPTOR")
  DR_PGDATA="$DR_SCRATCH_ROOT/pgdata"
  DR_SCRATCH_BUCKET=quantrule-dr-$DR_DRILL_ID
  mkdir -p "$DR_SCRATCH_ROOT" "$DR_SCRATCH_ROOT/pki" "$DR_SCRATCH_ROOT/secrets" "$DR_SCRATCH_ROOT/env" "$DR_WORK"
  chmod 700 "$DR_SCRATCH_ROOT" "$DR_SCRATCH_ROOT/env"
  dr_fact "scratch_root" "$DR_SCRATCH_ROOT"
  dr_fact "scratch_project" "$DR_SCRATCH_PROJECT"
  dr_fact "scratch_bucket" "$DR_SCRATCH_BUCKET"
  dr_fact_num "scratch_port" "$DR_SCRATCH_PORT"
}

dr_scratch_copy_secret() {
  local source=$1 target=$2
  [ -r "$source" ] || dr_die "the rendered secret $source is not readable"
  install -m 600 "$source" "$target"
}

dr_scratch_prepare() {
  local render="$DR_RENDER"
  [ -f "$render/pgbackrest-$DR_NODE.conf" ] \
    || dr_unverified "the render $render has no pgbackrest-$DR_NODE.conf; run qr-deploy render first"
  [ -d "$render/pki/postgres" ] || dr_unverified "the render $render has no Postgres PKI"
  DR_SCRATCH_SECRETS="$DR_SCRATCH_ROOT/secrets"
  DR_SCRATCH_PKI="$DR_SCRATCH_ROOT/pki"
  mkdir -p "$DR_SCRATCH_PKI/postgres" "$DR_SCRATCH_PKI/rabbitmq" "$DR_SCRATCH_PKI/minio/CAs"
  dr_scratch_copy_secret "$render/secrets/gateway_database_url" "$DR_SCRATCH_SECRETS/qr_gateway_database_url"
  dr_scratch_copy_secret "$render/secrets/data_database_url" "$DR_SCRATCH_SECRETS/qr_catalog_database_url"
  dr_scratch_copy_secret "$render/secrets/evidence_database_url" "$DR_SCRATCH_SECRETS/qr_evidence_database_url"
  dr_scratch_copy_secret "$render/secrets/simulation_database_url" "$DR_SCRATCH_SECRETS/qr_simulation_database_url"
  dr_scratch_copy_secret "$render/secrets/gateway_owner_database_url" "$DR_SCRATCH_SECRETS/qr_gateway_owner_database_url"
  dr_scratch_copy_secret "$render/secrets/data_owner_database_url" "$DR_SCRATCH_SECRETS/qr_catalog_owner_database_url"
  dr_scratch_copy_secret "$render/secrets/evidence_owner_database_url" "$DR_SCRATCH_SECRETS/qr_evidence_owner_database_url"
  dr_scratch_copy_secret "$render/secrets/simulation_owner_database_url" "$DR_SCRATCH_SECRETS/qr_simulation_owner_database_url"
  for name in queue_url object_store_access_key object_store_secret_key gateway_token; do
    dr_scratch_copy_secret "$render/secrets/$name" "$DR_SCRATCH_SECRETS/$name"
  done
  install -m 644 "$render/pki/postgres/server.crt" "$DR_SCRATCH_PKI/postgres/server.crt"
  install -m 600 "$render/pki/postgres/server.key" "$DR_SCRATCH_PKI/postgres/server.key"
  install -m 644 "$render/pki/rabbitmq/server.crt" "$DR_SCRATCH_PKI/rabbitmq/server.crt"
  install -m 600 "$render/pki/rabbitmq/server.key" "$DR_SCRATCH_PKI/rabbitmq/server.key"
  install -m 644 "$render/pki/minio/public.crt" "$DR_SCRATCH_PKI/minio/public.crt"
  install -m 600 "$render/pki/minio/private.key" "$DR_SCRATCH_PKI/minio/private.key"
  install -m 644 "$render/pki/minio/CAs/ca.crt" "$DR_SCRATCH_PKI/minio/CAs/ca.crt"
  if [ -r "$DR_DATABASE_CA" ]; then
    install -m 644 "$DR_DATABASE_CA" "$DR_SCRATCH_PKI/ca.crt"
  elif [ -r "$render/pki/database-ca.crt" ]; then
    install -m 644 "$render/pki/database-ca.crt" "$DR_SCRATCH_PKI/ca.crt"
  else
    dr_unverified "the database CA is not readable on the host or in the render"
  fi
  DR_SCRATCH_MINIO_ROOT_USER=$(cat "$render/secrets/minio_root_user")
  DR_SCRATCH_MINIO_ROOT_PASSWORD=$(cat "$render/secrets/minio_root_password")
  DR_SCRATCH_QUEUE_URL=$(cat "$render/secrets/queue_url")
  DR_SCRATCH_RABBIT_PASSWORD=$(if command -v openssl >/dev/null 2>&1; then
    openssl rand -hex 32
  else
    od -An -N32 -tx1 /dev/urandom | tr -d ' \n'
  fi)
  DR_SCRATCH_ENV="$DR_SCRATCH_ROOT/env/compose.env"
  chmod 600 "$DR_SCRATCH_ENV"
  {
    printf 'COMPOSE_PROJECT_NAME=%s\n' "$DR_SCRATCH_PROJECT"
    printf 'QR_IMAGE=%s\n' "$DR_SCRATCH_APP_IMAGE"
    printf 'QR_IMAGE_DIGEST=%s\n' "${DR_SCRATCH_APP_IMAGE##*@}"
    printf 'QR_PKI_DIR=%s\n' "$DR_SCRATCH_PKI"
    printf 'QR_SECRETS_DIR=%s\n' "$DR_SCRATCH_SECRETS"
    printf 'QR_DR_PGDATA=%s\n' "$DR_PGDATA"
    printf 'QR_DR_BUCKET=%s\n' "$DR_SCRATCH_BUCKET"
    printf 'QR_DR_PORT=%s\n' "$DR_SCRATCH_PORT"
    printf 'QR_DR_MINIO_ROOT_USER=%s\n' "$DR_SCRATCH_MINIO_ROOT_USER"
    printf 'QR_DR_MINIO_ROOT_PASSWORD=%s\n' "$DR_SCRATCH_MINIO_ROOT_PASSWORD"
    printf 'QR_POSTGRES_PASSWORD=%s\n' "$(cat "$render/secrets/postgres_superuser_password")"
    printf 'QR_RABBITMQ_PASSWORD=%s\n' "$DR_SCRATCH_RABBIT_PASSWORD"
    printf 'QR_OBJECT_STORE_PASSWORD=%s\n' "$DR_SCRATCH_MINIO_ROOT_PASSWORD"
  } > "$DR_SCRATCH_ENV"
  chmod 600 "$DR_SCRATCH_ENV"
  [ -z "$(find "$DR_PGDATA" -mindepth 1 -print -quit 2>/dev/null)" ] \
    || dr_unverified "the scratch PGDATA $DR_PGDATA is not empty; use a fresh root"
}

dr_scratch_compose() {
  docker compose --project-name "$DR_SCRATCH_PROJECT" --env-file "$DR_SCRATCH_ENV" \
    -f "$root/compose.yaml" -f "$root/deploy/v3/dr/compose.scratch.yaml" "$@"
}

dr_scratch_wait_service() {
  local service=$1 attempts=${2:-90} attempt=0 id status
  while [ "$attempt" -lt "$attempts" ]; do
    attempt=$((attempt + 1))
    id=$(dr_scratch_compose ps -q "$service" 2>/dev/null || true)
    if [ -n "$id" ]; then
      status=$(docker inspect -f '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}' "$id" 2>/dev/null || true)
      [ "$status" = "healthy" ] && return 0
      [ "$status" = "running" ] && return 0
    fi
    sleep 2
  done
  return 1
}

dr_scratch_restore() {
  mkdir -p "$DR_PGDATA"
  chmod 700 "$DR_PGDATA"
  [ -z "$(find "$DR_PGDATA" -mindepth 1 -print -quit 2>/dev/null)" ] \
    || dr_unverified "the scratch PGDATA $DR_PGDATA is not empty"
  dr_restore_pgdata "$DR_PGDATA" scratch
  # The Postgres image runs as UID 999. The one-shot restore may have written
  # root-owned files; the scratch data directory is the only path changed here.
  docker run --rm --user 0 -v "$DR_PGDATA:/data" \
    --entrypoint chown "$DR_PG_IMAGE" -R 999:999 /data \
    || dr_unverified "cannot set the scratch PGDATA owner"
}

dr_scratch_start_infra() {
  dr_say "scratch: starting pki-init, minio, rabbitmq in $DR_SCRATCH_PROJECT"
  dr_scratch_compose up -d pki-init minio rabbitmq \
    || dr_unverified "the scratch infrastructure did not start"
  for service in minio rabbitmq; do
    dr_scratch_wait_service "$service" \
      || dr_unverified "the scratch $service did not become healthy"
  done
}

dr_scratch_start_postgres() {
  dr_say "scratch: starting postgres on the restored PGDATA"
  dr_scratch_compose up -d postgres \
    || dr_unverified "the scratch postgres did not start"
  dr_scratch_wait_service postgres \
    || dr_unverified "the scratch postgres did not become healthy"
}

dr_scratch_object_restore() {
  if [ "$DR_SKIP_OBJECTS" -eq 1 ]; then
    dr_fact "object_status" "Unverified"
    DR_ROWS="${DR_ROWS}object=Unverified
"
    dr_say "object restore: skipped (--skip-objects); row Unverified"
    return 0
  fi
  local source_url=${QR_DR_ARCHIVE_SOURCE_URL:-}
  local source_access=${QR_DR_ARCHIVE_SOURCE_ACCESS_KEY_FILE:-}
  local source_secret=${QR_DR_ARCHIVE_SOURCE_SECRET_KEY_FILE:-}
  if [ -z "$source_url" ] || [ -z "$source_access" ] || [ -z "$source_secret" ]; then
    dr_fact "object_status" "Unverified"
    DR_ROWS="${DR_ROWS}object=Unverified
"
    dr_say "object restore: skipped; set QR_DR_ARCHIVE_SOURCE_URL/ACCESS_KEY_FILE/SECRET_KEY_FILE"
    return 0
  fi
  [ -n "$DR_ARCHIVE_TARGET" ] || dr_unverified "backup.object_archive.target is empty; no archive to restore"
  QR_OBJECT_ARCHIVE_SOURCE_URL="$source_url" \
  QR_OBJECT_ARCHIVE_SOURCE_BUCKET="$DR_ARCHIVE_TARGET" \
  QR_OBJECT_ARCHIVE_SOURCE_ACCESS_KEY_FILE="$source_access" \
  QR_OBJECT_ARCHIVE_SOURCE_SECRET_KEY_FILE="$source_secret" \
  QR_OBJECT_ARCHIVE_TARGET_URL="https://minio:9000" \
  QR_OBJECT_ARCHIVE_TARGET_BUCKET="$DR_SCRATCH_BUCKET" \
  QR_OBJECT_ARCHIVE_TARGET_ACCESS_KEY_FILE="$DR_RENDER/secrets/minio_root_user" \
  QR_OBJECT_ARCHIVE_TARGET_SECRET_KEY_FILE="$DR_RENDER/secrets/minio_root_password" \
  QR_OBJECT_ARCHIVE_IMAGE="$DR_MINIO_CLIENT_IMAGE" \
  QR_OBJECT_ARCHIVE_NETWORK="${DR_SCRATCH_PROJECT}_quantrule" \
  QR_OBJECT_ARCHIVE_CA="$DR_SCRATCH_PKI/minio/CAs/ca.crt" \
  QR_OBJECT_ARCHIVE_EVIDENCE="$DR_WORK/object-restore.json" \
    ./tools/object-archive.sh --restore --confirm \
    || dr_unverified "the archived objects were not restored into $DR_SCRATCH_BUCKET"
  dr_fact "object_status" "Measured"
  DR_ROWS="${DR_ROWS}object=Measured
"
  if [ -f "$DR_WORK/object-restore.json" ]; then
    dr_fact_num "object_count" "$(jq -r '.objects.count // 0' "$DR_WORK/object-restore.json")"
    dr_fact_num "object_bytes" "$(jq -r '.objects.bytes // 0' "$DR_WORK/object-restore.json")"
    dr_fact "object_manifest_sha256" "$(jq -r '.manifest.sha256 // ""' "$DR_WORK/object-restore.json")"
  fi
}

dr_scratch_app_user() {
  local work="$DR_WORK/mc"
  mkdir -p "$work"
  chmod 700 "$work"
  printf '{"version":"10","aliases":{"local":{"url":"https://minio:9000","accessKey":"%s","secretKey":"%s","api":"S3v4","path":"auto"}}}\n' \
    "$DR_SCRATCH_MINIO_ROOT_USER" "$DR_SCRATCH_MINIO_ROOT_PASSWORD" > "$work/config.json"
  chmod 600 "$work/config.json"
  local script='
set -eu
secret=$(cat /secrets/app_secret)
mc --config-dir /mc admin policy detach local quantrule-app --user qr_app >/dev/null 2>&1 || true
mc --config-dir /mc admin policy remove local quantrule-app >/dev/null 2>&1 || true
mc --config-dir /mc admin policy create local quantrule-app /policy.json >/dev/null
mc --config-dir /mc admin policy attach local quantrule-app --user qr_app >/dev/null
if ! printf "%s\n%s\n" qr_app "$secret" | mc --config-dir /mc admin user add local >/dev/null 2>&1; then
  printf "dr: qr_app exists; the app will verify its secret\n" >&2
fi
'
  docker run --rm --network "${DR_SCRATCH_PROJECT}_quantrule" \
    --entrypoint /bin/sh \
    -v "$work:/mc:ro" \
    -v "$DR_RENDER/minio-policy.json:/policy.json:ro" \
    -v "$DR_RENDER/secrets/object_store_secret_key:/secrets/app_secret:ro" \
    -v "$DR_SCRATCH_PKI/minio/CAs/ca.crt:/certs/ca.crt:ro" \
    -e SSL_CERT_FILE=/certs/ca.crt \
    "$DR_MINIO_CLIENT_IMAGE" -c "$script" \
    || dr_unverified "cannot create the scratch qr_app user"
}

dr_scratch_queue_user() {
  local script='
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
[ -n "$hash" ] || exit 2
result=$(rabbitmqctl -q eval "case rabbit_auth_backend_internal:add_user(<<\"$user\">>, <<\"$hash\">>) of ok -> ok; {error, user_already_exists} -> rabbit_auth_backend_internal:change_password(<<\"$user\">>, <<\"$hash\">>); Error -> Error end.")
[ "$result" = "ok" ]
rabbitmqctl -q set_permissions -p "$vhost" "$user" ".*" ".*" ".*" >/dev/null
'
  printf '%s\n' "$DR_SCRATCH_QUEUE_URL" \
    | dr_scratch_compose exec -T rabbitmq sh -c "$script" \
    || dr_unverified "cannot create the scratch queue application user"
}

dr_scratch_start_app() {
  local start
  start=$(dr_now_epoch)
  dr_say "scratch: starting the seven services against the restored PGDATA and bucket"
  dr_scratch_compose up -d --no-deps model data strategy evidence simulation backtest gateway \
    || dr_unverified "the scratch application services did not start"
  local attempt=0
  while [ "$attempt" -lt 90 ]; do
    attempt=$((attempt + 1))
    if curl -fsS --max-time 2 "http://127.0.0.1:$DR_SCRATCH_PORT/health" 2>/dev/null \
      | grep -q '"state":"healthy"'; then
      dr_startup_timing "$start"
      DR_API_URL="http://127.0.0.1:$DR_SCRATCH_PORT/api/v1"
      return 0
    fi
    sleep 2
  done
  dr_unverified "the scratch gateway did not become healthy on 127.0.0.1:$DR_SCRATCH_PORT"
}

dr_scratch_queue_counts() {
  if [ "$DR_SKIP_QUEUE" -eq 1 ]; then
    dr_fact "queue_status" "Unverified"
    dr_fact_num "queue_rearmed" -1
    dr_fact_num "queue_replayed_runs" -1
    dr_fact_num "queue_replayed_stages" -1
    DR_ROWS="${DR_ROWS}queue=Unverified
"
    dr_say "queue: skipped (--skip-queue); row Unverified"
    return 0
  fi
  local logs rearmed runs stages
  logs=$(dr_scratch_compose logs --no-color --tail 2000 evidence 2>&1 || true)
  rearmed=$(printf '%s\n' "$logs" | sed -n 's/.*rearmed["]*[=:][[:space:]]*\([0-9][0-9]*\).*/\1/p' | awk '{ sum += $1 } END { print sum + 0 }')
  runs=$(printf '%s\n' "$logs" | grep -c 'run notifications reconciled' || true)
  stages=$(printf '%s\n' "$logs" | grep -c 'stage notifications reconciled' || true)
  if [ "$runs" -eq 0 ] && [ "$stages" -eq 0 ]; then
    dr_fact_num "queue_rearmed" -1
    dr_fact_num "queue_replayed_runs" -1
    dr_fact_num "queue_replayed_stages" -1
    dr_fact "queue_status" "Unverified"
    DR_ROWS="${DR_ROWS}queue=Unverified
"
    return 0
  fi
  dr_fact_num "queue_rearmed" "$rearmed"
  dr_fact_num "queue_replayed_runs" "$runs"
  dr_fact_num "queue_replayed_stages" "$stages"
  dr_fact "queue_status" "Measured"
  DR_ROWS="${DR_ROWS}queue=Measured
"
}

dr_scratch_run() {
  dr_scratch_preflight
  dr_scratch_prepare
  dr_scratch_start_infra
  dr_scratch_object_restore
  dr_scratch_restore
  dr_scratch_start_postgres
  dr_scratch_app_user
  dr_scratch_queue_user
  dr_scratch_start_app
  dr_scratch_queue_counts
  DR_ROWS="${DR_ROWS}restore=Measured
startup=Measured
"
  dr_fact "scratch_api_url" "$DR_API_URL"
}
