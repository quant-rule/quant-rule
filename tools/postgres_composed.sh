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

image="${QR_TEST_POSTGRES_IMAGE:-postgres:17}"
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
  "quantrule-evidence-service research_ledger"
  "quantrule-evidence-service stage_conformance"
  "quantrule-evidence-service sqlite_conformance"
  "quantrule-gateway profile_conformance"
  "quantrule-runtime migration_lock"
  "quantrule-simulation-service sqlite_conformance"
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

echo "postgres tier: $index targets passed against a disposable Postgres"
