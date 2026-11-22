# Research Artifact Packs and Run v5

- Date: 2026-11-22 (UTC)
- Status: Strategy, portfolio policy and execution-simulation pack schemas implemented (RA-002, RA-003, RA-004); the run-v5 reference contract and config identity implemented (RA-005); the persistent pack registry, run lineage and artifact retention implemented (RA-006); the signal artifact contract and pure reference computation implemented (RA-007); the stateless strategy service and metadata-only registration implemented (RA-008); run-v5 catalog/worker reachability and the client surface target later work; accepted by [ADR-RA](../../adr/2026-11-22/1082-research-artifact-layer.md)
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
- Numbers use the JSON representation stored in the pack; bytes are not
  re-serialised after freeze.
- The pack ID is `sha256(canonical bytes)`.
- Canonicalisation happens once when the pack is created; every later identity
  check hashes the stored bytes without re-serialising them.

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

Rule purity: only arithmetic, comparison, clamp, sign and rounding; no fitted,
statistical, machine-learning or percentile parameter is permitted.

The validator rejects unknown fields, duplicate conditions, unregistered events,
diagnostic roles, conflicting entry directions, and fold-only signal fields.

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
  "constraints": [
    {"kind": "max_positions_per_instrument", "value": 1},
    {"kind": "min_holding_bars", "value": 1}
  ]
}
```

| Field | Rule |
|---|---|
| `sizing.mode` | `fixed_notional` or `equity_fraction` |
| `sizing.notional` | Positive finite; required for `fixed_notional` |
| `sizing.fraction` | `(0, 1]`; required for `equity_fraction` |
| `sizing.quantity_step` | Positive finite |
| `risk.per_trade_risk` | `(0, 1]` |
| `risk.max_exposure` | `(0, 1]` |
| `risk.max_open_positions` | Positive integer |
| `drawdown.limit` | `(0, 1]`; `action` is `halt` |
| `rebalance` | `none`, or `{"mode": "bar_interval", "bars": N}` with positive `N` |
| `constraints` | Strictly ascending by `kind`, no duplicates |

Constraint kinds in v1:

| `kind` | Value | Rule |
|---|---|---|
| `max_positions_per_instrument` | Positive integer | At most `risk.max_open_positions` |
| `min_holding_bars` | Positive integer | Minimum execution bars in a position |

`rebalance` and `constraints` are optional, defaulting to `none` and an empty
list. Initial cash and currency are account-specific run inputs, not pack fields.
The closed schema rejects `initial_cash`, `currency`, `leverage`, correlation,
covariance, optimisation and every other statistical or fitted field.

## 5. Execution-simulation pack payload

```json
{
  "fill_timeframe": "5m",
  "order_type": "market_next_open",
  "tie_break": "stop_first",
  "fee_rate": 0.0,
  "slippage_bps": 2.0,
  "commission": {"rate": 0.0002, "base": "equity", "per": "operation"},
  "funding": {"rate": 0.0, "interval_hours": 8},
  "zero_volume": "reject",
  "end_of_data": "leave_open"
}
```

| Field | Rule |
|---|---|
| `fill_timeframe` | `1m`, `5m`, `15m` or `1h` |
| `order_type` | `market_next_open` |
| `tie_break` | `stop_first` |
| `fee_rate` | Finite, `>= 0`; fraction of the fill notional |
| `slippage_bps` | `>= 0` and `< 10000` |
| `commission.rate` | Finite, `>= 0`; fraction of equity per fill |
| `commission.base` | `equity` |
| `commission.per` | `operation` |
| `funding.rate` | Finite; fraction of position notional per interval; negative reverses the payer |
| `funding.interval_hours` | `1`, `2`, `4`, `8`, `12` or `24` |
| `zero_volume` | `reject` |
| `end_of_data` | `leave_open` |

Only fields the simulation honours are permitted. The closed schema rejects
`broker`, `venue`, `order_routing`, `latency`, `participation`, order-book,
volatility and every other live-market or statistical field.

## 6. Signal artifact

The signal artifact is one document per model artifact and signal-mode strategy
pack. It carries the pack, model and data identity, the timeframe, the period,
and one row per execution bar with `bar_index`, `open_time`,
`available_bar_index`, `action`, `side`, `intent`, `reason`, and numeric event
and state `evidence`. It carries no fill, quantity, fee, cash, equity, P&L or
edge metric. A later bar cannot change an earlier row, and the same model
artifact plus pack plus schema gives one ID.

Signal mode uses a deterministic reference position: a market entry fills at the
next open, a two-sided bracket chooses its side at the first boundary, ATR
stop/target and the time stop close the reference position, and a strategy exit
is applied at the next open. Giveback is fold-mode only. The full row, action,
intent and reference-position semantics are in
[`signal-artifact.md`](signal-artifact.md).

The strategy service resolves `data_version` + `config_id` once, verifies the
manifest's canonical id and every object, refuses a fold pack before the model
call, and writes the canonical bytes directly under `artifacts/<sha256 hex>`.
It then registers the metadata with `POST /internal/v1/artifacts/signal/{id}`;
signal bytes never travel over HTTP and the 64 MiB PUT cap is unchanged.

## 7. Run v5

```json
{
  "schema": "quantrule.run/v5",
  "name": "btcusd-2024-alpha",
  "model_id": "M1.3",
  "data": "data.json",
  "model": "model.json",
  "model_artifact": { "id": "sha256:…", "hash": "sha256:…" },
  "strategy_pack": { "id": "sha256:…", "hash": "sha256:…" },
  "signal_artifact": { "id": "sha256:…", "hash": "sha256:…" },
  "portfolio_policy": { "id": "sha256:…", "hash": "sha256:…" },
  "execution_model": { "id": "sha256:…", "hash": "sha256:…" },
  "account": { "initial_cash": 10000.0, "currency": "USD" },
  "output_root": "../../../data/labs/btcusd-2024-alpha"
}
```

| Field | Rule |
|---|---|
| `strategy_pack` | Required; must resolve to a `strategy` pack |
| `portfolio_policy` | Required; must resolve to a `portfolio` pack |
| `execution_model` | Required; must resolve to an `execution` pack |
| `model_artifact` | Optional; required when `signal_artifact` is present |
| `signal_artifact` | Required for a signal-mode strategy pack; absent for fold mode |
| `account.initial_cash` | Finite and `> 0`; the run owns it |
| `account.currency` | Exactly `USD` in v1; the run owns it |
| `output_root` | A relative path inside the config directory |
| Legacy fields | `strategy`, `execution`, `portfolio`, `execution_id` and `portfolio_id` are rejected in v5 |

Each `{id, hash}` pair names one content-addressed object with both fields equal.
A pack's id is the SHA-256 of its canonical bytes; a signal artifact's id is the
SHA-256 of its stored bytes; a model artifact's id is the canonical digest over
its `(name, sha256)` entries, which the diagnostic stage also records. A
strategy-only change leaves the model artifact bytes and id unchanged.

The loader verifies every content hash, rejects a wrong-kind pack, and checks the
signal artifact's `strategy_pack_id`, `model_artifact_id` and `timeframe`. Fold
mode omits `signal_artifact`; signal mode requires it before submission. v3 and
v4 configs remain readable; a v5 config is new.

`RunId` remains `data_version ‖ config_hash ‖ model_version ‖ image_digest`.
The config hash covers the run, data and model files and each reference tag and
hash, so a pack or signal change changes the run identity; v3 and v4 keep their
hash and the canonical digest still covers result artifacts only.

A v5 config is validated through a supplied reference resolver
(`RunConfig::load_with`, `AppService::load_run_with`). The pack registry now
stores and serves the pack bytes, RA-007 provides the pure signal computation,
and the RA-008 strategy service writes the signal artifact and registers its
catalog row. Run submission, the catalog scan and the worker do not yet resolve
a v5 config or add the run lineage and `run:<id>` reference. Those integrations
are later work items; no client command is added here. The current fold
materializes strategy, execution, commission,
funding, sizing, drawdown and position limits; `per_trade_risk`, `max_exposure`,
`rebalance` and `constraints` are retained in `RunV5` but not applied yet.

## 8. Lifecycle and validation

| State | Allowed |
|---|---|
| `draft` | Edit, validate |
| `validated` | Freeze or reject |
| `frozen` | Use in runs; no mutation |
| `retired` | Existing runs only; no new runs |

Validation covers schema, canonical hash, registry roles, causality,
determinism, golden fixtures, duplicate identity and version immutability. Freeze
needs a human decision and an independent Quant Validator record.

### Registry storage and API

The data service owns `catalog.packs`,
`catalog.pack_validation_records` and `catalog.pack_lifecycle_events` (migration
7). The row is profile-scoped metadata and identity; the object store is the
single byte authority, under `packs/<hash>`, and the row records its key, hash
and size. Registration writes the object first and commits the metadata row
second, so a failed commit leaves an orphan object, never a row without bytes.
Canonicalisation happens once, the same hash is idempotent, and a `(name,
version)` label with different bytes is refused. Validate, freeze, retire, show
and byte fetch re-read the object and verify its hash; a missing object fails
closed. Validate re-parses the document; freeze and retire lock the row and
append one event in the same transaction. Both are idempotent; a frozen pack
cannot be re-validated and a retired pack is never deleted.

| Method | Path | Effect |
|---|---|---|
| `POST` | `/catalog/packs` | Create from a pack document |
| `GET` | `/catalog/packs?kind=&state=`, `/catalog/packs/{id}` | List and show |
| `GET` | `/catalog/packs/{id}/bytes` | Fetch canonical bytes by content id |
| `POST` | `/catalog/packs/{id}/validate` | Re-validate a draft |
| `POST` | `/catalog/packs/{id}/freeze` | Freeze a validated pack |
| `POST` | `/catalog/packs/{id}/retire` | Retire a frozen pack |
| `GET` | `/internal/v1/packs/{id}`, `.../bytes` | Service-to-service read |

No shell command and no browser screen is added. The routes take a profile
header and an identifier, never a client path.

### Signal artifacts, lineage and retention

Evidence adds `runs.run_lineage` and `runs.run_lineage_references` (migration
20): a run's model, strategy, signal, portfolio and execution references by id
and hash with pack provenance. A repeated set is idempotent; a different set is
refused. `PUT`/`GET /internal/v1/runs/{id}/lineage` is the run-v5 integration
seam; no submission or worker path reaches it yet.

The data service adds `catalog.artifacts` and `catalog.artifact_references`
(migration 8) as the signal-artifact seam. The catalog row is the pre-run signal
record; the strategy service writes the bytes and registers it. Evidence
`runs.run_lineage` is recorded at run submission, not by the strategy service,
because the signal must exist first; that path adds `run:<id>`. No new reference
kind is introduced. A referenced artifact is never collected, and the internal
pass collects an unreferenced signal artifact at least 30 days old, writes a
`catalog.retention_events` tombstone, then deletes an unshared object.

## 9. Compatibility and clients

- The 49 existing commands and screens are unchanged; RA-006 adds none.
- Planned additive commands: `pack create/list/show/validate/freeze/retire` and
  `signal create/list/show/fetch`; a run detail will show the hashes.
- No client computes a signal or a statistic; the server owns computation.

## 10. Non-goals

No live trading, broker, OMS, real-time risk, regulatory scope, statistical signal parameter, per-bar network fold or fifth database.
