#!/usr/bin/env bash
# Run the shared checker and checker-test lists.
#
# `tools/gate.sh` and the CI workflow call this script, so the local gate and the
# pull-request gate cannot drift: one list names the checkers, one list names the
# checker unit tests, and both entry points run both lists.
#
# Usage:
#   bash tools/checkers.sh                 run every checker, then every test
#   bash tools/checkers.sh check_links     run one checker and its test, if any
#   bash tools/checkers.sh --tests-only    run only the checker unit tests
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

tests_only=0
selected=()
while [ "$#" -gt 0 ]; do
  case "$1" in
    --tests-only)
      tests_only=1
      ;;
    *)
      selected+=("$1")
      ;;
  esac
  shift
done

if [ "${#selected[@]}" -gt 0 ]; then
  checkers=("${selected[@]}")
else
  mapfile -t checkers < tools/checkers.txt
fi

if [ "$tests_only" -ne 1 ]; then
  for checker in "${checkers[@]}"; do
    python3 "tools/${checker}.py"
  done
fi

if [ "${#selected[@]}" -gt 0 ]; then
  for checker in "${checkers[@]}"; do
    module="test_${checker}"
    if grep -qx "$module" tools/checker_tests.txt && [ -f "tools/${module}.py" ]; then
      (cd tools && python3 -m unittest "$module")
    fi
  done
else
  while IFS= read -r module; do
    [ -n "$module" ] || continue
    (cd tools && python3 -m unittest "$module")
  done < tools/checker_tests.txt
fi

echo "checker gate passed: ${#checkers[@]} checkers"
