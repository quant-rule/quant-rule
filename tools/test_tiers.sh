#!/usr/bin/env bash
# Run one of the three test tiers by name.
#
# Fast is the workspace gate. Composed needs the real Compose stack and stays
# opt-in. Parity validates the port registry. The tier names are the contract
# from AGENTS.md section 4 and ADR-13.
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
    if [ -f compose.yaml ] || [ -f compose.yml ] || [ -f docker-compose.yaml ] || [ -f docker-compose.yml ]; then
      echo "composed: the Compose stack exists but the composed suite is not wired yet; see #643/#644" >&2
    else
      echo "composed: no Compose stack in this tree; the tier is defined by ADR-13 and blocked by #643/#644" >&2
    fi
    exit 2
    ;;
  *)
    echo "usage: tools/test_tiers.sh [fast|composed|parity]" >&2
    exit 2
    ;;
esac
