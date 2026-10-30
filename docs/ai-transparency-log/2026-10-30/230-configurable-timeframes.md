# AI Transparency Log — Configurable Timeframes and 4h

- Date: 2026-10-30 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#230](https://github.com/quant-rule/quant-rule/issues/230)
- Parent plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- Status: Implemented; PR pending

## Intent and boundary

The human approved the unified ladder plan. Phase 2 makes the data engine's
available timeframe set configurable and adds 4h support. It does not yet wire
the strategy active ladder, model v2 per-timeframe settings, or execution on the
fastest active timeframe; those are later sub-issues.

The current path-based commands keep the historical four-timeframe default
(`1m`, `5m`, `15m`, `1h`) so Phase 2 does not churn unrelated model, dashboard,
report, store, and research goldens. A configured run uses the available
timeframes from `data.json` v3.

## Implemented

- Added `Timeframe::H4` (14,400 seconds).
- Added `Timeframe::DEFAULT_LADDER` for path-based commands.
- Updated `Timeframe::ALL` to five supported timeframes and `DERIVED` to four.
- Extended `TimeframeSet` to five series and four derived buckets.
- Added complete and partial 4h aggregation tests.
- Changed `data.json` to schema `quantrule.data/v3` with
  `available_timeframes`.
- Validated ascending, unique, non-empty, `1m`-first, supported-timeframe lists.
- Allowed any supported subset, including `1m/15m/4h`.
- Added a dedicated config test for a subset that includes 4h.
- Made the shell model-input container dynamic over the selected ladder.
- Configured model and backtest runs now use `data.available_timeframes`.
- Path-based commands keep the default four-timeframe ladder.
- Updated README, help, help fixtures, and the current data contract.
- Updated the example and fixture data configs to v3.

## Verification and evidence

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | Passed; 44 suites, 0 failures |
| `cargo doc --no-deps` | Passed |
| Complete 4h bucket from 240 M1 candles | Unit test passed |
| Partial 4h bucket with 239 M1 candles | Unit test passed; not published |
| `available_timeframes` subset `1m/15m/4h` | Config test passed |
| Existing 1m/5m/15m/1h tests | Passed |
| Source-cadence rejection | Passed |
| Worktree | `quant-rule--230-configurable-timeframes` |

## Deviations and decisions

- Path-based commands still use `1m/5m/15m/1h` by default. The active ladder and
  4h context are wired in the strategy phases. This avoids unrelated golden
  churn and keeps Phase 2 reviewable.
- The `timeframes` data command now prints five series, including 4h, because it
  is the data-engine view.
- `fetch` still lists native 1m/5m/15m/1h only. H4 is derived from M1 by the data
  engine; it is not advertised as a native source interval.
- `AGENTS.md` normative boundary edits remain deferred to the behavior-changing
  model/strategy phases.

## Learning and rejected suggestions

- Rejected making every path command build five timeframes immediately. It
  would change model, dashboard, report, store, and research outputs before the
  strategy context exists.
- Rejected removing the default ladder. A path command needs a deterministic
  default; `DEFAULT_LADDER` makes that explicit.
- Rejected allowing an empty or unordered available list. The data engine needs
  a stable ascending ladder for bucket construction and reporting.

## Residual risk

- Model output and reports still use four timeframes unless a configured run
  selects 4h. The active-ladder strategy will make the selection first-class.
- The store and research layer still default to four timeframes. Their
  available-ladder migration is part of the reporting/analytics phase.
- The 4h series is derived and tested at the unit level; a full configured run
  with 4h is covered in later phases.
