# AI Transparency Log — Ladder Verification and Experiment

- Date: 2026-10-30 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#237](https://github.com/quant-rule/quant-rule/issues/237)
- Parent plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- Status: Implemented; PR [#246](https://github.com/quant-rule/quant-rule/pull/246) open; CI passed

## Intent and boundary

Phase 9 verifies the unified ladder implementation and runs the pre-registered
control, A, B, and C matrix. It records descriptive evidence and a disclosed
sequential validation pass. It does not claim an edge and does not change the
strategy after seeing a result.

Verification found one program defect: the shell label map had no `4h` entry, so
`policy check`, `strategy explain`, and reports rendered 14,400 seconds as
`unknown`. The fix adds the label and a regression test. No model, strategy, or
portfolio computation changed.

## Implemented

- Added the pre-registered experiment
  `docs/experiments/2026-10-30/EXP-2026-10-30-02-unified-ladder.md`.
- Added the four cell configs under `docs/experiments/2026-10-30/config/`.
- Added the deterministic matrix manifest and used `tools/experiment/run-matrix.sh`.
- Added the OOS analysis helper `docs/experiments/2026-10-30/analyze_unified_ladder.py`.
- Added the Quant Validator record
  `docs/reports/2026-10-30/unified-ladder-validation.md`.
- Fixed the missing 4h shell label and added its unit test.
- Ran the 600k execution-bar context benchmark in release.
- Updated the plan checklists and evidence.

## Inputs

| Item | Value |
|---|---|
| Data | `dukascopy-bid-solusd-m1-2025-01-01_2025-12-31` |
| Source | `data/SOLUSD_M1_2025-01-01_2025-12-31.csv` |
| Rows | 522,633 accepted |
| Instrument | SOLUSD |
| In-sample | 2025-01-01..2025-08-31 |
| Out-of-sample | 2025-09-01..2025-12-31 |
| Costs | commission 0.1% of equity, slippage 2 bps, funding 0.0001/8h, execution fee 0.0 |

The source was fetched from Dukascopy BID with the shell `fetch` command. The
check-in does not include the 31 MB CSV; the experiment record gives the exact
fetch command.

## Results

| Cell | Full return | Full DD | Trades | OOS return | OOS trades |
|---|---|---|---|---|---|
| Control | +0.594% | 1.966% | 12 | -1.549% | 5 |
| A | -9.098% | 9.219% | 40 | -1.429% | 13 |
| B | +1.160% | 1.525% | 12 | -0.716% | 4 |
| C | -1.372% | 2.730% | 7 | -1.557% | 3 |

Every OOS cell is negative. Triage is `no edge` for this sample. The control and
B are positive over the full year, but the OOS read does not support a stability
claim. No threshold was changed after the result.

## Verification and evidence

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | Passed; 45 suites, 0 failures |
| `cargo doc --no-deps` with `RUSTDOCFLAGS=-D warnings` | Passed |
| Matrix | Four cells completed; five bundles and `backtest.json` each |
| Control rerun | All five bundles byte-identical |
| A rerun | All five bundles byte-identical |
| Context benchmark | 600,000 bars in 0.270 s (release) |
| 4h label | Fixed and unit-tested |
| OOS analysis | Reproduced from `trades.csv` and `equity.csv` |
| Local doc links | Resolved |
| PR | #246 opened |
| CI `verify` and `sync` | Passed |

## Quant validation (sequential pass)

The validation record checks look-ahead, split integrity, costs, execution
realism, determinism, capacity limits, edge cases, and residual risk. It is a
same-agent sequential pass, not an independent verifier or independent model
judgment. The OOS evidence is negative, so the verdict is "implementation
validated; no edge evidence".

## Learning and rejected suggestions

- Rejected changing the control thresholds after seeing the OOS result. That
  would be a post-hoc change and a new experiment.
- Rejected calling the full-year control positive a promising result. The OOS
  filter is negative.
- Rejected a separate OOS backtest that restarts at the split. It would lose the
  4h and model warm-up; the filtered continuous run is recorded with that
  limitation.
- Rejected adding 4h to the store schema. The store's three fixed higher blocks
  are a separate change.
- Rejected an R-multiple metric because the report field does not exist yet.

## Residual risk

- One instrument and one year; the sample is small for a regime claim.
- The OOS read filters a continuous run; it does not restart the model or
  strategy at the split.
- The validation is sequential, not independent.
- Capacity, market impact, and live execution are not tested.
- The execution loader is still v2; the v3 schema remains a target.
