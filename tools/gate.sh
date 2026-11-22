#!/usr/bin/env bash
# Run the workspace gate with a target directory inside this worktree.
#
# A shared Cargo target directory can serve a test binary compiled in another
# worktree; compile-time paths such as CARGO_MANIFEST_DIR then point at the
# removed worktree. The local target keeps the gate reproducible.
#
# The Rust checker registry is the single authority. `check --all` runs every
# registered checker, and CI calls the same command, so both entry points
# enforce the same checks.
#
# Usage:
#   tools/gate.sh                    full workspace gate
#   tools/gate.sh --changed <range>  classify the range and run one scope
#   tools/gate.sh --explain <range>  print the scope and reasons
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
export CARGO_TARGET_DIR="$root/target/gate"

cd "$root"

changed=""
explain=0
mode="full"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --changed)
      [ "$#" -ge 2 ] || { echo "usage: tools/gate.sh --changed <range>" >&2; exit 2; }
      changed="$2"
      shift 2
      ;;
    --explain)
      [ "$#" -ge 2 ] || { echo "usage: tools/gate.sh --explain <range>" >&2; exit 2; }
      explain=1
      changed="$2"
      shift 2
      ;;
    *)
      echo "usage: tools/gate.sh [--changed <range> | --explain <range>]" >&2
      exit 2
      ;;
  esac
done

if [ -n "$changed" ]; then
  if [ "$explain" -eq 1 ]; then
    cargo run --quiet -p quantrule-governance -- gate plan --changed "$changed" --root "$root"
    exit $?
  fi
  mode="$(cargo run --quiet -p quantrule-governance -- gate plan --changed "$changed" --root "$root" --field mode)" || mode="full"
fi

checkers() {
  cargo run --quiet -p quantrule-governance -- check --all
}

full_gate() {
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace --no-fail-fast
  RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
  checkers
}

governance_gate() {
  cargo fmt --all -- --check
  cargo clippy -p quantrule-governance --all-targets -- -D warnings
  cargo test -p quantrule-governance
  RUSTDOCFLAGS="-D warnings" cargo doc -p quantrule-governance --no-deps
  checkers
}

web_gate() {
  command -v bun >/dev/null || { echo "web scope needs bun on PATH" >&2; exit 1; }
  checkers
  (
    cd apps/web
    bun install --frozen-lockfile
    bun run check
    bun test
    bun run build
    bun run e2e
  )
}

case "$mode" in
  docs)
    checkers
    ;;
  governance)
    governance_gate
    ;;
  web)
    web_gate
    ;;
  full)
    full_gate
    ;;
  *)
    echo "unknown gate mode: $mode" >&2
    exit 1
    ;;
esac

echo "gate passed: $mode with CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
