#!/usr/bin/env bash
# Run the composed tier against a real Compose stack.
#
# This tier starts nothing. `make up` or the release workflow owns the stack,
# because a suite that silently starts a second project would test the wrong
# volumes and could pass against stale state. The tier checks the running
# gateway, then submits two equivalent runs and compares their canonical
# artifacts: the distributed contract the fast tier exercises through the
# in-process transport fake.
#
# Usage:
#   make up
#   tools/test_tiers.sh composed
#
# Environment:
#   QUANTRULE_API_URL   Gateway base (default http://127.0.0.1:8080/api/v1)
#   QUANTRULE_TIMEOUT   Seconds per submitted run (default 300)
#   QUANTRULE_VERIFY    Override for the run-parity suite (tests only)
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

api_url="${QUANTRULE_API_URL:-http://127.0.0.1:8080/api/v1}"
timeout="${QUANTRULE_TIMEOUT:-300}"
verify="${QUANTRULE_VERIFY:-$root/tools/verify.sh}"
python_bin="${QR_PYTHON:-python3}"

fail() {
  echo "composed: $*" >&2
  exit 1
}

health="$(curl -fsS --max-time 5 "$api_url/health" 2>/dev/null)" \
  || fail "the gateway is not reachable at $api_url; start the stack with 'make up'"

state="$(printf '%s' "$health" | "$python_bin" -c \
  'import json, sys; print(json.load(sys.stdin).get("state", ""))' 2>/dev/null)" \
  || fail "the /health response is not the expected JSON"
[ "$state" = "healthy" ] || fail "the gateway reports state '$state', not 'healthy'"

version="$(curl -fsS --max-time 5 "$api_url/version" 2>/dev/null)" \
  || fail "the gateway has no /version response"
product="$(printf '%s' "$version" | "$python_bin" -c \
  'import json, sys; document = json.load(sys.stdin); print(document.get("api", ""), document.get("product", ""))' 2>/dev/null)" \
  || fail "the /version response is not the expected JSON"
set -- $product
[ "${1:-}" = "quantrule.api/v1" ] \
  || fail "/version reports API '${1:-}', expected quantrule.api/v1"
[ -n "${2:-}" ] || fail "/version reports no product version"

"$verify" --api-url "$api_url" --timeout "$timeout"
echo "composed: gateway healthy, product $2, run parity verified"
