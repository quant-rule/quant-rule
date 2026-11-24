#!/usr/bin/env bash
# Render and operate the v3 Swarm HA control plane.
#
# This is the operator wrapper around the checked-in descriptor and the
# generated `bootstrap.sh`. It never reads a secret value, never passes one as
# an argument, never deletes a stack, secret, config, or volume, and never calls
# a Docker client crate. The fast test tier uses a fake `docker` binary and
# `QR_DEPLOY_BIN`; no Docker daemon is required for `validate` or `render`.
#
# `bootstrap` and `upgrade` verify the app image signature with
# `tools/release.sh verify` before they render or deploy. `swarm-ha` and
# `single-az` fail closed. `lab-single-host` may continue only with the explicit
# `--allow-unverified` flag, and prints `Unverified`, never a pass.
#
# Usage: tools/swarm.sh [--allow-unverified] <command>
#
#   validate   Validate the descriptor and the local secret/CA files.
#   render     Render the stack, infra skeletons, jobs, secrets, and PKI.
#   bootstrap  Verify, render, then run the generated bootstrap on a Swarm manager.
#   status     Show the stack services and the nodes that carry this deployment.
#   upgrade    Validate, verify, snapshot the current render, render the new one, and bootstrap.
#   rollback   Redeploy the last snapshot; never removes the stack or its data.
#   verify     Run `qr-deploy check` against the ingress; extra flags are forwarded.
#
# Environment:
#   DESCRIPTOR        descriptor path (default deploy/v3/deploy.example.json)
#   OUT               render directory (default deploy/v3/out)
#   TARGET            render target: swarm, infra, or all (default all)
#   STACK             override the stack name from render.json
#   QR_SWARM_STATE    snapshot directory (default <OUT>/.swarm-state)
#   QR_DEPLOY_BIN     qr-deploy binary; otherwise qr-deploy on PATH, then cargo
#   QR_RELEASE_PUBLIC_KEY / COSIGN_BIN  forwarded to tools/release.sh verify
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "$script_dir/.." && pwd)
cd "$root"

descriptor=${DESCRIPTOR:-deploy/v3/deploy.example.json}
out=${OUT:-deploy/v3/out}
target=${TARGET:-all}
state=${QR_SWARM_STATE:-$out/.swarm-state}

die() {
  echo "swarm: $*" >&2
  exit 1
}

usage() {
  awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
}

qr_deploy() {
  if [ -n "${QR_DEPLOY_BIN:-}" ]; then
    "$QR_DEPLOY_BIN" "$@"
  elif command -v qr-deploy >/dev/null 2>&1; then
    qr-deploy "$@"
  else
    cargo run --quiet -p quantrule-deploy -- "$@"
  fi
}

require_docker() {
  command -v docker >/dev/null 2>&1 || die "docker is required for $1"
  docker info >/dev/null 2>&1 || die "cannot reach the Docker daemon"
}

stack_name() {
  if [ -n "${STACK:-}" ]; then
    printf '%s' "$STACK"
    return 0
  fi
  if [ -f "$out/render.json" ]; then
    name=$(sed -n 's/.*"stack":[[:space:]]*"\([^"]*\)".*/\1/p' "$out/render.json" | head -n 1)
    if [ -n "$name" ]; then
      printf '%s' "$name"
      return 0
    fi
  fi
  printf 'quantrule'
}

render() {
  qr_deploy render "$descriptor" --out "$out" --target "$target"
}

snapshot_previous() {
  [ -f "$out/stack.yaml" ] || \
    die "upgrade needs the current rendered stack at $out/stack.yaml; render or bootstrap it first"
  [ -f "$out/render.json" ] || \
    die "upgrade needs the current render manifest at $out/render.json"
  stamp=$(date -u +%Y%m%dT%H%M%SZ)
  directory="$state/snapshots/$stamp"
  suffix=1
  while [ -e "$directory" ]; do
    directory="$state/snapshots/$stamp-$suffix"
    suffix=$((suffix + 1))
  done
  mkdir -p "$directory"
  cp "$out/stack.yaml" "$directory/stack.yaml"
  cp "$out/render.json" "$directory/render.json"
  printf '%s\n' "$(stack_name)" >"$directory/stack-name"
  docker stack services "$(stack_name)" >"$directory/services.txt" 2>/dev/null || true
  printf '%s\n' "$directory" >"$state/latest"
  printf 'swarm: snapshot %s\n' "$directory"
}

verify_release() {
  command -v jq >/dev/null 2>&1 || die "jq is required to read the descriptor image"
  local profile target
  profile=$(jq -r '.profile // empty' "$descriptor") || die "cannot read the descriptor profile"
  target=$(jq -r '(.image.reference // "") + "@" + (.image.digest // "")' "$descriptor") \
    || die "cannot read the descriptor image"
  case "$target" in
    *@sha256:*) ;;
    *) die "the descriptor image is not reference@sha256:<digest>" ;;
  esac
  if [ "$allow_unverified" -eq 1 ]; then
    if [ "$profile" = "lab-single-host" ]; then
      printf 'swarm: Unverified: image signature not verified for lab-single-host (--allow-unverified)\n' >&2
      return 0
    fi
    die "--allow-unverified is only valid for lab-single-host; $profile must verify"
  fi
  "$root/tools/release.sh" verify "$target" \
    || die "image signature verification failed; fix deploy/v3/cosign.pub or sign the digest"
}

allow_unverified=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --allow-unverified) allow_unverified=1; shift ;;
    *) break ;;
  esac
done

command=${1:-}
shift || true
if [ "$allow_unverified" -eq 1 ] && [ "$command" != "bootstrap" ] && [ "$command" != "upgrade" ]; then
  die "--allow-unverified is only valid for bootstrap or upgrade"
fi
case "$command" in
  '' | -h | --help | help)
    usage
    ;;
  validate)
    qr_deploy validate "$descriptor" --files
    ;;
  render)
    render
    ;;
  bootstrap)
    verify_release
    render
    (cd "$out" && sh bootstrap.sh)
    ;;
  status)
    require_docker status
    name=$(stack_name)
    printf 'Stack: %s\n' "$name"
    docker stack services "$name"
    printf '\nNodes labelled for %s:\n' "$name"
    docker node ls --filter "label=quantrule.deploy=$name" \
      --format 'table {{.ID}}\t{{.Hostname}}\t{{.Status}}\t{{.Availability}}\t{{.ManagerStatus}}'
    ;;
  upgrade)
    require_docker upgrade
    qr_deploy validate "$descriptor" --files
    verify_release
    snapshot_previous
    render
    (cd "$out" && sh bootstrap.sh)
    ;;
  rollback)
    require_docker rollback
    [ -f "$state/latest" ] || die "no snapshot under $state; run tools/swarm.sh upgrade first"
    directory=$(cat "$state/latest")
    [ -f "$directory/stack.yaml" ] || die "the last snapshot has no stack.yaml: $directory"
    name=$(stack_name)
    if [ -f "$directory/stack-name" ]; then
      name=$(cat "$directory/stack-name")
    fi
    docker stack deploy --with-registry-auth -c "$directory/stack.yaml" "$name"
    printf 'swarm: redeployed %s; migrations stay forward-only and no data was removed\n' \
      "$directory/stack.yaml"
    ;;
  verify)
    qr_deploy check "$descriptor" "$@"
    ;;
  *)
    echo "swarm: unknown command $command; run tools/swarm.sh --help" >&2
    exit 2
    ;;
esac
