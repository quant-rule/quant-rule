#!/usr/bin/env bash
# Archive the application object bucket to an operator-provided target, one way.
#
# This is an archive, not replication: it copies new or changed keys from the
# read-only `qr_archive_reader` source identity to the configured target and
# never deletes a source object, watches the source, or claims a second site.
# `mc mirror --overwrite --summary` is the only transfer; there is no `--remove`,
# no `--watch`, and no `mc replicate`. A key changed in the source is overwritten
# in the archive; a key removed from the source remains in the archive.
#
# The tool never prints or passes a secret on a command line. Source/target
# credentials are read from mode-600 files into a mode-700 temporary `mc`
# config; the source policy grants only `s3:GetObject` and `s3:ListBucket`.
# Target versioning and at-rest encryption are operator-owned. The evidence is
# value-free and records a redacted target identity.
#
# Usage: tools/object-archive.sh [--plan | --confirm] [options]
#
#   --plan              Print the one-way sequence; no Docker, jq, mc, SSH, or
#                       secret file is read.
#   --confirm           Run against the configured source and target.
#   --restore           Reverse the direction for a DR target: mirror the
#                       archive into a fresh scratch bucket before rows.
#   --descriptor FILE   v3 descriptor (default deploy/v3/deploy.example.json).
#   --source-bucket B   Source bucket (default: quantrule).
#   --target-bucket B   Target bucket (default: backup.object_archive.target).
#   --source-alias A    mc source alias (default: qr-source).
#   --target-alias A    mc target alias (default: qr-target).
#   --evidence FILE     Evidence JSON (default deploy/v3/evidence/object-archive-<UTC>.json).
#   -h | --help
#
# Environment:
#   DESCRIPTOR / OUT                 shared with the other v3 tools
#   QR_OBJECT_ARCHIVE_SOURCE_URL     source endpoint (default: endpoints.object_store)
#   QR_OBJECT_ARCHIVE_TARGET_URL     target endpoint; required live
#   QR_OBJECT_ARCHIVE_SOURCE_ACCESS_KEY_FILE / _SECRET_KEY_FILE
#   QR_OBJECT_ARCHIVE_TARGET_ACCESS_KEY_FILE / _SECRET_KEY_FILE
#   QR_OBJECT_ARCHIVE_CA             CA bundle (default: endpoints.object_store.tls_ca)
#   QR_OBJECT_ARCHIVE_MC             local mc executable (default mc)
#   QR_OBJECT_ARCHIVE_IMAGE          pinned mc image; runs in a container instead
#   QR_OBJECT_ARCHIVE_NETWORK        Docker network for the container form
#   QR_OBJECT_ARCHIVE_EVIDENCE       evidence path override
#
# `--plan` is the Docker/SSH/jq/secret-free contract. A live run requires
# `--confirm`; a missing source/target identity prints `Unverified` and exits
# non-zero. A successful archive is `Measured` for this operations run only; it
# is never a version-history, second-site, or `Real HA` claim.
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "$script_dir/.." && pwd)
cd "$root"

descriptor=${DESCRIPTOR:-deploy/v3/deploy.example.json}
plan=0
confirm=0
restore=0
source_bucket=${QR_OBJECT_ARCHIVE_SOURCE_BUCKET:-quantrule}
target_bucket=${QR_OBJECT_ARCHIVE_TARGET_BUCKET:-}
source_alias=${QR_OBJECT_ARCHIVE_SOURCE_ALIAS:-qr-source}
target_alias=${QR_OBJECT_ARCHIVE_TARGET_ALIAS:-qr-target}
source_url=${QR_OBJECT_ARCHIVE_SOURCE_URL:-}
target_url=${QR_OBJECT_ARCHIVE_TARGET_URL:-}
source_access=${QR_OBJECT_ARCHIVE_SOURCE_ACCESS_KEY_FILE:-}
source_secret=${QR_OBJECT_ARCHIVE_SOURCE_SECRET_KEY_FILE:-}
target_access=${QR_OBJECT_ARCHIVE_TARGET_ACCESS_KEY_FILE:-}
target_secret=${QR_OBJECT_ARCHIVE_TARGET_SECRET_KEY_FILE:-}
ca=${QR_OBJECT_ARCHIVE_CA:-}
mc_bin=${QR_OBJECT_ARCHIVE_MC:-mc}
mc_image=${QR_OBJECT_ARCHIVE_IMAGE:-}
network=${QR_OBJECT_ARCHIVE_NETWORK:-}
evidence=${QR_OBJECT_ARCHIVE_EVIDENCE:-}

usage() {
  awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
}

die() {
  printf 'object-archive: %s\n' "$*" >&2
  exit 1
}

usage_error() {
  printf 'object-archive: %s\n\n' "$*" >&2
  usage >&2
  exit 2
}

unverified() {
  printf 'object-archive: Unverified: %s\n' "$*" >&2
  exit 3
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --plan | --dry-run)
      plan=1
      shift
      ;;
    --confirm)
      confirm=1
      shift
      ;;
    --restore)
      restore=1
      shift
      ;;
    --descriptor)
      [ "$#" -ge 2 ] || usage_error "--descriptor needs a value"
      descriptor=$2
      shift 2
      ;;
    --source-bucket)
      [ "$#" -ge 2 ] || usage_error "--source-bucket needs a value"
      source_bucket=$2
      shift 2
      ;;
    --target-bucket)
      [ "$#" -ge 2 ] || usage_error "--target-bucket needs a value"
      target_bucket=$2
      shift 2
      ;;
    --source-alias)
      [ "$#" -ge 2 ] || usage_error "--source-alias needs a value"
      source_alias=$2
      shift 2
      ;;
    --target-alias)
      [ "$#" -ge 2 ] || usage_error "--target-alias needs a value"
      target_alias=$2
      shift 2
      ;;
    --evidence)
      [ "$#" -ge 2 ] || usage_error "--evidence needs a value"
      evidence=$2
      shift 2
      ;;
    -h | --help | help)
      usage
      exit 0
      ;;
    *)
      usage_error "unknown argument $1"
      ;;
  esac
done

# The source and target are always distinct buckets. A same-bucket mirror is a
# recursive copy or a no-op, never an archive; refuse it before a host is read.
[ -n "$source_bucket" ] || usage_error "--source-bucket must not be empty"
if [ -n "$target_bucket" ] && [ "$source_bucket" = "$target_bucket" ]; then
  usage_error "the source and target buckets must differ; '$source_bucket' is not an archive target"
fi
for name in source_alias target_alias; do
  value=${!name}
  case "$value" in
    '' | *[!A-Za-z0-9._-]*) usage_error "$name '$value' is not a safe mc alias" ;;
  esac
done
[ "$source_alias" != "$target_alias" ] \
  || usage_error "the source and target aliases must differ"

mode=archive
[ "$restore" -eq 1 ] && mode=restore

if [ "$plan" -eq 1 ]; then
  cat <<PLAN
object-archive: plan (no Docker, jq, mc, SSH, or secret file is read)
object-archive: mode: $mode
object-archive: descriptor: $descriptor
object-archive: source: bucket $source_bucket as $source_alias, read-only identity
object-archive: target: ${target_bucket:-<descriptor backup.object_archive.target>} as $target_alias, redacted in evidence
object-archive: 1 check: object_replication is false; the target bucket differs from the source
object-archive: 2 version: the source bucket keeps versioning enabled; the archive does not claim version history
object-archive: 3 mirror: mc mirror --overwrite --summary (one way; no --remove, no --watch, no mc replicate)
object-archive: 4 manifest: count/bytes and a SHA-256 for every key copied by this run
object-archive: 5 evidence: value-free JSON; target identity redacted; label Measured for this run only
object-archive: never: delete a source object, run mc replicate, or claim a second site, version history, or Real HA
object-archive: restore mode mirrors the archive into a fresh scratch bucket before rows; it never deletes the archive
object-archive: live requires --confirm; a missing source/target identity prints Unverified and exits non-zero
object-archive: $mode is an archive, not object replication; object_replication stays false
PLAN
  exit 0
fi

if [ "$confirm" -ne 1 ]; then
  printf 'object-archive: a live %s requires --confirm; no host was contacted\n' "$mode" >&2
  exit 2
fi

for tool in jq sha256sum awk; do
  command -v "$tool" >/dev/null 2>&1 || die "$tool is required for a live run"
done
[ -f "$descriptor" ] || die "descriptor $descriptor is missing"
jq -e . "$descriptor" >/dev/null || die "descriptor $descriptor is not JSON"
if [ "$restore" -eq 0 ]; then
  jq -e '.backup.object_archive.enabled == true' "$descriptor" >/dev/null \
    || unverified "backup.object_archive.enabled is false; no archive target is configured"
fi
replication=$(jq -r '.backup.object_replication // false' "$descriptor")
[ "$replication" = "false" ] \
  || die "backup.object_replication must be false; this tool is an archive, not replication"
if [ -z "$target_bucket" ]; then
  target_bucket=$(jq -r '.backup.object_archive.target // empty' "$descriptor")
fi
[ -n "$target_bucket" ] || die "no target bucket; pass --target-bucket or enable backup.object_archive"
[ "$source_bucket" != "$target_bucket" ] \
  || die "the source and target buckets are both $source_bucket; refuse a same-bucket archive"

if [ -z "$source_url" ]; then
  source_url=$(jq -r '.endpoints.object_store.scheme + "://" + .endpoints.object_store.host + ":" + (.endpoints.object_store.port | tostring)' "$descriptor")
fi
[ -n "$target_url" ] || die "QR_OBJECT_ARCHIVE_TARGET_URL is required; the target is operator-provided"
if [ -z "$ca" ]; then
  ca=$(jq -r '.endpoints.object_store.tls_ca // empty' "$descriptor")
fi
[ -n "$ca" ] && [ -r "$ca" ] || die "the object-store CA is not readable; set QR_OBJECT_ARCHIVE_CA"
for file in "$source_access" "$source_secret" "$target_access" "$target_secret"; do
  [ -n "$file" ] && [ -r "$file" ] \
    || die "source and target access/secret key files are required and must be readable"
done
for url in "$source_url" "$target_url"; do
  case "$url" in
    https://*) ;;
    *) die "the source and target endpoints must use https://" ;;
  esac
done

work=$(mktemp -d "${TMPDIR:-/tmp}/quantrule-object-archive.XXXXXX")
cleanup() {
  find "$work" -type f -delete 2>/dev/null || true
  find "$work" -depth -type d -empty -delete 2>/dev/null || true
}
trap cleanup EXIT
umask 077
mkdir -p "$work/mc"

# Build the client config from the files. The values are shell variables, not
# arguments; `printf` is a builtin, so no secret reaches a process list.
source_key=$(cat "$source_access")
source_credential=$(cat "$source_secret")
target_key=$(cat "$target_access")
target_credential=$(cat "$target_secret")
printf '{"version":"10","aliases":{"%s":{"url":"%s","accessKey":"%s","secretKey":"%s","api":"S3v4","path":"auto"},"%s":{"url":"%s","accessKey":"%s","secretKey":"%s","api":"S3v4","path":"auto"}}}\n' \
  "$source_alias" "$source_url" "$source_key" "$source_credential" \
  "$target_alias" "$target_url" "$target_key" "$target_credential" > "$work/mc/config.json"
chmod 600 "$work/mc/config.json"

mc_run() {
  if [ -n "$mc_image" ]; then
    local network_args=()
    [ -n "$network" ] && network_args=(--network "$network")
    docker run --rm "${network_args[@]}" \
      -v "$work/mc:/mc:ro" -v "$ca:/certs/ca.crt:ro" \
      -e SSL_CERT_FILE=/certs/ca.crt "$mc_image" \
      mc --config-dir /mc "$@"
  else
    SSL_CERT_FILE=$ca "$mc_bin" --config-dir "$work/mc" "$@"
  fi
}

started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
started_epoch=$(date -u +%s)
if [ -z "$evidence" ]; then
  evidence="deploy/v3/evidence/object-archive-$(printf '%s' "$started_at" | tr ':' '-').json"
fi
mkdir -p "$(dirname "$evidence")"
manifest="${evidence%.json}.manifest"

# The source policy is read-only, but the source bucket must still keep the
# versioning PLAT-005 owns. The archive does not enable it and makes no
# version-history claim; an archive of a versioned source is a copy, not a
# replacement.
if [ "$restore" -eq 0 ]; then
  versioning=$(mc_run version info "$source_alias/$source_bucket" 2>&1) \
    || unverified "cannot read versioning for $source_alias/$source_bucket"
  printf '%s\n' "$versioning" | grep -Eiq 'enabled' \
    || unverified "source versioning is not enabled; object version history is PLAT-005, not this archive"
  if printf '%s\n' "$versioning" | grep -Eiq 'not enabled|disabled'; then
    unverified "source versioning is disabled; refuse to call a copy an archive"
  fi
fi

mc_run mb --ignore-existing "$target_alias/$target_bucket" >/dev/null \
  || die "cannot create or reach the target bucket $target_bucket"

log="$work/mirror.jsonl"
summary="$work/summary.txt"
mc_run mirror --overwrite --summary --json \
  "$source_alias/$source_bucket" "$target_alias/$target_bucket" > "$log" 2> "$summary" \
  || die "mc mirror failed; no evidence is published"

# Counts and bytes come from the target, not from the transfer log, so a re-run
# that copies nothing still reports the archive size. The per-key manifest lists
# only keys copied by this run; an unchanged key is not re-hashed.
counts=$(mc_run du --json "$target_alias/$target_bucket" 2>/dev/null | tail -n 1 || true)
object_count=$(printf '%s\n' "$counts" | jq -r '.objects // 0' 2>/dev/null || printf '0')
object_bytes=$(printf '%s\n' "$counts" | jq -r '.size // 0' 2>/dev/null || printf '0')
case "$object_count$object_bytes" in
  *[!0-9]*) object_count=0; object_bytes=0 ;;
esac

printf '# quantrule.object-archive-manifest/v1\n' > "$manifest"
printf '# mode=%s source_bucket=%s target_bucket=%s\n' "$mode" "$source_bucket" "$target_bucket" >> "$manifest"
printf '# fields: key|bytes|sha256\n' >> "$manifest"
new_keys=0
new_bytes=0
while IFS= read -r line; do
  [ -n "$line" ] || continue
  target=$(printf '%s\n' "$line" | jq -r '.target // empty' 2>/dev/null || true)
  [ -n "$target" ] || continue
  key=${target#"$target_alias/$target_bucket/"}
  key=${key#"$target_bucket/"}
  case "$key" in
    '' | *$'\n'*) continue ;;
  esac
  bytes=$(printf '%s\n' "$line" | jq -r '.size // 0' 2>/dev/null || printf '0')
  case "$bytes" in
    *[!0-9]*) bytes=0 ;;
  esac
  digest=$(mc_run cat "$target_alias/$target_bucket/$key" 2>/dev/null | sha256sum | cut -d' ' -f1) \
    || die "cannot hash the archived key $key"
  printf '%s|%s|%s\n' "$key" "$bytes" "$digest" >> "$manifest"
  new_keys=$((new_keys + 1))
  new_bytes=$((new_bytes + bytes))
done < <(jq -c 'select(.status == "success")' "$log" 2>/dev/null || true)
chmod 600 "$manifest"

finished_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
finished_epoch=$(date -u +%s)
elapsed=$((finished_epoch - started_epoch))
descriptor_sha=$(sha256sum "$descriptor" | cut -d' ' -f1)
manifest_sha=$(sha256sum "$manifest" | cut -d' ' -f1)

jq -n \
  --arg schema "quantrule.object-archive/v1" \
  --arg mode "$mode" \
  --arg started "$started_at" \
  --arg finished "$finished_at" \
  --arg descriptor_sha256 "$descriptor_sha" \
  --arg source_bucket "$source_bucket" \
  --arg target_bucket "$target_bucket" \
  --arg manifest_sha256 "$manifest_sha" \
  --argjson object_count "$object_count" \
  --argjson object_bytes "$object_bytes" \
  --argjson new_keys "$new_keys" \
  --argjson new_bytes "$new_bytes" \
  --argjson elapsed_seconds "$elapsed" \
  --arg manifest "$manifest" \
  '{
    schema: $schema,
    mode: $mode,
    started_utc: $started,
    finished_utc: $finished,
    descriptor_sha256: $descriptor_sha256,
    source_bucket: $source_bucket,
    target: "redacted",
    target_bucket: $target_bucket,
    objects: {count: $object_count, bytes: $object_bytes},
    copied: {keys: $new_keys, bytes: $new_bytes},
    manifest: {path: $manifest, sha256: $manifest_sha256},
    elapsed_seconds: $elapsed_seconds,
    labels: ["Measured"],
    claims: {
      one_way: true,
      object_replication: false,
      version_history: false,
      second_site: false,
      source_delete: false
    },
    notes: [
      "This is an archive, not replication; the target is operator-owned.",
      "Object RPO is the archive interval, not the backup interval.",
      "Target versioning and at-rest encryption are operator-owned and not claimed."
    ]
  }' > "$evidence"
chmod 600 "$evidence"

printf 'object-archive: %s complete\n' "$mode"
printf 'object-archive: source bucket %s; target %s; objects %s files, %s bytes\n' \
  "$source_bucket" "$target_bucket" "$object_count" "$object_bytes"
printf 'object-archive: copied %s keys, %s bytes; manifest %s\n' "$new_keys" "$new_bytes" "$manifest"
printf 'object-archive: evidence %s; labels Measured for this run only\n' "$evidence"
printf 'object-archive: archive, not replication; object_replication false; Real HA is HA-014\n'
