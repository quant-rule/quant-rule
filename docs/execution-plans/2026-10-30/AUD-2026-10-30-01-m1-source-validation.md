# Reject Non-M1 Ladder Inputs

- Status: Proposed
- Date: 2026-10-30 (UTC)
- Work item: `AUD-2026-10-30-01`
- Audit: [`docs/audits/2026-10-30/mvp-pre-release-audit.md`](../../audits/2026-10-30/mvp-pre-release-audit.md)
- Branch (when approved): `fix/aud-2026-10-30-01-m1-source-validation`

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

## Issue and PR traceability (proposed; actual numbers at creation)

| Finding | Issue | Sub-issue | PR | Verification |
|---|---|---|---|---|
| AUD-2026-10-30-01 | Reject non-M1 ladder inputs | Data-engine cadence error | PR-01a | Unit/edge test for the error |
| AUD-2026-10-30-01 | Reject non-M1 ladder inputs | Shell ladder preflight | PR-01b | Shell test on the H1 fixture |
| AUD-2026-10-30-01 | Reject non-M1 ladder inputs | Command regression matrix | PR-01c | `timeframes`, `market-state`, `market-report`, `store`, `research` |
| AUD-2026-10-30-01 | Reject non-M1 ladder inputs | Docs and config contract | PR-01d | README/config review |

Each sub-issue is independently reviewable and has one PR. The container’s own
deliverable is the complete rejection behavior.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `timeframes data/EURUSD_H1_...csv` exits 1 with the cadence error.
- `market-state`, `market-report`, `store ingest`, and `research summary` fail
  with the same error.
- A valid M1 fixture still passes.

## Risks and recovery

- A sparse M1 source could be rejected. Keep the guard narrow: reject only when
  the observed minimum spacing is greater than one minute or when all higher
  series are empty while the M1 row count is large. A valid M1 file with gaps
  whose rows are still one minute apart is accepted.
- A config source path can be renamed. The guard is cadence-based, not
  filename-based.

## Stop conditions

- The guard cannot distinguish a native higher-timeframe file from a valid sparse
  M1 file without a new source-timeframe contract. Then stop for a decision.
- The change requires altering the model or strategy contract.
