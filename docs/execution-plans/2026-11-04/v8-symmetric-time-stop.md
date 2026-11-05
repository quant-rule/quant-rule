# v8 Symmetric Direction and Time Stop — Phase 0

- Date: 2026-11-04 (UTC)
- Status: **Verified** (human approved Phase 0 on 2026-11-04; acceptance criteria
  evaluated against the observed checks and the completed model pack)
- Branch: `feat/v8-symmetric-time-stop`
- Issue: none; the human approved the Phase 0 prerequisite directly
- Claim type: strategy-contract and evaluation-tooling change; no strategy or
  edge claim
- Quant roles: Quant Developer (implementation), Test Engineer (verification),
  Code Reviewer (same-session review); no Quant Validator record is required
  because this change does not alter model output or a strategy result

## 1. Intent

Make the five pre-registered 15m event-family briefs runnable as config-only
strategies in five independent sessions.

The v8 event-composite rule is the only generic event-driven strategy surface,
but it has two gaps:

1. every entry event must name a fixed direction, so one config cannot trade
   both sides of a symmetric event; and
2. it has no time stop, so a position can remain open indefinitely when neither
   the ATR stop nor the reward target is touched.

The path-validation harness has a third gap: its grade requires a
state-conditioned baseline, but three new event families do not claim a
published state.

## 2. Current state

- v8 is implemented in `crates/quantrule-backtest/src/strategy/event.rs` and
  parsed in `crates/quantrule-shell/src/config/strategy.rs`.
- `EventCompositeConfig::new` rejects `direction: 0` and statically rejects
  conflicting entry directions.
- `EventCompositeStrategy::decide` derives the side from a fixed `direction`.
- The v8 config has no `time_stop_bars` field; v9 and v10 do.
- `tools/improvement/path_validation.py` grades every family against a
  state-conditioned baseline; `state_missing` must be zero and the incremental
  lift must pass.
- `data/model-v4/` does not exist, so the path-validation matrix has no model
  artifacts to read.

## 3. Scope

### Included

- v8 entry event conditions may use `direction: 0`; the entry side is the
  direction of the matched event. Exact `-1` and `+1` still filter. If matched
  direction-0 events disagree, the bar produces no entry.
- v8 accepts optional `time_stop_bars`; a positive value exits at the next open
  after that many execution bars held. Zero or absence keeps the old behavior.
- The v8 `strategy explain` output prints the time stop.
- `path_validation.py` supports `"state": "none"`: the state-conditioned
  incremental test is not applicable, and the gate grades on the unconditional
  lift, cluster-robust LCB, random control, back-lag control, shuffle control,
  and instrument replication.
- Generate the shared v4 model artifact pack for `model-adx20.json` and
  `model-adx25.json` on `model_dev` (2018-2020) and `model_validation`
  (2021-2022) for BTCUSD, ETHUSD, and XAUUSD.

### Excluded

- No model, indicator, or event definition changes.
- No new strategy schema; v8 remains the schema.
- No execution, portfolio, policy, or analytics changes.
- No strategy run or strategy claim in this work item.
- No 4h timeframe, no statistics or machine learning in the signal path.

## 4. Assumptions and uncertainty

- The five briefs need a symmetric v8 entry and a time stop. If the extension
  changes existing v8 behavior, the change is wrong.
- Existing v8 configs omit `time_stop_bars`; absence must mean zero.
- Existing v8 configs use exact `±1`; the static conflict check must still
  reject two conflicting exact directions at parse time.
- The path-validation `state: none` mode is a methodology extension, not a
  relaxation of the lift, cluster, random, back-lag, shuffle, or replication
  tests. It must be reviewed as such.
- The model pack is local data under `data/`, which is ignored by Git. The five
  sessions need a shared or per-worktree copy; this plan generates it in the
  worktree and records the commands so it can be reproduced.

## 5. Decisions

| Decision | Options | Consequence | Approval |
|---|---|---|---|
| Where symmetry lives | new v11 schema / extend v8 | v8 is the generic event rule and already reads events, states, and bar identity; extending it avoids a second condition language | Human approved Phase 0 |
| Direction-0 semantics | wildcard match + side from event | one config trades both sides; conflicting matches are skipped | Human approved Phase 0 |
| Time stop | optional `time_stop_bars` on v8 | backward compatible; matches v9/v10 semantics | Human approved Phase 0 |
| No-state gate | `state: none` skips only the state incremental test | three new families can be gated without inventing a state claim | Human approved Phase 0 |
| Model pack | run the existing v4 matrix once | path validation reads the same frozen model artifacts as the strategies | Human approved Phase 0 |

## 6. Acceptance criteria

- `cargo fmt --check` passes.
- `cargo clippy --all-targets` is warning-free.
- `cargo test` passes.
- `cargo doc` builds without warnings.
- Python path-validation tests pass.
- A v8 config with `direction: 0` enters long on a bullish matched event and
  short on a bearish matched event, using one configuration.
- Two direction-0 matches with opposite signs produce no entry.
- An exact `direction: 1` still ignores a bearish event.
- `time_stop_bars: 3` exits a position held for three execution bars with
  `TimeStop`; omitted/zero never times out.
- `strategy explain` prints `time_stop_bars`.
- `policy check` accepts the extended v8 config.
- `path_validation.py` with `state: none` grades a family on the unconditional
  tests and reports the state columns as not applicable.
- The model pack exists for all 30 cells and the target 15m event families have
  usable counts.

## 7. Work breakdown

| Step | Work | Paths |
|---|---|---|
| 1 | Write this plan and the AI transparency log | `docs/execution-plans/2026-11-04/`, `docs/ai-transparency-log/2026-11-04/` |
| 2 | Extend `EventCompositeConfig` and `EventCompositeStrategy` | `crates/quantrule-backtest/src/strategy/event.rs` |
| 3 | Parse optional `time_stop_bars` and allow direction 0 | `crates/quantrule-shell/src/config/strategy.rs` |
| 4 | Print the time stop in `strategy explain` | `crates/quantrule-shell/src/command/strategy_explain.rs` |
| 5 | Add unit, runner, config, and shell tests | `crates/quantrule-backtest/src/strategy/event.rs`, `crates/quantrule-backtest/src/runner/tests.rs`, `crates/quantrule-shell/src/config/tests.rs` |
| 6 | Update the v8 contract | `docs/config/2026-11-03/strategy-v8.md` (removed in #463) |
| 7 | Add `state: none` to the path harness and tests | `tools/improvement/path_validation.py`, `tools/improvement/test_path_validation.py` |
| 8 | Document the no-state gate | `docs/model/2026-11-04/path-validation-v4.md` |
| 9 | Generate and verify the model pack | `data/model-v4/` (ignored local data) |
| 10 | Run the full verification set and update this plan | this file |

## 8. Verification

| Check | Command | Observed |
|---|---|---|
| Format | `cargo fmt --check` | no output |
| Lints | `cargo clippy --all-targets` | no warnings |
| Rust tests | `cargo test` | all pass |
| Docs | `cargo doc --no-deps` | no warnings |
| Python tests | `python3 -m unittest tools/improvement/test_path_validation.py` | all pass |
| Model pack | `ls data/model-v4/{dev,validation}/*/model/readings.csv` | 30/30 cells complete; 9.8 GB; all readings and events present |
| Event counts | count the five target families at 15m | all families above 3,400 dev and 3,400 validation events; table below |
| Explain | `target/release/quantrule strategy explain --config <fixture>` | prints `time_stop_bars` and `direction=either` |
| Policy check | `target/release/quantrule policy check --config <fixture>` | `strategy=event-composite` |
| Smoke backtest | `quantrule backtest` on BTCUSD 2018-01 with the pivot-break v8 config | completes; 56 trades, 29 long and 27 short, from one config |
| Time-stop smoke | the same config with `time_stop_bars: 1` | 72 `time_stop` decisions; maximum hold 2 bars |

### Model-pack event counts (15m, `adx25`)

| Family | Dev 2018–2020 | Validation 2021–2022 |
|---|---:|---:|
| Trend continuation | 4,119 | 3,417 |
| DI cross | 17,878 | 14,849 |
| MACD histogram flip | 60,373 | 48,816 |
| Major pivot break | 14,797 | 12,563 |
| EMA50 test | 33,347 | 27,698 |

The smoke backtest is a wiring test, not a strategy result. It used
`fill_timeframe: 15m`, which must match the execution timeframe; a 15m strategy
cannot use `fill_timeframe: 1m` in the current execution contract.

### Phase 1 handoff corrections

- **Development gate:** run `path_validation.py` on the dev matrix **without**
  `--config`. It must evaluate both `adx20` and `adx25`; `adx25` is the primary
  and `adx20` is the sensitivity check. Filtering to `--config adx25` removes
  the sensitivity row and forces every primary cell to grade C.
- **Validation gate:** use `--config adx25 --skip-sensitivity --primary-cells 1`
  because the validation is the one-shot confirmation of the frozen cell.
- **Execution config:** `fill_timeframe` must equal the strategy execution
  timeframe. The five 15m briefs use `"15m"`.
- **Model pack and sources:** each Phase 1 worktree should symlink the shared
  pack and the three source CSVs, because the matrix and data paths are
  relative to the worktree:

```bash
ln -s /home/zantgo/Proyectos/quant-rule/data/model-v4 data/model-v4
for instrument in BTCUSD ETHUSD XAUUSD; do
  ln -s "/home/zantgo/Proyectos/quant-rule/data/sources/${instrument}_M1_2018-01-01_2024-12-31.csv" \
    "data/sources/${instrument}_M1_2018-01-01_2024-12-31.csv"
done
```

The shared pack is generated in the main checkout at
`/home/zantgo/Proyectos/quant-rule/data/model-v4` because `data/` is local and
ignored by Git. A Phase 1 worktree that needs the gate should symlink its
`data/model-v4` to the main checkout's pack, or copy it; the matrix paths are
relative to the worktree.

Limitations: the model pack is local ignored data, not committed. The five
strategy briefs and their runs are Phase 1, not this work item. The same-session
review is not an independent Quant Validator record.

## 9. Risks and recovery

- **Behavior regression in v8.** The exact-direction path and the no-time-stop
  path must be unchanged. Recovery: revert the commit; no data migration.
- **Ambiguous direction-0 matches.** The rule skips the bar rather than guessing.
  This is conservative and deterministic.
- **Time-stop off-by-one.** The check is `bars_held >= time_stop_bars`, matching
  v9/v10. A test pins it.
- **Gate relaxation.** `state: none` must not skip the lift/LCB/control checks.
  A test asserts the grade still fails when the lift is too small.
- **Model pack mismatch.** The matrix paths and run output roots must agree. A
  cell check verifies one path before the full pack is used.

## 10. Stop conditions

- A v8 behavior change outside the two approved extensions.
- A test failure that cannot be fixed without weakening the test.
- A missing model config or data source.
- A need to change the model, execution, portfolio, or policy contract.
- A request to run a strategy matrix or open a validation partition.
- A request to commit, push, open a PR, or merge without explicit human
  authorization.
