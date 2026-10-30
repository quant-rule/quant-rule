# AI Transparency Log — M1 Source Validation

- Date: 2026-10-30 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential implementation and verification passes
- Work item: `AUD-2026-10-30-01`
- Plan: `docs/execution-plans/2026-10-30/AUD-2026-10-30-01-m1-source-validation.md`

## Intent and boundary

The human asked to remediate the Critical audit finding that native M5/M15/H1
CSVs are silently analyzed as M1. The approved plan chose Option A: reject a
source whose cadence is not one minute, without adding native-timeframe
analysis. The work stayed inside that scope. No model formula, strategy,
execution, portfolio, or analytics behavior changed. After human authorization,
the work was committed as `598708f`, pushed to
`fix/aud-2026-10-30-01-m1-source-validation`, tracked by
[#220](https://github.com/quant-rule/quant-rule/issues/220), and opened as
[#221](https://github.com/quant-rule/quant-rule/pull/221). Merge, board
acceptance, and independent verification remain open.

## Implemented

- Added `SourceCadence` to the data engine: it records every parsed source row
  in file order, keeps the smallest positive spacing, and ignores non-positive
  deltas so duplicate or out-of-order rows keep their own errors.
- Added `TimeframeError::SourceCadence`, which names the observed spacing and
  asks for an M1 source.
- Wired the check into the shared shell ladder loader before `TimeframeSet`
  finishing. The cadence is measured across the whole file before `--from` and
  `--to` filtering, so a one-row date window cannot hide a native H1 file.
- Added an H1 fixture and regression tests for `timeframes`, `market-state`,
  `market-report` (plain and one-row date bound), `store ingest`,
  `research summary`, and a `backtest --config` run whose `data.json` points at
  the H1 fixture, plus a single-row acceptance test and data-engine unit tests
  for M5, M15, H1, sparse M1, and non-positive deltas.
- Updated the README CSV and command contracts, the config contract, the
  in-shell glossary and `timeframes` help, the audit finding status and detail,
  and the help fixtures.
- Marked the plan `Verified` after recording observed evidence. Independent
  post-merge verification and human acceptance remain open.

## Verification and evidence

Commands actually run in this worktree:

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | Passed, no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed, no warnings |
| `cargo test --workspace` | All test binaries passed; 0 failed |
| `cargo doc --no-deps` | Passed, no warnings |
| `timeframes timeframes-h1.csv` | Exit 1, empty stdout, cadence error |
| `market-state timeframes-h1.csv` | Exit 1, empty stdout, same error |
| `market-report timeframes-h1.csv` | Exit 1, no output directory, same error |
| `store ingest timeframes-h1.csv` | Exit 1, no database, same error |
| `research summary timeframes-h1.csv` | Exit 1, empty stdout, same error |
| `timeframes timeframes-gaps.csv` | Exit 0; sparse M1 still accepted |
| `timeframes single.csv` | Exit 0; a single row has no measured spacing |
| `candles timeframes-h1.csv` | Exit 0; raw inspection is outside the ladder guard |
| `backtest --config` with an H1 `data.json` and a one-row date bound | Exit 1, no bundle root, cadence error |
| Fail-before run on an archive of `origin/main` with the new tests | 7 tests failed; old `store ingest` printed `ingested demo bars=6 timeframes=1`, and the old config run failed later with `the strategy timeframe 900 produced no bars` |
| Same tests on this branch | All 7 passed |

The fail-before run is the regression guard evidence: the tests fail on the
pre-fix code and pass on the fixed code. The old `store ingest` output also
reproduces the audit finding directly.

## Learning and rejected suggestions

- Rejected enforcing the guard inside `TimeframeSet::finish()`. Sparse
  whole-minute rows are a data-engine feature with existing gap tests; the
  ladder contract belongs to the source loader, not to every set consumer.
- Rejected measuring cadence after the date filter. A narrow `--from`/`--to`
  window can retain one row, which would make a native H1 file look valid; the
  one-row regression test keeps that hole closed.
- Rejected warning-only behavior. A warning would still let a report, store, or
  research export publish a mislabeled ladder.
- Kept `candles` and `candles-tolerant` outside the guard: they inspect raw
  source rows and never build the ladder, so rejecting a native H1 file there
  would remove a legitimate source-inspection use.
- Did not add `--source-timeframe` or native ladder analysis; that is Option B
  and remains a separate product decision.
