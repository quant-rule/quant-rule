#!/usr/bin/env bash
# Reference lab harness for the v3 topologies and the remote client path.
#
# The harness selects one descriptor, preflights release mode, and only then
# provisions. The app, `minio`, `minio_client`, and derived Patroni pgBackRest
# fixtures are blockers: a release validation failure prints the exact field and
# `Unverified`, and the script exits before Docker, SSH, jq, or a secret file is
# read. There is no bypass.
#
# Usage:
#   deploy/v3/lab.sh --plan
#   deploy/v3/lab.sh --reference lab --confirm
#   deploy/v3/lab.sh --reference lab --confirm --skip-database
#   deploy/v3/lab.sh --remote-client --confirm        # on the client
#
# Options:
#   --reference three-az|single-az|lab  descriptor selector (default lab)
#   --out <dir>                         render directory (default deploy/v3/out)
#   --plan | --dry-run                  print the sequence; no host is contacted
#   --confirm                           run the lab-only failure phase
#   --remote-client                     run only the client TLS/token phase
#   --skip-database|--skip-queue|--skip-queue-rebuild|--skip-object|--skip-observability
#                                       forward to the failure harness; row Unverified
#   --skip-failure                      provision only; failure rows Unverified
#   -h | --help
#
# Environment:
#   DESCRIPTOR            descriptor override when --reference is lab
#   OUT                   render directory
#   QR_DEPLOY_BIN         qr-deploy binary; otherwise qr-deploy, then cargo
#   QR_LAB_DOCKER         docker executable (default docker)
#   QR_LAB_EVIDENCE       evidence base (default deploy/v3/evidence/lab-<UTC>)
#   QR_LAB_PROVISIONED    set 1 on the client after the lab was provisioned
#   QR_LAB_FAILURE_EVIDENCE  failure evidence JSON used to label the remote file
#   QR_REMOTE_SHELL       quantrule binary for the remote phase
#   QR_REMOTE_PROFILE     profile for the remote phase (default default)
#   QUANTRULE_API_URL     https://quantrule.dev/api/v1
#   QUANTRULE_CA_CERT     readable private CA bundle on the client
#   QUANTRULE_TOKEN       gateway token (or QUANTRULE_TOKEN_FILE)
#
# Only `deploy/v3/deploy.lab.json` has lab hosts. The two reference descriptors
# are designs; `--reference three-az|single-az` validates them and stops, it
# never provisions them. `deploy/v3/test.sh` remains the failure harness.
# `Simulated HA` is the maximum; `Real HA` is HA-014. No token, CA body, key, or
# inspect body is written to evidence or printed.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd -- "$script_dir/../.." && pwd)"
cd "$root"

reference=lab
descriptor=""
out=${OUT:-deploy/v3/out}
docker_bin=${QR_LAB_DOCKER:-docker}
plan=0
confirm=0
remote_client=0
skip_database=0
skip_queue=0
skip_queue_rebuild=0
skip_object=0
skip_observability=0
dr=0
skip_dr=0
skip_failure=0

provision_stateful="Unverified"
provision_swarm="Unverified"
provision_observability="Unverified"
provision_check="Unverified"
failure_database="Unverified"
failure_queue="Unverified"
failure_object="Unverified"
failure_observability="Unverified"
failure_dr="Unverified"
overall_evidence="Unverified"

usage() {
  awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
}

say() {
  printf 'lab: %s\n' "$*"
}

unverified() {
  printf 'lab: UNVERIFIED: %s\n' "$*" >&2
  printf 'lab: evidence label: Unverified\n' >&2
  exit 1
}

while [ $# -gt 0 ]; do
  case "$1" in
    --reference)
      [ $# -ge 2 ] || { printf 'lab: --reference needs a value\n' >&2; exit 2; }
      reference=$2
      shift 2
      ;;
    --out)
      [ $# -ge 2 ] || { printf 'lab: --out needs a value\n' >&2; exit 2; }
      out=$2
      shift 2
      ;;
    --plan | --dry-run)
      plan=1
      shift
      ;;
    --confirm)
      confirm=1
      shift
      ;;
    --remote-client)
      remote_client=1
      shift
      ;;
    --skip-database)
      skip_database=1
      shift
      ;;
    --skip-queue)
      skip_queue=1
      shift
      ;;
    --skip-queue-rebuild)
      skip_queue_rebuild=1
      shift
      ;;
    --skip-object)
      skip_object=1
      shift
      ;;
    --skip-observability)
      skip_observability=1
      shift
      ;;
    --dr)
      dr=1
      shift
      ;;
    --skip-dr)
      skip_dr=1
      shift
      ;;
    --skip-failure)
      skip_failure=1
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      printf 'lab: unknown argument %s; run deploy/v3/lab.sh --help\n' "$1" >&2
      exit 2
      ;;
  esac
done

case "$reference" in
  three-az) descriptor=deploy/v3/reference/three-az.json ;;
  single-az) descriptor=deploy/v3/reference/single-az.json ;;
  lab) descriptor=${DESCRIPTOR:-deploy/v3/deploy.lab.json} ;;
  *)
    printf 'lab: --reference must be three-az, single-az, or lab, not %s\n' "$reference" >&2
    exit 2
    ;;
esac

if [ "$plan" -eq 1 ]; then
  cat <<PLAN
lab: plan (no Docker, SSH, jq, qr-deploy, or secret file is read)
lab: reference: $reference
lab: references: three-az and single-az are designs; lab is the three-VM profile
lab: descriptor: $descriptor
lab: preflight: qr-deploy validate $descriptor --release
lab: 1 stateful: tools/stateful.sh network, copy, start, wait, queue-bootstrap, object-bootstrap
lab: 2 swarm: tools/swarm.sh --allow-unverified bootstrap (lab only; references need a signed image)
lab: 3 observability: tools/observability.sh copy, start, wait, verify
lab: 4 check: qr-deploy check $descriptor --remote docker --overlay --runtime (7/7)
lab: 5 failure (lab only, --confirm): deploy/v3/test.sh --confirm --skip-deploy
lab: 5a queue rebuild: plan only at tools/queue-rebuild.sh --plan; the empty-broker drill is operator-run and Unverified
lab: 5b --skip-queue-rebuild forwards to the failure harness and leaves that row unrecorded
lab: 5c dr: deploy/v3/dr.sh --plan is Docker-free; --dr runs the in-place lab restore after the failure phase; --skip-dr leaves the DR row Unverified
lab: 6 remote client (--remote-client --confirm, from the client outside the Swarm):
lab:     export QUANTRULE_API_URL='https://quantrule.dev/api/v1'
lab:     export QUANTRULE_CA_CERT='<client ca.crt>'
lab:     export QUANTRULE_TOKEN='<gateway token>'
lab:     quantrule health
lab:     tools/verify.sh --api-url "\$QUANTRULE_API_URL" --cacert "\$QUANTRULE_CA_CERT" --profile <id>
lab:     openssl s_client -connect quantrule.dev:443 -servername quantrule.dev -CAfile "\$QUANTRULE_CA_CERT" -verify_return_error
lab:     curl --cacert "\$QUANTRULE_CA_CERT" https://quantrule.dev/health
lab:     curl --cacert /dev/null https://quantrule.dev/health   # must fail
lab: 7 browser (separate, lab-gated): cd apps/web && QR_REMOTE_BROWSER=1 QR_REMOTE_URL=https://quantrule.dev QR_REMOTE_TOKEN='<token>' bun run test:remote
lab: preconditions: three VMs 10.10.0.11..13, VIP 10.10.0.20, /etc/hosts or DNS quantrule.dev,
lab:   private CA and real ingress cert/key, 443/VRRP 112 plus the direct-edge 443 firewall control,
lab:   three managers, SSH/sudo, a resolved app image, and no fixture secret on a host
lab: evidence: value-free JSON under ${QR_LAB_EVIDENCE:-deploy/v3/evidence/lab-<UTC>}{-provision,-failure,-remote}.json
lab: boundary: Simulated HA at most; Real HA is HA-014; app/minio/minio_client/patroni stay fixture blockers; pgBackRest restore remains Unverified
lab: this plan is not evidence
PLAN
  if [ "$reference" = "lab" ]; then
    printf 'lab: failure phases (deploy/v3/test.sh --plan):\n'
    DESCRIPTOR="$descriptor" OUT="$out" ./deploy/v3/test.sh --plan
  else
    printf 'lab: failure phases: not applicable; only --reference lab has lab hosts\n'
  fi
  exit 0
fi

qr_deploy() {
  if [ -n "${QR_DEPLOY_BIN:-}" ]; then
    "$QR_DEPLOY_BIN" "$@"
  elif command -v qr-deploy >/dev/null 2>&1; then
    qr-deploy "$@"
  else
    cargo run --quiet -p quantrule-deploy -- "$@"
  fi
}

# shellcheck source=deploy/v3/lab/evidence.sh
. "$script_dir/lab/evidence.sh"
# shellcheck source=deploy/v3/lab/remote.sh
. "$script_dir/lab/remote.sh"

release_preflight() {
  local output field
  [ -f "$descriptor" ] || unverified "descriptor $descriptor is missing"
  if ! output=$(qr_deploy validate "$descriptor" --release 2>&1); then
    field=$(printf '%s\n' "$output" \
      | sed -n 's/.*deploy\.image\.fixture at \([^:]*\):.*/\1/p' \
      | head -n 1)
    if [ -n "$field" ]; then
      printf 'lab: blocker: %s\n' "$field" >&2
      unverified "deploy.image.fixture at $field (app/minio/minio_client/patroni are fixture digests); no host is contacted"
    fi
    printf '%s\n' "$output" >&2
    unverified "qr-deploy validate --release refused $descriptor; no host is contacted"
  fi
}

require_provision_tools() {
  command -v jq >/dev/null 2>&1 || unverified "jq is required for provisioning"
  command -v ssh >/dev/null 2>&1 || unverified "ssh is required for provisioning"
  command -v "$docker_bin" >/dev/null 2>&1 \
    || unverified "docker executable $docker_bin is not available"
  "$docker_bin" info >/dev/null 2>&1 || unverified "cannot reach the Docker daemon"
  if [ "$skip_failure" -eq 0 ]; then
    command -v curl >/dev/null 2>&1 || unverified "curl is required for the failure phase"
    command -v nc >/dev/null 2>&1 || unverified "nc is required for the failure phase"
  fi
}

provision() {
  local bootstrap=(--allow-unverified bootstrap)
  if [ "$reference" != "lab" ]; then
    bootstrap=(bootstrap)
  fi
  say "stateful: network"
  DESCRIPTOR="$descriptor" OUT="$out" ./tools/stateful.sh network
  provision_stateful="ok"
  say "stateful: copy"
  DESCRIPTOR="$descriptor" OUT="$out" ./tools/stateful.sh copy
  say "stateful: start"
  DESCRIPTOR="$descriptor" OUT="$out" ./tools/stateful.sh start
  say "stateful: wait"
  DESCRIPTOR="$descriptor" OUT="$out" ./tools/stateful.sh wait
  say "stateful: queue-bootstrap"
  DESCRIPTOR="$descriptor" OUT="$out" ./tools/stateful.sh queue-bootstrap
  say "stateful: object-bootstrap"
  DESCRIPTOR="$descriptor" OUT="$out" ./tools/stateful.sh object-bootstrap
  say "swarm: tools/swarm.sh ${bootstrap[*]}"
  DESCRIPTOR="$descriptor" OUT="$out" ./tools/swarm.sh "${bootstrap[@]}"
  provision_swarm="ok"
  if [ "$skip_observability" -eq 1 ]; then
    say "observability: skipped; row Unverified"
  else
    say "observability: copy"
    DESCRIPTOR="$descriptor" OUT="$out" ./tools/observability.sh copy
    say "observability: start"
    DESCRIPTOR="$descriptor" OUT="$out" ./tools/observability.sh start
    say "observability: wait"
    DESCRIPTOR="$descriptor" OUT="$out" ./tools/observability.sh wait
    say "observability: verify"
    DESCRIPTOR="$descriptor" OUT="$out" ./tools/observability.sh verify
    provision_observability="ok"
  fi
  say "check: qr-deploy check --remote docker --overlay --runtime"
  qr_deploy check "$descriptor" --remote docker --overlay --runtime \
    || unverified "the remote/overlay/runtime check failed; no 7/7 digest row"
  provision_check="ok"
}

failure_phase() {
  local args=(--confirm --skip-deploy)
  [ "$skip_database" -eq 1 ] && args+=(--skip-database)
  [ "$skip_queue" -eq 1 ] && args+=(--skip-queue)
  [ "$skip_queue_rebuild" -eq 1 ] && args+=(--skip-queue-rebuild)
  [ "$skip_object" -eq 1 ] && args+=(--skip-object)
  [ "$skip_observability" -eq 1 ] && args+=(--skip-observability)
  [ "$dr" -eq 1 ] && args+=(--dr)
  [ "$skip_dr" -eq 1 ] && args+=(--skip-dr)
  if [ "$skip_failure" -eq 1 ]; then
    say "failure: skipped (--skip-failure); rows Unverified"
    return 0
  fi
  if [ "$confirm" -eq 0 ]; then
    say "failure phases not run: pass --confirm"
    say "exact command: DESCRIPTOR=$descriptor OUT=$out QR_SWARM_EVIDENCE=$failure_evidence ./deploy/v3/test.sh ${args[*]}"
    return 2
  fi
  say "failure: deploy/v3/test.sh ${args[*]}"
  if ! DESCRIPTOR="$descriptor" OUT="$out" QR_SWARM_EVIDENCE="$failure_evidence" \
    ./deploy/v3/test.sh "${args[@]}"; then
    unverified "the failure harness failed; failure rows Unverified"
  fi
  [ "$skip_database" -eq 1 ] || failure_database="Simulated HA"
  [ "$skip_queue" -eq 1 ] || failure_queue="Simulated HA"
  [ "$skip_object" -eq 1 ] || failure_object="Simulated HA"
  [ "$skip_observability" -eq 1 ] || failure_observability="Simulated HA"
  if [ "$skip_dr" -eq 1 ] || [ "$dr" -eq 0 ]; then
    failure_dr="Unverified"
  else
    failure_dr="Simulated HA"
  fi
  if [ "$skip_database" -eq 0 ] && [ "$skip_queue" -eq 0 ] \
    && [ "$skip_object" -eq 0 ] && [ "$skip_observability" -eq 0 ]; then
    overall_evidence="Simulated HA"
  fi
}

evidence_base=${QR_LAB_EVIDENCE:-${QR_SWARM_EVIDENCE:-}}
if [ -z "$evidence_base" ]; then
  evidence_base="deploy/v3/evidence/lab-$(date -u +%Y-%m-%dT%H%M%SZ)"
fi
case "$evidence_base" in
  *.json) evidence_base=${evidence_base%.json} ;;
esac
provision_evidence="$evidence_base-provision.json"
failure_evidence="$evidence_base-failure.json"
remote_evidence="$evidence_base-remote.json"

if [ "$remote_client" -eq 1 ]; then
  remote_phase
  exit 0
fi

release_preflight

if [ "$reference" != "lab" ]; then
  unverified "only deploy/v3/deploy.lab.json has lab hosts; $descriptor is a design and was not provisioned"
fi
command -v jq >/dev/null 2>&1 || unverified "jq is required to read the descriptor profile"
profile=$(descriptor_field '.profile')
[ "$profile" = "lab-single-host" ] \
  || unverified "only the lab-single-host profile has lab hosts; $descriptor is ${profile:-unknown}"

require_provision_tools
provision
write_evidence "$provision_evidence" "Unverified"

failure_status=0
failure_phase || failure_status=$?
write_evidence "$provision_evidence" "$overall_evidence"
if [ "$failure_status" -eq 2 ]; then
  exit 2
fi
if [ "$failure_status" -ne 0 ]; then
  unverified "the failure phase did not complete"
fi
say "summary: provision ok; failure database $failure_database, queue $failure_queue, object $failure_object, observability $failure_observability, dr $failure_dr"
say "remote client: run deploy/v3/lab.sh --remote-client --confirm on the client; browser is the separate apps/web suite"
say "boundary: $overall_evidence at most; Real HA is HA-014"
