# Research Artifacts

- Status: The strategy, portfolio and execution-simulation packs (RA-002–RA-004), run-v5 reference and identity (RA-005), pack registry, lineage and retention (RA-006), signal artifact and reference computation (RA-007), seventh strategy service (RA-008), and the additive pack/signal client surface (RA-009) are current; accepted by [ADR-RA](../adr/2026-11-22/1082-research-artifact-layer.md)
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
| Signal artifact | Computed alpha output | `strategy` | Object store + catalog row; run lineage at submission | Content hash |
| Portfolio policy pack | Human-owned definition | Human, via `data` | Catalog metadata + object store | Content hash |
| Execution-simulation pack | Human-owned definition | Human, via `data` | Catalog metadata + object store | Content hash |
| Run v5 | Reference document | Human, via config | Catalog DB + evidence | Config hash |
| Run artifact set | Simulation result | `backtest` + `evidence` | Object store | Canonical digest |

The model artifact is the upstream fifth artifact. It is not a pack and does not
use the pack-freeze lifecycle.

## 2. Strategy pack

The strategy pack contains the human-owned alpha definition: pack ID, version,
schema, name and owner; execution and context timeframes; universe constraint;
mode `signal` or `fold`; entry and exit rules over registered events and states;
filters and arithmetic parameters; ATR stop, reward-to-risk and time stop;
validation status and lifecycle.

A pack is `draft`, `validated`, `frozen` or `retired`. Freeze is a human decision
backed by a Quant Validator record; a frozen pack is immutable, a change is a new
pack.

## 3. Signal artifact

The signal artifact is the computed output of a model artifact and a signal-mode
strategy pack. It carries one row per execution bar with `bar_index`,
`open_time`, the first knowable `available_bar_index`, `action`, `side`,
`intent`, a stable `reason`, and numeric event and state `evidence`. It carries
no fill, quantity, fee, cash, equity, P&L, return or edge metric.

A later bar cannot change an earlier row, and the same model artifact, pack and
schema produce the same bytes. Signal mode uses a deterministic reference
position: market entries fill at the next open, a two-sided bracket chooses its
side at the first boundary, ATR stop/target and the time stop close the reference
position, and giveback stays fold-mode unless candles become an explicit input.
The contract is [`docs/config/2026-11-22/signal-artifact.md`](../config/2026-11-22/signal-artifact.md).

## 4. Portfolio and execution packs

The portfolio policy pack owns reusable sizing, risk, exposure, maximum positions,
drawdown action, rebalance and constraint rules. Constraints are a strictly
ascending, duplicate-free list of closed rule kinds. It does not own initial cash
or currency; those are account-specific run inputs. Its closed schema contains no
correlation, covariance, optimisation, fitted or statistical field.

The execution-simulation pack owns the fill timeframe, the market-next-open order
and stop-first tie rule, fee and slippage, commission, funding, zero-volume
rejection and end-of-data leave-open. It is not a broker connection: its closed
schema has no venue, order-routing, latency, participation or live-market field.

## 5. Run v5

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
cannot be known at submission. The data service resolves every data-owned
reference at upload, the model service resolves the set before a full model
compute, and the backtest worker resolves it once before the fold. Each reader
verifies the id and hash; no stage resolves a reference per bar.

## 6. Services and flow

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
       backtest fold → run artifacts → evidence → research → human
```

- `strategy` is the seventh service: stateless, no database or queue, idempotent
  by model hash + pack hash. It resolves once per request, rejects fold-mode
  packs before the model call, and makes no per-bar network call.
- It writes canonical bytes under `artifacts/<hash>`, then calls the data
  service's metadata-only registration; signal bytes never travel over HTTP.
- Research stays inside `evidence`; `simulation` is a separate hindsight plane.

## 7. Registry and lineage

`qr_catalog` stores pack metadata, validation records and lifecycle events;
MinIO is the sole byte authority under `packs/<hash>`. Registration writes the
object first and the row second; reads verify the object hash, so a missing
object fails closed. A frozen pack is immutable; a retired one is never deleted.

`qr_evidence` stores a run's model, strategy, signal, portfolio and execution
references with pack provenance. The set is immutable: the same set is
idempotent and a different set is refused. No cross-database transaction exists
and no fifth database is added.

`catalog.artifacts` is the pre-run signal record: the strategy service writes
the bytes and registers the row. Run lineage is recorded at submission, because
the signal must exist first; that path adds `run:<id>` and no new reference kind.
A registered reference protects an artifact; retention removes only an
unreferenced artifact older than 30 days, writes a tombstone in the same
transaction, and deletes an unshared object after the commit.

## 8. Alpha and edge

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

## 9. Client surface

The pack, signal and lineage surface is additive. The shell adds `pack
create/list/show/validate/freeze/retire/fetch`, `signal create/list/show/fetch`,
and `run lineage`; the browser adds a Packs screen with Packs and Signals tabs
and a lineage block on run detail. The 49 commands that existed before the change
keep their name, arguments, output fields, errors and renderer.

The browser lists and shows signal metadata only and downloads the document
after verifying its hash. No client parses a signal row, computes a signal, or
renders a statistic. A deployment without the strategy service answers signal
create with `503 service.unavailable`; pack and signal reads still work.

## 10. Non-goals

- No live trading, broker, OMS, real-time risk or regulatory scope.
- No statistical or fitted signal parameter.
- No per-bar network fold.
- No separate research service.
- No fifth database.
- No change to the 49 existing command meanings.
