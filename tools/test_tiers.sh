#!/usr/bin/env bash
# Run one of the four test tiers by name.
#
# Fast is the workspace gate. Composed runs against a stack the operator
# started with `make up`; it checks the gateway and the distributed run path, so
# it cannot silently pass against a stack this script did not expect. Parity
# validates the port registry. Multi-host runs the local multi-host harness.
# The tier names are the contract from AGENTS.md section 4 and ADR-13.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

tier="${1:-fast}"

case "$tier" in
  fast)
    exec ./tools/gate.sh
    ;;
  parity)
    exec python3 tools/check_port_parity.py
    ;;
  composed)
    exec ./tools/composed.sh
    ;;
  multi-host)
    exec ./deploy/multi-host/test.sh
    ;;
  *)
    echo "usage: tools/test_tiers.sh [fast|composed|multi-host|parity]" >&2
    exit 2
    ;;
esac
