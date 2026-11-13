# Data Plane — The Deterministic Run

- Status: Target v1.0.0; approved 2026-11-12
- Part of: [`README.md`](README.md)
- Decision: [`ADR-2026-11-12-02`](../adr/2026-11-12/02-two-plane-control-and-data.md)
- Claim type: Behaviour Claim

The data plane computes one run. It is synchronous, ordered, and deterministic.
It never reads the clock, never reads the queue, and never depends on arrival
order.

## 1. Run choreography

```text
backtest worker (orchestrator)
  1. GET  data:/datasets/{data_version}/candles      → candle stream
  2. POST model:/compute {data_version, config_id, timeframes?} → model object ref
  3. compute strategy, execution, portfolio locally
  4. write artifacts to CAS and record the canonical digest
  5. publish run.computed
  6. evidence verifies the manifest and marks the run succeeded
```

`data` builds the timeframe ladder and writes it to CAS, so every run of the same
dataset reuses the same bytes. `model` shards by timeframe. `backtest` is a fold
over time and is never sharded.

## 2. Content addressing

Every intermediate and final object is stored under the SHA-256 of its bytes.

| Object | Key |
|---|---|
| Candle series | `candles/{data_version}` |
| Timeframe ladder | `ladder/{data_version}/{config_hash}/{timeframe}` |
| Model output | `model/{data_version}/{config_hash}/{model_version}/{ladder}.json` |
| Run artifacts | `runs/{run_id}/{name}` |
| Bundle | `runs/{run_id}/bundle.tar` |

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

Anything else that influences output is a defect. The set is exactly what
[`http-api.md`](http-api.md) requires for a submission and exactly what
provenance records.

## 6. Failure semantics

| Point of failure | Result |
|---|---|
| Before the first artifact is written | `failed`; nothing published |
| Mid-write | Bytes are written under a temporary key; the artifact is never visible |
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
partial pipeline.
