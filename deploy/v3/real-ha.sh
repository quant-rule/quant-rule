#!/usr/bin/env bash
# Operator-only Real HA gate for HA-014.
#
# This script validates a descriptor and an operator evidence file. It never
# runs a failure, contacts a host, or executes a power, reset, disk, or network
# command; those actions are the operator's, outside this repository. A lab
# descriptor is refused, and no evidence is accepted without a human
# attestation. The matrix row remains Unverified until the human records the
# acceptance.
#
# Usage:
#   deploy/v3/real-ha.sh --plan
#   deploy/v3/real-ha.sh --confirm-real-ha --descriptor deploy/v3/reference/three-az.json --evidence docs/reports/2026-11-24/ha-014/HA-22.json

set -euo pipefail

plan=0
confirm=0
descriptor=""
evidence=""

usage() {
  awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
}

fail() {
  printf 'real-ha: REFUSED: %s\n' "$*" >&2
  exit 1
}

while [ $# -gt 0 ]; do
  case "$1" in
    --plan | --dry-run) plan=1; shift ;;
    --confirm-real-ha) confirm=1; shift ;;
    --descriptor)
      [ $# -ge 2 ] || fail "--descriptor needs a value"
      descriptor=$2
      shift 2
      ;;
    --evidence)
      [ $# -ge 2 ] || fail "--evidence needs a value"
      evidence=$2
      shift 2
      ;;
    -h | --help) usage; exit 0 ;;
    *) fail "unknown argument $1" ;;
  esac
done

if [ "$plan" -eq 1 ]; then
  cat <<'PLAN'
real-ha: operator-only gate (no Docker, SSH, jq, or secret is read)
real-ha: accepts only a descriptor with separate physical hosts, at least three failure domains, and a Real HA evidence file
real-ha: evidence must name at least three hosts with distinct machine and product UUIDs, UTC dates, a before/after digest, and a human attestation
real-ha: refuses lab-single-host and a missing or non-human attestation
real-ha: runbook: record machine_id and product_uuid per host before and after the operator's power event
real-ha: runbook: run tools/verify.sh before and after on the same fixed run/config; the canonical digest must match
real-ha: runbook: record the failure window, the alerts, the one finalisation, and the human attestation under docs/reports/2026-11-24/ha-014/
real-ha: the matrix rows HA-22 and HA-23 remain Unverified until the human accepts the recorded evidence
real-ha: this plan is not evidence and does not execute a failure
PLAN
  exit 0
fi

[ "$confirm" -eq 1 ] || fail "pass --confirm-real-ha after the evidence is recorded"
[ -n "$descriptor" ] || fail "--descriptor is required"
[ -n "$evidence" ] || fail "--evidence is required"
[ -f "$descriptor" ] || fail "descriptor $descriptor is missing"
[ -f "$evidence" ] || fail "evidence $evidence is missing"
command -v jq >/dev/null 2>&1 || fail "jq is required"

profile=$(jq -r '.profile // empty' "$descriptor") || fail "cannot read the descriptor profile"
[ "$profile" != "lab-single-host" ] \
  || fail "the lab-single-host descriptor is not a Real HA topology"
domains=$(jq -r '[.hosts[].failure_domain] | unique | length' "$descriptor") \
  || fail "cannot read the descriptor failure domains"
[ "${domains:-0}" -ge 3 ] || fail "the descriptor declares fewer than three failure domains"
host_count=$(jq -r '.hosts | keys | length' "$descriptor") || fail "cannot read the descriptor hosts"
[ "$host_count" -ge 3 ] || fail "the descriptor declares fewer than three hosts"

schema=$(jq -r '.schema // empty' "$evidence") || fail "cannot read the evidence schema"
[ "$schema" = "quantrule.real-ha/v1" ] || fail "evidence schema must be quantrule.real-ha/v1"
recorded_on=$(jq -r '.recorded_on // empty' "$evidence") || fail "cannot read recorded_on"
[[ $recorded_on =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] || fail "recorded_on must be a UTC date"
recorded_descriptor=$(jq -r '.descriptor // empty' "$evidence") || fail "cannot read the evidence descriptor"
[ "$recorded_descriptor" = "$descriptor" ] || fail "the evidence names a different descriptor"
recorded_domains=$(jq -r '.failure_domains // 0' "$evidence") || fail "cannot read failure_domains"
[ "$recorded_domains" -ge 3 ] || fail "the evidence records fewer than three failure domains"

fingerprints=$(jq -c '.hosts // []' "$evidence") || fail "cannot read the evidence hosts"
[ "$(printf '%s' "$fingerprints" | jq 'length')" -ge 3 ] \
  || fail "the evidence names fewer than three hosts"
for field in host machine_id product_uuid; do
  if printf '%s' "$fingerprints" | jq -e --arg field "$field" \
    'all(.[]; (.[$field] // "") | length > 0)' >/dev/null; then
    :
  else
    fail "every evidence host needs a non-empty $field"
  fi
done
host_names=$(printf '%s' "$fingerprints" | jq -r '.[].host')
while IFS= read -r host; do
  [ -n "$host" ] || continue
  jq -e --arg host "$host" '.hosts[$host] != null' "$descriptor" >/dev/null \
    || fail "evidence host $host is not declared by the descriptor"
done <<< "$host_names"
machine_count=$(printf '%s' "$fingerprints" | jq '[.[].machine_id] | unique | length')
uuid_count=$(printf '%s' "$fingerprints" | jq '[.[].product_uuid] | unique | length')
[ "$machine_count" -ge 3 ] && [ "$uuid_count" -ge 3 ] \
  || fail "evidence machine and product UUIDs must be distinct across at least three hosts"

digest_before=$(jq -r '.digest_before // empty' "$evidence") || fail "cannot read digest_before"
digest_after=$(jq -r '.digest_after // empty' "$evidence") || fail "cannot read digest_after"
[[ $digest_before =~ ^sha256:[0-9a-f]{64}$ ]] || fail "digest_before must be a sha256 digest"
[[ $digest_after =~ ^sha256:[0-9a-f]{64}$ ]] || fail "digest_after must be a sha256 digest"
[ "$digest_before" = "$digest_after" ] || fail "the before/after canonical digests differ"

attested_by=$(jq -r '.attestation.by // empty' "$evidence") || fail "cannot read attestation.by"
[ "$attested_by" = "human" ] || fail "a human attestation is required"
attested_on=$(jq -r '.attestation.signed_on // empty' "$evidence") || fail "cannot read attestation.signed_on"
[[ $attested_on =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] || fail "attestation.signed_on must be a UTC date"
statement=$(jq -r '.attestation.statement // empty' "$evidence") || fail "cannot read attestation.statement"
[ -n "$statement" ] || fail "the human attestation has no statement"

printf 'real-ha: gate satisfied for %s\n' "$descriptor"
printf 'real-ha: %s hosts, %s failure domains, digest %s\n' "$host_count" "$domains" "$digest_after"
printf 'real-ha: runbook: record machine_id and product_uuid per host before and after the operator power event\n'
printf 'real-ha: runbook: run tools/verify.sh before and after on the same fixed run/config; the digest must match\n'
printf 'real-ha: runbook: record the failure window, alerts, one finalisation, and the human attestation\n'
printf 'real-ha: no failure was executed; the matrix row remains Unverified until the human accepts the record\n'
