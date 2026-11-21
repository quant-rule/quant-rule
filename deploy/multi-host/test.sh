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
# The real LAN verification (host firewall, overlay, cross-host failure) is
# tracked separately in #964. Run this from the repository root:
#
#   deploy/multi-host/test.sh
#
# Requirements: Docker, the quantrule-server image, curl, and the fixture data.
# The image is not built here; use `make up` or `docker build` first.
set -euo pipefail

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
QR_DATABASE_URL=postgres://quantrule:$db_password@172.31.238.20:5432/quantrule
QR_OBJECT_STORE_URL=http://172.31.238.21:9000
QR_OBJECT_BUCKET=quantrule
QR_QUEUE_URL=amqp://quantrule:$queue_password@172.31.238.22:5672/%2f
AWS_ACCESS_KEY_ID=quantrule
AWS_SECRET_ACCESS_KEY=$object_password
AWS_REGION=us-east-1
EOF
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
QR_MIGRATE=1
QR_CONFIG_ROOT=/srv/config
EOF
        ;;
      model)
        cat <<EOF
QR_BIND=172.31.238.12:18082
QR_INTERNAL_ONLY=1
QR_MIGRATE=0
QR_DATA_URL=http://172.31.238.11:18081
EOF
        ;;
      evidence)
        cat <<EOF
QR_BIND=172.31.238.14:18083
QR_INTERNAL_ONLY=1
QR_MIGRATE=1
QR_DATA_URL=http://172.31.238.11:18081
EOF
        ;;
      simulation)
        cat <<EOF
QR_BIND=172.31.238.15:18084
QR_INTERNAL_ONLY=1
QR_MIGRATE=1
EOF
        ;;
      backtest)
        cat <<EOF
QR_MIGRATE=0
QR_BACKTEST_CONCURRENCY=1
QR_DATA_URL=http://172.31.238.11:18081
QR_MODEL_URL=http://172.31.238.12:18082
QR_EVIDENCE_URL=http://172.31.238.14:18083
EOF
        ;;
      gateway)
        cat <<EOF
QR_BIND=0.0.0.0:$gateway_port
QR_INTERNAL_ONLY=0
QR_MIGRATE=1
QR_STATIC_DIR=/srv/static
QR_DATA_URL=http://172.31.238.11:18081
QR_MODEL_URL=http://172.31.238.12:18082
QR_EVIDENCE_URL=http://172.31.238.14:18083
QR_SIMULATION_URL=http://172.31.238.15:18084
EOF
        ;;
      *)
        fail "unknown env $name"
        ;;
    esac
  } > "$work/$name.env"
}

run_service() {
  local name=$1 ip=$2 binary=$3
  shift 3
  docker run -d --name "$prefix-$name" --network "$network" --ip "$ip" \
    --env-file "$work/$name.env" "$@" "$image" "$binary" >/dev/null
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

for name in data model evidence simulation backtest gateway; do
  write_env "$name"
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
