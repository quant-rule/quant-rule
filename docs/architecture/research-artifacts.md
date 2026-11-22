# Research Artifacts

- Status: Strategy, portfolio policy and execution-simulation packs current (RA-002–RA-004); the run-v5 reference and identity contract current (RA-005); the persistent pack registry, run lineage and artifact retention current (RA-006); the signal artifact contract and pure reference computation current (RA-007); the strategy service, run-v5 catalog/worker reachability and the client surface target the next release; accepted by [ADR-RA](../adr/2026-11-22/1082-research-artifact-layer.md)
- Part of: [`README.md`](README.md)
- Claim type: Behaviour Claim

The research artifact layer makes alpha a frozen, reusable and auditable object.
QuantRule is a research platform: it never trades, and "execution" and "portfolio"
below mean historical simulation.

## 1. Objects

| Object | Kind | Owner | Storage | Identity |
|---|---|---|---|---|
| Model artifact | Computed upstream artifact | `model` | Object store + evidence provenance | Content hash |
| Strategy pack | Human-owned definition | Human, via `data` | Catalog metadata + object store | Content hash |
| Signal artifact | Computed alpha output | `strategy` | Object store + evidence lineage | Content hash |
| Portfolio policy pack | Human-owned definition | Human, via `data` | Catalog metadata + object store | Content hash |
| Execution-simulation pack | Human-owned definition | Human, via `data` | Catalog metadata + object store | Content hash |
| Run v5 | Reference document | Human, via config | Catalog DB + evidence | Config hash |
| Run artifact set | Simulation result | `backtest` + `evidence` | Object store | Canonical digest |

The model artifact is the upstream fifth artifact. It is not a pack and does not
use the pack-freeze lifecycle.

## 2. Strategy pack

The strategy pack contains the human-owned alpha definition:

- pack ID, version, schema, name and owner label;
- execution and context timeframes;
- universe constraint;
- mode: `signal` or `fold`;
- entry and exit rules over registered events and states;
- filters and arithmetic parameters;
- risk: ATR stop, reward-to-risk, time stop;
- validation status and lifecycle.

A pack is `draft`, `validated`, `frozen` or `retired`. Freeze is a human decision
backed by a Quant Validator record; a frozen pack is immutable, a change is a new pack.

## 3. Signal artifact

The signal artifact is the computed output of a model artifact and a signal-mode
strategy pack. It carries one row per execution bar:

| Field | Meaning |
|---|---|
| `bar_index`, `open_time` | Published execution bar identity |
| `available_bar_index` | First bar on which the row was knowable |
| `action`, `side` | Hold, enter, exit or bracket, and the side |
| `intent` | Next-bar market entry, two-sided bracket, or flat exit |
| `reason` | Stable action code |
| `evidence` | Event and state references |

It carries no fill, quantity, fee, cash, equity, P&L or edge metric. A later bar
cannot change an earlier row, and the same model artifact, pack and schema
produce the same bytes. Signal mode uses a deterministic reference position:
market entries fill at the next open, a two-sided bracket chooses its side at
the first boundary, ATR stop/target and the time stop close the reference
position, and giveback stays fold-mode unless candles become an explicit input.
The contract is [`docs/config/2026-11-22/signal-artifact.md`](../config/2026-11-22/signal-artifact.md).

## 4. Portfolio policy pack

The portfolio policy pack owns reusable sizing, risk, exposure, maximum positions,
drawdown action, rebalance and constraint rules. Constraints are a strictly
ascending, duplicate-free list of closed rule kinds. It does not own initial cash
or currency; those are account-specific run inputs. Its closed schema contains no
correlation, covariance, optimisation, fitted or statistical field.

## 5. Execution-simulation pack

The execution-simulation pack owns the fill timeframe, the market-next-open order
and stop-first tie rule, fee and slippage, commission, funding, zero-volume
rejection and end-of-data leave-open. It is not a broker connection: its closed
schema has no venue, order-routing, latency, participation or live-market field.

## 6. Run v5

`quantrule.run/v5` references the model artifact, strategy pack, signal artifact,
portfolio policy pack and execution-simulation pack by ID and hash. It carries
cash and currency and does not inline pack bodies. The model and signal artifacts
are optional only for a fold-mode run; a signal-mode run requires both.

`RunId` remains `data_version ‖ config_hash ‖ model_version ‖ image_digest`. The
config hash covers every reference hash, so a pack change changes the run
identity, while the model-input hash still covers only data and model. The
canonical digest remains over result artifacts only; provenance and inputs are
excluded. v3 and v4 configs remain readable and keep their config hash.

The signal artifact must exist before `run submit`; otherwise the run identity
cannot be known at submission. A v5 config is loaded with a supplied reference
resolver; the catalog scan and worker do not yet resolve one.

## 7. Services and flow

```text
Model artifact + Strategy pack
              │
              ▼
       strategy service
              │
              ▼
       Signal artifact
              │
              ▼
Run v5 + Execution pack + Portfolio pack
              │
              ▼
       backtest fold
              │
              ▼
       Run artifacts → evidence → research → human
```

- `strategy` is stateless, has no database and no queue, scales horizontally and
  is idempotent by model hash + pack hash.
- It rejects fold-mode packs.
- The per-bar fold stays inside `backtest`; there is no network call per bar.
- Research stays inside `evidence`; it is not a separate service.
- `simulation` remains a separate hindsight plane and never reaches the signal
  path.

## 8. Registry and lineage

`qr_catalog` stores pack metadata, validation records and lifecycle events;
MinIO is the sole byte authority under `packs/<hash>`. Registration writes the
object first and the metadata row second; show, validate, freeze, retire and
fetch re-read the object and verify its hash, so a missing object fails closed.
A frozen pack is immutable and a retired pack is never deleted.

`qr_evidence` stores a run's model, strategy, signal, portfolio and execution
references with pack provenance. The set is immutable: the same set is
idempotent and a different set is refused. No cross-database transaction exists
and no fifth database is added.

`catalog.artifacts` is the generic signal-artifact seam. A registered consumer
reference protects an artifact; the retention pass removes only an unreferenced
artifact older than 30 days, writes a tombstone in the same transaction, and
deletes an unshared object after the commit.

## 9. Alpha and edge

| Layer | Question | Statistics |
|---|---|---|
| Model | What is the market state? | No |
| Strategy pack / signal | What would alpha do? | No |
| Execution simulation | How would the intent fill? | No |
| Portfolio simulation | What would the account hold? | No |
| Evidence / research | Does the alpha have an edge? | Yes, evaluation only |
| Simulation | What would hindsight show? | Human judgment |

Statistics never enter the signal path. Research cannot write a pack or signal. A
change to alpha is a new pack and a new hypothesis.

## 10. Client surface

The 49 existing commands and screens remain unchanged. Additive pack and signal
commands and a browser Packs surface are a later client slice; RA-006 adds none.

## 11. Non-goals

- No live trading, broker, OMS, real-time risk or regulatory scope.
- No statistical or fitted signal parameter.
- No per-bar network fold.
- No separate research service.
- No fifth database.
- No change to the 49 existing command meanings.
