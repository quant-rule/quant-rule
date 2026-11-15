#!/usr/bin/env bash
# Self-test for tools/verify.sh.
#
# The digest comparator and the --expected failure are checked without a
# gateway: the script's fixed artifact list exercises the canonical filter,
# the sorted encoding, the printed line, the accepted line forms, and the
# nonzero wrong-expected path. The end-to-end run path needs a healthy stack
# and is exercised by `make verify`, not here.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

verify="$root/tools/verify.sh"
expected="sha256:4acf7e9c323e2c30f293169bf96a41ee0387fb323d20950ad8e9ae043f2f9bb7"
wrong="sha256:$(printf '0%.0s' {1..64})"
work="$(mktemp -d "${TMPDIR:-/tmp}/quantrule-verify-test.XXXXXX")"
trap 'rm -rf "$work"' EXIT

fail() {
  printf 'test_verify: %s\n' "$*" >&2
  exit 1
}

bash -n "$verify" || fail "tools/verify.sh does not parse"

line="$("$verify" --self-test)" || fail "--self-test failed"
if [ "$line" != "canonical digest $expected" ]; then
  fail "the self-test line is '$line', expected 'canonical digest $expected'"
fi

verified="$("$verify" --self-test --expected "$line")" || fail "the printed line was rejected"
if [ "$verified" != "verified $expected" ]; then
  fail "the accepted line printed '$verified'"
fi

"$verify" --self-test --expected "$expected" >/dev/null \
  || fail "the bare digest was rejected"
"$verify" --self-test --expected "${expected#sha256:}" >/dev/null \
  || fail "the bare hexadecimal form was rejected"

if "$verify" --self-test --expected "$wrong" >"$work/out" 2>"$work/err"; then
  fail "a wrong --expected exited zero"
fi
grep -q "expected $wrong" "$work/err" || fail "the failure did not name the expected digest"
grep -q "observed $expected" "$work/err" || fail "the failure did not name the observed digest"

if "$verify" --self-test --expected sha256:xyz >/dev/null 2>&1; then
  fail "a malformed --expected was accepted"
fi

if "$verify" --strategy config/examples/verify-mini/strategy.json >/dev/null 2>&1; then
  fail "the retired --strategy flag was accepted"
fi

printf 'verify self-test ok\n'
