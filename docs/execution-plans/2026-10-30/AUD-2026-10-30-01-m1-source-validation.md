# Reject Non-M1 Ladder Inputs

- Status: Verified
- Date: 2026-10-30 (UTC)
- Approved: 2026-10-30 (UTC), human authorization to implement in the task worktree
- Merged: 2026-10-30 (UTC) as `98cf9d5` (PR #221)
- Verified: 2026-10-30 (UTC) on merged `main` by a sequential read-only pass; not an independent agent judgment. Human acceptance remains open.
- Work item: `AUD-2026-10-30-01`
- Issue: [#220](https://github.com/quant-rule/quant-rule/issues/220)
- PR: [#221](https://github.com/quant-rule/quant-rule/pull/221)
- Audit: [`docs/audits/2026-10-30/mvp-pre-release-audit.md`](../../audits/2026-10-30/mvp-pre-release-audit.md)
- Branch: `fix/aud-2026-10-30-01-m1-source-validation`

## Intent

Stop `timeframes`, `market-state`, `axis`, `events`, `legs`, `model-series`,
`market-dashboard`, `market-chart`, `market-report`, `store`, and `research`
from silently analyzing a native M5/M15/H1 CSV as if it were M1. The same input
must be accepted or rejected consistently, and the error must say how to fix the
input.

This plan does not change the model formulas, strategy behavior, or execution.

## Current state

- `TimeframeSet` checks whole-minute alignment and derives M5/M15/H1 from every
  row as if it were an M1 candle.
- A native H1 file passes; the derived higher series are empty; the model labels
  the hourly rows `1m`.
- `market-state`, `market-report`, `store ingest`, and `research summary` exit 0
  on the H1 fixture; `backtest` fails later because the 15m strategy series is
  empty.
- The audit finding is `AUD-2026-10-30-01`; the evidence and remediation options
  are in the audit.

## Scope

Included:

- A source-cadence guard before the ladder is built.
- One clear error for a file that is not one-minute spaced.
- Regression tests for `timeframes`, `market-state`, `market-report`,
  `store ingest`, and `research summary`.
- Documentation update in the command contracts and config contract.

Excluded:

- Native-timeframe analysis. The product still requires an M1 source.
- Fixing the other ten audit findings.
- Changing the data engine's bucket formulas.

## Decisions

- Option A (recommended): reject when the source cadence is not one minute.
  Simplest and prevents silent wrong output. A native higher-timeframe file is a
  source-inspection artifact, not an analysis input.
- Option B: add `--source-timeframe` and build from it. Larger command and data
  engine change; useful later, not MVP.
- Option C: warn only. Rejected: a warning on stderr does not protect a report,
  store, or research export from silent corruption.

## Acceptance criteria

- `timeframes`, `market-state`, and `market-report` reject a native H1 fixture
  with a deterministic error naming the source cadence.
- `store ingest` and `research summary` reject the same file before writing.
- A config-driven `backtest` whose `data.json` points at the same file rejects
  before the bundle root is created.
- A valid M1 file (including a sparse M1 file with gaps) is still accepted.
- The error is the same across commands.
- No model, strategy, execution, or portfolio behavior changes.
- The regression tests fail before the fix and pass after it.

## Work breakdown

| Step | Work | Roles | Paths |
|---|---|---|---|
| 1 | Add a source-cadence check and typed error | Implementation, Test | `crates/quantrule-data/src/`, `crates/quantrule-shell/src/csv.rs` |
| 2 | Wire the check into the ladder consumers | Implementation | `crates/quantrule-shell/src/command/` |
| 3 | Add shell/integration regression tests | Test | `crates/quantrule-shell/tests/` |
| 4 | Update command and config docs | Architect/Implementation | `README.md`, `docs/config/`, `docs/audits/` |

## Issue and PR traceability

| Finding | Issue | Deliverable | PR | Verification |
|---|---|---|---|---|
| AUD-2026-10-30-01 | #220 | Cadence guard, shell preflight, regression matrix, and docs | #221 | Unit/edge tests, path and config command tests, fail-before archive |

The four proposed slices shipped together in #221. They are not independently
mergeable: the data-engine error, the shell guard, the tests, and the docs
describe one contract. The container’s own deliverable remains the complete
rejection behavior.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `timeframes data/EURUSD_H1_...csv` exits 1 with the cadence error.
- `market-state`, `market-report`, `store ingest`, and `research summary` fail
  with the same error.
- A valid M1 fixture still passes.

### Evidence (2026-10-30, local worktree)

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | Passed, no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed, no warnings |
| `cargo test --workspace` | All test binaries passed; 0 failed |
| `cargo doc --no-deps` | Passed, no warnings |
| `timeframes timeframes-h1.csv` | Exit 1, no stdout, `source cadence is 3600s, not 60s; provide an M1 source file for the timeframe ladder` |
| `market-state timeframes-h1.csv` | Exit 1, no stdout, same error |
| `market-report timeframes-h1.csv` | Exit 1, no output directory, same error |
| `market-report` with a 2025-01-01 bound (one H1 row retained) | Exit 1, same error; proves the cadence is measured before the date filter |
| `store ingest timeframes-h1.csv` | Exit 1, no database, same error |
| `research summary timeframes-h1.csv` | Exit 1, no stdout, same error |
| `timeframes timeframes-gaps.csv` | Exit 0; sparse M1 with gaps still accepted |
| `timeframes single.csv` | Exit 0; a single row has no measured spacing and is not rejected |
| `candles timeframes-h1.csv` | Exit 0; raw candle inspection is outside the ladder guard by design |
| `backtest --config` with an H1 `data.json` and a one-row date bound | Exit 1, no bundle root, same error |
| Fail-before check on an archive of `origin/main` with the new tests | 4 `cli` tests, 2 `market_report` tests, and 1 `backtest_command` test failed; old `store ingest` printed `ingested demo bars=6 timeframes=1`, and the old config run failed later with `the strategy timeframe 900 produced no bars` |
| Same tests on this branch | All 7 passed |

The fail-before run confirms the tests exercise the defect and the fix. No
model, strategy, execution, or portfolio code changed.

### Closure verification (2026-10-30, merged `main` at `98cf9d5`)

| Check | Observed |
|---|---|
| 15 ladder consumers on `timeframes-h1.csv` | Exit 1, empty stdout, no artifact, same cadence error |
| `timeframes`, `market-state`, `axis`, `events`, `legs`, `market-dashboard`, `market-chart`, `market-report`, `store ingest`, `store verify`, `research summary`, `research export`, `strategy-run` | All rejected |
| Config-driven `backtest` and `report model` with an H1 `data.json` | Exit 1, no bundle root, same error |
| Sparse M1, single-row, and header-only fixtures | Accepted; gaps and partial buckets unchanged |
| `candles` and `candles-tolerant` on the H1 fixture | Accepted; raw inspection is outside the guard |
| One-row `market-report` date bound | Rejected; no output directory |
| Repeated H1 run | Byte-identical stderr |
| 7 regression tests on `0f21b61` | 7 failed |
| Same 7 tests on `98cf9d5` | 7 passed |
| `cargo fmt`, `clippy`, `test --workspace`, `doc --no-deps` | Passed |

The pass was sequential in the same agent session that implemented the fix; it
is not an independent agent or model judgment.

## Risks and recovery

- A sparse M1 source could be rejected. The guard rejects only when the
  observed minimum spacing is greater than one minute, so a valid M1 file with
  gaps whose rows are still one minute apart is accepted. A file with every row
  separated by more than one minute is not an M1 source under this contract;
  the error asks for an M1 file.
- A one-row source has no spacing to measure, so it cannot be distinguished
  from a native higher-timeframe file with one row; it is accepted as a
  degenerate series. A second row makes the cadence observable.
- A config source path can be renamed. The guard is cadence-based, not
  filename-based.

## Stop conditions

- The guard cannot distinguish a native higher-timeframe file from a valid sparse
  M1 file without a new source-timeframe contract. Then stop for a decision.
- The change requires altering the model or strategy contract.
