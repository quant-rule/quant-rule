# MVP Pre-Release Audit

- Date: 2026-10-30 (UTC)
- Audit ID: AUDIT-2026-10-30-mvp-pre-release
- Status: Published
- Auditor: Audit Agent (AI), sequential audit pass
- Baseline: `main` at `2739a67`
- Toolchain: Rust 1.99.1
- Data: committed EURUSD/H1 fixtures and generated M1 fixtures; no live source
- Limitations: the audit did not re-run every command on every market; the
  findings below are reproducible from the commands and code paths named.

## Summary

| Severity | Count |
|---|---:|
| Critical | 2 |
| Major | 4 |
| Minor | 4 |
| Info | 1 |

## Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| AUD-2026-10-30-01 | Critical | data | Native M5/M15/H1 CSVs are accepted as M1 | `timeframes data/EURUSD_H1_...csv`; `market-state ... --window latest`; `market-report`; `store ingest`; `research summary` | Silent wrong timeframe labels and wrong model/store/research output | A reject non-M1 cadence; B add `--source-timeframe`; C warn only | A | Reject an H1/M5 file in every ladder consumer | Fixed | [`plan`](../../execution-plans/2026-10-30/AUD-2026-10-30-01-m1-source-validation.md) |
| AUD-2026-10-30-02 | Critical | backtest | Strategy virtual state diverges from execution | `allow_short=false` run: strategy 166 entries, 93 executed; 73 `missing_quantity` | Backtest evaluates a different strategy from the one reported | A reconcile fills/rejections into strategy; B make strategy stateless; C reject mismatched exits | A | `allow_short=false` cannot report a short it did not execute; exit side is checked | Open | — |
| AUD-2026-10-30-03 | Major | backtest | A bar opening beyond the target but dipping below the stop fills as a stop | `execution/engine.rs` stop-first branch; entry 100, ATR 1, target 103, next open 104, low 97 → StopLoss at 98 | Impossible sequence; biases results toward losses | A fill target at open before stop; B define stop-first even for gap; C reject ambiguous cases | A | A gap-through-target test asserts TakeProfit at the open | Open | — |
| AUD-2026-10-30-04 | Major | portfolio | Fixed notional ignores the leverage/equity envelope | `portfolio/sizing.rs`; initial cash 1000, notional 100000 → quantity 96953, cash negative | 100x exposure despite `leverage: 1.0`; wipe-out risk | A cap/reject at fill; B enforce at sizing; C define fixed notional as leverage-free and remove the claim | A | A fixed-notional entry above the envelope is rejected | Open | — |
| AUD-2026-10-30-05 | Major | data | Zero/negative prices are accepted by data/model/store but rejected by backtest | `candles zero.csv` exit 0; `market-state` exit 0; `store ingest` exit 0; `backtest` exits 1 “price must be finite and positive” | Supported commands disagree on input validity | A reject non-positive prices in the data engine; B support them in portfolio; C document the split | A | The same candle is accepted or rejected consistently by every command | Open | — |
| AUD-2026-10-30-06 | Major | backtest | Quantity floor uses binary `f64` and can under-size or reject | `notional 3.0 / price 5.0 → 0.5`; `0.3 / 3.0 → BelowOneStep` | Silent one-step under-size or a valid entry rejected | A decimal/tolerance floor; B document the one-step rule; C reject non-exact steps | A | Exact step boundaries produce the exact quantity | Open | — |
| AUD-2026-10-30-07 | Minor | shell/store | `store remove` creates a new database when the path is missing | `store remove nope --db missing.db` exits 1 but creates a 135 KB DB | A typo leaves an empty database; surprising side effect | A open existing read-write only; B keep creation; C require `store init` | A | Missing DB is an error and no file is created | Open | — |
| AUD-2026-10-30-08 | Minor | shell | Tolerant reader classifies an invalid duplicate/out-of-order row as `invalid` | duplicate timestamp with `high < low` reports `invalid`, not `duplicate` | Quality counts disagree with the README rule | A validate ordering before candle validity; B document the precedence | A | A duplicate invalid row is counted as duplicate | Open | — |
| AUD-2026-10-30-09 | Minor | data | Aggregated volume can overflow to infinity | five `1e308` volumes then a new bucket | A finite input is rejected as `NonFinite` at an innocent row | A reject overflow with a bucket error; B use a checked sum policy | A | Overflow reports the bucket, not the next source row | Open | — |
| AUD-2026-10-30-10 | Minor | docs | `--window latest` bypasses the 100000 cap while help says “Input cap: 100000” | `market-state --window latest` on a 183k-row file succeeds | Ambiguous cap contract and surprising runtime | A state the latest-window exception; B enforce the cap; C add a warning | A | Help and README agree with the observed behavior | Open | — |
| AUD-2026-10-30-11 | Info | model | Percentile/readiness diagnostics are present but not read by the strategy | `StrategyInput` fields; model diagnostic fields | Needs a permanent boundary guard, not a behavior change now | A boundary test + doc; B move diagnostics to analytics later | A | The strategy cannot access a percentile/readiness field | Open | — |

## Detail

### AUD-2026-10-30-01 — Native higher-timeframe CSV accepted as M1

- Expected: a ladder consumer reads M1 and labels the source truthfully.
- Observed: `timeframes` reports `1m retained=1000`, `5m/15m/1h retained=0`;
  `market-state` returns `ladder:["1m"]`; `market-report` reports 3074 1m bars
  and 0 higher bars; `store ingest` stores `timeframe_secs=60`; `research
  summary` reports `timeframe=60`.
- Root cause: `TimeframeSet` only checks whole-minute alignment. It has no source
  cadence check, and the shell passes every CSV through the M1 constructor.
- Options:
  - A. Reject when the source cadence is not one minute (or when the higher
    series are empty while the M1 series has many rows).
  - B. Add an explicit `--source-timeframe` and build the ladder from it.
  - C. Warn only; the user must choose.
- Recommendation: A for MVP; B is the longer-term source model.
- Remediation: implemented in branch
  `fix/aud-2026-10-30-01-m1-source-validation` (not merged). Every ladder
  consumer measures the smallest spacing across all parsed source rows before a
  date bound is applied and rejects a file wider than 60s with
  `source cadence is <n>s, not 60s; provide an M1 source file for the timeframe
  ladder`. Regression tests cover `timeframes`, `market-state`, `market-report`
  (including a one-row date bound), `store ingest`, `research summary`, and a
  `backtest --config` run whose `data.json` points at the H1 fixture; the sparse
  M1 fixture with gaps remains accepted. Status is `Fixed`; independent
  verification is pending.

### AUD-2026-10-30-02 — Strategy virtual state diverges from execution

- Expected: the strategy reported by the system is the strategy the account ran.
- Observed: with `allow_short=false`, the strategy reports 166 entries (93 long,
  73 short) while execution has 93 entries and 73 `missing_quantity` rejections.
  The strategy keeps a virtual short that never existed.
- Root cause: `AxisAgreementStrategy` updates its position on the decision, but
  the runner never feeds fills, rejections, or bracket exits back to it. The
  engine also ignores `Exit.side`.
- Options:
  - A. Add `on_fill`/`on_reject` reconciliation; reject a mismatched exit side.
  - B. Make the strategy stateless and let execution own the position.
  - C. Keep the separation but report both states explicitly and forbid the
    strategy from using its position for future decisions.
- Recommendation: A for the current strategy; B is the cleaner long-term shape.

### AUD-2026-10-30-03 — Gap-through-target stop

- Expected: a bar that opens at or beyond the target fills the target at the
  open before the stop can be reached.
- Observed: entry 100, ATR 1, stop 98, target 103; next bar open 104, low 97;
  engine returns `StopLoss` at 98.
- Root cause: `apply_bracket` checks the stop first. The stop-first tie rule is
  applied even when the open proves the target was reached first.
- Options:
  - A. Check a gap at the open before the intrabar tie rule.
  - B. Define stop-first even for gaps and document the bias.
  - C. Reject bars whose open/target/stop sequence is ambiguous.
- Recommendation: A; B is inconsistent with the target-gap reference rule.

### AUD-2026-10-30-04 — Fixed notional exceeds leverage

- Expected: `leverage: 1.0` caps exposure at equity.
- Observed: initial cash 1000, fixed notional 100000, quantity 96953; cash is
  negative and no entry is rejected.
- Root cause: `SizingMode::FixedNotional` ignores equity; `leverage` is only a
  config value.
- Options:
  - A. Reject or cap at sizing/fill when notional exceeds equity × leverage.
  - B. Enforce at the portfolio ledger.
  - C. Remove the leverage claim from fixed notional and document it as
    leverage-free.
- Recommendation: A.

### AUD-2026-10-30-05 — Non-positive prices disagree across commands

- Expected: one candle-validity contract.
- Observed: a zero-price M1 file is accepted by `candles`, `market-state`, and
  `store ingest`; `backtest` fails at the ledger with “price must be finite and
  positive”.
- Root cause: `Candle` validates finite and OHLC ordering but not positivity;
  the ledger/fill require positivity.
- Options:
  - A. Add a positive-price invariant to the data engine.
  - B. Support non-positive prices in the portfolio (e.g., certain futures).
  - C. Document the split and make it explicit.
- Recommendation: A unless a supported instrument requires non-positive prices.

### AUD-2026-10-30-06 — Quantity floor

- Expected: `notional / price` floored to the exact step.
- Observed: `3.0 / 5.0` floors to 0.5 with step 0.1; `0.3 / 3.0` rejects as
  below one step.
- Root cause: binary `f64` division and `floor` at the boundary.
- Options: decimal arithmetic; a checked tolerance; or a documented one-step
  rule. Recommend the tolerance/checked floor.

### AUD-2026-10-30-07 — `store remove` creates a database

- Expected: removing from a missing database is an error and creates nothing.
- Observed: the command creates a 135 KB database then reports the missing
  dataset.
- Root cause: `store remove` uses the create-capable `Store::open`.
- Options: open an existing database read-write; require `store init`; keep the
  behavior. Recommend the first.

### AUD-2026-10-30-08 — Tolerant duplicate classification

- Expected: a duplicate timestamp is counted as `duplicate`, even if the row is
  also invalid.
- Observed: `duplicate=0`, `invalid=1` for an invalid duplicate.
- Root cause: candle validity is checked before ordering.
- Options: check ordering first, or document the precedence. Recommend ordering
  first.

### AUD-2026-10-30-09 — Volume overflow

- Expected: an overflow is reported on the bucket that overflowed.
- Observed: `NonFinite` is reported at the next source row.
- Root cause: unchecked volume sum and aggregate candle validation.
- Options: checked sum with a bucket error; or a documented finite-input policy.

### AUD-2026-10-30-10 — Latest-window cap wording

- Expected: the help/README state the actual `--window latest` exception.
- Observed: help says “Input cap: 100000” while the latest path accepts a larger
  file and retains the last 1000.
- Options: fix the docs, enforce the cap, or warn. Recommend the docs.

### AUD-2026-10-30-11 — Diagnostic statistics boundary

- Expected: strategy input has no statistical field.
- Observed: the current `StrategyInput` has only the three primary axes and bar
  identity; model percentiles/readiness are diagnostics.
- Action: add a permanent boundary test and keep statistics in analytics. If a
  statistical field ever influences an axis value or entry/exit, escalate to a
  model-contract decision.
