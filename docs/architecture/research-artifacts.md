# Research Artifacts

- Status: Strategy, portfolio policy and execution-simulation packs current (RA-002–RA-004); the run-v5 reference and identity contract current (RA-005); signal computation, the persistent registry and the client surface target the next release; accepted by [ADR-RA](../adr/2026-11-22/1082-research-artifact-layer.md)
- Part of: [`README.md`](README.md)
- Claim type: Behaviour Claim

The research artifact layer makes alpha a frozen, reusable and auditable object.
QuantRule is a research platform: it never trades, and "execution" and "portfolio"
below mean historical simulation.

## 1. Objects

| Object | Kind | Owner | Storage | Identity |
|---|---|---|---|---|
| Model artifact | Computed upstream artifact | `model` | Object store + evidence provenance | Content hash |
| Strategy pack | Human-owned definition | Human, via `data` | Catalog DB + object store | Content hash |
| Signal artifact | Computed alpha output | `strategy` | Object store + evidence lineage | Content hash |
| Portfolio policy pack | Human-owned definition | Human, via `data` | Catalog DB + object store | Content hash |
| Execution-simulation pack | Human-owned definition | Human, via `data` | Catalog DB + object store | Content hash |
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
supported by an independent Quant Validator record. A frozen pack is immutable.
A change is a new pack version.

## 3. Signal artifact

The signal artifact is the computed output of a model artifact and a signal-mode
strategy pack. It contains one row per bar:

| Field | Meaning |
|---|---|
| `bar_index` | Zero-based position in the timeframe |
| `open_time` | UTC Unix second |
| `available_bar_index` | First bar on which the row was knowable |
| `action` | Hold, enter, exit or bracket intent |
| `side` | Long, short or null |
| `intent` | Target position or conditional order |
| `reason` | Stable code |
| `evidence` | Event and state references |

It carries no fill, quantity, fee, cash, equity, P&L or edge metric. A later bar
cannot change an earlier signal. The same model artifact, pack and schema produce
the same signal artifact.

Signal mode uses a deterministic reference position. Giveback remains a fold-mode
feature unless candles become an explicit signal-service input.

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
the account cash and currency. It does not inline pack bodies. The model
artifact and signal artifact are optional only for a fold-mode run that computes
the model; a signal-mode run requires both.

`RunId` remains `data_version ‖ config_hash ‖ model_version ‖ image_digest`. The
config hash covers every reference hash, so a pack change changes the run
identity, while the model-input hash still covers only data and model. The
canonical digest remains over result artifacts only; provenance and inputs are
excluded. v3 and v4 configs remain readable and keep their config hash.

The signal artifact must exist before `run submit`; otherwise the run identity
cannot be known at submission. Because the persistent registry is not
implemented yet, a v5 config is loaded with a supplied reference resolver. The
catalog and worker reach it when the registry lands; no client command or
service boundary is added by this contract.

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

`qr_catalog` stores pack definitions, files, validation records and lifecycle
events. MinIO stores pack bytes and signal artifacts. `qr_evidence` stores signal
lineage, model references and run identity. There is no fifth database.

A referenced pack or signal is never deleted. Unreferenced signal artifacts are
garbage-collected after the retention window with a tombstone. A retired pack
remains readable for existing runs and is refused for new runs.

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

The 49 existing commands and screens remain unchanged. Additive commands cover
pack create/list/show/validate/freeze/retire and signal create/list/show/fetch.
The browser gains a Packs surface and run detail shows pack hashes and the signal
artifact. Both clients reach every capability.

## 11. Non-goals

- No live trading, broker, OMS, real-time risk or regulatory scope.
- No statistical or fitted signal parameter.
- No per-bar network fold.
- No separate research service.
- No fifth database.
- No change to the 49 existing command meanings.
