#!/usr/bin/env bash
# Chaos cases for the v3 `ha` lab tier.
#
# Sourced by `deploy/v3/test.sh` (and for the plan by `deploy/v3/lab.sh`).
# `--plan` is Docker/SSH/jq/secret-free. `--chaos` requires `--confirm`; the
# default is `--skip-chaos`, which leaves every chaos row `Unverified`. The
# cases use only `docker kill --signal=SIGKILL`, `docker stop/start`, and
# `node drain/active`; they never remove a stack, volume, secret, config, node,
# or service. A case is `Simulated HA` only after it passes on the lab. No case
# can claim `Real HA`.
#
# Usage (standalone):
#   deploy/v3/chaos.sh --plan

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  set -euo pipefail
fi

chaos_plan() {
  cat <<'PLAN'
chaos: HA-18 worker SIGKILL: fixed run in flight; no succeeded run without a digest; lease expiry/retry; one succeeded run and one digest; F03/F07
chaos: HA-19 evidence-leader SIGKILL: one higher lease term, one finalisation, one digest; F06/F07
chaos: HA-20 no partial publish: a killed run is not succeeded without a digest and every referenced artifact fetches 200; F07
chaos: HA-21 digest parity: tools/verify.sh --expected equals the clean baseline after both restarts; F07
chaos: HA-22 power event on separate physical hosts: operator-only real-ha gate, never run by this script; Unverified
chaos: HA-23 AZ loss, disk-controller failure, or true network partition: operator-only real-ha gate; the AZ1 MinIO/ingress limitation is an accepted risk, not a verified row
chaos: F rows: F01..F17 are the static alert/check rows HA-01..HA-17; the live cases map to F01, F03, F06, F07, and F11
chaos: extra live mappings: non-VIP gateway SIGKILL -> F01; etcd member stop/start -> F11
chaos: flags: --chaos requires --confirm; --skip-chaos is the default and leaves every chaos row Unverified
chaos: evidence: value-free quantrule.ha-evidence/v1 JSON per row under QR_HA_EVIDENCE or QR_SWARM_EVIDENCE; Simulated HA only after a pass; no Real HA
PLAN
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  case "${1:-}" in
    --plan | --dry-run) chaos_plan ;;
    -h | --help)
      awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "${BASH_SOURCE[0]}"
      ;;
    *)
      printf 'chaos: only --plan is valid without the ha harness\n' >&2
      exit 2
      ;;
  esac
  exit 0
fi

chaos_script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=deploy/v3/chaos/lib.sh
. "$chaos_script_dir/chaos/lib.sh"
# shellcheck source=deploy/v3/chaos/cases.sh
. "$chaos_script_dir/chaos/cases.sh"
