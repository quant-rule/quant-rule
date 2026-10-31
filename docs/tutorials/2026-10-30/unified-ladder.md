# Unified Ladder Tutorial

- Date: 2026-10-30 (UTC)
- Issue: [#236](https://github.com/quant-rule/quant-rule/issues/236)
- Plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- Strategy contract: [`docs/config/2026-10-30/strategy-v3.md`](../../config/2026-10-30/strategy-v3.md)
- Trade intent: [`docs/config/2026-10-30/trade-intent.md`](../../config/2026-10-30/trade-intent.md)
- Migration: [`docs/config/2026-10-30/migration-v2-to-v3.md`](../../config/2026-10-30/migration-v2-to-v3.md)

## 1. What this tutorial runs

This procedure runs the shell end to end over the checked-in
`crates/quantrule-shell/tests/fixtures/state_m1.csv` source and the tutorial
config under `docs/tutorials/2026-10-30/config-state-fixture/`. It covers:

- the available ladder in `data.json`;
- the active ladder in `strategy.json`;
- `policy check`, `strategy explain`, `strategy-run`, `backtest`, and
  `report strategy`;
- the migration from the retired v2 strategy config.

Paths are relative to the repository root. Run every command from there.

## 2. Available versus active ladder

| Ladder | Owner | Meaning |
|---|---|---|
| Available | `data.json` | The timeframes the data engine can build: `1m`, `5m`, `15m`, `1h`, and `4h`. |
| Active | `strategy.json` | The strategy-owned subset. `ladder[0]` is execution; `ladder[1..]` are context timeframes. |
| Policy universe | `policy.json` | The tickets and timeframes the account is allowed to use. Every active member must be here. |

The tutorial config uses `["5m", "15m"]`: execution on 5m, context on 15m.
`execution.fill_timeframe` must equal `ladder[0]`.

## 3. Inspect the tutorial config

```bash
ls docs/tutorials/2026-10-30/config-state-fixture
```

The eight files are the same shape as the example run. The source points at the
checked-in M1 fixture, and the output root is under `target/`.

## 4. Validate the policy envelope

```bash
cargo run -q -p quantrule-shell -- policy check \
  --config docs/tutorials/2026-10-30/config-state-fixture/run.json
```

Observed output:

```text
policy ok run=tutorial-state-fixture account=USD 10000 target=0.1 365d max_drawdown=0.2 risk=1 leverage=1 positions=1 tickets=SOLUSD timeframes=5m,15m strategy=ladder-context execution=5m ladder=5m,15m stop_atr=2 reward_to_risk=1.5
```

`policy check` reads no candles. It validates the policy, data, model,
strategy, execution, portfolio, and analytics contracts together.

## 5. Read the strategy in trader words

```bash
cargo run -q -p quantrule-shell -- strategy explain \
  --config docs/tutorials/2026-10-30/config-state-fixture/run.json
```

Observed output:

```text
strategy ladder-context
execution 5m
context 15m
thesis Trade with the higher-timeframe regime after a lower-timeframe structure shift.
entry-long direction > 0.1, momentum > 0.1, structure >= 0, shift <= 0 -> >= 1
entry-short direction < -0.1, momentum < -0.1, structure <= 0, shift >= 0 -> <= -1
context-long direction share >= 0.5, momentum bearish share <= 0.5, conflicts <= 1, progression bullish, late/terminal <= 1
context-short direction share >= 0.5, momentum bullish share <= 0.5, conflicts <= 1, progression bearish, late/terminal <= 1
exit context_lost,structure_lost
risk stop=2 ATR target=3 ATR reward_to_risk=1.5
```

The `explain` command renders the structured config; it does not run the model
or invent a rule.

## 6. Walk the strategy

```bash
cargo run -q -p quantrule-shell -- strategy-run \
  crates/quantrule-shell/tests/fixtures/state_m1.csv \
  --strategy ladder-context \
  --data-version tutorial:state:v1 \
  --config docs/tutorials/2026-10-30/config-state-fixture/run.json
```

The command prints one NDJSON entry or exit per decision. The fixture is only
260 M1 rows, so the 15m context never reaches the model's 200-bar Direction
warm-up. The observed output is empty for this tiny source. The command still
exercises the active ladder, the as-of context builder, and the virtual
position walk; a full-year source is needed for entries.

Use `--json` to print one envelope with `strategy`, `strategy_config`,
`data_version`, `requested_period`, `timeframe_secs`, and `records`.

## 7. Run the full backtest

```bash
rm -rf /tmp/quantrule-tutorial
cargo run -q -p quantrule-shell -- backtest \
  --config docs/tutorials/2026-10-30/config-state-fixture/run.json \
  --out /tmp/quantrule-tutorial
```

The command writes five bundles and prints their paths:

```text
/tmp/quantrule-tutorial/model
/tmp/quantrule-tutorial/strategy
/tmp/quantrule-tutorial/execution
/tmp/quantrule-tutorial/portfolio
/tmp/quantrule-tutorial/analytical
```

`backtest` loads the source once, computes the model once, runs the strategy,
fills on the execution timeframe, applies the portfolio accounting, and writes
the reports from the same run. The strategy bundle uses the executed walk and
marks `execution_state` as `executed`.

## 8. Read the executed strategy report

```bash
cargo run -q -p quantrule-shell -- report strategy \
  --config docs/tutorials/2026-10-30/config-state-fixture/run.json \
  --out /tmp/quantrule-tutorial-report
```

Observed summary on the tiny fixture:

```text
quantrule strategy report
data_version: tutorial:state:v1
requested_period: 2025-01-01..2025-01-01
timeframe_secs: 300
ladder: 5m,15m
bars: 52
execution_state: virtual
executed_entries: -
executed_exits: -
decisions: 0
entries: 0
exits: 0
long_entries: 0
short_entries: 0
exit_reasons:
  context_lost: 0
  structure_lost: 0
context:
  complete_bars: 50
  missing_bars: 2
  conflict_bars: 0
  late_or_terminal_bars: 3
open_position: flat
```

The standalone `report strategy` is the virtual walk
(`execution_state: virtual`). The `strategy` bundle from `backtest` is the
executed walk (`execution_state: executed`) and includes `executed_entries` and
`executed_exits`.

## 9. What the reports contain

| Bundle | Contents |
|---|---|
| `model` | Per-timeframe candles, readings, four axes, events, and descriptive report views. |
| `strategy` | Execution snapshot, as-of context counts, decisions, virtual or executed state, entry/exit counts, and context/structure exit reasons. |
| `execution` | Orders, fills, rejections, slippage, and fees; no cash or P&L. |
| `portfolio` | Cash, equity, trades, costs, funding, drawdown, and mark-to-market. |
| `analytical` | Descriptive across-timeframe views; never a strategy input. |

The strategy report carries `context_lost` and `structure_lost`. A portfolio
trade closed by a strategy decision carries `strategy`; a bracket close carries
`stop` or `target`.

## 10. Migrate a v2 strategy config

A `quantrule.strategy/v2` file fails with:

```text
quantrule.strategy/v2 is retired; use quantrule.strategy/v3 with rule "ladder-context"
```

The old `axis-agreement` rule is gone. The replacement declares an active
ladder, context weights, execution and context conditions, exit reasons, and the
risk intent. See
[`docs/config/2026-10-30/migration-v2-to-v3.md`](../../config/2026-10-30/migration-v2-to-v3.md)
for the field-by-field mapping.

## 11. What this tutorial proves, and what it does not

It proves that the shell reaches the unified strategy and writes deterministic
reports from one run. It does not prove an edge: the fixture is too small for a
decision, and no performance claim is made here. The experiment record in
Phase 9 is the separate evaluation artifact.

## 12. Role labs

The role manuals are the current procedures:

- [Quant Researcher](../../roles/2026-10-29/quant-researcher-manual.md)
- [Quant Developer](../../roles/2026-10-29/quant-developer-manual.md)
- [Quant Validator](../../roles/2026-10-29/quant-validator-manual.md)

The 2026-10-29 full-pipeline tutorial is preserved as a dated record for the
retired rule. Use this tutorial for the current command surface.
