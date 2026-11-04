# AI Transparency Log — v8 Symmetric Direction and Time Stop

- Date: 2026-11-04 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: Phase 0 prerequisite for the five 15m event-family strategy briefs
- Plan: [`docs/execution-plans/2026-11-04/v8-symmetric-time-stop.md`](../../execution-plans/2026-11-04/v8-symmetric-time-stop.md)

## Intent and boundary

The human approved Phase 0 of a five-strategy program. The agent was asked to
implement the shared prerequisite only:

- v8 event conditions with `direction: 0` and side-from-event;
- optional `time_stop_bars` on v8;
- `state: "none"` in the path-validation harness;
- the shared v4 model artifact pack.

The agent did not run a strategy, did not open a validation partition, and did
not claim an edge. The model, execution, portfolio, policy, and analytics
contracts were out of scope.

## Proposed work versus implemented work

| Item | Status |
|---|---|
| Execution plan and this log | Written |
| v8 direction-0 and time-stop implementation | Implemented and verified |
| Path-validation `state: none` | Implemented and verified |
| Model artifact pack | 30/30 cells complete; counts recorded below |
| Strategy runs | Not started; Phase 1 |

## Verification and evidence

Observed on 2026-11-04 in the Phase 0 worktree:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | no output |
| `cargo clippy --all-targets` | no warnings |
| `cargo test` | all tests pass |
| `cargo doc --no-deps` | builds without warnings |
| `python3 -m unittest test_path_validation` (from `tools/improvement`) | 23 tests pass |
| `cargo test -p quantrule-backtest --lib event::tests` | 10 tests pass |
| `cargo test -p quantrule-shell --bin quantrule strategy_v8` | 2 tests pass |
| `cargo test -p quantrule-shell --test cli strategy_explain_prints_the_symmetric_event_rule` | 1 test passes |
| `quantrule policy check --config target/v8-explain/run.json` | `strategy=event-composite`, exit 0 |
| `quantrule backtest` on BTCUSD 2018-01 with the pivot-break v8 config | 56 trades, 29 long and 27 short, from one config |
| The same config with `time_stop_bars: 1` | 72 `time_stop` decisions; maximum hold 2 bars |
| `quantrule report model --config config/model-v4/dev/btc-2018/run-adx25.json` | 19.28 s; `readings.csv` has the v4 ADX/DI/context columns |
| Full model pack | 30/30 cells complete; 9.8 GB; all readings and events present |

The smoke backtest also exposed a Phase 1 config detail: `fill_timeframe` must
equal the strategy execution timeframe. A 15m strategy uses `fill_timeframe:
15m`, not `1m`. The Phase 1 common brief's execution config is corrected
accordingly in the final Phase 0 report.

A second handoff correction: the development gate must run the dev matrix
**without** `--config`, so it evaluates both `adx20` and `adx25` and the
sensitivity check can run. Filtering to `--config adx25` removes the adx20 rows
and forces every primary cell to grade C. The validation gate uses
`--config adx25 --skip-sensitivity --primary-cells 1`.

The full 30-cell pack is complete. The 15m `adx25` counts are:

| Family | Dev 2018–2020 | Validation 2021–2022 |
|---|---:|---:|
| Trend continuation | 4,119 | 3,417 |
| DI cross | 17,878 | 14,849 |
| MACD histogram flip | 60,373 | 48,816 |
| Major pivot break | 14,797 | 12,563 |
| EMA50 test | 33,347 | 27,698 |

The earlier BTCUSD 2018 single-cell check gave trend continuation 454, DI cross
1,699, MACD histogram flip 5,756, major pivot break 1,598, and EMA50 test 2,729,
consistent with the pack.

## Learning and rejected suggestions

- A new v11 schema was considered and rejected: it would duplicate the v8
  condition language. Extending v8 is smaller and preserves one event surface.
- Using v9 for non-compression events was rejected: v9 is the
  compression-breakout contract, and repurposing it would change an existing
  command's meaning.
- Running one-sided v8 configs was rejected: it would make the five briefs
  directional rather than symmetric and would double the multiple-testing
  surface.
- The path-validation `state: none` mode must not skip the lift, cluster,
  random, back-lag, shuffle, or replication tests. It removes only the
  state-conditioned incremental test, which is not applicable to a family with
  no claimed state.
