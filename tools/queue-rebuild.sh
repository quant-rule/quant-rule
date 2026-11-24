#!/usr/bin/env bash
# Rebuild an emptied RabbitMQ control plane from its topology and the evidence
# rows, without publishing work directly.
#
# The queue is not archived. A message is derived from a run or stage row, so a
# lost queue is rebuilt in three steps: restore the topology into an empty
# broker/cluster, recreate the application user/vhost/permissions, and let the
# evidence run and stage outboxes re-arm the exact unfinished set. This tool
# orchestrates those steps; it never purges or deletes a queue, user, vhost,
# message, service, volume, secret, or config, and never calls `docker stack rm`,
# `docker volume rm`, `docker secret rm`, `docker config rm`, `docker node rm`,
# `docker service rm`, or `docker swarm leave`.
#
# Usage: tools/queue-rebuild.sh [--plan | --confirm] [options]
#
#   --plan              Print the sequence; no Docker, SSH, jq, or secret is read.
#   --confirm           Run against the configured descriptor and stack.
#   --definitions FILE  Topology file to import (default: newest stored export
#                       on the first broker).
#   --descriptor FILE   v3 descriptor (default deploy/v3/deploy.example.json).
#   --stack NAME        Swarm stack (default: render.json stack, else quantrule).
#   --evidence SERVICE  Evidence service name (default <stack>_evidence).
#   --manager HOST      SSH host of a Swarm manager; default: local docker.
#   --wait SECONDS      Wait this long for the re-arm logs (default 15).
#   -h | --help
#
# Environment:
#   DESCRIPTOR / OUT / STACK         shared with the other v3 tools
#   QR_QUEUE_REBUILD_DOCKER          docker executable (default docker)
#   QR_QUEUE_REBUILD_SSH             ssh command and options
#   QR_QUEUE_REBUILD_SUDO            remote privilege command (default "sudo -n")
#   QR_QUEUE_REBUILD_DEFINITIONS_DIR definitions directory (default /srv/quantrule/rabbitmq/definitions)
#   QR_QUEUE_DEFINITIONS_SSH/SUDO/DOCKER/DIR  shared with queue-definitions.sh
#
# A live run requires `--confirm`; without it the plan is printed and no host is
# contacted. A stack that is not provisioned prints `Unverified` and exits
# non-zero. The queue-rebuild result is `Simulated HA` evidence at most; `Real
# HA` remains HA-014.
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "$script_dir/.." && pwd)
cd "$root"

descriptor=${DESCRIPTOR:-deploy/v3/deploy.example.json}
out=${OUT:-deploy/v3/out}
stack_opt=${STACK:-}
definitions=${QR_QUEUE_REBUILD_DEFINITIONS:-}
evidence_opt=${QR_QUEUE_REBUILD_EVIDENCE_SERVICE:-}
manager=${QR_QUEUE_REBUILD_MANAGER:-}
wait_seconds=${QR_QUEUE_REBUILD_WAIT_SECONDS:-15}
docker_bin=${QR_QUEUE_REBUILD_DOCKER:-${QR_QUEUE_DEFINITIONS_DOCKER:-docker}}
ssh_bin=${QR_QUEUE_REBUILD_SSH:-${QR_QUEUE_DEFINITIONS_SSH:-ssh -o BatchMode=yes -o ConnectTimeout=5}}
sudo_bin=${QR_QUEUE_REBUILD_SUDO-${QR_QUEUE_DEFINITIONS_SUDO-sudo -n}}
definitions_dir=${QR_QUEUE_DEFINITIONS_DIR:-/srv/quantrule/rabbitmq/definitions}
plan=0
confirm=0

die() {
  printf 'queue-rebuild: %s\n' "$*" >&2
  exit 1
}

usage() {
  awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
}

usage_error() {
  printf 'queue-rebuild: %s\n\n' "$*" >&2
  usage >&2
  exit 2
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
    --definitions)
      [ "$#" -ge 2 ] || usage_error "--definitions needs a value"
      definitions=$2
      shift 2
      ;;
    --descriptor)
      [ "$#" -ge 2 ] || usage_error "--descriptor needs a value"
      descriptor=$2
      shift 2
      ;;
    --stack)
      [ "$#" -ge 2 ] || usage_error "--stack needs a value"
      stack_opt=$2
      shift 2
      ;;
    --evidence)
      [ "$#" -ge 2 ] || usage_error "--evidence needs a value"
      evidence_opt=$2
      shift 2
      ;;
    --manager)
      [ "$#" -ge 2 ] || usage_error "--manager needs a value"
      manager=$2
      shift 2
      ;;
    --wait)
      [ "$#" -ge 2 ] || usage_error "--wait needs a value"
      wait_seconds=$2
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

if [ "$plan" -eq 1 ]; then
  cat <<PLAN
queue-rebuild: plan (no Docker, SSH, jq, or secret is read)
queue-rebuild: descriptor: $descriptor
queue-rebuild: 1 import: tools/queue-definitions.sh import <definitions> -- no --force; the broker/cluster must be empty
queue-rebuild: 2 user: tools/stateful.sh queue-bootstrap -- recreate the app user, vhost, and permissions from queue_url
queue-rebuild: 3 rearm: restart only the evidence service; its run and stage outboxes reconcile and drain the exact unfinished set
queue-rebuild: 4 report: print re-armed counts, the queue depth, and the DLQ inspection command; no work is published directly
queue-rebuild: never: purge/delete a queue, user, vhost, message; docker stack/volume/secret/config/node/service rm; swarm leave
queue-rebuild: the queue is not archived; the run row and the stage row are authoritative
queue-rebuild: live rebuild requires --confirm; a missing stack prints Unverified and exits non-zero
queue-rebuild: empty-broker drill (lab, not run by this tool): publish a confirmed probe, recreate the broker with an empty data directory,
queue-rebuild:   import the stored definitions, run queue-bootstrap, restart evidence and backtest, then assert the acknowledged row is
queue-rebuild:   still readable and no second execution/artifact exists; inspect DLQ messages, never replay them
queue-rebuild: Simulated HA is the maximum; Real HA is HA-014
PLAN
  exit 0
fi

if [ "$confirm" -ne 1 ]; then
  printf 'queue-rebuild: live rebuild requires --confirm; no host was contacted\n' >&2
  exit 2
fi

for tool in "$docker_bin" jq; do
  command -v "$tool" >/dev/null 2>&1 || die "$tool is required for a live rebuild"
done
[ -f "$descriptor" ] || die "descriptor $descriptor is missing"
[ -f "$descriptor" ] && jq -e . "$descriptor" >/dev/null || die "descriptor $descriptor is not JSON"

stack=${stack_opt:-}
if [ -z "$stack" ] && [ -f "$out/render.json" ]; then
  stack=$(sed -n 's/.*"stack":[[:space:]]*"\([^"]*\)".*/\1/p' "$out/render.json" | head -n 1)
fi
stack=${stack:-quantrule}
evidence_service=${evidence_opt:-${stack}_evidence}

run_manager() {
  if [ -n "$manager" ]; then
    # shellcheck disable=SC2086
    $ssh_bin "$manager" "$sudo_bin $*"
  else
    "$@"
  fi
}

provisioned() {
  if [ -n "$manager" ]; then
    # shellcheck disable=SC2086
    $ssh_bin "$manager" "$sudo_bin $docker_bin service inspect $evidence_service" >/dev/null 2>&1
  else
    "$docker_bin" service inspect "$evidence_service" >/dev/null 2>&1
  fi
}

if ! provisioned; then
  printf 'queue-rebuild: Unverified: service %s is not provisioned on stack %s; no queue, user, or service was touched\n' \
    "$evidence_service" "$stack"
  exit 3
fi

tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/quantrule-queue-rebuild.XXXXXX")
cleanup() {
  rm -f "$tmp_dir"/* 2>/dev/null || true
  rmdir "$tmp_dir" 2>/dev/null || true
}
trap cleanup EXIT

# Step 1: the topology file. A stored export is already stripped of users and
# permissions and verified by queue-definitions.sh; a local file is accepted.
if [ -z "$definitions" ]; then
  case "$definitions_dir" in
    /srv/*) ;;
    *) die "the definitions directory must be an absolute /srv path" ;;
  esac
  case "$definitions_dir" in
    *[!A-Za-z0-9/._-]*) die "the definitions directory contains an unsafe character" ;;
  esac
  broker=$(jq -r '.stateful.rabbitmq.nodes[0] // empty' "$descriptor")
  [ -n "$broker" ] || die "stateful.rabbitmq.nodes is empty"
  case "$broker" in
    *[!A-Za-z0-9._-]*) die "the first broker name contains an unsafe character" ;;
  esac
  container=quantrule-rabbitmq-$broker
  # shellcheck disable=SC2086
  listing=$($ssh_bin "$broker" "$sudo_bin $docker_bin exec -u rabbitmq $container sh -c 'ls -1 $definitions_dir/*.json 2>/dev/null || true'") \
    || die "cannot list stored definitions on $broker"
  latest=$(printf '%s\n' "$listing" | grep -v '\.manifest\.json$' | sort | tail -n 1)
  [ -n "$latest" ] || die "no stored definitions file on $broker; pass --definitions FILE"
  name=${latest##*/}
  [[ "$name" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{6}Z\.json$ ]] \
    || die "the newest stored definitions name is not a timestamped export"
  case "$latest" in
    "$definitions_dir"/*.json) ;;
    *) die "the definitions path is not the configured directory" ;;
  esac
  # shellcheck disable=SC2086
  $ssh_bin "$broker" "$sudo_bin $docker_bin exec -u rabbitmq $container cat '$latest'" > "$tmp_dir/definitions.json" \
    || die "cannot read $latest on $broker"
  chmod 600 "$tmp_dir/definitions.json"
  definitions=$tmp_dir/definitions.json
  printf 'queue-rebuild: using stored definitions %s\n' "$latest"
fi
[ -f "$definitions" ] || die "definitions file $definitions is missing"

export QR_QUEUE_DEFINITIONS_SSH="${QR_QUEUE_DEFINITIONS_SSH:-$ssh_bin}"
export QR_QUEUE_DEFINITIONS_SUDO="${QR_QUEUE_DEFINITIONS_SUDO:-$sudo_bin}"
export QR_QUEUE_DEFINITIONS_DOCKER="${QR_QUEUE_DEFINITIONS_DOCKER:-$docker_bin}"
export QR_QUEUE_DEFINITIONS_DIR="$definitions_dir"
export QR_QUEUE_DEFINITIONS_GENERATION="${QR_QUEUE_DEFINITIONS_GENERATION:-${QR_QUEUE_GENERATION:-1}}"
printf 'queue-rebuild: 1/4 importing the topology into the empty broker/cluster; no purge or delete\n'
DESCRIPTOR="$descriptor" ./tools/queue-definitions.sh import "$definitions"
printf 'queue-rebuild: 1/4 topology imported and verified live\n'

printf 'queue-rebuild: 2/4 recreating the application user, vhost, and permissions\n'
DESCRIPTOR="$descriptor" ./tools/stateful.sh queue-bootstrap || die "queue bootstrap failed"
printf 'queue-rebuild: 2/4 queue user ready\n'

printf 'queue-rebuild: 3/4 restarting %s so its outboxes re-arm; no work is published directly\n' "$evidence_service"
started_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
run_manager "$docker_bin" service update --force --detach=true "$evidence_service" >/dev/null \
  || die "cannot restart $evidence_service"

printf 'queue-rebuild: 4/4 waiting up to %s s for the re-arm logs\n' "$wait_seconds"
logs=""
attempt=0
while [ "$attempt" -lt "$wait_seconds" ]; do
  attempt=$((attempt + 1))
  if [ -n "$manager" ]; then
    # shellcheck disable=SC2086
    logs=$($ssh_bin "$manager" "$sudo_bin $docker_bin service logs --since $started_at --tail 2000 $evidence_service 2>&1" || true)
  else
    logs=$("$docker_bin" service logs --since "$started_at" --tail 2000 "$evidence_service" 2>&1 || true)
  fi
  if printf '%s\n' "$logs" | grep -Eq 'notifications reconciled|rearmed'; then
    break
  fi
  sleep 1
done

rearmed=$(printf '%s\n' "$logs" | sed -n 's/.*rearmed["]*[=:][[:space:]]*\([0-9][0-9]*\).*/\1/p' | awk '{ sum += $1 } END { print sum + 0 }')
run_lines=$(printf '%s\n' "$logs" | grep -c 'run notifications reconciled' || true)
stage_lines=$(printf '%s\n' "$logs" | grep -c 'stage notifications reconciled' || true)
if [ "$run_lines" -eq 0 ] && [ "$stage_lines" -eq 0 ]; then
  printf 'queue-rebuild: Unverified: no re-arm log line observed in %s s; inspect the service log and the pending gauge\n' "$wait_seconds"
  printf 'queue-rebuild: inspect: %s service logs --since %s %s\n' "$docker_bin" "$started_at" "$evidence_service"
  exit 4
fi
printf 'queue-rebuild: run reconcile lines %s, stage reconcile lines %s, rearmed notifications %s\n' \
  "$run_lines" "$stage_lines" "$rearmed"
printf 'queue-rebuild: pending gauges: query /metrics for quantrule_outbox_pending and quantrule_stage_outbox_pending\n'
generation=${QR_QUEUE_GENERATION:-1}
dlq=runs.dead-letter.g$generation
printf 'queue-rebuild: queue depth: rabbitmqctl list_queues -p / name messages consumers\n'
printf 'queue-rebuild: DLQ %s inspection (never replay): docker exec -u rabbitmq <broker> rabbitmqctl list_queues -p / name messages | grep %s\n' \
  "$dlq" "$dlq"
printf 'queue-rebuild: evidence is Simulated HA at most; Real HA is HA-014\n'
