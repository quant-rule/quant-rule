#!/usr/bin/env bash
# Round-trip test for tools/backup.sh and tools/restore.sh.
#
# Opt-in and destructive to its own Compose project only. It starts the default
# stack under a separate project name, creates a dataset, a catalogued config,
# a sealed hypothesis, a run with artifacts, and one simulation row; backs up
# the four logical databases and the objects; checks the refusals; wipes the
# project's volumes; restores; and compares every application row, every object,
# and every run artifact byte-for-byte.
#
# The run row is seeded directly because the current gateway cannot submit or
# read a run: the Postgres dataset row decodes `quality` and `created_at` in the
# wrong order, and migration 0013 is not in the compiled migration list. Both
# are pre-existing defects outside #671. The bytes and rows seeded here are the
# same rows and objects the worker would publish; this test exercises the
# backup and restore path, not the run path.
#
# Usage: tools/test_backup.sh [--keep] [--plan]
#   --keep   leave the test stack and its work directory for inspection
#   --plan   print the planned four-database steps without Docker, a build, or
#            a port check
#
# Requires: Linux host tools (GNU date, sha256sum), Docker with Compose,
# cargo, curl, and tar. Refuses when port 8080 is already served by another
# project or process.
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "$script_dir/.." && pwd)
# shellcheck source=tools/backup_lib.sh
. "$script_dir/backup_lib.sh"

project=${QR_TEST_PROJECT:-quantrule-backup-test}
case "$project" in
  *test*) ;;
  *) backup_die "QR_TEST_PROJECT must name a test project; refusing '$project'" ;;
esac
export COMPOSE_PROJECT_NAME=$project

keep=0
plan=0
while [ $# -gt 0 ]; do
  case "$1" in
    --keep) keep=1; shift ;;
    --plan) plan=1; shift ;;
    -h | --help)
      sed -n '2,25p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) backup_die "unknown argument: $1" ;;
  esac
done

if [ "$plan" -eq 1 ]; then
  cat <<PLAN
backup test: plan (no Docker, build, network, or secret file is read)
backup test: project ${QR_TEST_PROJECT:-quantrule-backup-test}
backup test: 1 start the isolated Compose stack and wait for /health
backup test: 2 seed qr_gateway, qr_catalog, qr_evidence, and qr_simulation rows and objects
backup test: 3 snapshot every table digest and every object hash
backup test: 4 run tools/backup.sh and verify the four dumps and the sidecar
backup test: 5 refuse a non-interactive restore, a changed payload, a missing dump, and a non-clean target
backup test: 6 wipe the project, run tools/restore.sh --yes, start the stack, and compare rows, objects, and the run digest
backup test: 7 databases: $BACKUP_DATABASES
backup test: the queue is not archived; acknowledged messages stay in the quorum queues
PLAN
  exit 0
fi

api=http://127.0.0.1:8080
work=$(mktemp -d "${TMPDIR:-/tmp}/quantrule-backup-test.XXXXXX")
shell_bin="$root/target/debug/quantrule"
export QUANTRULE_API_URL=$api
export QUANTRULE_HOME="$work/home"
mkdir -p "$QUANTRULE_HOME"

step() {
  printf '\n== %s\n' "$*" >&2
}

fail() {
  printf 'FAIL: %s\n' "$*" >&2
  exit 1
}

wait_health() {
  local attempt
  for attempt in $(seq 1 60); do
    if curl -fsS --max-time 2 "$api/health" 2>/dev/null | grep -q '"state":"healthy"'; then
      return 0
    fi
    sleep 2
  done
  backup_compose ps >&2 || true
  fail 'the stack did not become healthy'
}

cleanup() {
  local status=$?
  trap - EXIT
  if [ "$keep" -eq 0 ]; then
    backup_compose down -v --remove-orphans >/dev/null 2>&1 || true
    [ "$status" -eq 0 ] && rm -rf "$work"
  fi
  if [ "$status" -ne 0 ]; then
    printf 'work directory kept: %s\n' "$work" >&2
  elif [ "$keep" -eq 1 ]; then
    printf 'test stack and work directory kept: %s\n' "$work" >&2
  fi
  exit "$status"
}
trap cleanup EXIT

# Refuse to touch a stack we do not own on the published port. An old run of
# this test is torn down; another project or a host process is a refusal.
if curl -fsS --max-time 2 "$api/health" >/dev/null 2>&1; then
  owner=$(docker ps --filter publish=8080 --format '{{.Label "com.docker.compose.project"}}')
  owner=${owner%%$'\n'*}
  [ "$owner" = "$project" ] ||
    backup_die "port 8080 is served by ${owner:-a non-Compose process}, not $project"
fi

backup_require_commands docker cargo curl tar awk
step 'building the shell and starting the isolated stack'
cargo build --manifest-path "$root/Cargo.toml" -p quantrule-shell >/dev/null
backup_compose down -v --remove-orphans >/dev/null 2>&1 || true
backup_compose up -d --build --wait >/dev/null
wait_health

step 'creating a dataset through the gateway'
csv="$work/candles.csv"
{
  printf 'Date,Open,High,Low,Close,Volume\n'
  for index in $(seq 0 1439); do
    timestamp=$((1704067200 + index * 60))
    printf '%s,' "$(date -u -d "@$timestamp" +'%Y-%m-%d %H:%M:%S+00:00')"
    awk -v i="$index" 'BEGIN {
      o = 100 + (i % 20) * 0.5
      c = o + ((i % 7) - 3) * 0.1
      h = (o > c ? o : c) + 0.3
      l = (o < c ? o : c) - 0.3
      v = 10 + (i % 5)
      printf "%.2f,%.2f,%.2f,%.2f,%d\n", o, h, l, c, v
    }'
  done
} > "$csv"
upload=$("$shell_bin" dataset upload "$csv" --name backup-round-trip.csv)
data_version=$(printf '%s\n' "$upload" | sed -n 's/^data_version: //p')
[ -n "$data_version" ] || fail 'the dataset upload returned no data_version'

step 'cataloguing the repository config and sealing a hypothesis'
"$shell_bin" config list > "$work/configs.txt"
config_id=$(awk '$2 == "name=examples/verify-mini/run.json" { print $1; exit }' "$work/configs.txt")
[ -n "$config_id" ] || fail 'the verify-mini run config is not in the catalog'
curl -fsS -X POST -H 'X-QuantRule-Profile: default' \
  -H 'content-type: application/json' \
  --data "{\"premise\":\"backup round trip preserves the record\",\"rule\":\"config/examples/verify-mini/run.json\",\"dataset\":\"$data_version\",\"costs\":{\"spread_bps\":0.0,\"commission_bps\":0.0,\"slippage_bps\":2.0},\"split\":{\"development\":{\"name\":\"backup_dev\",\"start\":1704067200,\"end\":1704153600},\"validation\":{\"name\":\"backup_validation\",\"start\":1704153600,\"end\":1704240000}},\"criteria\":\"the record survives a backup round trip\"}" \
  "$api/hypotheses" >/dev/null

step 'seeding the run rows and artifact blobs'
run_id="sha256:$(printf 'backup-round-trip-run' | sha256sum | cut -d' ' -f1)"
mkdir -p "$work/seed/artifacts" "$work/seed/objects/artifacts" "$work/seed/objects/runs/$run_id"
printf '{"schema":"quantrule.backtest-results/v1","fixture":"backup-round-trip"}\n' \
  > "$work/seed/results.json"
printf 'bar,side,price\n1,long,100.0\n2,short,101.0\n' > "$work/seed/trades.csv"
printf '{"schema":"quantrule.provenance/v1","note":"seeded durable state for the backup round trip"}\n' \
  > "$work/seed/provenance.json"

results_hash=$(backup_sha256 "$work/seed/results.json")
trades_hash=$(backup_sha256 "$work/seed/trades.csv")
provenance_hash=$(backup_sha256 "$work/seed/provenance.json")
results_bytes=$(backup_file_bytes "$work/seed/results.json")
trades_bytes=$(backup_file_bytes "$work/seed/trades.csv")
provenance_bytes=$(backup_file_bytes "$work/seed/provenance.json")
run_digest="sha256:$({
  printf 'results.json\0sha256:%s\n' "$results_hash"
  printf 'trades.csv\0sha256:%s\n' "$trades_hash"
} | sha256sum | cut -d' ' -f1)"

cp "$work/seed/results.json" "$work/seed/objects/artifacts/$results_hash"
cp "$work/seed/trades.csv" "$work/seed/objects/artifacts/$trades_hash"
cp "$work/seed/provenance.json" "$work/seed/objects/artifacts/$provenance_hash"
cat > "$work/seed/objects/runs/$run_id/manifest.json" <<JSON
{"schema":"quantrule.run-manifest/v1","run_id":"$run_id","digest":"$run_digest","artifacts":[{"name":"provenance.json","bytes":$provenance_bytes,"sha256":"sha256:$provenance_hash"},{"name":"results.json","bytes":$results_bytes,"sha256":"sha256:$results_hash"},{"name":"trades.csv","bytes":$trades_bytes,"sha256":"sha256:$trades_hash"}]}
JSON

postgres_user=${QR_POSTGRES_USER:-quantrule}
evidence_db=qr_evidence
gateway_db=qr_gateway
catalog_db=qr_catalog
simulation_db=qr_simulation
config_hash=$(backup_compose exec -T postgres psql -qAt -U "$postgres_user" -d "$catalog_db" \
  -c "SELECT config_hash FROM catalog.configs WHERE profile_id = 'default' AND config_id = '$config_id'")
[ -n "$config_hash" ] || fail 'the catalogued config has no hash'
backup_compose exec -T postgres psql -q -v ON_ERROR_STOP=1 -U "$postgres_user" -d "$evidence_db" <<SQL
INSERT INTO runs.runs
  (run_id, config_id, data_version, owner, status, submitted_at, started_at, finished_at,
   digest, config_hash, model_version, image_digest)
VALUES
  ('$run_id', '$config_id', '$data_version', 'default', 'succeeded',
   '2026-11-14T00:00:00Z', '2026-11-14T00:00:01Z', '2026-11-14T00:00:02Z',
   '$run_digest', '$config_hash', 'M1.0',
   'sha256:0000000000000000000000000000000000000000000000000000000000000000');
INSERT INTO runs.stages (run_id, name, state, elapsed_ms) VALUES
  ('$run_id', 'data', 'done', 0),
  ('$run_id', 'model', 'done', 0),
  ('$run_id', 'backtest', 'done', 0),
  ('$run_id', 'evidence', 'done', 0);
INSERT INTO runs.artifacts (run_id, name, bytes, sha256) VALUES
  ('$run_id', 'provenance.json', $provenance_bytes, '$provenance_hash'),
  ('$run_id', 'results.json', $results_bytes, '$results_hash'),
  ('$run_id', 'trades.csv', $trades_bytes, '$trades_hash');
INSERT INTO runs.submissions (profile_id, run_id, submitted_at)
VALUES ('default', '$run_id', '2026-11-14T00:00:00Z');
SQL
# The gateway profile and diagnostic stage schemas are part of a normal stack.
# Seed one stage row so the round trip proves those schemas are restored, not
# merely present because the migration ran again.
stage_run_id="sha256:$(printf 'backup-round-trip-stage' | sha256sum | cut -d' ' -f1)"
stage_artifact_body="$work/seed/stage-model.json"
printf '{"stage":"backup"}\n' > "$stage_artifact_body"
stage_artifact_hex=$(backup_sha256 "$stage_artifact_body")
stage_artifact_bytes=$(backup_file_bytes "$stage_artifact_body")
mkdir -p "$work/seed/objects/stages/blobs"
cp "$stage_artifact_body" "$work/seed/objects/stages/blobs/$stage_artifact_hex"
backup_compose exec -T postgres psql -q -v ON_ERROR_STOP=1 -U "$postgres_user" -d "$evidence_db" <<SQL
INSERT INTO stage.runs
  (stage_run_id, kind, status, owner, config_id, input_kind, input_digest,
   data_version, config_hash, model_version, image_digest, artifact_id, submitted_at)
VALUES
  ('$stage_run_id', 'model', 'succeeded', 'default', '$config_id', 'data_version',
   '$data_version', '$data_version', '$config_hash', 'M1.3',
   'sha256:0000000000000000000000000000000000000000000000000000000000000000',
   'sha256:$stage_artifact_hex', '2026-11-14T00:00:00Z');
INSERT INTO stage.submissions (profile_id, stage_run_id, submitted_at)
VALUES ('default', '$stage_run_id', '2026-11-14T00:00:00Z');
INSERT INTO stage.artifacts (stage_run_id, name, bytes, sha256)
VALUES ('$stage_run_id', 'model.json', $stage_artifact_bytes, '$stage_artifact_hex');
SQL
# One simulation row and one mark prove the research-plane database is in the
# archive even though no archive tool runs against it.
backup_compose exec -T postgres psql -q -v ON_ERROR_STOP=1 -U "$postgres_user" -d "$simulation_db" <<SQL
INSERT INTO simulation.runs
  (sim_id, profile_id, premise, bundle, inputs, inputs_digest, seed, criteria,
   window_spec, oracle_spec, check_config, status, created_at)
VALUES
  ('sim-backup-round-trip', 'default', 'backup round trip preserves the mark',
   'refs/bundles/backup-round-trip', '$data_version', '$data_version', 7,
   'the mark survives a backup round trip',
   '{"kind":"window","bars":1440}', '{"kind":"oracle"}', '{"kind":"check"}',
   'reviewed', '2026-11-14T00:00:00Z');
INSERT INTO simulation.marks
  (mark_id, sim_id, profile_id, chart, bar, marked_time_utc, action, quality, side,
   origin, note, created_at)
VALUES
  ('mark-backup-round-trip', 'sim-backup-round-trip', 'default', 1, 42,
   '2026-11-14T00:00:00Z', 'entry', 'good', 'long', 'human',
   'seeded durable state for the backup round trip', '2026-11-14T00:00:00Z');
SQL
tar -C "$work/seed/objects" -cf - . | backup_minio_run \
  'rm -rf /tmp/qr-test-seed
   mkdir -p /tmp/qr-test-seed
   tar -C /tmp/qr-test-seed -xf -
   mc mirror --overwrite --quiet /tmp/qr-test-seed "qr-backup/$QR_BACKUP_BUCKET" >/dev/null'

table_digest() {
  local database=$1 schema=$2 table=$3
  backup_compose exec -T postgres psql -qAt -U "$postgres_user" -d "$database" -c \
    "SET TIME ZONE 'UTC';
     SELECT '$database.$schema.$table', count(*),
            coalesce(md5(string_agg(t::text, E'\n' ORDER BY t::text)), '-')
       FROM $schema.$table t"
}

table_digests() {
  table_digest "$gateway_db" gateway profiles
  table_digest "$catalog_db" catalog profiles
  table_digest "$catalog_db" catalog datasets
  table_digest "$catalog_db" catalog configs
  table_digest "$catalog_db" catalog config_bundles
  table_digest "$evidence_db" runs runs
  table_digest "$evidence_db" runs stages
  table_digest "$evidence_db" runs artifacts
  table_digest "$evidence_db" runs submissions
  table_digest "$evidence_db" research hypotheses
  table_digest "$evidence_db" stage runs
  table_digest "$evidence_db" stage submissions
  table_digest "$evidence_db" stage artifacts
  table_digest "$simulation_db" simulation runs
  table_digest "$simulation_db" simulation marks
}

object_hashes() {
  backup_minio_run 'rm -rf /tmp/qr-test-hash
    mkdir -p /tmp/qr-test-hash
    mc mirror --overwrite --quiet "qr-backup/$QR_BACKUP_BUCKET" /tmp/qr-test-hash >/dev/null
    cd /tmp/qr-test-hash
    find . -type f | LC_ALL=C sort | while IFS= read -r file; do
      printf "%s  %s\n" "$(sha256sum "$file" | cut -d" " -f1)" "${file#./}"
    done'
}

snapshot() {
  local directory="$1"
  mkdir -p "$directory"
  table_digests > "$directory/rows.txt"
  object_hashes > "$directory/objects.txt"
  backup_compose exec -T postgres psql -qAt -U "$postgres_user" -d "$evidence_db" \
    -c "SELECT digest FROM runs.runs WHERE run_id = '$run_id'" > "$directory/run-digest.txt"
}

# Recompute the canonical digest from restored artifact rows and verify each
# blob against its recorded hash.
verify_run() {
  local listing="$work/artifact-rows.txt"
  backup_compose exec -T postgres psql -qAt -U "$postgres_user" -d "$evidence_db" \
    -c "SELECT name || '|' || sha256 FROM runs.artifacts WHERE run_id = '$run_id' ORDER BY name" \
    > "$listing"
  [ "$(wc -l < "$listing" | tr -d ' ')" -eq 3 ] || fail 'the restored run lost an artifact row'
  while IFS='|' read -r name expected; do
    [ -n "$name" ] || continue
    actual=$(backup_minio_run "mc cat --quiet \"qr-backup/\$QR_BACKUP_BUCKET/artifacts/$expected\"" </dev/null |
      sha256sum | cut -d' ' -f1)
    [ "$actual" = "$expected" ] ||
      fail "artifact $name changed: expected $expected, observed $actual"
  done < "$listing"
  canonical=$(backup_compose exec -T postgres psql -qAt -U "$postgres_user" -d "$evidence_db" \
    -c "SELECT name || '|' || sha256 FROM runs.artifacts
        WHERE run_id = '$run_id'
          AND (name = 'results.json' OR name LIKE '%.csv' OR name LIKE '%.ndjson')
        ORDER BY name")
  recomputed="sha256:$({
    while IFS='|' read -r name hash; do
      [ -n "$name" ] || continue
      printf '%s\0sha256:%s\n' "$name" "$hash"
    done <<< "$canonical"
  } | sha256sum | cut -d' ' -f1)"
  recorded=$(cat "$work/before/run-digest.txt")
  [ "$recomputed" = "$recorded" ] ||
    fail "the run digest changed: expected $recorded, observed $recomputed"
  printf 'run %s: 3 artifacts verified, digest %s\n' "$run_id" "$recomputed" >&2
}

step 'snapshotting the durable state'
snapshot "$work/before"

step 'taking the backup'
backup_dir="$work/backups"
tools/backup.sh --out "$backup_dir" >/dev/null
archive=$(find "$backup_dir" -maxdepth 1 -name 'quantrule-*.tar.gz' -print -quit)
[ -n "$archive" ] || fail 'the backup wrote no archive'
tools/restore.sh "$archive" --check > "$work/check.log"
grep -q 'databases: 4 dumps' "$work/check.log" ||
  fail 'the archive does not carry four database dumps'
wait_health

step 'checking that restore refuses an unsafe target and a bad archive'
if tools/restore.sh "$archive" </dev/null > "$work/refusal-no-yes.log" 2>&1; then
  fail 'restore accepted a non-interactive invocation without --yes'
fi
grep -q 'needs --yes' "$work/refusal-no-yes.log" ||
  fail 'the no---yes refusal did not explain itself'

corrupt="$work/corrupt"
mkdir -p "$corrupt/unpack"
tar -xzf "$archive" -C "$corrupt/unpack"
archive_root=$(basename -- "$archive" .tar.gz)
printf 'X' | dd of="$corrupt/unpack/$archive_root/postgres/qr_catalog.dump" \
  bs=1 seek=100 conv=notrunc status=none
tar -C "$corrupt/unpack" -czf "$corrupt/$archive_root.tar.gz" "$archive_root"
(cd "$corrupt" && backup_sha256_line "$archive_root.tar.gz" "$archive_root.tar.gz" > "$archive_root.tar.gz.sha256")
if tools/restore.sh "$corrupt/$archive_root.tar.gz" --yes </dev/null \
  > "$work/refusal-corrupt.log" 2>&1; then
  fail 'restore accepted a changed payload'
fi
grep -q 'hash mismatch' "$work/refusal-corrupt.log" ||
  fail 'the corrupt-payload refusal did not name the hash mismatch'

incomplete="$work/incomplete"
incomplete_unpack="$work/incomplete-unpack"
mkdir -p "$incomplete_unpack" "$incomplete"
tar -xzf "$archive" -C "$incomplete_unpack"
object_key=$(find "$incomplete_unpack/$archive_root/objects" -type f -print -quit)
[ -n "$object_key" ] || fail 'the archive holds no objects to remove'
rm -f "$object_key"
tar -C "$incomplete_unpack" -czf "$incomplete/$archive_root.tar.gz" "$archive_root"
(cd "$incomplete" && backup_sha256_line "$archive_root.tar.gz" "$archive_root.tar.gz" > "$archive_root.tar.gz.sha256")
if tools/restore.sh "$incomplete/$archive_root.tar.gz" --yes </dev/null \
  > "$work/refusal-incomplete.log" 2>&1; then
  fail 'restore accepted an archive with a missing payload file'
fi
grep -q 'incomplete' "$work/refusal-incomplete.log" ||
  fail 'the incomplete-archive refusal did not explain the missing file'

partial="$work/partial"
partial_unpack="$work/partial-unpack"
mkdir -p "$partial_unpack" "$partial"
tar -xzf "$archive" -C "$partial_unpack"
rm -f "$partial_unpack/$archive_root/postgres/qr_simulation.dump"
grep -v '^database|postgres/qr_simulation.dump|' \
  "$partial_unpack/$archive_root/$BACKUP_MANIFEST" > "$partial_unpack/$archive_root/$BACKUP_MANIFEST.new"
mv "$partial_unpack/$archive_root/$BACKUP_MANIFEST.new" "$partial_unpack/$archive_root/$BACKUP_MANIFEST"
tar -C "$partial_unpack" -czf "$partial/$archive_root.tar.gz" "$archive_root"
(cd "$partial" && backup_sha256_line "$archive_root.tar.gz" "$archive_root.tar.gz" > "$archive_root.tar.gz.sha256")
if tools/restore.sh "$partial/$archive_root.tar.gz" --check </dev/null \
  > "$work/refusal-partial.log" 2>&1; then
  fail 'restore accepted a partial four-database set'
fi
grep -q 'partial four-database set' "$work/refusal-partial.log" ||
  fail 'the partial-set refusal did not name the missing database'

if tools/restore.sh "$archive" --yes </dev/null > "$work/refusal-nonclean.log" 2>&1; then
  fail 'restore replaced a non-empty stack without --force'
fi
grep -q 'not clean' "$work/refusal-nonclean.log" ||
  fail 'the non-clean refusal did not explain the target state'
wait_health

step 'checking that the refusals changed nothing'
snapshot "$work/after-refusals"
cmp -s "$work/before/rows.txt" "$work/after-refusals/rows.txt" ||
  fail 'a refused restore changed the database rows'
cmp -s "$work/before/objects.txt" "$work/after-refusals/objects.txt" ||
  fail 'a refused restore changed the objects'

step 'wiping the project and restoring'
backup_compose down -v --remove-orphans >/dev/null
[ -z "$(backup_compose ps -q)" ] || fail 'the project still has containers after down'
tools/restore.sh "$archive" --yes >/dev/null
backup_compose up -d --wait >/dev/null
wait_health

step 'comparing the restored state'
snapshot "$work/after"
cmp -s "$work/before/rows.txt" "$work/after/rows.txt" ||
  fail 'the restored database rows differ'
cmp -s "$work/before/objects.txt" "$work/after/objects.txt" ||
  fail 'the restored objects differ'
verify_run

fingerprint=$(backup_compose exec -T postgres psql -qAt -U "$postgres_user" -d "$evidence_db" \
  -c 'SELECT fingerprint FROM research.hypotheses LIMIT 1')
[ -n "$fingerprint" ] || fail 'the restored hypothesis row is missing'
curl -fsS -H 'X-QuantRule-Profile: default' "$api/hypotheses/$fingerprint" |
  grep -q 'backup round trip preserves the record' ||
  fail 'the restored hypothesis is not readable through the gateway'

printf '\nbackup round trip ok: %s rows, %s objects, %s\n' \
  "$(wc -l < "$work/before/rows.txt" | tr -d ' ')" \
  "$(wc -l < "$work/before/objects.txt" | tr -d ' ')" \
  "$run_id"
