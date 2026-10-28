# Per-Engine Configuration — MVP v1

- Date: 2026-10-28 (UTC)
- Issue: [#168](https://github.com/quant-rule/quant-rule/issues/168)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Status: Approved and implemented by the config loader (#170)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md)
- ADR: [`docs/adr/2026-10-28/config-driven-engines-and-reports.md`](../../adr/2026-10-28/config-driven-engines-and-reports.md)
- Schema version: `quantrule.*/v1`

## 1. Purpose

The configuration is the specification of a run. One file configures each engine,
and `run.json` names the six files. The command line carries only the config path,
an optional output override, and an optional JSON output flag.

The loader validates the whole run before reading candles or writing output. A
config error therefore leaves the filesystem untouched.

## 2. Layout and resolution

```text
config/examples/solusd-2025/
  run.json
  data.json
  model.json
  strategy.json
  execution.json
  portfolio.json
  analytics.json
```

| Rule | Behavior |
|---|---|
| Reference resolution | A path in `run.json` resolves against the directory containing `run.json` |
| Source resolution | `data.json.source.path` resolves against the directory containing `data.json` |
| Output resolution | `run.json.output_root` resolves against the directory containing `run.json` |
| Absolute paths | Accepted; they are not copied into a report |
| `~` | Not expanded |
| Environment variables | Not expanded |
| Missing file | `ConfigError::MissingFile`, naming the resolved path |

The example uses `../../../data/...` because the config lives three directories
below the repository root.

## 3. Common rules

- The top level of every file is a JSON object.
- Every file has a `schema` field with the exact string in this document.
- Unknown fields are rejected at every object level.
- A required field that is absent is rejected; it is never defaulted silently.
- Numbers are JSON numbers, finite, and in the stated range. A numeric string is
  rejected.
- Booleans are JSON booleans. `"true"` is rejected.
- Strings are non-empty where the field requires a value.
- `null` is accepted only where a field says so.
- An enum value is matched exactly and case-sensitively.
- Duplicate keys are not part of the contract; a standard JSON parser keeps the
  last value. Authors must not rely on duplicates.
- `from` and `to` are inclusive UTC dates in `YYYY-MM-DD` form.

## 4. `run.json`

```json
{
  "schema": "quantrule.run/v1",
  "name": "solusd-2025",
  "data": "data.json",
  "model": "model.json",
  "strategy": "strategy.json",
  "execution": "execution.json",
  "portfolio": "portfolio.json",
  "analytics": "analytics.json",
  "output_root": "../../../data/backtest-solusd-2025"
}
```

| Field | Type | Required | Rule |
|---|---|---|---|
| `schema` | string | yes | Exactly `quantrule.run/v1` |
| `name` | string | yes | Non-empty; recorded in provenance |
| `data` | string | yes | Non-empty path to `data.json` |
| `model` | string | yes | Non-empty path to `model.json` |
| `strategy` | string | yes | Non-empty path to `strategy.json` |
| `execution` | string | yes | Non-empty path to `execution.json` |
| `portfolio` | string | yes | Non-empty path to `portfolio.json` |
| `analytics` | string | yes | Non-empty path to `analytics.json` |
| `output_root` | string | yes | Non-empty path; the run root |

`run.json` is the composition root. It does not repeat engine values.

## 5. `data.json`

```json
{
  "schema": "quantrule.data/v1",
  "source": {
    "kind": "csv",
    "path": "../../../data/SOLUSD_M1_2025-01-01_2025-12-31.csv"
  },
  "data_version": "dukascopy-bid-solusd-m1-2025-01-01_2025-12-31",
  "from": "2025-01-01",
  "to": "2025-12-31",
  "source_timeframe": "1m",
  "ladder": ["1m", "5m", "15m", "1h"],
  "max_candles": 600000
}
```

| Field | Type | Required | Rule |
|---|---|---|---|
| `schema` | string | yes | Exactly `quantrule.data/v1` |
| `source` | object | yes | Only `kind` and `path` |
| `source.kind` | string | yes | Exactly `csv` in v1 |
| `source.path` | string | yes | Non-empty path to the M1 CSV |
| `data_version` | string | yes | Non-empty identity of the input series |
| `from` | string or null | yes | Inclusive UTC date; null means no lower bound |
| `to` | string or null | yes | Inclusive UTC date; null means no upper bound |
| `source_timeframe` | string | yes | Exactly `1m` |
| `ladder` | array of strings | yes | Exactly `["1m","5m","15m","1h"]`, in that order |
| `max_candles` | integer | yes | `1..=600000` |

The ladder has one owner: `data.json`. The model and strategy configs refer to a
member of it; they never restate it.

`max_candles` is a refusal bound. The data engine rejects an input that would
exceed it rather than truncating. The existing extended cap of 600,000 remains the
maximum.

No pre-range warm-up is applied. The model sees only bars inside the requested
range. A higher-timeframe bucket that begins before `from` is built from the M1
rows that survive the range filter, following the existing loader semantics.

## 6. `model.json`

```json
{
  "schema": "quantrule.model/v1",
  "structure_scale": "major"
}
```

| Field | Type | Required | Rule |
|---|---|---|---|
| `schema` | string | yes | Exactly `quantrule.model/v1` |
| `structure_scale` | string | yes | `major` or `minor` |

The model's EMA(20/50/200), RSI(14), ATR(14), RVOL(20), percentile windows, axis
formulas, event rules, and aggregation rules are fixed by the model contract.
They are not tunable in v1. Changing one is a model-contract change, not a config
change.

The model uses the ladder from `data.json`; the two configs cannot disagree.

## 7. `strategy.json`

```json
{
  "schema": "quantrule.strategy/v1",
  "name": "axis-agreement",
  "timeframe": "15m",
  "long_threshold": 0.1,
  "short_threshold": -0.1
}
```

| Field | Type | Required | Rule |
|---|---|---|---|
| `schema` | string | yes | Exactly `quantrule.strategy/v1` |
| `name` | string | yes | Exactly `axis-agreement` in v1 |
| `timeframe` | string | yes | A member of the data ladder |
| `long_threshold` | number | yes | Finite, `>= 0` |
| `short_threshold` | number | yes | Finite, `<= 0` |
| `long_threshold > short_threshold` | invariant | yes | Otherwise the config is rejected |

Equality at a threshold is neutral. A missing reading is unavailable, not
neutral. The strategy emits at most one decision per closed bar and never
reverses on the same bar.

## 8. `execution.json`

```json
{
  "schema": "quantrule.execution/v1",
  "fill_timeframe": "15m",
  "order_type": "market_next_open",
  "stop_atr_multiple": 2.0,
  "target_atr_multiple": 3.0,
  "tie_break": "stop_first",
  "fee_rate": 0.0,
  "slippage_bps": 2.0,
  "zero_volume": "reject",
  "end_of_data": "leave_open"
}
```

| Field | Type | Required | Rule |
|---|---|---|---|
| `schema` | string | yes | Exactly `quantrule.execution/v1` |
| `fill_timeframe` | string | yes | A ladder member; must equal the strategy timeframe |
| `order_type` | string | yes | Exactly `market_next_open` |
| `stop_atr_multiple` | number | yes | Finite, `> 0` |
| `target_atr_multiple` | number | yes | Finite, `> 0` |
| `tie_break` | string | yes | Exactly `stop_first` |
| `fee_rate` | number | yes | Finite, `>= 0`; fraction of fill notional |
| `slippage_bps` | number | yes | Finite, `>= 0` and `< 10000` |
| `zero_volume` | string | yes | Exactly `reject` |
| `end_of_data` | string | yes | Exactly `leave_open` |

The fill policy:

- A decision on bar `N` is submitted on bar `N` and fills on bar `N + 1` at the
  open, never on bar `N`.
- The bracket uses the decision bar's ATR(14), anchored at the fill:
  long stop `fill - stop_atr_multiple × ATR`, target
  `fill + target_atr_multiple × ATR`; short stop
  `fill + stop_atr_multiple × ATR`, target `fill - target_atr_multiple × ATR`.
- If one bar touches both, the stop wins.
- A zero-volume bar cannot fill a market order or trigger a bracket.
- An order with no next bar is rejected at `finish`; an open position is not
  force-closed.

`fee_rate` is the optional exchange fee on the fill notional. It is separate from
the portfolio commission. The example sets it to `0.0` to avoid double charging.

## 9. `portfolio.json`

```json
{
  "schema": "quantrule.portfolio/v1",
  "initial_cash": 10000.0,
  "currency": "USD",
  "sizing": {
    "mode": "fixed_notional",
    "notional": 10000.0,
    "quantity_step": 0.0001
  },
  "leverage": 1.0,
  "max_open_positions": 1,
  "allow_short": true,
  "commission": {
    "rate": 0.001,
    "base": "equity",
    "per": "operation"
  },
  "funding": {
    "rate": 0.0001,
    "interval_hours": 8
  },
  "max_drawdown": {
    "limit": 0.20,
    "action": "halt"
  },
  "mark_to_market": "bar_close"
}
```

| Field | Type | Required | Rule |
|---|---|---|---|
| `schema` | string | yes | Exactly `quantrule.portfolio/v1` |
| `initial_cash` | number | yes | Finite, `> 0` |
| `currency` | string | yes | Exactly `USD` in v1 |
| `sizing` | object | yes | One of the two modes below |
| `leverage` | number | yes | Exactly `1.0` in v1 |
| `max_open_positions` | integer | yes | Exactly `1` in v1 |
| `allow_short` | boolean | yes | `true` or `false` |
| `commission` | object | yes | `rate`, `base`, `per` |
| `commission.rate` | number | yes | Finite, `>= 0` |
| `commission.base` | string | yes | Exactly `equity` |
| `commission.per` | string | yes | Exactly `operation` |
| `funding` | object | yes | `rate`, `interval_hours` |
| `funding.rate` | number | yes | Finite; fraction of notional per interval |
| `funding.interval_hours` | integer | yes | One of `1, 2, 4, 8, 12, 24` |
| `max_drawdown` | object | yes | `limit`, `action` |
| `max_drawdown.limit` | number or null | yes | `(0, 1]`, or null for no limit |
| `max_drawdown.action` | string | yes | `halt` or `none` |
| `mark_to_market` | string | yes | Exactly `bar_close` in v1 |

### 9.1 Sizing modes

Fixed notional:

```json
{ "mode": "fixed_notional", "notional": 10000.0, "quantity_step": 0.0001 }
```

| Field | Type | Rule |
|---|---|---|
| `mode` | string | Exactly `fixed_notional` |
| `notional` | number | Finite, `> 0` |
| `quantity_step` | number | Finite, `> 0` |

Quantity is `notional / sizing_price`, floored to `quantity_step`.

Equity fraction:

```json
{ "mode": "equity_fraction", "fraction": 0.10, "quantity_step": 0.0001 }
```

| Field | Type | Rule |
|---|---|---|
| `mode` | string | Exactly `equity_fraction` |
| `fraction` | number | Finite, `> 0` and `<= 1` |
| `quantity_step` | number | Finite, `> 0` |

Quantity is `(equity × fraction) / sizing_price`, floored to `quantity_step`.

The sizing price is the decision bar's close. The fill occurs at the next bar's
open, so the realized notional can differ from the sizing notional. A quantity
below one step rejects the entry; it is never rounded up to a step.

### 9.2 Accounting rules

- Commission is `rate × max(equity_before_operation, 0)`, charged on every fill,
  entry and exit. It is the product's "percentage of total portfolio per
  operation" model.
- The execution fee is `fee_rate × fill_notional`, recorded on the fill, and is
  not the commission.
- Funding is `rate × abs(quantity) × bar_open` at aligned UTC boundaries, before
  the bar's fills, using the bar open as the mark price. A positive rate means a
  long pays and a short receives. A negative rate reverses the sign. A missing bar
  at the boundary means no charge for that boundary.
- Mark to market is at the bar close.
- Max drawdown is `1 - equity / peak_equity`. At or above `limit`, `halt` latches
  for the rest of the run: no new entries, an existing position continues to its
  strategy, stop, or target exit. It does not liquidate.
- Leverage is a sizing-time check only. `1.0` is the only accepted value. There
  is no margin call, liquidation, or borrow interest.
- An open position at the end is marked to market and reported, not force-closed.

Worked example with the values above:

| Quantity | Value |
|---|---|
| Initial equity | 10,000.00 |
| Commission rate | 0.001 |
| Commission per fill | 10.00 |
| Round trip | 20.00 |
| Position notional | 10,000.00 |
| Cost as a share of notional | 0.20% |
| Funding per 8h | 1.00 |

## 10. `analytics.json`

```json
{
  "schema": "quantrule.analytics/v1",
  "structure_scale": "major",
  "timeframes": ["1m", "5m", "15m", "1h"],
  "relations": true
}
```

| Field | Type | Required | Rule |
|---|---|---|---|
| `schema` | string | yes | Exactly `quantrule.analytics/v1` |
| `structure_scale` | string | yes | `major` or `minor`; must equal `model.json` |
| `timeframes` | array of strings | yes | Non-empty, unique, ascending ladder members |
| `relations` | boolean | yes | Include pairwise correlations |

The analytical report is descriptive. It does not run the strategy, execution, or
portfolio. When `relations` is `false`, `relations.csv` has a header and no rows,
and the JSON `relations` list is empty; a missing correlation is never written as
zero.

## 11. Cross-config validation

| Rule | Error |
|---|---|
| All seven files exist and parse as JSON objects | `MissingFile` / `InvalidJson` |
| Each `schema` is the expected fixed string | `Schema` |
| No unknown fields in any object | `UnknownField` |
| No missing required fields | `MissingField` |
| No wrong types | `Type` |
| All numbers are finite and in range | `Range` |
| `strategy.timeframe` is in the data ladder | `Inconsistent` |
| `execution.fill_timeframe` equals `strategy.timeframe` | `Inconsistent` |
| `analytics.structure_scale` equals `model.structure_scale` | `Inconsistent` |
| `analytics.timeframes` is a subset of the ladder | `Inconsistent` |
| `leverage == 1.0` and `max_open_positions == 1` | `Unsupported` |
| `from <= to` when both are present | `Range` |
| `output_root` is non-empty | `Range` |
| `slippage_bps < 10000` | `Range` |

## 12. Error format

```text
config error: <file>: <json-path>: expected <expected>, found <observed>
```

Examples:

```text
config error: portfolio.json: $.leverage: expected 1.0, found 2.0
config error: execution.json: $.fill_timeframe: expected 15m, found 1h
config error: data.json: $.ladder: expected ["1m","5m","15m","1h"], found ["1m","15m"]
```

The message names the file and the JSON path. It never prints a stack trace, an
absolute path, or a partial run.

## 13. Change control

The field names, enum values, and error categories are frozen for MVP v1. A new
field arrives as a new schema version (`quantrule.*/v2`), not as a silent
addition. Removing a field follows the same rule. The contract is the
specification the loader implements; a change to one is a change to both.
