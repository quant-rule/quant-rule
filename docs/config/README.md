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
| Deployment v3 | Target for the next release; Swarm HA descriptor, four databases, ingress and stateful placement | `docs/config/2026-11-22/deploy-v3.md` |
| Four logical databases | Current for local Compose and v2 multi-host; owner/app roles, per-service app URLs, one-shot owner migration jobs | `docs/config/2026-11-22/four-databases.md` |
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
