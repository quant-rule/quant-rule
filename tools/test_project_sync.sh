#!/usr/bin/env bash
#
# Unit tests for the project-sync status decision.
#
# The script is testable because its field loading and status decision are
# functions; sourcing it does not run main. The tests never reach the network.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=../.github/scripts/project-sync.sh
source "$ROOT/.github/scripts/project-sync.sh"

failures=0

assert_status() {
  local name="$1" expected="$2" actual="$3"
  if [ "$actual" != "$expected" ]; then
    echo "not ok: $name: expected $expected, got $actual" >&2
    failures=$((failures + 1))
  else
    echo "ok: $name"
  fi
}

assert_status "done leaf open" "DONE" \
  "$(desired_status "Done" "Issue" "OPEN" "[]" "Behaviour Claim")"
assert_status "done leaf closed" "DONE" \
  "$(desired_status "Done" "Issue" "CLOSED" "[]" "Behaviour Claim")"
assert_status "done container with reopened child" "DONE" \
  "$(desired_status "Done" "Issue" "OPEN" '[{"state":"OPEN","status":"In Progress"}]' "Behaviour Claim")"
assert_status "active leaf" "IN_PROGRESS" \
  "$(desired_status "In Progress" "Issue" "OPEN" "[]" "Behaviour Claim")"
assert_status "closed leaf" "DONE" \
  "$(desired_status "Todo" "Issue" "CLOSED" "[]" "Behaviour Claim")"
assert_status "open pull request" "IN_REVIEW" \
  "$(desired_status "Todo" "PullRequest" "OPEN" "[]" "Behaviour Claim")"
assert_status "missing claim stays todo" "TODO" \
  "$(desired_status "Todo" "Issue" "OPEN" "[]" "")"
assert_status "todo leaf stays todo" "TODO" \
  "$(desired_status "Todo" "Issue" "OPEN" "[]" "Behaviour Claim")"

if [ "$failures" -ne 0 ]; then
  echo "$failures project-sync test(s) failed" >&2
  exit 1
fi

echo "project sync ok: Done is terminal; status decisions match the board rules"
