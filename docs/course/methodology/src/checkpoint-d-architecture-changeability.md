# Checkpoint D - Architecture and Changeability

## Spoken script

QuantRule is a service-based system for one operator. The browser, the command shell, and the desktop app are clients. They speak one HTTP contract through the gateway. Inside the boundary are six services: gateway, data, model, backtest, evidence, and simulation. The server profile runs them in nine containers with Postgres, S3-compatible object storage, and RabbitMQ. The desktop profile runs the same six sidecars with SQLite, a filesystem content-addressed store, and an in-process queue. One image, one port, one digest.

A run flows like this. I upload candles to data, which assigns a data_version. I submit a dataset and a config through the gateway. Evidence creates the run and puts a message on the queue. The backtest worker asks data for the config and dataset, asks model for the model bundle, runs strategy, execution, and portfolio, writes artifacts to the content-addressed store, and records the canonical digest. Evidence verifies the manifest and marks the run succeeded. I can close the client; the run continues.

The authoritative data is the catalog dataset record, the runs/research run record and provenance, the canonical digest, and the object-store bytes. The external systems are Postgres, MinIO, and RabbitMQ in the server profile; SQLite, the filesystem CAS, and the in-process queue in the desktop profile. The contracts are `quantrule.api/v1`, `quantrule.internal/v1`, and one schema per service. No service reads another service's schema.

Three choices matter. First, service-based and domain-partitioned, not microservices. Six coarse services in one image. The trade-off is Docker and more moving parts; the gain is that a run outlives the client and can move to another host. Second, a synchronous data plane and an asynchronous control plane. The run is deterministic; the queue is retryable. The trade-off is that status is eventual; the gain is detachability and safe retries. Third, content-addressed exchange, references instead of payloads. The trade-off is an object-store dependency; the gain is idempotent retries and reproducible bytes.

The hard-to-reverse decisions are the canonical digest definition and the run identity. Changing the digest invalidates every recorded comparison. Changing the run identity invalidates deduplication and every run id. I keep them stable and revisit only when a digest misses a real artifact class or a new input changes results.

What this makes cheap: a rule change, a new reading, a client screen, an endpoint behind a version, and a second worker. What it makes expensive: the digest definition, the run identity, and a seventh service.

The main unknown is cross-machine determinism and throughput at scale. The project's own pre-release kata alignment rated deployability 2/5, responsiveness 2/5, scalability 2/5, and elasticity 1/5. I use that rating as a pre-release check on the architecture, not as a current measurement.

The next architectural change is to retire the pinned legacy MinIO archive and record a maintained object-store source. It touches the object-store adapter selection, deployment, and backup, not the engines. The desktop profile already uses the filesystem CAS.

The distribution left the engines untouched. I can change a service without rewriting a rule or a client screen. The concrete ripple is in the appendix.

## Architecture view

| View | Content |
|---|---|
| Actors and boundary | one operator/researcher; one reviewer; browser, shell, desktop; gateway, six services, Postgres/S3/RabbitMQ, object store |
| Components and flow | gateway, data, model, backtest, evidence, simulation; upload -> data_version -> submit -> queue -> backtest -> model -> artifacts -> evidence -> client ([services.md](../../../architecture/services.md)) |
| Authoritative data | catalog dataset; runs/research record; canonical digest; object bytes |
| External systems and contracts | Postgres, MinIO, RabbitMQ; desktop SQLite, filesystem CAS, in-process queue; `quantrule.api/v1`, `quantrule.internal/v1`, one schema per service |

## Evidence appendix

### What we wanted and what we measured

| Characteristic and target | What we measured |
|---|---|
| Reproducibility: same inputs and image digest -> same bytes | server and desktop parity reproduced `sha256:453c90b4…`; cross-machine is reviewer-executed ([v1.2.0 release note](../../../releases/2026-11-16/v1.2.0.md)) |
| Temporal integrity: no later bar changes an earlier value | causality and prefix tests pass |
| Auditability: every result names its inputs and digests | provenance, `make verify`, and run records |
| Scalability: one to four workers, same digest; 2.5x throughput | one pass and four shards share one digest; throughput not measured |
| Deployability: healthy within 10 minutes, one port | `make up` starts nine containers and one port; the 10-minute target is not measured |

### Changeability

- Add profiles or a reading: profiles cost one column in four schemas, one header, and one client store; a reading costs one module, registry, parser, writer, and one fixture. Neither changes service responsibility.
- Add a service: two decisions and nine work items. The engines stay untouched.

