#!/usr/bin/env bash
# Run the workspace gate with a target directory inside this worktree.
#
# A shared Cargo target directory can serve a test binary compiled in another
# worktree; compile-time paths such as CARGO_MANIFEST_DIR then point at the
# removed worktree. The local target keeps the gate reproducible.
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
export CARGO_TARGET_DIR="$root/target/gate"

cd "$root"

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

for checker in \
  check_command_surface \
  check_links \
  check_terms \
  check_legacy \
  check_structure \
  check_workflows \
  check_changelog \
  check_doc_length \
  check_releases \
  check_partitions \
  check_registry \
  check_labels \
  check_validation \
  check_verification \
  check_verification_matrix \
  check_event_emitters \
  check_file_size \
  check_port_parity \
  check_claims \
  check_execution_plans \
  check_board_permanence
do
  python3 "tools/$checker.py"
done

echo "workspace gate passed with CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
