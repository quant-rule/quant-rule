# Checkpoint D - Architecture and Changeability

> **Tag map.** Current release [`v1.2.0`](../../../releases/2026-11-16/v1.2.0.md); server platform `v0.9.0`; desktop package `v1.0.0`; 1.1.0 alignment `v1.1.0`. Authoritative map: [`docs/releases/README.md`](../../../releases/README.md).

- Status: Released v1.2.0; updated 2026-11-16
- Release: [`docs/releases/2026-11-16/v1.2.0.md`](../../../releases/2026-11-16/v1.2.0.md) (current tag `v1.2.0`)

**Claim.** For one researcher, the released v1.2.0 architecture keeps one rule
in and one reproducible bundle out while a service, adapter, reading, or client
screen changes without touching the engines.

## Architecture

Service-Based, domain-partitioned, six services, one image, nine containers.
The current v1.2.0 release covers the server profile and the self-contained
desktop package; the distributed server platform itself was released as v0.9.0.
Its historical note was prepared as v1.0.0 before the 2026-11-15 renumbering;
see the [tag map](../../../releases/README.md).

| Service | Code | Capability |
|---|---|---|
| gateway | `crates/quantrule-gateway` | HTTP boundary, static client, profile, upload |
| data | `crates/quantrule-data-service` | ingestion, content identity, timeframe ladder, catalog |
| model | `crates/quantrule-model-service` | readings, states, events, progression |
| backtest | `crates/quantrule-backtest-service` | orchestration, strategy, execution, portfolio |
| evidence | `crates/quantrule-evidence-service` | provenance, digest, run records, memory |
| simulation | `crates/quantrule-simulation-service` | windows, marks, oracle, review, check |

The engines live in `crates/quantrule-data`, `crates/quantrule-model`, and
`crates/quantrule-backtest`; the shared application service is
`crates/quantrule-app`; the contracts are `crates/quantrule-api` and
`crates/quantrule-service-api`.

The pipeline is synchronous; the run lifecycle is asynchronous; research is a
separate, isolated plane. Stages exchange content references, never payload sets.
Database, object store, and queue sit behind traits, so a second implementation
is additive.

Key constraints: a client sends identifiers, never filesystem paths; every record
is `profile_id`-scoped and a profile is organisational, not a security boundary;
one service owns each schema; the image digest and canonical digest are part of
the evidence identity; the desktop profile replaces the three adapters without
changing the engines.

## L16 architecture view

| Element | Current view |
|---|---|
| Actors | One operator/researcher; one reviewer; browser, command shell, and desktop clients |
| System boundary | Gateway, six services, Postgres/S3/RabbitMQ, and the object store; candle bytes and run artifacts stay inside |
| Key request/data flows | Upload -> catalog -> run submit -> queue -> model/backtest worker -> evidence finalize -> client fetch |
| Authoritative data | `catalog` dataset record; `runs`/`research` run record and provenance; canonical digest; object-store bytes |
| External systems | Postgres, S3-compatible MinIO, RabbitMQ; the desktop profile substitutes SQLite, filesystem CAS, and an in-process queue |
| Contracts | `quantrule.api/v1`, `quantrule.internal/v1`, and per-service schemas; no service reads another schema |

Code and contract references: [`services.md`](../../../architecture/services.md),
[`data-plane.md`](../../../architecture/data-plane.md),
[`control-plane.md`](../../../architecture/control-plane.md),
[`http-api.md`](../../../architecture/http-api.md), and the six
`crates/quantrule-*-service` crates.

## Decisions

Twelve recorded decisions (the 2026-11-12 ADR set). The two that are hard to reverse:

| Decision | Why hard to reverse | Revisit when |
|---|---|---|
| Canonical digest definition | Invalidates every recorded comparison | a digest misses a real artefact class |
| Run identity inputs | Invalidates deduplication and every run id | a new input changes results |

## Quantified characteristics: target vs observed

Targets are the design goal. Observed evidence is what the repository or release
record actually shows; an unmeasured target is not presented as observed.

| Characteristic | Target / good means | Observed evidence | Status |
|---|---|---|---|
| Testability | every reading and event has causality, determinism, and golden evidence; fast suite <= 5 min | 49 indicators and 150 active events have registry/evidence rows; the v1.2.0 gate passed 2,118 tests; the v1.0.0 release gate passed 2,134; 24 checkers | Coverage observed; 5-minute target not measured |
| Modularity | 6 services, 0 cycles, gateway imports 0 engine symbols | Six service crates; service-boundary guard; service catalog | Observed |
| Maintainability | a rule change touches <= 1 crate and <= 3 modules; no file over 1,000 lines | Add-a-reading trace touches one model crate, registry, parser, writer, and fixture; file-size checker passes | Structural trace observed; no timed metric |
| Deployability | clean clone to healthy <= 10 min; one published port | `make up` starts nine containers and one published port; release note records the local image and `/health` | Path observed; 10-minute target not measured |
| Fault tolerance | no `succeeded` run with a missing artifact; resubmission gives identical bytes | No-partial-publish, queue lease/redelivery/dead-letter, CAS, and backup/restore tests; release note records the queue/CAS suite | Observed in tests; no production failure injection |
| Responsiveness | submit <= 200 ms; chart window <= 200 ms; 1 year over 8 timeframes <= 60 s on 8 cores | No benchmark in the release record | Target; unverified |
| Scalability | model stage 1 to 4 workers: identical digest, >= 2.5x throughput | `one_pass_and_four_shards_share_one_digest`, reversed ladder, and unsorted-merge counterexample pass (2026-11-14) | Digest invariance observed; 2.5x throughput target unverified |
| Partitioning type | domain-driven: 6 services by capability, no layer split | Service catalog and ADR-2026-11-12-01 | Observed |

The project's own [kata alignment](../../../execution-plans/2026-11-13/kata-alignment.md)
rates deployability 2/5, responsiveness 2/5, scalability 2/5, and elasticity
1/5. The table above therefore separates the architecture goal from measured
maturity.

## Changeability evidence

| Change | Ripple | Cost | What stayed easy |
|---|---|---|---|
| Add a service | 2 decisions, 9 work items | one documentation pass | the engines were untouched |
| Add profiles | one column in four schemas, one header, one client store | small | no service changed its responsibility |
| Add a reading | one module, registry, parser, writer, one fixture | the widest ripple in the system | bounded to one crate |

Cheap to change: a rule, a reading, a client screen, an endpoint behind a
version. Expensive: the digest definition, the run identity, the service count.

## Next architectural change

Retire the pinned legacy MinIO archive and record a maintained object-store
source. It touches the object-store adapter selection, deployment, backup, and
the release note, not the engines. The human registry decision is still open;
the desktop profile already uses the filesystem CAS instead.

## Consistency

Eight requirements -> eight characteristics -> six services -> twelve decisions ->
one released v1.2.0 tree. One increment throughout: one rule in, one
reproducible bundle out.

## Likely questions

- **Cheap:** a rule change, a new reading, a client screen, a second worker.
- **Expensive:** the digest definition, the run identity, a seventh service.
- **Residual risk:** cross-machine determinism and throughput at scale are not
  yet measured; the object-store source is a stopgap; no HA or failover.
