#!/usr/bin/env bash
# Run one of the five test tiers by name.
#
# Fast is the workspace gate. Composed runs against a stack the operator
# started with `make up`; it checks the gateway and the distributed run path, so
# it cannot silently pass against a stack this script did not expect. Parity
# validates the port registry. Multi-host runs the local multi-host harness.
# HA runs the opt-in Swarm lab harness in `deploy/v3/test.sh`; it needs a real
# Swarm, defaults to the three-VM `deploy/v3/deploy.lab.json`, and never runs in
# the fast gate. The harness drains a worker, stops a manager, moves the VIP,
# pauses the Patroni primary, stops the RabbitMQ quorum leader, and stops one
# MinIO node. The tier names are the contract from AGENTS.md section 4 and ADR-13.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

tier="${1:-fast}"

case "$tier" in
  fast)
    exec ./tools/gate.sh
    ;;
  parity)
    exec cargo run --quiet -p quantrule-governance -- check --checker port-parity
    ;;
  composed)
    exec ./tools/composed.sh
    ;;
  multi-host)
    exec ./deploy/multi-host/test.sh
    ;;
  ha)
    shift
    export DESCRIPTOR="${DESCRIPTOR:-deploy/v3/deploy.lab.json}"
    exec ./deploy/v3/test.sh "$@"
    ;;
  *)
    echo "usage: tools/test_tiers.sh [fast|composed|multi-host|parity|ha]" >&2
    exit 2
    ;;
esac
