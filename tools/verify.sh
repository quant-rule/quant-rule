#!/usr/bin/env bash
# Reviewer verification through the distributed run path.
#
# The script uploads the mini fixture and two checked-in configs through the
# gateway, submits two distinct runs with the same effective model, strategy,
# execution, and portfolio parameters, waits for both, and prints one canonical
# digest. The two configs differ only in the run name, so their config hashes
# and run ids differ while their data artifacts must not. The digest covers
# results.json, *.csv, and *.ndjson and excludes provenance, manifests, logs,
# and images, exactly as the run contract defines. Every canonical artifact is
# downloaded through the gateway and hashed; the run record's hash and the
# digest route are cross-checks.
#
# The script holds no engine and never runs a pipeline stage. Every run is
# submitted and read through the gateway; the shell is a thin HTTP client.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

config="config/examples/verify-mini/run.json"
replica="config/examples/verify-mini-replica/run.json"
data=""
expected=""
api_url="${QUANTRULE_API_URL:-http://127.0.0.1:8080/api/v1}"
profile="default"
timeout=300
self_test=0
evidence_dir=""
# The interpreter is a host tool, not a product runtime. A Windows runner
# exposes the unversioned name, so the caller may name it.
python_bin="${QR_PYTHON:-python3}"

# The canonical digest of the fixture artifact list used by --self-test.
self_test_digest="4acf7e9c323e2c30f293169bf96a41ee0387fb323d20950ad8e9ae043f2f9bb7"

usage() {
  cat <<'USAGE'
usage: tools/verify.sh [--config <run.json>] [--replica <run.json>]
                       [--data <candles.csv>] [--expected <digest>]
                       [--profile <id>] [--api-url <url>] [--timeout <seconds>]
                       [--evidence-dir <dir>] [--self-test]

Submits two configs with the same effective behavior through the gateway,
compares their canonical digests, and prints one line. The default pair is
config/examples/verify-mini/run.json and its replica, which differ only in the
run name. The data defaults to candles.csv next to the first config.

  --expected     A "canonical digest sha256:..." line, "verified sha256:...", or
                 the digest itself. A different value exits nonzero.
  --profile      The gateway profile (default: default).
  --api-url      The gateway base (default: QUANTRULE_API_URL or
                 http://127.0.0.1:8080/api/v1).
  --timeout      Seconds to wait for each run (default: 300).
  --evidence-dir Keep the downloaded canonical artifacts under <dir>/artifacts,
                 every run status under <dir>/status, the assigned config and
                 run ids in <dir>/runs.tsv, and the digest in <dir>/digest, so a
                 parity harness can compare the artifact bytes of two profiles.
  --self-test    Compare a fixed artifact list; no gateway is contacted.

The strategy is part of the run config; the old --strategy flag is gone.
Run "make up" first, or point --api-url at a healthy stack.

QR_PYTHON names the host interpreter used to build the config bundle
(default: python3), which a Windows Git Bash host may expose as python.
USAGE
}

fail() {
  printf 'verify: %s\n' "$*" >&2
  exit 1
}

usage_error() {
  printf 'verify: %s\n' "$*" >&2
  usage >&2
  exit 2
}

while [ $# -gt 0 ]; do
  case "$1" in
    --config)
      [ $# -ge 2 ] || usage_error "--config needs a value"
      config="$2"
      shift 2
      ;;
    --replica)
      [ $# -ge 2 ] || usage_error "--replica needs a value"
      replica="$2"
      shift 2
      ;;
    --data)
      [ $# -ge 2 ] || usage_error "--data needs a value"
      data="$2"
      shift 2
      ;;
    --expected)
      [ $# -ge 2 ] || usage_error "--expected needs a value"
      expected="$2"
      shift 2
      ;;
    --profile)
      [ $# -ge 2 ] || usage_error "--profile needs a value"
      profile="$2"
      shift 2
      ;;
    --api-url)
      [ $# -ge 2 ] || usage_error "--api-url needs a value"
      api_url="$2"
      shift 2
      ;;
    --timeout)
      [ $# -ge 2 ] || usage_error "--timeout needs a value"
      timeout="$2"
      shift 2
      ;;
    --evidence-dir)
      [ $# -ge 2 ] || usage_error "--evidence-dir needs a value"
      evidence_dir="$2"
      shift 2
      ;;
    --self-test)
      self_test=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage_error "unknown argument $1"
      ;;
  esac
done

case "$timeout" in
  ''|*[!0-9]*) usage_error "--timeout must be a whole number of seconds" ;;
esac
[ "$timeout" -gt 0 ] || usage_error "--timeout must be greater than zero"
api_url="${api_url%/}"

normalize_expected() {
  local value="$1"
  value="${value#canonical digest }"
  value="${value#verified }"
  case "$value" in
    sha256:*) ;;
    *) value="sha256:$value" ;;
  esac
  local hex="${value#sha256:}"
  [ ${#hex} -eq 64 ] || usage_error "an expected digest needs 64 hexadecimal characters"
  case "$hex" in
    *[!0-9a-f]*) usage_error "an expected digest contains a non-hexadecimal character" ;;
  esac
  printf '%s' "$value"
}

# The canonical digest over the canonical artifact names. The input is a list
# of "name<TAB>sha256-hex" lines; the output is the bare 64-hex digest.
canonical_digest() {
  awk -F'\t' '
    $1 == "results.json" || $1 ~ /\.csv$/ || $1 ~ /\.ndjson$/ { print $1 "\t" $2 }
  ' | LC_ALL=C sort | while IFS=$'\t' read -r name sha; do
    [ -n "$name" ] || continue
    sha="${sha#sha256:}"
    printf '%s\0sha256:%s\n' "$name" "$sha"
  done | sha256sum | cut -d' ' -f1
}

canonical_artifacts() {
  printf '%s\n' "$1" \
    | sed -n 's/^artifact \(.*\): bytes=[0-9][0-9]* sha256=sha256:\([0-9a-f]*\)$/\1\t\2/p' \
    | awk -F'\t' '$1 == "results.json" || $1 ~ /\.csv$/ || $1 ~ /\.ndjson$/'
}

compare_expected() {
  local digest="$1"
  if [ -n "$expected" ]; then
    local want
    want="$(normalize_expected "$expected")" || exit 2
    if [ "$want" != "sha256:$digest" ]; then
      printf 'verify: expected %s, observed sha256:%s\n' "$want" "$digest" >&2
      exit 1
    fi
    printf 'verified sha256:%s\n' "$digest"
  else
    printf 'canonical digest sha256:%s\n' "$digest"
  fi
}

run_self_test() {
  local detail artifacts digest
  detail="$(cat <<'EOF'
artifact results.json: bytes=11 sha256=sha256:1111111111111111111111111111111111111111111111111111111111111111
artifact portfolio/equity.csv: bytes=22 sha256=sha256:3333333333333333333333333333333333333333333333333333333333333333
artifact strategy/decisions.ndjson: bytes=33 sha256=sha256:4444444444444444444444444444444444444444444444444444444444444444
artifact trades.csv: bytes=44 sha256=sha256:2222222222222222222222222222222222222222222222222222222222222222
artifact provenance.json: bytes=55 sha256=sha256:5555555555555555555555555555555555555555555555555555555555555555
artifact summary.txt: bytes=66 sha256=sha256:6666666666666666666666666666666666666666666666666666666666666666
artifact manifest.json: bytes=77 sha256=sha256:7777777777777777777777777777777777777777777777777777777777777777
artifact model/result.json: bytes=88 sha256=sha256:8888888888888888888888888888888888888888888888888888888888888888
EOF
)"
  artifacts="$(canonical_artifacts "$detail")"
  digest="$(printf '%s\n' "$artifacts" | canonical_digest)"
  if [ "$digest" != "$self_test_digest" ]; then
    fail "self-test: canonical digest $digest, expected $self_test_digest"
  fi
  compare_expected "$digest"
}

if [ "$self_test" -eq 1 ]; then
  run_self_test
  exit 0
fi

if [ -n "${QUANTRULE_SHELL:-}" ]; then
  shell="$QUANTRULE_SHELL"
else
  cargo build --quiet -p quantrule-shell >/dev/null
  shell="$root/target/debug/quantrule"
fi
[ -x "$shell" ] || fail "the shell client is not executable at $shell"

command -v curl >/dev/null || fail "curl is required to talk to the gateway"
command -v "$python_bin" >/dev/null || fail "$python_bin is required to build the config bundle"
[ -f "$config" ] || fail "the run config $config is missing"
[ -f "$replica" ] || fail "the replica run config $replica is missing"
[ -n "$data" ] || data="$(dirname "$config")/candles.csv"
[ -f "$data" ] || fail "the fixture data $data is missing"

export QUANTRULE_API_URL="$api_url"

work="$(mktemp -d "${TMPDIR:-/tmp}/quantrule-verify.XXXXXX")"
trap 'rm -rf "$work"' EXIT

if [ -n "$evidence_dir" ]; then
  mkdir -p "$evidence_dir/artifacts" "$evidence_dir/status"
  : > "$evidence_dir/runs.tsv"
fi

say() {
  printf 'verify: %s\n' "$*" >&2
}

require_stack() {
  local health
  health="$(curl -fsS --max-time 5 "$api_url/health" 2>/dev/null)" \
    || fail "the gateway is not reachable at $api_url; run 'make up' first"
  printf '%s' "$health" | grep -q '"state":"healthy"' \
    || fail "the gateway at $api_url is not healthy; run 'make up' first"
}

bundle_json() {
  "$python_bin" - "$1" "$2" <<'PY'
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
label = sys.argv[2]
run_path = root / "run.json"
if not run_path.is_file():
    sys.exit(f"verify: {run_path} is missing")
try:
    run = json.loads(run_path.read_text(encoding="utf-8"))
except (OSError, ValueError) as error:
    sys.exit(f"verify: cannot read {run_path}: {error}")
if not isinstance(run, dict):
    sys.exit(f"verify: {run_path} is not a JSON object")
names = ["run.json"]
for key in ("data", "model", "execution", "portfolio", "strategy"):
    name = run.get(key)
    if name is None:
        continue
    if not isinstance(name, str) or not name:
        sys.exit(f"verify: {run_path}: {key} must be a file name")
    if name not in names:
        names.append(name)
files = {}
for name in names:
    path = root / name
    try:
        files[name] = path.read_text(encoding="utf-8")
    except (OSError, ValueError) as error:
        sys.exit(f"verify: cannot read {path}: {error}")
print(json.dumps({"name": label, "files": files}))
PY
}

upload_config() {
  local run_path="$1"
  local directory label body response status config_id valid message
  directory="$(dirname "$run_path")"
  label="$(basename "$directory")"
  body="$work/bundle.json"
  response="$work/bundle-response.json"
  bundle_json "$directory" "$label" > "$body" \
    || fail "cannot build the config bundle for $run_path"
  status="$(curl -sS -o "$response" -w '%{http_code}' \
    -X POST -H "X-QuantRule-Profile: $profile" \
    -H 'content-type: application/json' \
    --data-binary @"$body" "$api_url/catalog/configs")" \
    || fail "the gateway at $api_url refused the config upload"
  case "$status" in
    200|201) ;;
    *)
      printf '%s\n' "$(cat "$response")" >&2
      fail "the config $run_path was refused with HTTP $status"
      ;;
  esac
  config_id="$(sed -n 's/.*"config_id":"\([^"]*\)".*/\1/p' "$response")"
  valid="$(sed -n 's/.*"valid":\(true\|false\).*/\1/p' "$response")"
  [ -n "$config_id" ] || fail "the upload of $run_path returned no config id"
  if [ "$valid" != "true" ]; then
    message="$(sed -n 's/.*"message":"\([^"]*\)".*/\1/p' "$response")"
    fail "the config $run_path is invalid: ${message:-unknown reason}"
  fi
  printf '%s\n' "$config_id"
}

upload_dataset() {
  local output data_version
  output="$("$shell" --profile "$profile" dataset upload "$data" --name "$(basename "$data")")" \
    || fail "the dataset upload failed"
  data_version="$(printf '%s\n' "$output" | sed -n 's/^data_version: //p')"
  [ -n "$data_version" ] || fail "the dataset upload returned no data version"
  printf '%s\n' "$data_version"
}

submit_run() {
  local data_version="$1" config_id="$2" output run_id created
  output="$("$shell" --profile "$profile" run submit "$data_version" "$config_id")" \
    || fail "the run submission failed for $config_id"
  run_id="$(printf '%s\n' "$output" | sed -n 's/^run_id: //p')"
  created="$(printf '%s\n' "$output" | sed -n 's/^created: //p')"
  [ -n "$run_id" ] || fail "the run submission returned no run id"
  say "run $run_id from $config_id (created=$created)"
  printf '%s\n' "$run_id"
}

run_detail=""
wait_run() {
  local run_id="$1" deadline=$((SECONDS + timeout)) detail state error
  while :; do
    detail="$("$shell" --profile "$profile" run status "$run_id")" \
      || fail "cannot read the run $run_id"
    state="$(printf '%s\n' "$detail" | sed -n 's/^status: //p')"
    case "$state" in
      queued|running) ;;
      succeeded)
        run_detail="$detail"
        return 0
        ;;
      failed|cancelled)
        error="$(printf '%s\n' "$detail" | sed -n 's/^error: //p')"
        fail "run $run_id ended $state${error:+: $error}"
        ;;
      *)
        fail "run $run_id reported the unknown state '${state:-none}'"
        ;;
    esac
    if [ "$SECONDS" -ge "$deadline" ]; then
      fail "run $run_id did not finish within ${timeout}s (last state $state)"
    fi
    sleep 2
  done
}

# Download every canonical artifact, verify it against the run record's hash,
# and return the "name<TAB>sha256" list computed from the downloaded bytes.
fetch_artifacts() {
  local run_id="$1" artifacts="$2" name sha code file actual directory listing
  local total_bytes=0
  if [ -n "$evidence_dir" ]; then
    directory="$evidence_dir/artifacts/${run_id//:/_}"
  else
    directory="$work/artifacts/${run_id//:/_}"
  fi
  listing="$work/artifacts/${run_id//:/_}.tsv"
  mkdir -p "$work/artifacts"
  : > "$listing"
  while IFS=$'\t' read -r name sha; do
    [ -n "$name" ] || continue
    case "$name" in
      ""|/*|*\\*|*//*|.|..|*/./*|*/../*|*/.|*/..)
        fail "the run lists the unsafe artifact name '$name'"
        ;;
    esac
    file="$directory/$name"
    mkdir -p "$(dirname "$file")"
    code="$(curl -sS -o "$file" -w '%{http_code}' \
      -H "X-QuantRule-Profile: $profile" \
      "$api_url/runs/${run_id//:/%3A}/artifacts/$name")" \
      || fail "cannot reach the gateway for $name"
    [ "$code" = "200" ] || fail "artifact $name answered with HTTP $code"
    actual="$(sha256sum "$file" | cut -d' ' -f1)"
    [ "$actual" = "$sha" ] || fail "artifact $name does not match the run record"
    total_bytes=$((total_bytes + $(wc -c < "$file")))
    printf '%s\t%s\n' "$name" "$actual" >> "$listing"
  done <<< "$artifacts"
  say "run $run_id: $(wc -l < "$listing" | tr -d ' ') canonical artifacts, $total_bytes bytes verified"
  cat "$listing"
}

run_digest() {
  local run_id="$1" artifacts fetched digest recorded
  wait_run "$run_id"
  if [ -n "$evidence_dir" ]; then
    printf '%s\n' "$run_detail" > "$evidence_dir/status/${run_id//:/_}.txt"
  fi
  artifacts="$(canonical_artifacts "$run_detail")"
  [ -n "$artifacts" ] || fail "run $run_id has no canonical artifacts"
  fetched="$(fetch_artifacts "$run_id" "$artifacts")" || exit 1
  digest="$(printf '%s\n' "$fetched" | canonical_digest)"
  [ ${#digest} -eq 64 ] || fail "the canonical digest for $run_id is malformed"
  recorded="$("$shell" --profile "$profile" verify "$run_id")" \
    || fail "cannot read the recorded digest for $run_id"
  recorded="$(printf '%s\n' "$recorded" | sed -n 's/^digest: sha256:\([0-9a-f]*\)$/\1/p')"
  [ -n "$recorded" ] || fail "the gateway recorded no digest for $run_id"
  if [ "$recorded" != "$digest" ]; then
    fail "the recomputed digest sha256:$digest differs from the recorded sha256:$recorded"
  fi
  printf '%s\n' "$digest"
}

require_stack

data_version="$(upload_dataset)" || exit 1
config_id="$(upload_config "$config")" || exit 1
replica_id="$(upload_config "$replica")" || exit 1
if [ "$config_id" = "$replica_id" ]; then
  fail "the two configs are byte-identical; they must differ in non-result metadata"
fi
say "config $config -> $config_id"
say "config $replica -> $replica_id"

run_a="$(submit_run "$data_version" "$config_id")" || exit 1
run_b="$(submit_run "$data_version" "$replica_id")" || exit 1
if [ "$run_a" = "$run_b" ]; then
  fail "the two configs resolved to one run; a reproducibility check needs two runs"
fi

digest_a="$(run_digest "$run_a")" || exit 1
digest_b="$(run_digest "$run_b")" || exit 1
if [ "$digest_a" != "$digest_b" ]; then
  printf 'verify: the two runs produced different digests\n' >&2
  printf '  %s: sha256:%s\n' "$run_a" "$digest_a" >&2
  printf '  %s: sha256:%s\n' "$run_b" "$digest_b" >&2
  exit 1
fi

if [ -n "$evidence_dir" ]; then
  printf '%s\n' "$digest_a" > "$evidence_dir/digest"
  printf '%s\t%s\t%s\n' "$config_id" "$run_a" "$digest_a" >> "$evidence_dir/runs.tsv"
  printf '%s\t%s\t%s\n' "$replica_id" "$run_b" "$digest_b" >> "$evidence_dir/runs.tsv"
fi

compare_expected "$digest_a"
