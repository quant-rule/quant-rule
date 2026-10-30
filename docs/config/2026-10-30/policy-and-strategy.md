# Policy and Strategy Blueprints

- Date: 2026-10-30
- Applies to: run schema `quantrule.run/v2`, policy `v1`, strategy `v2`,
  execution `v2`
- Related: `docs/adr/2026-10-30/abstraction-levels-and-engine-boundaries.md`

## Run composition

`run.json` names the policy, the six engine files, and the output root:

```json
{
  "schema": "quantrule.run/v2",
  "name": "eurusd-2025",
  "policy": "policy.json",
  "data": "data.json",
  "model": "model.json",
  "strategy": "strategy.json",
  "execution": "execution.json",
  "portfolio": "portfolio.json",
  "analytics": "analytics.json",
  "output_root": "../../../data/backtest-eurusd-2025"
}
```

## Policy

```json
{
  "schema": "quantrule.policy/v1",
  "account": { "currency": "USD", "initial_cash": 10000.0 },
  "objective": {
    "performance_target": 0.10,
    "evaluation_days": 365,
    "max_drawdown": 0.20
  },
  "risk": {
    "max_position_fraction": 1.0,
    "max_leverage": 1.0,
    "max_open_positions": 1
  },
  "universe": {
    "tickets": ["EURUSD"],
    "timeframes": ["15m"]
  }
}
```

| Field | Rule |
|---|---|
| `account.currency` | Exactly `USD` in v1 |
| `account.initial_cash` | Finite, `> 0`; must equal `portfolio.initial_cash` |
| `objective.performance_target` | Finite, `> 0`; evaluation target, never a signal |
| `objective.evaluation_days` | Integer `1..=3650` |
| `objective.max_drawdown` | `(0, 1]`; portfolio limit must be at most this and must halt |
| `risk.max_position_fraction` | `(0, 1]`; fixed notional must be at most account × fraction; equity fraction must be at most this |
| `risk.max_leverage` | Exactly `1.0` in v1 |
| `risk.max_open_positions` | Exactly `1` in v1 |
| `universe.tickets` | Non-empty, unique, no whitespace; `data.instrument` must be present |
| `universe.timeframes` | Non-empty, unique, ascending; subset of the data ladder; strategy timeframe must be present |

There are deliberately no news, weekend, session, or daily-loss fields.

## Strategy

```json
{
  "schema": "quantrule.strategy/v2",
  "name": "axis-agreement",
  "thesis": "Trade only when direction, momentum, and structure agree; exit when they disagree.",
  "timeframe": "15m",
  "long_threshold": 0.1,
  "short_threshold": -0.1,
  "stop_atr_multiple": 2.0,
  "reward_to_risk": 1.5
}
```

| Field | Rule |
|---|---|
| `name` | Exactly `axis-agreement` in v2 |
| `thesis` | Non-empty human explanation; never executable |
| `timeframe` | A ladder member and a policy timeframe |
| `long_threshold` | Finite, `>= 0`; greater than `short_threshold` |
| `short_threshold` | Finite, `<= 0`; less than `long_threshold` |
| `stop_atr_multiple` | Finite, `> 0` |
| `reward_to_risk` | Finite, `> 0`; target distance is stop × reward |

The strategy receives only the three primary axis values and bar identity. It
cannot read percentiles, readiness, events, context, execution, portfolio, or
policy.

`strategy explain --config <run.json>` renders the rule in trader words.

## Execution

`execution.json` no longer owns the bracket. It owns mechanics and costs:

```json
{
  "schema": "quantrule.execution/v2",
  "fill_timeframe": "15m",
  "order_type": "market_next_open",
  "tie_break": "stop_first",
  "fee_rate": 0.0,
  "slippage_bps": 2.0,
  "zero_volume": "reject",
  "end_of_data": "leave_open"
}
```

The engine receives `stop_atr_multiple` and `target_atr_multiple` from the
strategy risk block; they are not duplicated in the execution file.

## Data

`data.json` adds `instrument` and uses schema `quantrule.data/v2`:

```json
{
  "schema": "quantrule.data/v2",
  "source": { "kind": "csv", "path": "../../../data/EURUSD_M1_2025-01-01_2025-12-31.csv" },
  "instrument": "EURUSD",
  "data_version": "dukascopy-bid-eurusd-m1-2025",
  "from": "2025-01-01",
  "to": "2025-12-31",
  "source_timeframe": "1m",
  "ladder": ["1m", "5m", "15m", "1h"],
  "max_candles": 600000
}
```

## Commands

```bash
quantrule policy check --config <run.json>
quantrule strategy explain --config <run.json>
quantrule backtest --config <run.json>
```

`policy check` and `strategy explain` are read-only: they read no candles and
write no artifacts.
