# Code Module Map

- Status: Target v1.0.0; approved 2026-11-12
- Part of: [`docs/architecture/README.md`](README.md)
- Normative rules: [`AGENTS.md`](../../AGENTS.md) section 2
- Claim type: Behaviour Claim

The navigation map. It answers: which crate owns a responsibility, which module
to read first, and where a new reading, event, endpoint, or command belongs. It
is not a contract.

## 1. Dependency direction

```text
apps/web ── HTTP ──┐
quantrule-shell ───┤
                   ├──> quantrule-api
quantrule-server ──┘         │
   (5 binaries)              │
        └──> quantrule-app ──> quantrule-backtest ──> quantrule-model ──> quantrule-data
```

| Crate | Owns | Must not own |
|---|---|---|
| `quantrule-data` | Candle validation, bounded window, timeframe construction, gaps, UTC | Files, indicators, events, strategy |
| `quantrule-model` | Readings, states, levels, events, progression, context, report, spec | Files, strategy, fills, cash |
| `quantrule-backtest` | Strategy, execution, portfolio, benchmark | Files, transport, rendering |
| `quantrule-app` | Workspace, dataset, configuration, run orchestration, artifact I/O, memory, shared use cases | Indicators, rules, fills, P&L, transport |
| `quantrule-api` | Contract types, error envelope, state enum, version constants | Any engine dependency |
| `quantrule-research` | Hypothesis evaluation: expectancy, significance, drawdown, comparison | I/O, clock, engine dependency |
| `quantrule-simulation` | Research plane: deterministic windows and the mark schema | I/O, clock, run artifacts |
| `quantrule-server` | Runtime, gateway, workers, migrations, profiles | Domain logic |
| `quantrule-simulation` | Window selection, marks, oracle, review, check, exit lab | Model computation, run artifacts |
| `quantrule-shell` | Parsing, terminal rendering, HTTP calls | Domain logic, engine dependency |
| `apps/web` | Svelte screens, stores, SVG chart, rendering | Domain logic, computation |
| `apps/desktop-shell` | A Tauri window over the gateway URL | Domain logic, local data, IPC commands |

Two guard tests enforce the graph: the gateway may not import an engine module,
and `quantrule-api` may not depend on a workspace engine crate.

## 2. Server crate

| Module | Responsibility |
|---|---|
| `runtime/config.rs` | Environment configuration; no file required |
| `runtime/health.rs` | Dependency checks and aggregation |
| `runtime/pg.rs`, `runtime/s3.rs`, `runtime/amqp.rs` | Pool, client, channel |
| `runtime/error.rs` | One error type and the JSON envelope |
| `runtime/shutdown.rs` | Graceful drain on SIGTERM |
| `runtime/profile.rs` | Profile resolution from the request header |
| `runtime/adapters/` | Database, object store, and queue traits, with one implementation each |
| `gateway/router.rs` | Route table; one handler per endpoint |
| `gateway/principal.rs` | The `Authorization` seam; injects `operator` |
| `gateway/static.rs` | SPA assets and history fallback |
| `gateway/upload.rs` | Streaming upload and size limit |
| `data/ingest.rs` | CSV parse, hash, catalog row |
| `data/catalog.rs` | Datasets and configs |
| `data/ladder.rs` | Timeframe construction over the engine |
| `model/compute.rs` | Model run, shard, merge |
| `backtest/orchestrate.rs` | Run choreography across services |
| `backtest/artifacts.rs` | Artifact write and manifest |
| `evidence/provenance.rs` | Provenance assembly |
| `evidence/digest.rs` | Canonical digest |
| `evidence/finalize.rs` | Completeness check and `succeeded` transition |
| `evidence/memory.rs` | Research memory, replacing SQLite |
| `simulation/windows.rs` | Seeded, non-overlapping window selection |
| `simulation/marks.rs` | Mark schema, validation, atomic write |
| `simulation/oracle.rs` | Deterministic hindsight labeler |
| `simulation/check.rs` | Event study on the unmarked window |
| `simulation/review.rs` | Mark comparison and candidate-rule table |
| `migrations/` | Forward-only SQL |

## 3. Where to add a change

| Change | Start here | Also update |
|---|---|---|
| Base reading | `model/src/readings/<family>.rs` | registry, config parser, writer, golden test |
| Event | `model/src/events/<family>.rs` | codes, registry, dispatch, writer, test |
| Endpoint | `server/src/gateway/router.rs` | `quantrule-api`, `http-api.md`, client store, test |
| Service responsibility | `server/src/<service>/` | `services.md`, guard tests |
| Worker behaviour | `server/src/backtest/orchestrate.rs` | `control-plane.md`, retry test |
| Client screen | `apps/web/src/lib/<Name>Panel.svelte` | store, `client.md`, Playwright test |
| Command | `shell/src/command/<family>.rs` | `command-surface.md`, help, tutorial, checker |
| Config field | `app/src/config/<file>.rs` | input type, provenance, docs, registry |
| Artifact field | `server/src/evidence/` | artifact contract, golden bundle |

## 4. Reading order

1. `AGENTS.md` sections 2–4.
2. `docs/architecture/README.md`, `kata.md`, `characteristics.md`.
3. `services.md`, `data-plane.md`, `control-plane.md`.
4. `crates/quantrule-data/src/lib.rs`.
5. `crates/quantrule-model/src/lib.rs`, `timeframe.rs`, `run.rs`.
6. `crates/quantrule-backtest/src/lib.rs`, `strategy/mod.rs`.
7. `crates/quantrule-app/src/lib.rs`, `pipeline.rs`, `service.rs`.
8. `crates/quantrule-server/src/gateway/router.rs`.
9. `crates/quantrule-shell/src/command.rs`.
10. `apps/web/README.md`.

## 5. Maintenance

Update this map in the same pull request as any module move. A module that is not
on the map is either new and incomplete or stale.
