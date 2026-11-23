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
expected="sha256:773aea9ea27012c85ebaebe6661c5917ad8fe7c5bbc9d2e8ee22308c548f2ab2"
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

if "$verify" --cacert >/dev/null 2>&1; then
  fail "--cacert without a value was accepted"
fi

if "$verify" --cacert "$work/missing-ca.pem" >"$work/out" 2>"$work/err"; then
  fail "an unreadable --cacert exited zero"
fi
grep -q "is not readable" "$work/err" || fail "the unreadable --cacert was not named"

ca="$work/ca.pem"
printf '%s\n' "-----BEGIN CERTIFICATE-----" "fixture" "-----END CERTIFICATE-----" > "$ca"
mkdir -p "$work/bin"
cat > "$work/bin/curl" <<'FAKE'
#!/bin/sh
printf '%s\n' "$*" >> "$QR_FAKE_CURL_LOG"
exit 7
FAKE
chmod +x "$work/bin/curl"
: > "$work/curl.log"
if PATH="$work/bin:$PATH" QR_FAKE_CURL_LOG="$work/curl.log" QUANTRULE_SHELL=/bin/true \
  "$verify" --cacert "$ca" --api-url http://127.0.0.1:1 >/dev/null 2>"$work/err"; then
  fail "the fake curl failure was reported as success"
fi
grep -q -- "--cacert $ca" "$work/curl.log" || fail "curl did not receive --cacert"

printf 'verify self-test ok\n'
