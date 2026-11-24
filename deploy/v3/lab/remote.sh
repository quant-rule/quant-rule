#!/usr/bin/env bash
# Remote client phase for deploy/v3/lab.sh.
#
# Sourced by the lab harness. It runs on the client, outside the Swarm, and
# prints only status and digest. It never prints the token, the CA body, or a
# key. The browser suite is separate and lab-gated; this phase prints
# `Unverified` for that row.
#
# The shell's remote path is the environment path: QUANTRULE_API_URL,
# QUANTRULE_CA_CERT, and QUANTRULE_TOKEN (or QUANTRULE_TOKEN_FILE). `--remote`,
# `--cacert`, and `--url` are not shell flags; `--remote` belongs to
# `qr-deploy check`.

remote_phase() {
  local shell state image digest tls_out code health_out verify_out
  [ "$confirm" -eq 1 ] || unverified "the remote client phase needs --confirm"
  [ "${QR_LAB_PROVISIONED:-0}" = "1" ] \
    || unverified "QR_LAB_PROVISIONED=1 is required; provision the lab first"
  [ "${QUANTRULE_API_URL:-}" = "https://quantrule.dev/api/v1" ] \
    || unverified "QUANTRULE_API_URL must be https://quantrule.dev/api/v1"
  if [ -z "${QUANTRULE_CA_CERT:-}" ] || [ ! -r "${QUANTRULE_CA_CERT}" ]; then
    unverified "QUANTRULE_CA_CERT must name a readable private CA bundle"
  fi
  if [ -z "${QUANTRULE_TOKEN:-}" ] && [ -z "${QUANTRULE_TOKEN_FILE:-}" ]; then
    unverified "QUANTRULE_TOKEN or QUANTRULE_TOKEN_FILE is required"
  fi
  command -v curl >/dev/null 2>&1 || unverified "curl is required on the client"
  command -v openssl >/dev/null 2>&1 || unverified "openssl is required on the client"
  command -v jq >/dev/null 2>&1 || unverified "jq is required on the client"
  shell=${QR_REMOTE_SHELL:-}
  if [ -z "$shell" ]; then
    shell=$(command -v quantrule 2>/dev/null || true)
  fi
  if [ -z "$shell" ] && [ -x "$root/target/debug/quantrule" ]; then
    shell="$root/target/debug/quantrule"
  fi
  [ -n "$shell" ] || unverified "set QR_REMOTE_SHELL or put quantrule on PATH"

  health_out=$("$shell" health 2>&1) \
    || unverified "quantrule health failed over the private CA"
  state=$(printf '%s\n' "$health_out" | sed -n 's/^state: //p' | head -n 1)
  image=$(printf '%s\n' "$health_out" | sed -n 's/^image_digest: //p' | head -n 1)
  [ "$state" = "healthy" ] || unverified "health state is ${state:-unknown}, not healthy"
  [ -n "$image" ] || unverified "health printed no image digest"
  local expected
  expected=$(descriptor_field '.image.digest')
  [ -n "$expected" ] || unverified "cannot read the descriptor image digest"
  [ "$image" = "$expected" ] \
    || unverified "health image digest $image does not equal the descriptor $expected"
  say "remote health: $state"
  say "remote image digest: $image"

  verify_out=$(./tools/verify.sh \
    --api-url "$QUANTRULE_API_URL" \
    --cacert "$QUANTRULE_CA_CERT" \
    --profile "${QR_REMOTE_PROFILE:-default}" 2>&1) \
    || unverified "tools/verify.sh failed over the private CA"
  digest=$(printf '%s\n' "$verify_out" \
    | sed -n 's/^canonical digest sha256:\([0-9a-f]\{64\}\)$/\1/p' \
    | head -n 1)
  [ -n "$digest" ] || unverified "tools/verify.sh printed no canonical digest"
  say "remote canonical digest: sha256:$digest"

  tls_out=$(openssl s_client -connect quantrule.dev:443 -servername quantrule.dev \
    -CAfile "$QUANTRULE_CA_CERT" -verify_return_error </dev/null 2>&1) \
    || unverified "openssl s_client refused the ingress certificate"
  printf '%s\n' "$tls_out" \
    | grep -Eq 'Verification: OK|Verify return code: 0 \(ok\)' \
    || unverified "openssl s_client did not verify the ingress certificate"
  say "remote TLS: verified"

  code=$(curl --cacert "$QUANTRULE_CA_CERT" -sS -o /dev/null -w '%{http_code}' \
    --max-time 15 https://quantrule.dev/health) \
    || unverified "curl with the private CA could not reach https://quantrule.dev/health"
  [ "$code" = "200" ] || unverified "curl with the private CA answered HTTP $code"
  say "remote curl: $code"

  if curl --cacert /dev/null -sS -o /dev/null --max-time 15 \
    https://quantrule.dev/health >/dev/null 2>&1; then
    unverified "the empty-trust-store curl succeeded; the negative control is not isolated"
  fi
  say "remote untrusted CA: refused"

  write_remote_evidence "$remote_evidence" ok ok ok "$code" refused "$digest"
  say "browser: Unverified here; run the lab-gated apps/web suite (QR_REMOTE_BROWSER=1, QR_REMOTE_URL)"
}
