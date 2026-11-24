#!/usr/bin/env bash
# Evidence helpers for deploy/v3/lab.sh.
#
# Sourced by the lab harness after the reference and descriptor are resolved.
# The JSON is value-free: it records hashes, digests, tool versions, and phase
# labels. It never records a token, a CA body, a key, or an inspect body.

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    printf 'absent'
  fi
}

descriptor_field() {
  jq -r "$1 // empty" "$descriptor" 2>/dev/null || true
}

tool_versions_json() {
  local docker_version="absent" jq_version="absent" openssl_version="absent"
  local curl_version="absent" bash_version="absent" shell_sha="absent"
  command -v jq >/dev/null 2>&1 && jq_version=$(jq --version 2>/dev/null | head -n 1)
  command -v openssl >/dev/null 2>&1 \
    && openssl_version=$(openssl version 2>/dev/null | head -n 1)
  command -v curl >/dev/null 2>&1 \
    && curl_version=$(curl --version 2>/dev/null | head -n 1)
  command -v bash >/dev/null 2>&1 \
    && bash_version=$(bash --version 2>/dev/null | head -n 1)
  command -v "$docker_bin" >/dev/null 2>&1 \
    && docker_version=$("$docker_bin" --version 2>/dev/null | head -n 1)
  if [ -n "${QR_REMOTE_SHELL:-}" ] && [ -x "${QR_REMOTE_SHELL}" ]; then
    shell_sha=$(sha256_file "${QR_REMOTE_SHELL}")
  fi
  jq -n \
    --arg docker "$docker_version" \
    --arg jq "$jq_version" \
    --arg openssl "$openssl_version" \
    --arg curl "$curl_version" \
    --arg bash "$bash_version" \
    --arg shell_sha256 "$shell_sha" \
    '{docker: $docker, jq: $jq, openssl: $openssl, curl: $curl, bash: $bash,
      shell_sha256: $shell_sha256}'
}

write_evidence() {
  local path=$1 label=$2
  command -v jq >/dev/null 2>&1 || unverified "jq is required for the evidence file"
  mkdir -p "$(dirname "$path")"
  jq -n \
    --arg evidence "$label" \
    --arg reference "$reference" \
    --arg profile "$(descriptor_field '.profile')" \
    --arg descriptor "$descriptor" \
    --arg descriptor_sha256 "$(sha256_file "$descriptor")" \
    --arg image_digest "$(descriptor_field '.image.digest')" \
    --arg recorded_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --arg stateful "$provision_stateful" \
    --arg swarm "$provision_swarm" \
    --arg observability "$provision_observability" \
    --arg check "$provision_check" \
    --arg database "$failure_database" \
    --arg queue "$failure_queue" \
    --arg object "$failure_object" \
    --arg failure_observability "$failure_observability" \
    --arg dr "$failure_dr" \
    --argjson tools "$(tool_versions_json)" \
    '{
      v: "quantrule.lab-evidence/v1",
      evidence: $evidence,
      reference: $reference,
      profile: $profile,
      descriptor: $descriptor,
      descriptor_sha256: $descriptor_sha256,
      image_digest: $image_digest,
      recorded_at: $recorded_at,
      provision: {stateful: $stateful, swarm: $swarm,
                  observability: $observability, remote_check: $check},
      failure: {database: $database, queue: $queue, object: $object,
                observability: $failure_observability, dr: $dr},
      remote_cli: {health: "Unverified", tls: "Unverified", verify: "Unverified",
                   untrusted_ca: "Unverified", canonical_digest: null},
      browser: "Unverified",
      tool_versions: $tools,
      claims: {max_evidence: "Simulated HA", real_ha: "HA-014",
               fixture_blockers: []}
    }' > "$path"
  say "evidence: $path ($label)"
}

remote_failure_evidence_is_simulated_ha() {
  local path=${QR_LAB_FAILURE_EVIDENCE:-}
  [ -n "$path" ] || return 1
  [ -r "$path" ] || return 1
  command -v jq >/dev/null 2>&1 || return 1
  jq -e '.evidence == "Simulated HA"' "$path" >/dev/null 2>&1
}

write_remote_evidence() {
  local path=$1 health=$2 verify=$3 tls=$4 curl_status=$5 untrusted=$6 digest=$7
  local label="Unverified"
  remote_failure_evidence_is_simulated_ha && label="Simulated HA"
  mkdir -p "$(dirname "$path")"
  jq -n \
    --arg evidence "$label" \
    --arg profile "${QR_REMOTE_PROFILE:-default}" \
    --arg descriptor_sha256 "$(sha256_file "$descriptor")" \
    --arg image_digest "$(descriptor_field '.image.digest')" \
    --arg recorded_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --arg health "$health" \
    --arg verify "$verify" \
    --arg tls "$tls" \
    --arg curl_status "$curl_status" \
    --arg untrusted_ca "$untrusted" \
    --arg digest "$digest" \
    --argjson tools "$(tool_versions_json)" \
    '{
      v: "quantrule.lab-evidence/v1",
      evidence: $evidence,
      reference: "lab",
      profile: $profile,
      descriptor: "deploy/v3/deploy.lab.json",
      descriptor_sha256: $descriptor_sha256,
      image_digest: $image_digest,
      recorded_at: $recorded_at,
      remote: {health: $health, verify: $verify, tls: $tls,
               curl: $curl_status, untrusted_ca: $untrusted,
               canonical_digest: (if $digest == "" then null else "sha256:" + $digest end),
               ca: "present", token: "present"},
      browser: "Unverified",
      tool_versions: $tools,
      claims: {max_evidence: "Simulated HA", real_ha: "HA-014",
               note: "remote rows alone are not HA evidence"}
    }' > "$path"
  say "evidence: $path ($label)"
}
