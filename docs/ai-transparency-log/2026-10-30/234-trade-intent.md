# AI Transparency Log — Trade Intent Boundary

- Date: 2026-10-30 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#234](https://github.com/quant-rule/quant-rule/issues/234)
- Parent plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- Status: Implemented; PR pending

## Intent and boundary

Phase 6 introduces the typed boundary between the strategy and execution. The
strategy owns direction and risk intent; execution owns fill mechanics and
resolves the risk specification. The strategy stays stateless: execution
reports the position it actually holds, and a rejected entry cannot become a
virtual strategy position.

This phase does not introduce the unified ladder rule. The existing
`axis-agreement` rule remains and keeps its default 2-ATR stop and 1.5
reward-to-risk target, so the refactor is behaviorally safe.

## Implemented

- Added `TradeIntent`, `EntryStyle`, `StopSpec`, and `TargetSpec`.
- Added `StrategyPosition` (`side`, `entry_bar`, `bars_held`) as the
  strategy-facing position. The execution `PositionState` enum remains
  execution-internal.
- Added `RiskPolicy`; the strategy builds each entry intent from it.
- Removed the stop and target multiples from `ExecutionConfig`; it now holds
  costs only.
- Extended `OrderIntent::Enter` to carry the `TradeIntent`.
- Moved bracket resolution into execution:
  - ATR stop: `multiple * ATR(decision_bar)`.
  - Absolute stop: the stated price.
  - Reward-to-risk target: the stop distance times the multiple.
  - Absolute target: the stated price.
  - An ATR is required only for an ATR stop.
- Added execution rejection reasons: `UnsupportedEntryStyle`, `ExpiredIntent`,
  and `SlippageLimit`.
- Added `ExecutionEngine::position_view()` returning `StrategyPosition`.
- Updated the runner to pass the actual execution position and to map entry
  intents into `OrderIntent`.
- Updated the virtual diagnostic walk to track `StrategyPosition`.
- Updated the shell strategy config, execution config, pipeline, commands, and
  reports.
- Updated the trade-intent, execution, strategy, plan, and index documents.
- Split the execution batch records, engine errors, and bracket resolution into
  `batch.rs`, `error.rs`, and `bracket.rs`; the engine stays one cohesive
  simulator and is now under 1,000 lines.

## Verification and evidence

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | Passed; 45 suites, 0 failures |
| `cargo doc --no-deps` | Passed |
| Backtest unit tests | 184 passed, 1 ignored benchmark |
| Backtest doctests | 41 passed |
| Shell unit tests | 274 passed |
| `strategy-run` golden NDJSON | Unchanged and passing |
| `backtest` five bundles | Written and matching standalone reports |
| Default risk policy | 2-ATR stop, 1.5 reward-to-risk target |
| `AUD-02` regression guard | Rejected short never becomes a strategy position |
| Final position | Strategy position matches the portfolio position |
| `OrderIntent` boundary test | Execution does not import the strategy module |
| Strategy boundary test | Strategy does not import execution or portfolio |

## Deviations and decisions

- The approved contract names the strategy-facing type `PositionState`, but
  `execution::PositionState` already exists as an internal enum with the same
  name. The strategy-facing type is named `StrategyPosition` to avoid a name
  collision and to make clear that it carries no stop, target, or quantity. The
  contract document now records this name.
- `Decision::Enter` carries an `Entry { bar, intent }` record rather than a bare
  `TradeIntent`. The bar identity is needed by the reports and by execution's
  out-of-order submission check. The entry contains no fill data.
- The shell still uses the v2 strategy and execution schemas. Phase 7 changes
  the strategy schema and removes `axis-agreement`; Phase 6 only moves the risk
  distances out of execution.
- `valid_until` is interpreted as the last submission bar. An intent is never
  silently deferred to a later bar.
- `max_slippage_bps` is checked against execution's configured slippage when
  the entry is submitted. Execution does not change its configured costs.

## Quant validation (sequential pass)

The sequential pass checked:

- the default policy produces the same 2-ATR/1.5-R:R bracket as before;
- the runner's golden decisions, fills, trades, and metrics are unchanged;
- a rejected entry leaves the engine and the strategy position flat;
- a bracket exit is applied before the next strategy decision;
- ATR and absolute stop specs resolve on the correct side;
- an absolute target with an ATR stop uses the decision-bar ATR;
- an absolute stop with a reward-to-risk target needs no ATR;
- unsupported, expired, and over-slippage intents are rejected;
- the strategy receives no cash, equity, P&L, or execution config.

This is a same-agent pass, not an independent agent or model judgment. No
profitability claim is made.

## Learning and rejected suggestions

- Rejected a single ATR requirement for every entry. An absolute stop and
  absolute target do not need an ATR; requiring one would reject valid intents
  for an irrelevant reason.
- Rejected letting the strategy infer its position from its own decisions. That
  is the AUD-02 failure mode; execution must report the position.
- Rejected storing stop and target multiples in `ExecutionConfig`. That would
  keep execution as the risk owner.
- Rejected a second order path for intents. The existing `OrderIntent` path was
  extended so there is one deterministic order lifecycle.
- Rejected silently renormalizing or defaulting an invalid stop or target. An
  invalid specification is a rejection.

## Residual risk

- The unified ladder strategy is not implemented yet; `axis-agreement` is still
  the only rule.
- The strategy does not yet attach `valid_until` or `max_slippage_bps` to its
  intents. Execution supports both; Phase 7 may use them.
- The shell still parses the v2 config schemas.
- The `Entry` wrapper adds a strategy record around `TradeIntent`; the contract
  was updated to match.
