#!/usr/bin/env bash
# Run the ignored Postgres conformance suites against a disposable database.
#
# The fast gate compiles these tests but skips them: each needs an empty
# Postgres database and the production store's migrations. This script starts a
# throwaway Postgres container, gives every test target its own database, runs
# the ignored test, and removes the container. It is part of the composed tier.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

image="${QR_TEST_POSTGRES_IMAGE:-postgres@sha256:846a7ddaa67a4b354ae1d21087b683901aa6113a4ed626d3787cfbf6c85e3e07}"
container="quantrule-postgres-tier-$$"
cleanup() {
  docker rm -f "$container" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run --rm -d --name "$container" \
  -e POSTGRES_USER=quantrule \
  -e POSTGRES_PASSWORD=quantrule \
  -e POSTGRES_DB=postgres \
  -p 127.0.0.1::5432 \
  "$image" >/dev/null

ready=0
for _ in $(seq 1 60); do
  if docker exec "$container" pg_isready -U quantrule -d postgres >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done
[ "$ready" -eq 1 ] || {
  echo "postgres tier: the disposable database did not start" >&2
  exit 1
}

address="$(docker port "$container" 5432/tcp | head -n 1)"
port="${address##*:}"
base="postgres://quantrule:quantrule@127.0.0.1:${port}"
target_dir="${CARGO_TARGET_DIR:-$root/target/gate}"

targets=(
  "quantrule-data-service catalog_conformance"
  "quantrule-data-service d1"
  "quantrule-evidence-service outbox_conformance"
  "quantrule-evidence-service research_ledger"
  "quantrule-evidence-service stage_conformance"
  "quantrule-gateway profile_conformance"
  "quantrule-runtime migration_lock"
  "quantrule-simulation-service concurrency"
)

index=0
for target in "${targets[@]}"; do
  read -r crate test <<<"$target"
  index=$((index + 1))
  database="qr_test_${index}"
  docker exec "$container" createdb -U quantrule "$database"
  if ! QR_TEST_DATABASE_URL="$base/$database" CARGO_TARGET_DIR="$target_dir" \
    cargo test -p "$crate" --test "$test" -- --ignored --test-threads=1; then
    echo "postgres tier: $crate $test failed" >&2
    exit 1
  fi
done

# The four-database bootstrap is a deployment contract, not a Rust unit test:
# run the checked-in script against this disposable server and exercise the
# role boundary it installs.
echo 'postgres tier: four databases, owner/app roles, and no cross-database access'
docker cp deploy/postgres/bootstrap.sh "$container":/tmp/bootstrap.sh
if ! docker exec \
  -e PGHOST=127.0.0.1 -e PGPORT=5432 \
  -e PGUSER=quantrule -e PGPASSWORD=quantrule -e PGDATABASE=postgres \
  -e QR_DB_ROLE_PASSWORD=quantrule \
  "$container" sh /tmp/bootstrap.sh >/dev/null; then
  echo "postgres tier: the database bootstrap failed" >&2
  exit 1
fi

for database in qr_gateway qr_catalog qr_evidence qr_simulation; do
  if ! docker exec -e PGPASSWORD=quantrule "$container" \
    psql -U "${database}_app" -d "$database" -c 'SELECT 1' >/dev/null 2>&1; then
    echo "postgres tier: ${database}_app cannot connect to $database" >&2
    exit 1
  fi
  if ! docker exec -e PGPASSWORD=quantrule "$container" \
    psql -U "${database}_owner" -d "$database" -c 'SELECT 1' >/dev/null 2>&1; then
    echo "postgres tier: ${database}_owner cannot connect to $database" >&2
    exit 1
  fi
done

if docker exec -e PGPASSWORD=quantrule "$container" \
  psql -U qr_catalog_app -d qr_evidence -c 'SELECT 1' >/dev/null 2>&1; then
  echo "postgres tier: an application role reached another database" >&2
  exit 1
fi
if docker exec -e PGPASSWORD=quantrule "$container" \
  psql -U qr_catalog_owner -d qr_evidence -c 'SELECT 1' >/dev/null 2>&1; then
  echo "postgres tier: an owner role reached another database" >&2
  exit 1
fi
if docker exec -e PGPASSWORD=quantrule "$container" \
  psql -U qr_catalog_app -d qr_catalog -v ON_ERROR_STOP=1 \
  -c 'CREATE TABLE catalog.role_probe (id integer)' >/dev/null 2>&1; then
  echo "postgres tier: the application role created a table" >&2
  exit 1
fi
if ! docker exec -e PGPASSWORD=quantrule "$container" \
  psql -U qr_catalog_owner -d qr_catalog -v ON_ERROR_STOP=1 \
  -c 'CREATE TABLE catalog.role_probe (id bigserial PRIMARY KEY, note text)' >/dev/null; then
  echo "postgres tier: the owner role cannot create a table" >&2
  exit 1
fi
if ! docker exec -e PGPASSWORD=quantrule "$container" \
  psql -U qr_catalog_app -d qr_catalog -v ON_ERROR_STOP=1 \
  -c "INSERT INTO catalog.role_probe (note) VALUES ('app-write'); SELECT count(*) FROM catalog.role_probe" \
  >/dev/null; then
  echo "postgres tier: the application role cannot write a grant-covered table" >&2
  exit 1
fi

echo "postgres tier: $index targets passed against a disposable Postgres"
