#!/usr/bin/env bash
# Build the derived Patroni image that adds pgBackRest for HA-013.
#
# The helper never pulls a registry image and never pushes one. It requires the
# pinned base to be present locally, builds from Dockerfile in this directory,
# and prints the local image id plus any repository digests Docker reports. A
# pushed image's repository digest is the descriptor pin; the local image id is
# not a registry manifest digest.
#
# Usage:
#   deploy/v3/images/patroni/build.sh [--plan]
#
# Environment:
#   QR_PATRONI_TAG       local tag (default ghcr.io/quant-rule/patroni-pgbackrest:ha-013)
#   QR_PATRONI_REVISION  optional revision label
set -euo pipefail

base='batonogov/patroni-docker@sha256:0de7dc8bc5919c76721bee1475ed30155b1848c0a6bac7298605bf39eef1a7eb'
reference='ghcr.io/quant-rule/patroni-pgbackrest'
tag=${QR_PATRONI_TAG:-$reference:ha-013}
directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)

usage() {
  sed -n '2,15p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

case "${1:-}" in
  --plan)
    cat <<PLAN
patroni image: plan (no Docker, network, registry, or secret is read)
patroni image: base $base
patroni image: build docker build --pull=false --label org.opencontainers.image.revision=<revision> -t $tag $directory
patroni image: digest docker image inspect --format '{{.Id}}' $tag
patroni image: repository digest docker image inspect --format '{{index .RepoDigests 0}}' $tag after a deliberate push
patroni image: pin descriptor images.patroni.reference=$reference and images.patroni.digest=<repository digest>
patroni image: no push and no pull are performed by this helper
PLAN
    exit 0
    ;;
  -h | --help)
    usage
    exit 0
    ;;
  '') ;;
  *)
    printf 'patroni image: unknown argument %s\n' "$1" >&2
    exit 2
    ;;
esac

command -v docker >/dev/null 2>&1 || {
  printf 'patroni image: docker is required\n' >&2
  exit 1
}

if ! docker image inspect "$base" >/dev/null 2>&1; then
  printf 'patroni image: the pinned base %s is not present locally\n' "$base" >&2
  printf 'patroni image: load or pull that exact digest deliberately; this helper does not pull\n' >&2
  exit 1
fi

revision=${QR_PATRONI_REVISION:-unknown}
docker build \
  --pull=false \
  --label "org.opencontainers.image.revision=$revision" \
  --tag "$tag" \
  "$directory"

image_id=$(docker image inspect --format '{{.Id}}' "$tag")
printf 'patroni image: tag %s\n' "$tag"
printf 'patroni image: local image id %s\n' "$image_id"
repo_digests=$(docker image inspect --format '{{join .RepoDigests "\n"}}' "$tag")
if [ -n "$repo_digests" ]; then
  printf 'patroni image: repository digest(s), pin one in images.patroni.digest after a deliberate push:\n'
  printf '%s\n' "$repo_digests"
else
  printf 'patroni image: no repository digest yet; push deliberately and inspect the repository digest\n'
fi
