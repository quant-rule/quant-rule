#!/usr/bin/env bash
# Shared DR evidence and timing helpers. Sourced by deploy/v3/dr.sh.

dr_say() {
  printf 'dr: %s\n' "$*"
}

dr_unverified() {
  printf 'dr: UNVERIFIED: %s\n' "$*" >&2
  printf 'dr: labels: Unverified; Real HA is HA-014\n' >&2
  if [ -n "${DR_WORK:-}" ] && [ -f "$DR_WORK/facts.jsonl" ]; then
    dr_fact "error" "$*" 2>/dev/null || true
  fi
  exit 3
}

dr_die() {
  printf 'dr: %s\n' "$*" >&2
  exit 1
}

dr_now_utc() {
  date -u +%Y-%m-%dT%H:%M:%SZ
}

dr_now_epoch() {
  date -u +%s
}

# A fact is one JSON object appended to a JSONL file. Numbers stay numbers;
# strings are escaped by jq. The final evidence is `jq -s add` over the file.
dr_fact() {
  jq -n --arg key "$1" --arg value "$2" '{($key): $value}' >> "$DR_WORK/facts.jsonl"
}

dr_fact_num() {
  case "$2" in
    '' | *[!0-9-]*) dr_die "the fact $1 is not a whole number: $2" ;;
  esac
  jq -n --arg key "$1" --argjson value "$2" '{($key): $value}' >> "$DR_WORK/facts.jsonl"
}

dr_fact_raw() {
  printf '%s\n' "$2" >> "$DR_WORK/facts.jsonl"
}

# A redacted endpoint keeps host and port and drops a scheme or userinfo.
dr_redact_endpoint() {
  local value="$1"
  value=${value#*://}
  value=${value##*@}
  printf '%s' "$value" | sed 's:/.*::'
}

dr_elapsed() {
  local start=$1 end=$2
  [ "$end" -ge "$start" ] || return 1
  printf '%s' "$((end - start))"
}

# The canonical digest line is the last line of tools/verify.sh output.
dr_parse_digest() {
  sed -n 's/^.*sha256:\([0-9a-f]\{64\}\)$/\1/p' "$1" | tail -n 1
}

dr_run_verify() {
  local directory=$1 expected=${2:-}
  mkdir -p "$directory"
  local args=(--api-url "$DR_API_URL" --evidence-dir "$directory")
  [ -n "$DR_CACERT" ] && args+=(--cacert "$DR_CACERT")
  [ -n "$expected" ] && args+=(--expected "$expected")
  if ! ./tools/verify.sh "${args[@]}" > "$directory/stdout" 2> "$directory/stderr"; then
    return 1
  fi
  dr_parse_digest "$directory/stdout"
}

# The last object-archive evidence, newest by filename (the filename carries the
# UTC start). An explicit QR_DR_ARCHIVE_EVIDENCE wins.
dr_last_archive_evidence() {
  if [ -n "${QR_DR_ARCHIVE_EVIDENCE:-}" ]; then
    printf '%s' "$QR_DR_ARCHIVE_EVIDENCE"
    return 0
  fi
  find deploy/v3/evidence -maxdepth 1 -name 'object-archive-*.json' -type f 2>/dev/null \
    | LC_ALL=C sort | tail -n 1
}

dr_archive_epoch() {
  local file=$1
  [ -f "$file" ] || return 1
  local value
  value=$(jq -r '.finished_utc // empty' "$file" 2>/dev/null) || return 1
  [ -n "$value" ] || return 1
  date -u -d "$value" +%s 2>/dev/null
}

# A target after the last object archive loses the objects between the archive
# and the target. Refuse rather than restore rows newer than the archive.
dr_check_archive_time() {
  local target_epoch=$1 file archive_epoch
  file=$(dr_last_archive_evidence)
  if [ -z "$file" ] || [ ! -f "$file" ]; then
    printf 'dr: object archive evidence is absent; the object row is Unverified\n' >&2
    return 2
  fi
  archive_epoch=$(dr_archive_epoch "$file") || return 2
  DR_ARCHIVE_EPOCH=$archive_epoch
  DR_ARCHIVE_EVIDENCE_FILE=$file
  if [ "$target_epoch" -gt "$archive_epoch" ]; then
    printf 'dr: target %s is after the last object archive %s; refuse the restore\n' \
      "$target_epoch" "$archive_epoch" >&2
    return 1
  fi
  return 0
}

# Write the value-free quantrule.dr/v1 document. Every fact was added with
# dr_fact/dr_fact_num; labels and row status are added here so a skipped row is
# never silently a pass.
dr_write_evidence() {
  local status=$1
  local labels_json rows_json
  labels_json=$(printf '%s\n' "$DR_LABELS" | jq -R . | jq -sc .)
  rows_json=$(printf '%s\n' "$DR_ROWS" | sed '/^$/d' | jq -R . | jq -sc .)
  dr_fact_raw "labels" "$(jq -cn --argjson value "$labels_json" '{labels: $value}')"
  dr_fact_raw "rows" "$(jq -cn --argjson value "$rows_json" '{rows: $value}')"
  dr_fact_raw "claims" '{"cluster_rpo":"RPO 0 is the synchronous-standby property","backup_rpo":"WAL lag","object_rpo":"object archive interval","rto":"restore+startup+digest measured, not the SLO","object_replication":false,"second_site":false,"version_history":false,"real_ha":false}'
  dr_fact "status" "$status"
  dr_fact "real_ha" "false"
  dr_fact "simulated_ha_maximum" "true"
  dr_fact "note" "RPO is measured, not an SLO proof; Real HA is HA-014"
  mkdir -p "$(dirname "$DR_EVIDENCE")"
  local temporary="$DR_EVIDENCE.tmp"
  if ! jq -s 'add' "$DR_WORK/facts.jsonl" > "$temporary"; then
    rm -f "$temporary" 2>/dev/null || true
    dr_die "cannot write $DR_EVIDENCE"
  fi
  chmod 600 "$temporary"
  mv "$temporary" "$DR_EVIDENCE"
  printf 'dr: evidence %s\n' "$DR_EVIDENCE"
}
