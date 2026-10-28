# AI Transparency Log — Strategy Run Command

- Date: 2026-10-28 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#160](https://github.com/quant-rule/quant-rule/issues/160)
- Container: [#11](https://github.com/quant-rule/quant-rule/issues/11)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md`](../../execution-plans/2026-10-27/141-market-analysis-and-backtest.md)
- Boundary: [`docs/adr/2026-10-27/analysis-and-backtest-boundaries.md`](../../adr/2026-10-27/analysis-and-backtest-boundaries.md)
- Branch/worktree: `feat/160-strategy-run` in `quant-rule--160-strategy-run`
- PR: [#162](https://github.com/quant-rule/quant-rule/pull/162)
- Status: Merged in PR #162.

## Intent and boundary

S3 exposes the S2 `AxisAgreementStrategy` through a shell command. The command
loads the extended M1 range, derives only the strategy timeframe, feeds every
closed model bar to the strategy, and prints the resulting entries and exits.
It stops at the strategy decision.

The command creates no order, fill, position, equity, trade, or P&L record.
Those belong to the execution and portfolio work items. Model Signal events
remain a separate event type; the command does not relabel them as entries or
exits. No existing `market-*` command contract changed.

## Work performed

1. Added the `quantrule-backtest` dependency to `quantrule-shell`.
2. Added `Command::StrategyRun`, `StrategyRunArgs`, `ParseError::StrategyRun`,
   `StrategyRunError`, and `RunError::StrategyRun`, and wired the command through
   `parse_argv`, the interactive parser, execution dispatch, and help.
3. Added `crates/quantrule-shell/src/command/strategy.rs`: argument parsing,
   the M15 model run, the strategy loop, NDJSON rendering, the `--json`
   provenance envelope, and nine unit tests.
4. Added `crates/quantrule-shell/tests/strategy_run.rs` with eleven CLI tests:
   the golden NDJSON, the absence of fill/P&L fields, repeated-run determinism,
   interactive parity, JSON parity and provenance, date filtering, an empty
   period, a series too short for M15 bars, an unknown strategy, and a missing
   data version.
5. Added `crates/quantrule-shell/tests/fixtures/strategy-run.ndjson` with 13
   records over the generated 8,000-row M1 series.
6. Updated `README.md`, `docs/README.md`, the program plan, the help text and
   its golden, and this log.

## Verification and evidence

Commands were run in `quant-rule--160-strategy-run` on 2026-10-28. `cargo` is
not on `PATH` in this environment, so each command was invoked through
`~/.cargo/bin/cargo`.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --all-targets -- -D warnings` | passed |
| `cargo test` | 1,156 passed, 0 failed, 6 ignored |
| `cargo doc --no-deps` | passed |

The total is the previous 1,136 plus 9 new unit tests and 11 new integration
tests. No test was removed, skipped, or weakened.

The golden run over the 8,000-row generated M1 series produces 13 records,
from a short entry at bar 245 to an open short entry at bar 530. The final
position remains open, as the strategy has no end-of-data exit. A byte-for-byte
comparison across repeated runs passes, and the interactive and non-interactive
outputs match for both the bare NDJSON and `--json` forms.

## Decisions and learning

- **The golden is an output snapshot.** `strategy-run.ndjson` was captured from
  the command over a deterministic generated series. It locks the CLI record
  shape and rendering, not the rule's correctness; the S2 hand-derived golden
  covers the rule. Regenerating it is a deliberate review step.
- **`--json` is one envelope.** The plan's machine-readable form is a single
  provenance envelope whose `records` array carries the same records as the
  bare NDJSON. A second per-record JSON encoding would add a format without a
  consumer.
- **No P&L fields.** The record shape is asserted to carry no `fill`,
  `quantity`, `position`, `equity`, `pnl`, `fee`, `slippage`, `order`, or
  `trade` key. A later backtest command adds execution records; it does not
  widen this command's contract.
- **Only the strategy timeframe is computed.** The command runs
  `TimeframeModel` for M15 rather than the full ladder, so the extended range
  costs one timeframe and the strategy cannot read a timeframe it was not
  configured for.
- **Empty results fail.** A period with no M1 candles or no M15 bars returns an
  error instead of printing an empty stream, so an accidental empty range is
  visible.
- **Rejected:** adding execution fields to `strategy-run`. That would collapse
  the approved boundary between a strategy decision and a simulated fill.
- **Rejected:** reusing the `market-*` output or adding a `--window` option.
  The command has its own contract and always uses the extended range.
