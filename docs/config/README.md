# Configuration Index

- Status: Current
- Run contract: [`docs/workflows/run-contract.md`](../workflows/run-contract.md)
- Config tree: [`config/README.md`](../../config/README.md)

This folder holds the run, strategy, and engine configuration contracts. The
config tree holds the current examples and the historical evidence; a config
document states which schema and run version it applies to.

| Contract | Status | Path |
|---|---|---|
| Run v4 and the strategy | Current for a file-based full run; `run v4` binds the strategy to the run; data schema 3, model schema 4, `execution v2`, `portfolio v2` | `docs/config/2026-11-13/run-v4-strategy.md` |
| Research packs and run v5 | Strategy, portfolio policy and execution-simulation packs, the run-v5 reference contract, the persistent pack registry, run lineage, artifact retention, the signal artifact reference computation and the stateless strategy service implemented (RA-002–RA-008); run-v5 catalog/worker reachability and the client surface target later work | `docs/config/2026-11-22/research-artifact-packs.md` |
| Signal artifact | Current; `quantrule.signal-artifact/v1`, the reference position, causality, identity, fold rejection and the internal strategy service route | `docs/config/2026-11-22/signal-artifact.md` |
| Deployment v3 | Current for schema, validate, render, check, and the read-only remote/overlay inspector; Swarm HA descriptor, four databases, 443-only ingress, Patroni/etcd/HAProxy, RabbitMQ broker/front door, MinIO erasure/front door/object bootstrap, pinned stateful units | `docs/config/2026-11-22/deploy-v3.md` |
| Reference topologies | Current for PLAT-008 phase 1: the 3-AZ and single-AZ reference descriptors, the four-rack layout, sizing/support matrix, lab addresses and preconditions, the managed-stateful boundary, and the 14 resolved pins plus 3 blockers | `docs/config/2026-11-24/reference-topologies.md` |
| Image registry and supply chain | Current for PLAT-007 phase 1 and PLAT-008 phase 1: GHCR pull-by-digest, offline cosign, SPDX SBOM, Grype plus `cargo audit` gate, pinned tools, descriptor pin, release-note fields, 14 resolved infrastructure pins, and the app/MinIO blockers | `docs/config/2026-11-24/registry.md` |
| Patroni cluster | Current for the v3 Patroni/etcd/HAProxy render, synchronous mode, watchdog, mTLS, boot order, cluster bootstrap, secrets, check rows, and the database-failover lab; a live lab is `Unverified` here | `docs/config/2026-11-23/patroni-cluster.md` |
| RabbitMQ cluster | Current for the v3 three-node broker render, TLS-only 5671, Erlang cookie, HAProxy queue front door, seed/wait boot order, app-user bootstrap, consumer reconnect, check rows, definitions tool, and broker-loss lab; a live lab is `Unverified` here and the end-to-end digest is `Measured` only after `tools/verify.sh` | `docs/config/2026-11-23/rabbitmq-cluster.md` |
| MinIO cluster | Current for the v3 distributed erasure render, EC:2/EC:3 parity/quorum, TLS-only 9000, HAProxy `minio` front door, bucket/versioning/`qr_app` policy, object bootstrap, check rows, and node-loss lab; a live lab is `Unverified` here and the end-to-end digest is `Measured` only after `tools/verify.sh` | `docs/config/2026-11-23/minio-cluster.md` |
| Observability | Current for phases 1–2: 16 image pins, Prometheus/Alertmanager distinct-host placement, seven-service and stateful scrape, 38 rules over 17 rows, one internal null receiver, Loki/promtail, four dashboards, `tools/observability.sh`, leader/term/missing-digest gauges, `check/observability.rs`, and the lab assertions | `docs/config/2026-11-23/observability.md` |
| Ingress | Current for the v3 cert-mode Traefik ingress; VIP/DNS, keepalived, static/dynamic config ownership, cert rotation, headers and limits, ingress check rows, private-CA client trust, and the VIP-loss lab | `docs/config/2026-11-23/ingress.md` |
| Swarm control plane | Current; three odd managers, node labels, bootstrap order, secret/config drift, stop-first rolling update, remote check, `ha` lab tier, and the `tools/swarm.sh` wrapper | `docs/config/2026-11-23/swarm-control.md` |
| Four logical databases | Current for local Compose, v2 multi-host, and the v3 Patroni cluster; owner/app roles, per-service app URLs, one-shot owner migration jobs | `docs/config/2026-11-22/four-databases.md` |
| Backup and restore | Current for HA-013 phases 1a and 2: one `quantrule` pgBackRest stanza, 13 check rows, the derived Patroni image, the four-database Compose archive, the backup-age signal, the off-host S3 reference, the stage lease/outbox, and `tools/queue-rebuild.sh`; the live drill and empty-broker drill are `Unverified` | `docs/config/2026-11-24/backup-restore.md` |
| HA failure and chaos matrix | Current for HA-014: the 23-row `quantrule.ha-matrix/v1` record, the `ha-matrix` governance checker, the opt-in `--chaos` lab cases, and the operator Real HA gate; HA-18..HA-23 and every live F row are `Unverified` here, and the AZ1 MinIO/ingress limitation is an accepted risk | `docs/config/2026-11-24/chaos-matrix.md` |
| Research readiness | Current for HA-015: the eight-row `quantrule.research-readiness/v1` record, the research-only boundary, the accepted single-host lab risk, and the `research-readiness` governance checker | `docs/config/2026-11-25/research-readiness.md` |
| File-backed secrets | Current; `*_FILE` contract, explicit S3 credentials and gateway token rotation | `docs/config/2026-11-23/secret-files.md` |
| Schema migrations | Current; forward-only ledgers, checksum pin, expand/contract and rollback runbook | `docs/architecture/migrations.md` |
| Run v3 and portfolio constraints | Accepted for a model-only run; `run v3` must not carry a strategy | `docs/config/2026-11-11/run-v3-and-portfolio-constraints.md` |
| Local research memory | Historical; the local SQLite module was removed on 2026-11-16; memory lives in the evidence service (#661) | `docs/config/2026-11-11/research-memory-v1.md` |
| Client surfaces | Product 1.5.0: Svelte browser client and command shell; no desktop package | `docs/architecture/client.md` |
| Distribution | Current; container image and GitHub Release assets. The static page was retired | `ADR-2026-11-12-08` |
| Desktop profile | Removed in v1.5.0 | — |
| Policy blueprint | Historical; superseded by run v3 | `docs/config/2026-10-30/policy-and-strategy.md` |
| Config tree | Current; current, evidence, and historical directories | `config/README.md` |
| Available timeframes | `quantrule.data/v3` | `docs/config/2026-10-30/data-v3.md` |
| Session coverage | Current; optional weekly calendar and session-aware coverage | `docs/config/2026-11-09/data-change-SESSION-COVERAGE-01.md` |
| MACD reading and events | Current | `docs/config/2026-11-03/model-macd.md` |
| Engine artifacts | Current; stage bundle, provenance, and handoff contract | `docs/config/2026-11-06/engine-artifacts.md` |
| Strategy configuration | Current; `quantrule.strategy/v1`, both generic engines, and the decision record | `docs/config/2026-11-06/strategy-config.md` |
| Qualification criteria | Current; human-owned criteria, verdict semantics, and human acceptance | `docs/config/2026-11-06/qualification-criteria.md` |
| Analytics benchmarks | Current; buy-and-hold and matched random-timing evaluation output | `docs/config/2026-11-09/analytics-benchmarks.md` |
| Trade context | Historical; the Python exporter was retired by DIST-105, with the artifact and mark schemas retained as a record | `docs/config/2026-11-09/trade-context.md` |
| Strategy position excursions | Current; entry price/ATR, unrealized, MFE/MAE, giveback exit | `docs/config/2026-11-10/strategy-position-excursions.md` |
| Improvement run contract | Current; `quantrule.improvement-run/v3` is current, v2 is historical | `docs/workflows/run-contract.md` |

A file-based run uses `quantrule.run/v4`, which references the strategy alongside
the four engine files. `quantrule.run/v3` remains accepted for a model-only run
and must not carry a `strategy` field. `quantrule.run/v5` is additive: it names
the strategy pack, portfolio policy, execution-simulation pack, and the optional
model and signal artifacts by content hash. A v5 config is loaded through a
supplied reference resolver until the persistent registry lands; v3 and v4 keep
their existing loaders and config hashes. `analytics` is still rejected; the
qualification criteria remain explicit command arguments.
