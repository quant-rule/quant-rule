# Policy Blueprint and Current Run Contracts

- Date: 2026-10-30 (updated 2026-11-07; superseded 2026-11-11 by run v3)
- Status: Historical
- Applies to: run schema `quantrule.run/v2`, policy `v1`, data `v3`, model
  `v4`, execution `v2`, portfolio `v2`
- Related: `docs/adr/2026-10-30/abstraction-levels-and-engine-boundaries.md`

## Run composition

`run.json` names the policy, the four engine files, and the output root:

```json
{
  "schema": "quantrule.run/v2",
  "name": "eurusd-2025",
  "model_id": "M1.0",
  "policy_id": "POL1.0",
  "execution_id": "E1.0",
  "portfolio_id": "P1.0",
  "policy": "policy.json",
  "data": "data.json",
  "model": "model.json",
  "execution": "execution.json",
  "portfolio": "portfolio.json",
  "output_root": "../../../data/labs/eurusd-2025"
}
```

`strategy` and `analytics` are not current run fields. The loader rejects them
with an unknown-field error. The strategy file and the qualification criteria are
explicit command arguments; the run schema stays `quantrule.run/v2`. The identity
fields are optional, but a present identity must carry its type prefix (`M`,
`POL`, `E`, `P`). No strategy or analytics identity is part of the current run;
adding one is a separate decision.

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
    "timeframes": ["5m", "15m"]
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
| `risk.max_position_fraction` | `(0, 1]`; fixed notional must be at most initial account × fraction; at each fill the backtest runner caps the entry notional at current equity × leverage × fraction |
| `risk.max_leverage` | Exactly `1.0` in v1 |
| `risk.max_open_positions` | Exactly `1` in v1 |
| `universe.tickets` | Non-empty, unique, no whitespace; `data.instrument` must be present |
| `universe.timeframes` | Non-empty, unique, ascending; a subset of the data ladder; the execution fill timeframe must be present |

There are deliberately no news, weekend, session, or daily-loss fields.

## Strategy

The ladder-context strategy, its composite weights, its as-of context, and the
retired strategy-schema parsers were removed with the retired-architecture
cutover. The generic bracket and event-composite engines remain in
`quantrule-backtest`. The current shell reaches them through
`strategy explain`, `strategy run`, and `backtest run`.

The strategy is a deterministic rule over the published events, states, confirmed
levels, progression values, and bar identity. It does not restore the old
composite scores. The configuration contract is
[`docs/config/2026-11-06/strategy-config.md`](../2026-11-06/strategy-config.md);
the research process is
[`docs/workflows/strategy-research.md`](../../workflows/strategy-research.md).

## Portfolio

`portfolio.json` uses `quantrule.portfolio/v2` and owns capital and accounting:

| Field | Rule |
|---|---|
| `initial_cash` | Finite, `> 0` |
| `currency` | `USD` |
| `sizing` | `fixed_notional` or `equity_fraction`; positive notional/fraction and `quantity_step` |
| `leverage` | Exactly `1.0` |
| `max_open_positions` | Exactly `1` |
| `commission` | `rate >= 0`, `base` `equity`, `per` `operation` |
| `funding` | `rate` finite, `interval_hours` in the supported set |
| `max_drawdown` | Optional `(0, 1]` limit; action `halt`; checked after an entry, an exit, a funding charge, and a mark |
| `mark_to_market` | `bar_close` |

## Execution

`execution.json` no longer owns the bracket. It owns mechanics and costs:

```json
{
  "schema": "quantrule.execution/v2",
  "fill_timeframe": "5m",
  "order_type": "market_next_open",
  "tie_break": "stop_first",
  "fee_rate": 0.0,
  "slippage_bps": 2.0,
  "zero_volume": "reject",
  "end_of_data": "leave_open"
}
```

Execution receives a `TradeIntent` from the strategy. The stop and target
specifications live in the intent, not in the execution file or the execution
config. Execution resolves them against the fill; it never stores or chooses a
distance. The current shell reaches execution through `execution run` and the
coupled `backtest run`. The research process is
[`docs/workflows/portfolio-research.md`](../../workflows/portfolio-research.md).

## Portfolio accounting

Commission is `rate × max(equity_before_operation, 0)`. `equity_before_operation`
is the last published equity state before the fill: the prior bar's mark when one
exists, or the state left by the previous fill. A gap exit therefore charges on
the prior mark, not on the exit fill price. The execution fee is the fill's own
fee and is never recomputed.

## Data

`data.json` adds `instrument` and uses schema `quantrule.data/v3`:

```json
{
  "schema": "quantrule.data/v3",
  "source": { "kind": "csv", "path": "../../../data/sources/EURUSD_M1_2025-01-01_2025-12-31.csv" },
  "instrument": "EURUSD",
  "data_version": "dukascopy-bid-eurusd-m1-2025",
  "from": "2025-01-01",
  "to": "2025-12-31",
  "source_timeframe": "1m",
  "available_timeframes": ["1m", "5m", "15m", "1h"],
  "max_candles": 600000
}
```

`source_timeframe` is fixed at `1m`. The `available_timeframes` list is
ascending, unique, and includes `1m`. The data engine supports `1m`, `5m`,
`15m`, `1h`, and `4h`; the M1.0 model ceiling is `1h`, so a run with `4h` in its
ladder is rejected. The CSV must be one-minute spaced:
the loader measures every parsed source row before `from`/`to` is applied and
rejects a native M5/M15/H1 file, even when the date bound would retain only one
row. A sparse M1 file with gaps is accepted while at least one adjacent pair is
one minute apart; a single row has no pair and is accepted. The error names the
observed cadence.

## Commands

```bash
quantrule run check --config <run.json>
quantrule report model --config <run.json> --out <dir>
quantrule events <csv-path>
```

This document is historical. The current run contract is
[`docs/config/2026-11-11/run-v3-and-portfolio-constraints.md`](../2026-11-11/run-v3-and-portfolio-constraints.md).
`run check` is the current validation command.
`report model` reads the configured candles and writes the flat bundle.

## Related workflows

- [`policy-change.md`](../../workflows/policy-change.md) — the human-owned
  envelope change process.
- [`strategy-research.md`](../../workflows/strategy-research.md) — the strategy
  tier.
- [`portfolio-research.md`](../../workflows/portfolio-research.md) — the
  execution and portfolio tier.
- [`data-lifecycle.md`](../../workflows/data-lifecycle.md) — source, version,
  partition, and admissibility.
- [`model-change.md`](../../workflows/model-change.md) — indicator and event
  implementation.
