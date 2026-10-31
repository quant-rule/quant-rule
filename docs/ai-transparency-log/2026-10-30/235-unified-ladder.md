# AI Transparency Log — Unified Ladder Strategy

- Date: 2026-10-30 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#235](https://github.com/quant-rule/quant-rule/issues/235)
- Parent plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- Status: Implemented; merged in PR [#244](https://github.com/quant-rule/quant-rule/pull/244) as `5c6ad59`

## Intent and boundary

Phase 7 replaces the retired `axis-agreement` rule with the unified
`ladder-context` strategy. The strategy selects the active ladder, builds the
as-of higher-timeframe context, applies the execution and context conditions,
and emits the `TradeIntent` introduced in Phase 6. Execution still owns fill
mechanics; the strategy remains stateless.

The old rule is removed from the public config and command surface. Its golden
fixture and a retirement ADR are archived as evidence, not as live tests.

## Implemented

- Added `LadderContextStrategy` and the v3 strategy configuration:
  active ladder, per-axis context weights, execution conditions, context
  conditions, exit reasons, risk policy, and no-trade policy.
- Added strict v3 parsing and a migration error for `quantrule.strategy/v2`.
- Added `ExecutionEntryConfig`, `ContextEntryConfig`, `EntryConfig`, and
  `ExitConfig` with validation.
- Expanded `StrategyInput` with the execution Structure state and previous
  state, published Progression fields, and a contiguity flag.
- Wired the streaming `LadderContextBuilder` into the backtest runner and the
  virtual strategy walk.
- Implemented long and short mirroring, context loss, and structure loss.
- Removed `Agreement`, `AxisAgreementStrategy`, the old golden test, and the
  live `axis-agreement` command/config path.
- Changed the live rule name to `ladder-context` in `strategy-run`, `policy
  check`, `strategy explain`, reports, help, and example configs.
- Changed the portfolio's strategy-close label from `agreement_lost` to
  `strategy`; the strategy report carries `context_lost` and
  `structure_lost`.
- Updated the strategy report summary with context-complete, missing, conflict,
  and late/terminal bar counts.
- Archived the old golden fixture at
  `docs/adr/2026-10-30/axis-agreement-golden.csv` and added
  `docs/adr/2026-10-30/axis-agreement-retirement.md`.
- Added `crates/quantrule-backtest/tests/ladder_context_strategy_golden.csv`
  and its golden test.
- Regenerated the runner golden decisions, trades, and metrics for the new rule.
- Regenerated the `strategy-run` golden NDJSON.
- Updated config examples, policy universe timeframes, execution fill
  timeframe, contracts, README, and the plan.

## Verification and evidence

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | Passed; 45 suites, 0 failures |
| `cargo doc --no-deps` with `RUSTDOCFLAGS=-D warnings` | Passed |
| Backtest unit tests | 165 passed, 1 ignored benchmark |
| Shell unit tests | 272 passed |
| New strategy golden | 3 tests passed |
| PR | #244 opened |
| CI `verify` and `sync` | Passed |
| Runner golden | Decisions, fills, trades, and metrics regenerated and passing |
| `strategy-run` golden | Regenerated and passing |
| `backtest` five bundles | Written and matching standalone reports |
| `strategy explain` | Prints the unified rule |
| v2 migration | Rejected with a migration message |

## Deviations and decisions

- The current loader still reads `quantrule.execution/v2`. The approved
  `quantrule.execution/v3` target also contains fields this delivery does not
  use (`bracket_order_type`, `max_slippage_bps`, `partial_fills`, and the
  `close_at_last` end policy). Phase 7 only checks that
  `execution.fill_timeframe` equals `ladder[0]`. Implementing v3 would be a
  separate schema change with no behavior behind several fields.
- `StrategyConfig::default()` keeps the plan's strict context limits
  (`conflict_count_max = 0`, `late_or_terminal_count_max = 0`). The example
  configs use 1/1 so the short synthetic fixtures exercise the rule; the
  strategy unit and golden tests use the strict defaults.
- The portfolio exit reason for a strategy close is now `strategy`, because the
  portfolio sees only a market exit fill and cannot know whether the strategy
  exited on context or structure. The strategy report carries the specific
  reason.
- The strategy-facing position type remains `StrategyPosition`, as in Phase 6.
- `strategy-run` with `--config` now builds the model over the active ladder;
  without a config it uses the 5m/15m control ladder.

## Quant validation (sequential pass)

The sequential pass checked:

- a long entry requires every execution and context condition;
- a short entry mirrors every condition;
- a missing execution value or a missing Structure state blocks entry;
- a missing context, a conflict, or a late leg blocks entry;
- a non-contiguous execution bar blocks entry;
- context loss exits before structure loss when both apply;
- structure loss exits only on a measured flip against the position;
- a missing Structure state does not count as a loss;
- the rule never reverses on the exit bar;
- repeated runs and a prefix run are deterministic and causal;
- the v2 config is rejected with a migration message;
- the old rule is gone from the live command and config surface;
- the runner's strategy position matches the portfolio position.

This is a same-agent pass, not an independent agent or model judgment. No
profitability claim is made.

## Learning and rejected suggestions

- Rejected keeping `axis-agreement` behind a compatibility shim. It would keep
  two decision paths alive.
- Rejected auto-migrating v2 configs. The old flat fields do not express an
  active ladder or context weights.
- Rejected using the execution config to supply context thresholds. The
  strategy owns its rule.
- Rejected averaging Structure or Progression codes. The context counts and
  histograms remain ordinal.
- Rejected making the strategy infer its position from its decisions. Execution
  still reports the actual position.

## Residual risk

- The example configs use relaxed context limits; the strict default may trade
  rarely on short samples.
- The execution v3 schema is not implemented.
- The strategy report and portfolio trade reason changed; historical reports
  still use `agreement_lost` and are dated records.
- The synthetic runner and strategy-run fixtures are deterministic but small;
  they do not establish an edge.
