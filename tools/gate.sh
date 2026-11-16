#!/usr/bin/env bash
# Run the workspace gate with a target directory inside this worktree.
#
# A shared Cargo target directory can serve a test binary compiled in another
# worktree; compile-time paths such as CARGO_MANIFEST_DIR then point at the
# removed worktree. The local target keeps the gate reproducible.
#
# The checker and checker-test lists live in tools/checkers.txt and
# tools/checker_tests.txt. CI calls tools/checkers.sh too, so both entry points
# enforce the same checks.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
export CARGO_TARGET_DIR="$root/target/gate"

cd "$root"

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

bash tools/checkers.sh

echo "workspace gate passed with CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
