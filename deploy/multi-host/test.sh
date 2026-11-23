#!/usr/bin/env bash
# Local multi-host approximation for the six services.
#
# This is not the real three-host LAN verification. It puts the six services on
# one Docker network with distinct addresses and distinct ports, uses the same
# image and the same explicit URLs a multi-host deployment uses, and proves:
#
#   - one image digest on every service;
#   - only the gateway is published to the host;
#   - the fixture run completes and the canonical digest matches across two
#     runs with one and two backtest workers.
#
# This harness is a placement approximation, not the TLS contract: it starts
# the stateful services without certificates and is not evidence for HA-010.
# Composed TLS handshakes and CA overlap are later composed/HA evidence.
#
# The real LAN verification (host firewall, overlay, cross-host failure) is
# tracked separately in #964. Run this from the repository root:
#
#   deploy/multi-host/test.sh
#
# Requirements: Docker, the quantrule-server image, curl, and the fixture data.
# The image is not built here; use `make up` or `docker build` first.
set -euo pipefail
umask 077

root=$(git rev-parse --show-toplevel)
cd "$root"
image=${QR_IMAGE:-quantrule-server:local}
network=qr-mh-test
prefix=qr-mh-test
work=$(mktemp -d "${TMPDIR:-/tmp}/quantrule-mh.XXXXXX")
gateway_port=18080
digest=$(docker image inspect --format '{{.Id}}' "$image")
db_password=test-db-password
object_password=test-object-password
queue_password=test-queue-password

cleanup() {
  local status=$?
  trap - EXIT
  docker ps -aq --filter "name=$prefix" | xargs -r docker rm -f >/dev/null 2>&1 || true
  docker network rm "$network" >/dev/null 2>&1 || true
  rm -rf "$work"
  exit "$status"
}
trap cleanup EXIT

fail() {
  printf 'multi-host test: %s\n' "$*" >&2
  exit 1
}

docker image inspect "$image" >/dev/null 2>&1 ||
  fail "image $image is missing; run make up or docker build first"
command -v curl >/dev/null || fail 'curl is required'

docker ps -aq --filter "name=$prefix" | xargs -r docker rm -f >/dev/null
docker network rm "$network" >/dev/null 2>&1 || true
docker network create --subnet 172.31.238.0/24 "$network" >/dev/null

common_env() {
  cat <<EOF
QR_PROFILE=server
QR_IMAGE_DIGEST=$digest
QR_OBJECT_STORE_URL=http://172.31.238.21:9000
QR_OBJECT_BUCKET=quantrule
QR_QUEUE_GENERATION=1
EOF
}

database_url() {
  local role=$1 database=$2
  printf 'postgres://%s:%s@172.31.238.20:5432/%s' "$role" "$db_password" "$database"
}

database_env() {
  local database=$1 service=$2
  printf 'QR_DATABASE_URL_FILE=/run/secrets/%s_database_url\n' "$service"
  printf '%s\n' "$(database_url "${database}_app" "$database")" > "$work/secrets/${service}_database_url"
}

database_owner_env() {
  local database=$1 service=$2
  printf 'QR_DATABASE_OWNER_URL_FILE=/run/secrets/%s_owner_database_url\n' "$service"
  printf '%s\n' "$(database_url "${database}_owner" "$database")" \
    > "$work/secrets/${service}_owner_database_url"
}

object_store_env() {
  cat <<EOF
QR_OBJECT_STORE_ACCESS_KEY_FILE=/run/secrets/object_store_access_key
QR_OBJECT_STORE_SECRET_KEY_FILE=/run/secrets/object_store_secret_key
EOF
}

queue_env() {
  printf 'QR_QUEUE_URL_FILE=/run/secrets/queue_url\n'
}

write_env() {
  local name=$1
  {
    common_env
    case "$name" in
      data)
        cat <<EOF
QR_BIND=172.31.238.11:18081
QR_INTERNAL_ONLY=1
QR_MIGRATE=0
QR_CONFIG_ROOT=/srv/config
$(database_env qr_catalog data)
$(object_store_env)
EOF
        ;;
      model)
        cat <<EOF
QR_BIND=172.31.238.12:18082
QR_INTERNAL_ONLY=1
QR_MIGRATE=0
QR_DATA_URL=http://172.31.238.11:18081
$(object_store_env)
EOF
        ;;
      evidence)
        cat <<EOF
QR_BIND=172.31.238.14:18083
QR_INTERNAL_ONLY=1
QR_MIGRATE=0
QR_DATA_URL=http://172.31.238.11:18081
$(database_env qr_evidence evidence)
$(queue_env)
$(object_store_env)
EOF
        ;;
      simulation)
        cat <<EOF
QR_BIND=172.31.238.15:18084
QR_INTERNAL_ONLY=1
QR_MIGRATE=0
$(database_env qr_simulation simulation)
$(object_store_env)
EOF
        ;;
      backtest)
        cat <<EOF
QR_MIGRATE=0
QR_BACKTEST_CONCURRENCY=1
QR_DATA_URL=http://172.31.238.11:18081
QR_MODEL_URL=http://172.31.238.12:18082
QR_EVIDENCE_URL=http://172.31.238.14:18083
$(queue_env)
$(object_store_env)
EOF
        ;;
      gateway)
        cat <<EOF
QR_BIND=0.0.0.0:$gateway_port
QR_INTERNAL_ONLY=0
QR_MIGRATE=0
QR_STATIC_DIR=/srv/static
QR_DATA_URL=http://172.31.238.11:18081
QR_MODEL_URL=http://172.31.238.12:18082
QR_EVIDENCE_URL=http://172.31.238.14:18083
QR_SIMULATION_URL=http://172.31.238.15:18084
$(database_env qr_gateway gateway)
EOF
        ;;
      *)
        fail "unknown env $name"
        ;;
    esac
  } > "$work/$name.env"
}

write_migrate_env() {
  local name=$1 database=$2
  {
    printf 'QR_PROFILE=server\n'
    printf 'QR_MIGRATE=0\n'
    database_owner_env "$database" "$name"
  } > "$work/$name-migrate.env"
}

run_service() {
  local name=$1 ip=$2 binary=$3
  shift 3
  docker run -d --name "$prefix-$name" --network "$network" --ip "$ip" \
    --env-file "$work/$name.env" -v "$work/secrets:/run/secrets:ro" \
    "$@" "$image" "$binary" >/dev/null
}

run_migration() {
  local name=$1 binary=$2
  docker run --rm --network "$network" \
    --env-file "$work/$name-migrate.env" -v "$work/secrets:/run/secrets:ro" \
    "$image" "$binary" --migrate-only
}

wait_exec() {
  local name=$1
  shift
  for _ in $(seq 1 60); do
    if docker exec "$prefix-$name" "$@" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  docker logs "$prefix-$name" >&2 || true
  fail "$name did not become ready"
}

wait_http() {
  local name=$1 url=$2
  for _ in $(seq 1 60); do
    if docker exec "$prefix-$name" curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  docker logs "$prefix-$name" >&2 || true
  fail "$name did not answer $url"
}

mkdir -p "$work/secrets"
chmod 700 "$work/secrets"
printf 'amqp://quantrule:%s@172.31.238.22:5672/%%2f\n' "$queue_password" \
  > "$work/secrets/queue_url"
printf 'quantrule\n' > "$work/secrets/object_store_access_key"
printf '%s\n' "$object_password" > "$work/secrets/object_store_secret_key"
chmod 600 "$work/secrets/"*

for name in data model evidence simulation backtest gateway; do
  write_env "$name"
done
write_migrate_env data qr_catalog
write_migrate_env evidence qr_evidence
write_migrate_env simulation qr_simulation
write_migrate_env gateway qr_gateway
for name in data evidence simulation gateway; do
  if grep -q '^QR_DATABASE_OWNER_URL=' "$work/$name.env"; then
    fail "$name.env carries the owner credential"
  fi
done
cp "$work/backtest.env" "$work/backtest-2.env"

echo 'multi-host test: infrastructure'
docker run -d --name "$prefix-postgres" --network "$network" --ip 172.31.238.20 \
  -e POSTGRES_USER=quantrule -e POSTGRES_PASSWORD="$db_password" -e POSTGRES_DB=quantrule \
  "${QR_POSTGRES_IMAGE:-postgres@sha256:846a7ddaa67a4b354ae1d21087b683901aa6113a4ed626d3787cfbf6c85e3e07}" >/dev/null
docker run -d --name "$prefix-minio" --network "$network" --ip 172.31.238.21 \
  -e MINIO_ROOT_USER=quantrule -e MINIO_ROOT_PASSWORD="$object_password" \
  -e MINIO_DEFAULT_BUCKETS=quantrule -e MINIO_BROWSER=off \
  "${QR_OBJECT_STORE_IMAGE:-bitnamilegacy/minio@sha256:451fe6858cb770cc9d0e77ba811ce287420f781c7c1b806a386f6896471a349c}" >/dev/null
docker run -d --name "$prefix-rabbitmq" --network "$network" --ip 172.31.238.22 \
  -e RABBITMQ_DEFAULT_USER=quantrule -e RABBITMQ_DEFAULT_PASS="$queue_password" \
  -e RABBITMQ_DEFAULT_VHOST=/ \
  "${QR_RABBITMQ_IMAGE:-rabbitmq@sha256:753099a19b577c322117f793f864b3129cbba96d40317221a69d73976e3c843c}" >/dev/null
wait_exec postgres pg_isready -U quantrule -d quantrule
wait_exec minio curl -fsS http://127.0.0.1:9000/minio/health/ready
wait_exec rabbitmq rabbitmq-diagnostics -q ping

echo 'multi-host test: database bootstrap and role isolation'
postgres_image=${QR_POSTGRES_IMAGE:-postgres@sha256:846a7ddaa67a4b354ae1d21087b683901aa6113a4ed626d3787cfbf6c85e3e07}
docker run --rm --network "$network" \
  -e PGHOST=172.31.238.20 -e PGPORT=5432 \
  -e PGUSER=quantrule -e PGPASSWORD="$db_password" -e PGDATABASE=quantrule \
  -e QR_DB_ROLE_PASSWORD="$db_password" \
  -v "$root/deploy/postgres/bootstrap.sh:/bootstrap/bootstrap.sh:ro" \
  --entrypoint /bin/sh "$postgres_image" /bootstrap/bootstrap.sh
for spec in \
  "qr_gateway_app qr_catalog" \
  "qr_catalog_app qr_evidence" \
  "qr_evidence_app qr_simulation" \
  "qr_simulation_app qr_gateway"; do
  set -- $spec
  if docker exec -e PGPASSWORD="$db_password" "$prefix-postgres" \
    psql -U "$1" -d "$2" -c 'SELECT 1' >/dev/null 2>&1; then
    fail "$1 reached $2 across databases"
  fi
done
if docker exec -e PGPASSWORD="$db_password" "$prefix-postgres" \
  psql -U qr_catalog_app -d qr_catalog -v ON_ERROR_STOP=1 \
  -c 'CREATE TABLE catalog.role_probe (id integer)' >/dev/null 2>&1; then
  fail 'qr_catalog_app created a table; the application role has DDL'
fi

echo 'multi-host test: one-shot owner migrations'
run_migration data qr-data
run_migration evidence qr-evidence
run_migration simulation qr-simulation
run_migration gateway qr-gateway

echo 'multi-host test: core and compute'
run_service data 172.31.238.11 qr-data -v "$root/config:/srv/config:ro"
wait_http data http://172.31.238.11:18081/health
run_service model 172.31.238.12 qr-model
wait_http model http://172.31.238.12:18082/health
run_service evidence 172.31.238.14 qr-evidence
wait_http evidence http://172.31.238.14:18083/health
run_service simulation 172.31.238.15 qr-simulation
wait_http simulation http://172.31.238.15:18084/health
run_service backtest 172.31.238.13 qr-backtest

echo 'multi-host test: gateway'
run_service gateway 172.31.238.10 qr-gateway -p "127.0.0.1:$gateway_port:$gateway_port"
for _ in $(seq 1 60); do
  if curl -fsS "http://127.0.0.1:$gateway_port/ready" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
curl -fsS "http://127.0.0.1:$gateway_port/ready" >/dev/null ||
  fail 'the gateway did not become ready'

published=$(docker ps --filter "name=$prefix" --format '{{.Names}} {{.Ports}}' | grep -v "^$prefix-gateway " | grep -c -- '->' || true)
[ "$published" -eq 0 ] || fail 'a non-gateway service published a host port'
for spec in "gateway 172.31.238.10 18080" "data 172.31.238.11 18081" "model 172.31.238.12 18082" "evidence 172.31.238.14 18083" "simulation 172.31.238.15 18084"; do
  set -- $spec
  name=$1; ip=$2; port=$3
  reported=$(docker exec "$prefix-$name" curl -fsS "http://$ip:$port/version" |
    sed -n 's/.*"image_digest"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
  [ "$reported" = "$digest" ] || fail "$name digest $reported != $digest"
done

echo 'multi-host test: first verification (one worker)'
first=$(tools/verify.sh --api-url "http://127.0.0.1:$gateway_port/api/v1" |
  grep -o 'sha256:[0-9a-f]\{64\}' | tail -1)
[ -n "$first" ] || fail 'the first verification printed no digest'

echo 'multi-host test: second verification (two workers)'
run_service backtest-2 172.31.238.16 qr-backtest
second=$(tools/verify.sh --api-url "http://127.0.0.1:$gateway_port/api/v1" |
  grep -o 'sha256:[0-9a-f]\{64\}' | tail -1)
[ "$first" = "$second" ] || fail "digest changed with two workers: $first != $second"

printf 'multi-host test: ok, canonical digest %s\n' "$first"
