#!/usr/bin/env bash
# Build release evidence for the QuantRule application image.
#
# The operator/CI seam for the supply chain. It never signs in CI, holds no
# private key, pushes no image, and starts no registry. `--plan` reads no Docker,
# network, registry, key, or secret; the other commands read only their named
# inputs and write inside their output directory.
#
# Usage:
#   tools/release.sh --plan
#   tools/release.sh sbom <reference@sha256:...> <out.spdx.json>
#   tools/release.sh scan <sbom.spdx.json> <out-dir>
#   tools/release.sh sign <reference@sha256:...> <out-dir> [sbom.spdx.json]
#   tools/release.sh verify <reference@sha256:...> [bundle-dir]
#   tools/release.sh bundle <reference@sha256:...> <out-dir> [evidence-dir]
#   tools/release.sh pin --descriptor <deploy.json> --reference <ref> --digest <sha256:...>
#   tools/release.sh install-tools <dir>
#
# Environment: QR_RELEASE_ALLOWLIST, QR_RELEASE_PUBLIC_KEY, QR_RELEASE_PRIVATE_KEY
# (or COSIGN_PRIVATE_KEY), QR_RELEASE_IMAGE, QR_RELEASE_TODAY, SYFT_BIN, GRYPE_BIN,
# COSIGN_BIN, SKOPEO_BIN, QR_DEPLOY_BIN.
#
# No secret is printed or passed as an argument. No `curl | sh`, no unverified
# download, and no removal outside this tool's own temporary directory.
set -euo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "$script_dir/.." && pwd)
cd "$root"
readonly COSIGN_VERSION="v3.1.4"
readonly COSIGN_SHA256="424e2c97d0b45fd7a6b608a87f090f35633f4c23ece5f092d48bba11aaf4bbfa"
readonly SYFT_VERSION="1.57.2"
readonly SYFT_SHA256="ad92bd3ec1b94a3df01d4d0aec199afa9c42b72e86da436ff5f837914d32706b"
readonly GRYPE_VERSION="0.122.1"
readonly GRYPE_SHA256="f1ba7e2917ee1395618de20a98da0aa7c0fb1788927797651047c50502a37538"
readonly SKOPEO_IMAGE="quay.io/skopeo/stable@sha256:7e7d5b9c9f30896e8fd256d33ba351408299399442f04b8ef3a9452f3c2fa50e"
readonly REGISTRY_IMAGE="registry:2@sha256:a3d8aaa63ed8681a604f1dea0aa03f100d5895b6a58ace528858a7b332415373"
readonly CARGO_AUDIT_VERSION="0.23.0"
readonly ALLOWLIST_VERSION="quantrule.cve-allowlist/v1"
readonly SIGNATURE_VERSION="quantrule.image-signature/v1"
readonly REPORT_VERSION="quantrule.cve-report/v1"
allowlist=${QR_RELEASE_ALLOWLIST:-deploy/release/cve-allowlist.json}
public_key=${QR_RELEASE_PUBLIC_KEY:-deploy/v3/cosign.pub}
today=${QR_RELEASE_TODAY:-}
command=${1:-}
shift || true
die() {
  printf 'release: %s\n' "$*" >&2
  exit 1
}
usage_error() {
  printf 'release: %s\n\n' "$*" >&2
  usage >&2
  exit 2
}
usage() {
  awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
}
need_command() {
  command -v "$1" >/dev/null 2>&1 || die "$1 is required for $2"
}
sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    die "sha256sum or shasum is required"
  fi
}
is_digest() {
  printf '%s' "$1" | grep -Eq '^sha256:[0-9a-f]{64}$'
}
is_reference() {
  case "$1" in
    *[[:space:]]* | "" | *://*) return 1 ;;
    */*) return 0 ;;
    *) return 1 ;;
  esac
}
# Accept `reference@sha256:...` or a bare digest plus QR_RELEASE_IMAGE.
parse_target() {
  local input=$1
  case "$input" in
    *@sha256:*)
      reference=${input%@*}
      digest=${input##*@}
      ;;
    sha256:*)
      digest=$input
      reference=${QR_RELEASE_IMAGE:-}
      [ -n "$reference" ] || die "a bare digest needs QR_RELEASE_IMAGE naming the repository"
      ;;
    *)
      die "target must be reference@sha256:<64 lowercase hex>, got $input"
      ;;
  esac
  is_reference "$reference" || die "image reference is not a registry reference: $reference"
  is_digest "$digest" || die "image digest is not sha256:<64 lowercase hex>: $digest"
  target="$reference@$digest"
}
resolve_tool() {
  local name=$1 override=${2:-}
  if [ -n "$override" ]; then
    [ -x "$override" ] || die "$name override is not executable: $override"
    printf '%s' "$override"
    return 0
  fi
  command -v "$name" >/dev/null 2>&1 || die "$name is required; run tools/release.sh install-tools <dir> or install it from the pinned source"
  command -v "$name"
}
utc_today() {
  if [ -n "$today" ]; then
    printf '%s' "$today"
  else
    date -u +%F
  fi
}
check_date() {
  printf '%s' "$1" | grep -Eq '^[0-9]{4}-[0-9]{2}-[0-9]{2}$' || return 1
}
plan() {
  cat <<PLAN
release: plan (no Docker, network, registry, signing key, or secret is read)
release: 1 gate: tools/gate.sh and the annotated tag/release-note checks pass
release: 2 build: one linux/amd64 image from Dockerfile.release; keep release-binaries-<sha>
release: 3 local checks: no Python/Bun, version contract, composed tier on the local image
release: 4 staging push: push the image as staging-<commit>; inspect its digest
release: 5 sbom: syft $SYFT_VERSION SPDX JSON from the exact staging digest
release: 6 scan: Grype $GRYPE_VERSION plus cargo-audit $CARGO_AUDIT_VERSION; Critical and fixable High fail
release: 7 version push: vX.Y.Z and sha-<commit> only after the scan passes; digest must equal staging
release: 8 release: attach sbom-<tag>.spdx.json and cve-report-<tag>.json; body carries digest, hashes, signing step, fixture blocker
release: manual: after the version image is pushed, the operator signs with the offline key:
release:   COSIGN_PASSWORD=<vault> tools/release.sh sign ghcr.io/quant-rule/quantrule-platform@sha256:<digest> out/release sbom-<tag>.spdx.json
release:   # writes cosign.bundle, attestation.bundle, signed-image/, and signature.json metadata
release:   tools/release.sh verify ghcr.io/quant-rule/quantrule-platform@sha256:<digest> out/release
release:   # offline proof: cosign verify --key cosign.pub --local-image out/release/signed-image <target>
release:   tools/release.sh bundle ghcr.io/quant-rule/quantrule-platform@sha256:<digest> out/bundle out/release
release: tools: cosign=$COSIGN_VERSION sha256=$COSIGN_SHA256
release: tools: syft=$SYFT_VERSION sha256=$SYFT_SHA256
release: tools: grype=$GRYPE_VERSION sha256=$GRYPE_SHA256
release: tools: skopeo image=$SKOPEO_IMAGE (bundle path)
release: tools: registry image=$REGISTRY_IMAGE (transient offline serve)
release: tools: cargo-audit=$CARGO_AUDIT_VERSION (crates.io)
release: evidence: sbom-<tag>.spdx.json, cve-report-<tag>.json, Sigstore cosign.bundle + attestation.bundle, signed-image/
release: blocker: app, minio, minio_client, and the derived Patroni pgBackRest image remain fixture digests; --release names each field (PLAT-008, HA-013)
release: boundary: a live push, signature, SBOM, scan, and the three-VM lab are Unverified here; Real HA is HA-014
PLAN
}
fetch_verified() {
  local url=$1 checksum=$2 destination=$3 label=$4
  curl -fsSL --retry 3 --retry-delay 2 -o "$destination" "$url" \
    || die "cannot download $label from $url"
  printf '%s  %s\n' "$checksum" "$destination" | sha256sum -c - >/dev/null \
    || die "$label checksum mismatch"
}
install_tools() {
  local directory=${1:-}
  [ -n "$directory" ] || usage_error "install-tools needs an output directory"
  need_command curl install-tools; need_command tar install-tools; need_command sha256sum install-tools
  mkdir -p "$directory"
  local temporary
  temporary=$(mktemp -d "${TMPDIR:-/tmp}/qr-release-tools.XXXXXX")
  trap 'rm -rf "$temporary"' EXIT
  fetch_verified "https://github.com/sigstore/cosign/releases/download/$COSIGN_VERSION/cosign-linux-amd64" \
    "$COSIGN_SHA256" "$temporary/cosign" "cosign $COSIGN_VERSION"
  install -m 755 "$temporary/cosign" "$directory/cosign"
  fetch_verified "https://github.com/anchore/syft/releases/download/v$SYFT_VERSION/syft_${SYFT_VERSION}_linux_amd64.tar.gz" \
    "$SYFT_SHA256" "$temporary/syft.tar.gz" "syft $SYFT_VERSION"
  tar -xzf "$temporary/syft.tar.gz" -C "$temporary" syft
  install -m 755 "$temporary/syft" "$directory/syft"
  fetch_verified "https://github.com/anchore/grype/releases/download/v$GRYPE_VERSION/grype_${GRYPE_VERSION}_linux_amd64.tar.gz" \
    "$GRYPE_SHA256" "$temporary/grype.tar.gz" "grype $GRYPE_VERSION"
  tar -xzf "$temporary/grype.tar.gz" -C "$temporary" grype
  install -m 755 "$temporary/grype" "$directory/grype"
  printf 'release: installed cosign %s, syft %s, grype %s in %s\n' "$COSIGN_VERSION" "$SYFT_VERSION" "$GRYPE_VERSION" "$directory"
  printf 'release: skopeo is pinned as %s; cargo install cargo-audit --version %s --locked\n' "$SKOPEO_IMAGE" "$CARGO_AUDIT_VERSION"
}
sbom() {
  [ "$#" -eq 2 ] || usage_error "sbom needs <reference@sha256:...> <out.spdx.json>"
  local output=$2
  parse_target "$1"
  local syft
  syft=$(resolve_tool syft "${SYFT_BIN:-}")
  mkdir -p "$(dirname -- "$output")"
  "$syft" --quiet "$target" -o "spdx-json=$output" >/dev/null
  [ -s "$output" ] || die "syft wrote no SBOM to $output"
  printf 'subject\t%s\n' "$target"
  printf 'sbom\t%s\n' "$output"
  printf 'sha256\t%s\n' "$(sha256_file "$output")"
}
validate_allowlist() {
  [ -f "$allowlist" ] || die "allowlist is missing: $allowlist"
  jq -e --arg v "$ALLOWLIST_VERSION" '
    .v == $v and (.entries | type == "array")
    and ([.entries[] | (keys | sort) == (["expires","id","owner","package","reason","severity","ticket","version"] | sort)
      and (.id | type == "string" and length > 0) and (.package | type == "string" and length > 0)
      and (.version | type == "string" and length > 0)
      and (.severity | type == "string" and (. == "High" or . == "Critical"))
      and (.reason | type == "string" and length > 0) and (.owner | type == "string" and length > 0)
      and (.expires | type == "string" and test("^[0-9]{4}-[0-9]{2}-[0-9]{2}$"))
      and (.ticket | type == "string" and length > 0)] | all)
  ' "$allowlist" >/dev/null || die "allowlist is malformed: $allowlist"
  local current
  current=$(utc_today)
  check_date "$current" || die "QR_RELEASE_TODAY must be YYYY-MM-DD"
  if jq -e --arg today "$current" '.entries[] | select(.expires < $today)' "$allowlist" >/dev/null; then
    die "allowlist has an expired entry: $allowlist"
  fi
}
allowlisted() {
  local id=$1 package=$2 version=$3 severity=$4
  jq -e --arg id "$id" --arg package "$package" --arg version "$version" \
    --arg severity "$severity" '
      .entries[] | select(.id == $id and .package == $package
        and .version == $version and .severity == $severity)
    ' "$allowlist" >/dev/null
}
scan() {
  [ "$#" -eq 2 ] || usage_error "scan needs <sbom.spdx.json> <out-dir>"
  local report=$1 output=$2
  [ -s "$report" ] || die "SBOM is missing or empty: $report"
  local grype
  grype=$(resolve_tool grype "${GRYPE_BIN:-}")
  need_command jq scan
  mkdir -p "$output"
  local grype_json="$output/grype.json"
  local grype_text="$output/grype.txt"
  local audit_json="$output/cargo-audit.json"
  local audit_err="$output/cargo-audit.err"
  "$grype" "$report" -o json --file "$grype_json" >/dev/null
  jq -r '
    "severity\tid\tpackage\tversion\tfix",
    (.matches[]? | [
      (.vulnerability.severity // "Unknown"),
      (.vulnerability.id // ""),
      (.artifact.name // ""),
      (.artifact.version // ""),
      (if ((.vulnerability.fix.state // "") == "fixed"
           or (((.vulnerability.fix.versions // []) | length) > 0))
       then "fixed" else "unfixed" end)
    ] | @tsv)
  ' "$grype_json" >"$grype_text"
  local audit_status=0
  set +e
  cargo audit --json >"$audit_json" 2>"$audit_err"
  audit_status=$?
  set -e
  if [ "$audit_status" -gt 1 ]; then
    sed -n '1,20p' "$audit_err" >&2
    die "cargo audit failed to run"
  fi
  jq -r '
    (.vulnerabilities.list[]? | [
      (if (((.advisory.cvss // []) | map(.score // 0) | max // 0) >= 9.0)
       then "Critical" else "High" end),
      (.advisory.id // ""),
      (.package.name // ""),
      (.package.version // ""),
      (if (((.versions.patched // []) | length) > 0) then "fixed" else "unfixed" end)
    ] | @tsv)
  ' "$audit_json" >>"$grype_text"
  validate_allowlist
  local critical=0 high=0 fixable=0 medium=0 low=0 allowed=0 violations=0
  local severity id package version fix
  while IFS=$'\t' read -r severity id package version fix; do
    [ "$severity" = "severity" ] && continue
    [ -n "$id" ] || continue
    case "$severity" in
      Critical)
        critical=$((critical + 1)); violations=$((violations + 1))
        printf 'release: FAIL Critical %s %s %s\n' "$id" "$package" "$version" >&2 ;;
      High)
        high=$((high + 1))
        if [ "$fix" = "fixed" ]; then
          fixable=$((fixable + 1)); violations=$((violations + 1))
          printf 'release: FAIL fixable High %s %s %s\n' "$id" "$package" "$version" >&2
        elif allowlisted "$id" "$package" "$version" High; then
          allowed=$((allowed + 1))
        else
          violations=$((violations + 1))
          printf 'release: FAIL unfixed High without allowlist %s %s %s\n' "$id" "$package" "$version" >&2
        fi ;;
      Medium) medium=$((medium + 1)) ;;
      Low) low=$((low + 1)) ;;
    esac
  done <"$grype_text"
  local sbom_sha grype_sha audit_sha gate
  sbom_sha=$(sha256_file "$report"); grype_sha=$(sha256_file "$grype_json"); audit_sha=$(sha256_file "$audit_json")
  gate=pass; [ "$violations" -eq 0 ] || gate=fail
  jq -n --arg v "$REPORT_VERSION" --arg gate "$gate" --arg sbom_sha "$sbom_sha" --arg grype_sha "$grype_sha" \
    --arg audit_sha "$audit_sha" --argjson critical "$critical" --argjson high "$high" --argjson fixable "$fixable" \
    --argjson medium "$medium" --argjson low "$low" --argjson allowed "$allowed" --argjson violations "$violations" \
    '{v:$v, gate:$gate, sbom_sha256:$sbom_sha, grype_sha256:$grype_sha, cargo_audit_sha256:$audit_sha,
      counts:{critical:$critical, high:$high, fixable_high:$fixable, medium:$medium, low:$low,
        allowlisted:$allowed, violations:$violations}}' >"$output/summary.json"
  printf 'release: scan gate=%s critical=%s high=%s fixable_high=%s medium=%s low=%s allowlisted=%s violations=%s\n' \
    "$gate" "$critical" "$high" "$fixable" "$medium" "$low" "$allowed" "$violations"
  printf 'release: report %s\nrelease: report sha256 %s\n' \
    "$output/summary.json" "$(sha256_file "$output/summary.json")"
  [ "$violations" -eq 0 ] || return 1
}
fingerprint() {
  local key=$1 der
  [ -f "$key" ] || die "public key is missing: $key (generate a cosign key pair and commit deploy/v3/cosign.pub)"
  if command -v openssl >/dev/null 2>&1; then
    der=$(mktemp "${TMPDIR:-/tmp}/qr-release-key.XXXXXX")
    if openssl pkey -pubin -in "$key" -outform DER -out "$der" 2>/dev/null; then
      sha256_file "$der"
      rm -f "$der"
      return 0
    fi
    rm -f "$der"
  fi
  sha256_file "$key"
}
sign() {
  [ "$#" -ge 2 ] && [ "$#" -le 3 ] || usage_error "sign needs <reference@sha256:...> <out-dir> [sbom.spdx.json]"
  local output=$2 sbom=${3:-}
  parse_target "$1"
  local cosign key private
  cosign=$(resolve_tool cosign "${COSIGN_BIN:-}")
  private=${QR_RELEASE_PRIVATE_KEY:-${COSIGN_PRIVATE_KEY:-}}
  [ -n "$private" ] || die "set QR_RELEASE_PRIVATE_KEY or COSIGN_PRIVATE_KEY to the offline cosign private key path"
  [ -f "$private" ] || die "cosign private key path is not a file"
  key=$public_key
  mkdir -p "$output"
  local log="$output/cosign-sign.txt"
  if ! "$cosign" sign --key "$private" --bundle "$output/cosign.bundle" --yes "$target" >"$log" 2>&1; then
    sed -n '1,20p' "$log" >&2
    die "cosign sign failed"
  fi
  [ -s "$output/cosign.bundle" ] || die "cosign sign wrote no signature bundle"
  local attest_sha=""
  if [ -n "$sbom" ]; then
    [ -s "$sbom" ] || die "SBOM is missing or empty: $sbom"
    if ! "$cosign" attest --type spdxjson --predicate "$sbom" --key "$private" --bundle "$output/attestation.bundle" --yes "$target" >"$output/cosign-attest.txt" 2>&1; then
      sed -n '1,20p' "$output/cosign-attest.txt" >&2
      die "cosign attest failed"
    fi
    [ -s "$output/attestation.bundle" ] || die "cosign attest wrote no attestation bundle"
    attest_sha=$(sha256_file "$output/attestation.bundle")
    printf 'release: SBOM attested (attestation bundle sha256:%s)\n' "$attest_sha"
    attest_sha="sha256:$attest_sha"
  fi
  if ! "$cosign" save --dir "$output/signed-image" "$target" >"$output/cosign-save.txt" 2>&1; then
    sed -n '1,20p' "$output/cosign-save.txt" >&2
    die "cosign save failed"
  fi
  [ -s "$output/signed-image/index.json" ] || die "cosign save wrote no signed-image OCI layout"
  local key_fp log_sha sig_sha bundle
  key_fp=$(fingerprint "$key")
  log_sha=$(sha256_file "$log")
  sig_sha=$(sha256_file "$output/cosign.bundle")
  bundle="$output/signature.json"
  jq -n --arg v "$SIGNATURE_VERSION" --arg reference "$reference" --arg digest "$digest" \
    --arg key_fp "sha256:$key_fp" --arg log_sha "sha256:$log_sha" \
    --arg sig_sha "sha256:$sig_sha" --arg attest_sha "$attest_sha" --arg signed_image "signed-image" \
    '{v:$v, reference:$reference, digest:$digest, public_key_fingerprint:$key_fp,
      signature_bundle_sha256:$sig_sha, attestation_bundle_sha256:$attest_sha,
      signed_image:$signed_image, cosign_log_sha256:$log_sha, keyless:false}' >"$bundle"
  printf 'release: signed %s\n' "$target"
  printf 'release: public key fingerprint sha256:%s\n' "$key_fp"
  printf 'release: signature bundle %s sha256:%s\n' "$output/cosign.bundle" "$sig_sha"
  if [ -n "$attest_sha" ]; then
    printf 'release: attestation bundle %s %s\n' "$output/attestation.bundle" "$attest_sha"
  fi
  printf 'release: signed image %s\n' "$output/signed-image"
  printf 'release: metadata %s sha256:%s\n' "$bundle" "$(sha256_file "$bundle")"
}

check_recorded_hash() {
  local manifest=$1 key=$2 file=$3 label=$4 recorded actual
  recorded=$(jq -r --arg key "$key" '.[$key] // empty' "$manifest")
  [ -n "$recorded" ] || return 0
  [ -f "$file" ] || die "the bundle records $label but $file is missing"
  actual="sha256:$(sha256_file "$file")"
  [ "$recorded" = "$actual" ] || die "$label digest mismatch: recorded $recorded, computed $actual"
}
verify() {
  [ "$#" -ge 1 ] && [ "$#" -le 2 ] || usage_error "verify needs <reference@sha256:...> [bundle-dir]"
  local bundle=${2:-}
  parse_target "$1"
  local cosign
  cosign=$(resolve_tool cosign "${COSIGN_BIN:-}")
  [ -f "$public_key" ] || die "public key is missing: $public_key; verification fails closed"
  local proof=registry manifest
  if [ -n "$bundle" ]; then
    manifest="$bundle/signature.json"
    [ -f "$manifest" ] || die "bundle has no signature.json: $bundle"
    jq -e --arg v "$SIGNATURE_VERSION" --arg digest "$digest" '
      .v == $v and .digest == $digest
    ' "$manifest" >/dev/null || die "bundle does not describe $digest"
    check_recorded_hash "$manifest" signature_bundle_sha256 "$bundle/cosign.bundle" "signature bundle"
    check_recorded_hash "$manifest" attestation_bundle_sha256 "$bundle/attestation.bundle" "attestation bundle"
    if [ -d "$bundle/signed-image" ]; then
      proof=signed-image
    fi
  fi
  if [ "$proof" = signed-image ]; then
    if ! "$cosign" verify --key "$public_key" --local-image "$bundle/signed-image" "$target" >/dev/null 2>&1; then
      die "cosign verification failed for $target from the signed-image bundle"
    fi
  elif ! "$cosign" verify --key "$public_key" "$target" >/dev/null 2>&1; then
    die "cosign verification failed for $target"
  fi
  if [ -n "$bundle" ]; then
    printf 'release: bundle %s sha256:%s\n' "$manifest" "$(sha256_file "$manifest")"
  fi
  printf 'release: verified %s (%s proof)\n' "$target" "$proof"
  printf 'release: public key %s fingerprint sha256:%s\n' "$public_key" "$(fingerprint "$public_key")"
}
bundle() {
  [ "$#" -ge 2 ] && [ "$#" -le 3 ] || usage_error "bundle needs <reference@sha256:...> <out-dir> [evidence-dir]"
  local output=$2 evidence=${3:-}
  parse_target "$1"
  local skopeo
  skopeo=$(resolve_tool skopeo "${SKOPEO_BIN:-}")
  mkdir -p "$output"
  "$skopeo" copy "docker://$target" "oci:$output:latest" >/dev/null
  local proof=absent offline_verify=""
  if [ -n "$evidence" ]; then
    local manifest="$evidence/signature.json"
    [ -f "$manifest" ] || die "evidence has no signature.json: $evidence"
    jq -e --arg v "$SIGNATURE_VERSION" --arg digest "$digest" '
      .v == $v and .digest == $digest
    ' "$manifest" >/dev/null || die "evidence signature.json does not describe $digest"
    [ -f "$evidence/cosign.bundle" ] || die "evidence has no cosign.bundle: $evidence"
    [ -d "$evidence/signed-image" ] || die "evidence has no signed-image/ directory: $evidence"
    check_recorded_hash "$manifest" signature_bundle_sha256 "$evidence/cosign.bundle" "signature bundle"
    cp "$manifest" "$output/signature.json"
    cp "$evidence/cosign.bundle" "$output/cosign.bundle"
    if [ -f "$evidence/attestation.bundle" ]; then
      check_recorded_hash "$manifest" attestation_bundle_sha256 "$evidence/attestation.bundle" "attestation bundle"
      cp "$evidence/attestation.bundle" "$output/attestation.bundle"
    elif jq -e '.attestation_bundle_sha256 | length > 0' "$manifest" >/dev/null; then
      die "evidence records an attestation bundle that is missing: $evidence/attestation.bundle"
    fi
    mkdir -p "$output/signed-image"
    cp -R "$evidence/signed-image/." "$output/signed-image/"
    proof=present
    offline_verify="cosign verify --key $public_key --local-image $output/signed-image $target"
  fi
  local temporary files bundle_hash
  temporary=$(mktemp "${TMPDIR:-/tmp}/qr-release-bundle.XXXXXX")
  (cd "$output" && find . -type f ! -name bundle.json -print0 | sort -z \
    | while IFS= read -r -d '' file; do
        printf '%s\t%s\n' "${file#./}" "$(sha256_file "$file")"
      done) >"$temporary"
  files=$(jq -Rn '[inputs | split("\t") | {path: .[0], sha256: .[1]}]' <"$temporary")
  rm -f "$temporary"
  jq -n --arg reference "$reference" --arg digest "$digest" --arg proof "$proof" \
    --arg offline "$offline_verify" --argjson files "$files" \
    '{v:"quantrule.oci-bundle/v1", reference:$reference, digest:$digest,
      signed_image:$proof, offline_verify:$offline, files:$files}' \
    >"$output/bundle.json"
  bundle_hash=$(sha256_file "$output/bundle.json")
  printf 'release: bundle %s\n' "$output"
  printf 'reference\t%s\ndigest\t%s\nsigned_image\t%s\nbundle_sha256\t%s\n' \
    "$reference@$digest" "$digest" "$proof" "$bundle_hash"
  printf 'release: offline load: skopeo copy oci:%s:latest docker-daemon:%s\n' "$output" "$reference@$digest"
  if [ "$proof" = present ]; then
    printf 'release: offline verify: %s\n' "$offline_verify"
  else
    printf 'release: offline verify: unavailable; signed-image proof absent (pass the sign evidence directory)\n'
  fi
  printf 'release: transient serve: docker run -d --rm -p 127.0.0.1:5000:5000 %s (loopback only; not started by this tool)\n' "$REGISTRY_IMAGE"
}
pin() {
  local descriptor="" reference="" digest=""
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --descriptor) [ "$#" -ge 2 ] || usage_error "--descriptor needs a value"; descriptor=$2; shift 2 ;;
      --reference) [ "$#" -ge 2 ] || usage_error "--reference needs a value"; reference=$2; shift 2 ;;
      --digest) [ "$#" -ge 2 ] || usage_error "--digest needs a value"; digest=$2; shift 2 ;;
      *) usage_error "unknown pin argument $1" ;;
    esac
  done
  [ -n "$descriptor" ] || usage_error "pin needs --descriptor <path>"
  [ -n "$reference" ] || usage_error "pin needs --reference <registry/repository>"
  [ -n "$digest" ] || usage_error "pin needs --digest <sha256:...>"
  is_reference "$reference" || die "--reference must be a registry reference without a tag or digest"
  case "${reference##*/}" in *:*) die "--reference must not carry a tag; pass the repository only" ;; esac
  is_digest "$digest" || die "--digest must be sha256:<64 lowercase hex>"
  [ -f "$descriptor" ] || die "descriptor is missing: $descriptor"
  need_command jq pin
  local mode
  mode=$(stat -c %a "$descriptor" 2>/dev/null || printf '644')
  jq -e '.v == "quantrule.deploy/v3" and (.image | type == "object")' "$descriptor" >/dev/null \
    || die "descriptor is not a quantrule.deploy/v3 document with an image pin"
  local before after output
  before=$(jq -S -c '.images' "$descriptor")
  output=$(mktemp "${descriptor}.XXXXXX")
  trap 'rm -f "$output"' EXIT
  jq --arg reference "$reference" --arg digest "$digest" \
    '.image = {reference:$reference, digest:$digest}' "$descriptor" >"$output"
  chmod "$mode" "$output"
  after=$(jq -S -c '.images' "$output")
  [ "$before" = "$after" ] || die "refusing to change the infrastructure pins"
  if [ -n "${QR_DEPLOY_BIN:-}" ]; then
    "$QR_DEPLOY_BIN" validate "$output" >/dev/null || die "qr-deploy validate rejected the pinned descriptor"
  elif command -v qr-deploy >/dev/null 2>&1; then
    qr-deploy validate "$output" >/dev/null || die "qr-deploy validate rejected the pinned descriptor"
  else
    cargo run --quiet -p quantrule-deploy -- validate "$output" >/dev/null \
      || die "qr-deploy validate rejected the pinned descriptor"
  fi
  mv "$output" "$descriptor"
  trap - EXIT
  printf 'release: pinned %s\n' "$reference@$digest"
  printf 'release: descriptor %s\n' "$descriptor"
}
case "$command" in
  '' | -h | --help | help) usage ;;
  --plan | plan) [ "$#" -eq 0 ] || usage_error "--plan takes no arguments"; plan ;;
  install-tools) install_tools "$@" ;;
  sbom) sbom "$@" ;;
  scan) scan "$@" ;;
  sign) sign "$@" ;;
  verify) verify "$@" ;;
  bundle) bundle "$@" ;;
  pin) pin "$@" ;;
  *) usage_error "unknown command $command" ;;
esac
