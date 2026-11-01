# AI Transparency Log — Diagnosis Checks

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #311, branch `fix/311-diagnosis-checks`
- Plan: `docs/execution-plans/2026-10-31/full-system-audit.md` (W4)
- Audit: `docs/audits/2026-10-31/full-system-audit.md` (AUD-2026-10-31-44, -45, -50, -68)

## Intent and boundary

The human approved the full-system audit plan on 2026-10-31. W4 corrects the
diagnosis evidence: the axis-correlation check must measure all six pairs, an
`unknown` check must not receive failure wording, CSV nulls must be empty cells,
and `backtest --diagnose` must read the five written report versions instead of
synthesizing them from constants.

The change is confined to the analytics diagnosis and the shell diagnosis
projection. It does not touch a model axis value, a strategy rule, an
indicator, execution, portfolio accounting, or a diagnosis threshold. It adds
no dependency.

## Implemented

- Added the four missing pairwise correlations to `ModelHealth`:
  Direction/Progression, Momentum/Structure, Momentum/Progression, and
  Structure/Progression. The six pairs are accumulated in bar order.
- The `model.axis_correlation` check takes the maximum absolute value over all
  six pairs and names the worst pair in its evidence.
- Added `ModelHealth` fields to the diagnosis JSON projection.
- Made triage status-aware: `unknown` checks use measure/record wording; the
  negative-expectancy, slippage-exceeds, costs-dominate, and limit-breached
  text appears only for a failed check.
- `checks.csv` and `model-health.csv` write an empty cell for a missing number;
  `summary.txt` keeps `-`.
- `report_versions` reads the five `report.json` files under the bundle root
  (`backtest --diagnose` passes the output root; `lab diagnose` passes the lab)
  and falls back to the constants only when no root is supplied.
- Updated the diagnosis contract and added the W4 validator record.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo test -p quantrule-analytics --lib diagnosis::compute::tests` | 5 passed, 0 failed |
| `cargo test -p quantrule-analytics --test diagnosis` | 15 passed, 0 failed |
| `cargo test -p quantrule-shell --bin quantrule diagnosis::tests` | 1 passed, 0 failed |
| `cargo test -p quantrule-shell --test diagnosis` | 10 passed, 0 failed |
| `cargo test --workspace` | 1,543 passed, 0 failed, 7 ignored; 47 suites |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The cross-cutting regression is the pair test with a crafted 0.9
Momentum/Structure correlation, the no-trade/no-fill/no-limit triage test, the
null CSV test, and the bundle-version test. The byte-identical diagnosis test
still passes.

## Learning and rejected suggestions

- The old check used only the two correlations the report already carried. The
  four missing pairs were computable from the same per-bar values; adding them
  made the check match its contract rather than narrowing the contract.
- The triage function skipped only `Pass`, so every `Unknown` fell through to a
  failure sentence. The fix branches on status, not on a second check list.
- The `report_versions` constants are still the fallback for a caller with no
  bundle root, but the `backtest --diagnose` and `lab diagnose` paths now pass
  the written bundles. A missing or unreadable bundle fails instead of being
  reported as a constant.
- I considered changing `model-health.csv` to include the six correlations, but
  the contract's CSV is axis statistics; the correlations belong in
  `diagnosis.json`, as before. The check and JSON carry the extra pairs.

## Limitations

- The pass is sequential and same-agent; the validator record is not an
  independent model judgment.
- The tests use deterministic fixtures, not live market data.
- The correlations are diagnostics. No signal path reads them, and the
  `model.axis_correlation` threshold is unchanged.
