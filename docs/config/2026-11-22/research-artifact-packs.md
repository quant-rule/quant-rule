# Research Artifact Packs and Run v5

- Date: 2026-11-22 (UTC)
- Status: Target for the next release; accepted by [ADR-RA](../../adr/2026-11-22/1082-research-artifact-layer.md)
- Claim type: Behaviour Claim
- Related: [Research artifacts](../../architecture/research-artifacts.md), [Run v4](../2026-11-13/run-v4-strategy.md)

QuantRule is a research platform. It never trades. Execution and portfolio packs
describe historical simulation only.

## 1. Pack envelope

A pack is immutable, versioned and content-addressed:

```json
{
  "schema": "quantrule.pack/v1",
  "kind": "strategy",
  "version": "1.0.0",
  "name": "compression-fade-5m",
  "owner": "operator",
  "payload": {}
}
```

| Field | Rule |
|---|---|
| `schema` | `quantrule.pack/v1` |
| `kind` | `strategy`, `portfolio` or `execution` |
| `version` | Human label; not the identity |
| `name` | Non-empty label |
| `owner` | Label, not an authorisation boundary |
| `payload` | The kind-specific document |

`pack_id` is the SHA-256 of the canonical pack bytes. Same bytes means the same
pack. Same `name` and `version` with different bytes is rejected. The frozen and
retired lifecycle is metadata outside the hash.

## 2. Canonical hash

- JSON keys are sorted lexicographically.
- No insignificant whitespace.
- UTF-8, no BOM, trailing newline only.
- Numbers use the JSON representation stored in the pack; the bytes are not
  re-serialised after freeze.
- File paths are sorted; each path, byte length and bytes are hashed.
- The pack ID is `sha256(canonical bytes)`.

## 3. Strategy pack payload

```json
{
  "mode": "signal",
  "engine": "event_composite",
  "execution_timeframe": "5m",
  "context_timeframes": ["15m"],
  "universe": ["BTCUSD"],
  "event_window_bars": 1,
  "time_stop_bars": 0,
  "giveback_atr": null,
  "giveback_arm_atr": 0.0,
  "require_contiguous": true,
  "risk": {
    "stop_atr_multiple": 2.0,
    "reward_to_risk": 1.5
  },
  "entry": [],
  "exit": []
}
```

| Field | Rule |
|---|---|
| `mode` | `signal` or `fold`; signal is reusable, fold stays in backtest |
| `engine` | `event_composite` or `compression_bracket` |
| `execution_timeframe` | Published timeframe; must match the run's fill timeframe |
| `context_timeframes` | Unique, ascending, different from execution |
| `universe` | Optional constraint; the run data must satisfy it |
| `event_window_bars` | Positive integer; execution bars |
| `time_stop_bars` | Zero disables for event composite; bracket requires positive |
| `giveback_atr` | Fold mode only in v1; null in signal mode |
| `giveback_arm_atr` | Fold mode only; non-negative |
| `risk.stop_atr_multiple` | Positive finite |
| `risk.reward_to_risk` | Positive finite |
| `entry` / `exit` | Registered event and state conditions |

Rule purity: only arithmetic, comparison, clamp, sign and rounding. No fitted,
statistical, machine-learning or percentile parameter is permitted.

The validator rejects unknown fields, duplicate conditions, unregistered events,
diagnostic roles, conflicting entry directions, and a fold-only field in signal
mode.

## 4. Portfolio policy pack payload

```json
{
  "sizing": {
    "mode": "equity_fraction",
    "fraction": 0.05,
    "quantity_step": 0.0001
  },
  "risk": {
    "per_trade_risk": 0.01,
    "max_exposure": 1.0,
    "max_open_positions": 1
  },
  "drawdown": {
    "limit": 0.15,
    "action": "halt"
  },
  "rebalance": "none",
  "constraints": []
}
```

| Field | Rule |
|---|---|
| `sizing` | `fixed_notional` or `equity_fraction`; positive values |
| `risk.per_trade_risk` | Positive finite fraction |
| `risk.max_exposure` | `(0, 1]` |
| `risk.max_open_positions` | Positive integer |
| `drawdown.limit` | `(0, 1]`; action `halt` |
| `rebalance` | `none` or a deterministic rule |
| `constraints` | Declarative, rule-based only |

Initial cash and currency are not in this pack; they are account-specific run
inputs. No correlation, covariance, optimisation or fitted statistic is allowed.

## 5. Execution-simulation pack payload

```json
{
  "fill_timeframe": "5m",
  "order_type": "market_next_open",
  "tie_break": "stop_first",
  "fee_rate": 0.0,
  "slippage_bps": 2.0,
  "commission": {
    "rate": 0.0002,
    "base": "equity",
    "per": "operation"
  },
  "funding": {
    "rate": 0.0,
    "interval_hours": 8
  },
  "zero_volume": "reject",
  "end_of_data": "leave_open"
}
```

Only fields the simulation honours are permitted. No broker, venue, order
routing, latency claim or live-market field is part of this pack.

## 6. Signal artifact

```json
{
  "schema": "quantrule.signal-artifact/v1",
  "strategy_pack_id": "sha256:…",
  "model_artifact_id": "sha256:…",
  "data_version": "sha256:…",
  "timeframe": "5m",
  "period": { "from": "2024-01-01", "to": "2024-12-31" },
  "rows": [
    {
      "bar_index": 0,
      "open_time": 1704067200,
      "available_bar_index": 0,
      "action": "hold",
      "side": null,
      "intent": null,
      "reason": null,
      "evidence": []
    }
  ]
}
```

The artifact contains no fill, quantity, fee, cash, equity, P&L or edge metric.
`available_bar_index` is the first bar on which the row was knowable. A later bar
cannot change an earlier row. The same model artifact plus pack plus schema
produces the same signal ID.

## 7. Run v5

```json
{
  "schema": "quantrule.run/v5",
  "name": "btcusd-2024-alpha",
  "model_id": "M1.3",
  "data": "data.json",
  "model": "model.json",
  "strategy_pack": { "id": "sha256:…", "hash": "sha256:…" },
  "signal_artifact": { "id": "sha256:…", "hash": "sha256:…" },
  "portfolio_policy": { "id": "sha256:…", "hash": "sha256:…" },
  "execution_model": { "id": "sha256:…", "hash": "sha256:…" },
  "account": { "initial_cash": 10000.0, "currency": "USD" }
}
```

For fold mode, `signal_artifact` is absent. For signal mode it is required and
must exist before submission. v3 and v4 configs remain readable; a v5 config is
new.

`RunId` remains `data_version ‖ config_hash ‖ model_version ‖ image_digest`. Pack
and signal hashes enter through `config_hash` and provenance. The canonical digest
is over result artifacts only.

## 8. Lifecycle and validation

| State | Allowed |
|---|---|
| `draft` | Edit, validate |
| `validated` | Freeze or reject |
| `frozen` | Use in runs; no mutation |
| `retired` | Existing runs only; no new runs |

Validation covers schema, canonical hash, registry roles, causality,
determinism, golden fixtures, duplicate identity and version immutability. Freeze
is a human decision with an independent Quant Validator record.

## 9. Compatibility and clients

- The 49 existing commands and screens are unchanged.
- Additive commands: `pack create/list/show/validate/freeze/retire` and
  `signal create/list/show/fetch`.
- A run detail shows model, pack and signal hashes.
- No client computes a signal or a statistic; the server owns computation.

## 10. Non-goals

No live trading, broker, OMS, real-time risk, regulatory scope, statistical
signal parameter, per-bar network fold or fifth database.
