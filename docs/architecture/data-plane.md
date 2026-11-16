# Data Plane — The Deterministic Run

- Status: Current; released as v0.9.0 on 2026-11-15 (prepared as v1.0.0); unchanged in the v1.0.0 desktop release; approved 2026-11-12
- Part of: [`README.md`](README.md)
- Decision: [`ADR-2026-11-12-02`](../adr/2026-11-12/02-two-plane-control-and-data.md)
- Claim type: Behaviour Claim

The data plane computes one run. It is synchronous, ordered, and deterministic.
It never reads the clock, never reads the queue, and never depends on arrival
order.

## 1. Run choreography

```text
backtest worker (orchestrator)
  1. GET  data:/internal/v1/configs/{config_id}      → config bundle
     GET  data:/internal/v1/datasets/{data_version}  → dataset object ref
  2. POST model:/internal/v1/model/bundle {data_version, config_id} → model bundle manifest
  3. read the published model bars and candles; run strategy, execution, portfolio
  4. write artifacts to CAS and record the canonical digest
  5. publish run.computed
  6. evidence verifies the manifest and marks the run succeeded
```

`data` builds the timeframe ladder and writes it to CAS, so every run of the same
dataset reuses the same bytes. `model` shards by timeframe and writes the bundle;
the worker never recomputes a reading. `backtest` is a fold over time and is
never sharded.

## 2. Content addressing

Every intermediate and final object is stored under the SHA-256 of its bytes.

| Object | Key |
|---|---|
| Candle series | `datasets/{data_version-hex}` |
| Timeframe ladder | `ladder/{data_version-hex}/full/{timeframe}` |
| Model output | `model/{data_version}/{config_hash}/{model_version}/{ladder}.json` |
| Artifact blobs | `artifacts/{sha256}` |
| Run manifest | `runs/{run_id}/manifest.json` |

A repeated computation produces the same key. This is what makes retries safe:
redelivery recomputes and lands on the same bytes.

## 3. Ordering rules

| Rule | Reason |
|---|---|
| Merge shards sorted by `(symbol, timeframe, bar_index)` | Worker completion order must not matter |
| Serialize with float-roundtrip | Same value, same text, everywhere |
| Sort map keys before serialization | No hash iteration order in output |
| Fixed accumulation order | Splitting the work must not change the sum |
| No wall-clock value in an artifact | The bundle must not encode when it ran |

## 4. What may cross a hop

Only a reference and a small metadata envelope:

```json
{ "v": 1, "ref": "sha256:…", "content_type": "application/x-ndjson" }
```

Never a candle series, a reading series, or a bundle. The network carries
references; the object store carries bytes.

## 5. Determinism inputs

A run's output is a pure function of:

```text
(data_version, config_hash, model_version, image_digest)
```

Anything else that influences output is a defect. A client submits
`{data_version, config_id}`; the server derives `config_hash`, `model_version`,
and `image_digest` and records them in provenance.

## 6. Failure semantics

| Point of failure | Result |
|---|---|
| Before the first artifact is written | `failed`; nothing published |
| Mid-write | Blobs may exist under their hash; no manifest is written, so no artifact set is visible |
| After artifacts, before finalization | `running`; finalization retries |
| During finalization | `running` or `failed`; never `succeeded` with a missing artifact |

A run is `succeeded` only when every artifact in the manifest is present and its
hash verifies.

## 7. What is not in the data plane

Simulation is not a run. It reads a recorded bundle, marks with the future
visible, and writes research records. It has no place in this order, no `run_id`,
and no canonical digest. See [`services.md`](services.md) §7.

## 8. Read-only inspection

Inspection is outside the run and writes nothing:

| Operation | Endpoint | Output |
|---|---|---|
| Candles | `GET /inspect/candles?data_version=…` | counts, coverage, first and last bar |
| Timeframes | `GET /inspect/timeframes?data_version=…` | bars and partial buckets per timeframe |
| Events | `GET /inspect/events?data_version=…&timeframe=…` | event batch |

Inspection exists so a human or an agent can stop and look, without executing a
partial pipeline. Candles and timeframes are data inspection; events compute a
model, so the gateway forwards `/inspect/events` to the model service's internal
event-batch route.
