#!/usr/bin/env bash
# Digest, marker/RPO, and final evidence phases. Sourced by dr.sh.

dr_digest_pre() {
  if [ "$DR_SKIP_DIGEST" -eq 1 ]; then
    dr_fact "digest_status" "Unverified"
    dr_fact "digest_skip" "true"
    DR_ROWS="${DR_ROWS}digest=Unverified
"
    dr_say "digest: skipped (--skip-digest); row Unverified"
    return 0
  fi
  local start end digest
  start=$(dr_now_epoch)
  digest=$(dr_run_verify "$DR_WORK/pre-digest") \
    || dr_unverified "the pre-backup verification run failed; no expected digest"
  end=$(dr_now_epoch)
  [ ${#digest} -eq 64 ] || dr_unverified "the pre-backup digest is malformed"
  DR_EXPECTED_DIGEST=$digest
  dr_fact "expected_digest" "sha256:$digest"
  dr_fact_num "pre_digest_seconds" "$(dr_elapsed "$start" "$end")"
  DR_ROWS="${DR_ROWS}pre_digest=Measured
"
  dr_say "digest: expected sha256:$digest"
}

dr_digest_post() {
  if [ "$DR_SKIP_DIGEST" -eq 1 ]; then
    return 0
  fi
  local start end digest
  [ -n "$DR_EXPECTED_DIGEST" ] || dr_unverified "no expected digest; the pre-backup run did not complete"
  start=$(dr_now_epoch)
  digest=$(dr_run_verify "$DR_WORK/post-digest" "$DR_EXPECTED_DIGEST") || {
    dr_fact "observed_digest" "unavailable"
    dr_fact "digest_status" "Unverified"
    dr_unverified "the restored stack did not reproduce sha256:$DR_EXPECTED_DIGEST"
  }
  end=$(dr_now_epoch)
  DR_OBSERVED_DIGEST=$digest
  DR_DIGEST_SECONDS=$(dr_elapsed "$start" "$end")
  dr_fact "observed_digest" "sha256:$digest"
  dr_fact "digest_status" "Measured"
  dr_fact_num "digest_seconds" "$DR_DIGEST_SECONDS"
  if printf '%s\n' "$DR_ROWS" | grep -qx 'queue=Measured'; then
    dr_fact_num "queue_duplicate_artifacts" 0
    dr_fact "queue_duplicate_status" "Measured"
  else
    dr_fact_num "queue_duplicate_artifacts" -1
    dr_fact "queue_duplicate_status" "Unverified"
  fi
  DR_ROWS="${DR_ROWS}digest=Measured
"
  dr_say "digest: observed sha256:$digest; matches the pre-backup digest"
}

dr_marker_phase() {
  if [ "$DR_SKIP_MARKERS" -eq 1 ]; then
    mkdir -p "$DR_WORK/markers"
    : > "$DR_WORK/markers/rows.txt"
    DR_MARKER_ROWS=0
    DR_MARKER_COVERED=0
    DR_RPO_SECONDS=-1
    dr_fact_num "marker_rows" 0
    dr_fact_num "marker_covered_rows" 0
    dr_fact_num "rpo_seconds" -1
    dr_fact "rpo_basis" "markers skipped"
    DR_ROWS="${DR_ROWS}rpo=Unverified
"
    dr_say "markers: skipped (--skip-markers); row RPO Unverified"
    return 0
  fi
  dr_say "markers: sampling the source primary every ${DR_MARKER_INTERVAL:-10}s for ${DR_MARKER_WINDOW:-60}s"
  dr_marker_window "${DR_MARKER_INTERVAL:-10}" "${DR_MARKER_WINDOW:-60}"
}

dr_rpo_phase() {
  if [ "$DR_SKIP_MARKERS" -eq 1 ]; then
    return 0
  fi
  dr_compute_rpo "$DR_TARGET_EPOCH"
  if [ "$DR_RPO_SECONDS" -ge 0 ]; then
    DR_ROWS="${DR_ROWS}rpo=Measured
"
    dr_say "rpo: ${DR_RPO_SECONDS}s from the last covered marker"
  else
    DR_ROWS="${DR_ROWS}rpo=Unverified
"
    dr_say "rpo: no covered marker; row Unverified"
  fi
}

dr_object_phase() {
  if [ "$DR_SKIP_OBJECTS" -eq 1 ]; then
    dr_fact "object_status" "Unverified"
    DR_ROWS="${DR_ROWS}object=Unverified
"
    dr_say "objects: skipped (--skip-objects); row Unverified"
    return 0
  fi
  if dr_object_rpo "$DR_TARGET_EPOCH"; then
    dr_fact "object_status" "Measured"
    DR_ROWS="${DR_ROWS}object_rpo=Measured
"
    dr_say "objects: last archive is ${DR_OBJECT_RPO}s from the target; archive interval ${DR_ARCHIVE_INTERVAL}s"
  else
    dr_fact "object_status" "Unverified"
    DR_ROWS="${DR_ROWS}object_rpo=Unverified
"
    dr_say "objects: no measured archive time; row Unverified (the target may be after the last archive)"
  fi
}

# Build the final label/status. A required row that is absent is Unverified; a
# single Unverified row makes the whole drill Unverified, never a partial pass.
dr_finalize() {
  local required row missing=0 unverified=0 labels
  for required in backup restore startup queue digest object object_rpo rpo; do
    if ! printf '%s\n' "$DR_ROWS" | grep -qx "$required=Measured"; then
      if ! printf '%s\n' "$DR_ROWS" | grep -qx "$required=Unverified"; then
        DR_ROWS="${DR_ROWS}${required}=Unverified
"
      fi
    fi
  done
  if printf '%s\n' "$DR_ROWS" | grep -q '=Unverified'; then
    unverified=1
  fi
  if [ "$unverified" -eq 1 ]; then
    labels="Unverified"
    [ "$DR_MODE" = "in-place" ] && labels="Simulated HA
Unverified"
  elif [ "$DR_MODE" = "in-place" ]; then
    labels="Simulated HA
Measured"
  else
    labels="Measured"
  fi
  DR_LABELS=$labels
  dr_total_timing
  if [ "$unverified" -eq 1 ]; then
    dr_write_evidence "Unverified"
    dr_say "summary: $DR_MODE Unverified; RPO ${DR_RPO_SECONDS:-unverified}s; restore ${DR_RESTORE_SECONDS:-unverified}s, startup ${DR_STARTUP_SECONDS:-unverified}s, digest ${DR_DIGEST_SECONDS:-unverified}s, total ${DR_TOTAL_SECONDS}s"
    dr_say "Real HA is HA-014; the three-VM lab is Simulated HA at most"
    exit 3
  fi
  dr_write_evidence "Measured"
  dr_say "summary: $DR_MODE Measured; RPO ${DR_RPO_SECONDS}s; restore ${DR_RESTORE_SECONDS}s, startup ${DR_STARTUP_SECONDS}s, digest ${DR_DIGEST_SECONDS}s, total ${DR_TOTAL_SECONDS}s"
  dr_say "digest: expected sha256:$DR_EXPECTED_DIGEST; observed sha256:$DR_OBSERVED_DIGEST"
  dr_say "labels: $DR_MODE is Measured for this operations run; the three-VM lab is Simulated HA at most; Real HA is HA-014"
  exit 0
}
