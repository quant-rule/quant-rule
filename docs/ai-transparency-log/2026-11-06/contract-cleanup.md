# AI Transparency Log — Contract Cleanup

- Date: 2026-11-06 (UTC)
- Plan: [`docs/execution-plans/2026-11-06/contract-cleanup.md`](../../execution-plans/2026-11-06/contract-cleanup.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash), single session
- Intent and boundary: Read-only audit first; then implement the approved current-surface cleanup in `../quant-rule--contract-cleanup` on branch `chore/contract-cleanup`. No push, PR, merge, tag, or release.

## 1. Audit pass

The first pass inspected the repository, ran the local gates, and reproduced two output-level defects. Observed results:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace --no-fail-fast` | pass; 0 failed |
| `cargo doc --workspace --no-deps` | pass; no warnings observed |
| `python3 tools/check_registry.py` | pass; 46 indicators, 143 events, 110 parameters |
| `python3 tools/check_validation.py` | pass; 188 claims |
| `python3 tools/check_links.py` | pass; no broken relative links |
| `python3 tools/check_terms.py` | pass; no banned terms |
| `python3 tools/check_legacy.py` | pass; 353 current files |
| `git status --short --branch` | clean before the worktree was created |

Reproduced evidence:

- `events` over `state_m1.csv` violated the documented per-timeframe availability order: 1m 259→1, 5m 51→0, 15m 16→2. The same violations appeared in the generated `events.csv`.
- `readings::roles.rs` had no caller outside its own module and tests.
- `readings::roles` published the four retired `AxisCode` values.
- The registry retained eight `strategy.*` parameters and one `analytics.*` parameter.
- `check_registry.py` defined `SCHEMA` but never used it.
- `check_validation.py` accepted any non-empty test string; two claims named tests that do not exist.
- `target/debug/quantrule help roles` and `help tutorial` returned unknown-topic errors.
- A name-resolution check over the 188 claims found `event_validation::bos_and_choch` and `defect::session_range_contraction` absent from the Rust sources.

## 2. Implemented work

The remediation plan is recorded in the execution plan. The following was actually changed:

- Added `TimeframeModel::all_events` and used it in the `events` renderer and `events.csv`, so core and extended records share one availability order.
- Fixed the extended-event dedup key to include direction and reference value; added direct tests for distinct MFI threshold crossings and exact duplicates.
- Removed the unused four-axis `readings::roles` module and the `LEGACY_UNREGISTERED` code 0 and its test bypass.
- Removed the eight `strategy.*` and one `analytics.*` registry parameters, the legacy axis source codes 9–11, and the corresponding shell labels.
- Bumped the registry and validation versions to `2026-11-06.1`; added the 1,024-record retention bound to the M1.0 contract and corrected its unit to `records`.
- Hardened `check_registry.py` with schema-shape checks, Rust event-code and `REGISTRY_VERSION` checks, and the removal of the strategy-path default skip. Hardened `check_validation.py` to resolve construct tests to Rust test functions and report active versus total events.
- Fixed the two dangling construct-test claims and the `construct-gaps.md` row.
- Corrected the AGENTS workspace/crate/current-surface statements; the `data-v3`, policy, MACD, architecture, model, registry-history, and index documents; the `help` text; and the stale role/workflow status lines.
- Marked the pre-M1.0 quant manuals historical and the tiered improvement workflow/run contract S3.x; aligned the tool READMEs and CI step labels.
- Strengthened the workspace-manifest architecture guard and the report-model determinism test.

## 3. Verification

Observed on 2026-11-06:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace --no-fail-fast` | pass; all test binaries and doctests reported ok, 0 failed |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | pass |
| `python3 tools/check_registry.py` | pass; 46 indicators, 143 events, 101 parameters |
| `python3 tools/check_validation.py` | pass; 188 claims for 46 indicators and 142 active events |
| `python3 tools/check_links.py` | pass; 548 Markdown files |
| `python3 tools/check_terms.py` | pass; 324 files |
| `python3 tools/check_legacy.py` | pass; 353 current files |
| `git diff --check` | pass |

The new ordering tests pass on `state_m1.csv` and the generated `events.csv`. The new MFI dedup tests pass. The registry gate now fails on a Rust/registry code or version divergence, and the validation gate now fails on a construct-test name that does not resolve to a `#[test]` function.

## 4. Learning and corrections

- The first registry cleanup exposed two stale `benchmark.*` entries still marked `analytics`; they were reassigned to a new `benchmark` affect rather than deleted, because the benchmark module is retained.
- The first schema cleanup used a full JSON rewrite that produced a noisy diff; it was replaced with a minimal edit to the tracked schema.
- The original architecture guard read the shell manifest, not the workspace manifest; that was confirmed and corrected.
- The 2026-11-05 decision to keep `readings/**` as a whole allowed the four-axis role registry to survive. The current cleanup removes that module rather than broadening the keep rule.
- The human authorized publication after the commit. The branch is pushed to
  `origin/chore/contract-cleanup`, PR #491 is open, and the `verify` and `sync`
  GitHub checks pass. No merge, tag, or release was performed.
