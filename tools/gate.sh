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
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
export CARGO_TARGET_DIR="$root/target/gate"

cd "$root"

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

cargo run --quiet -p quantrule-governance -- check --all

echo "workspace gate passed with CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
