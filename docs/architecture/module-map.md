# Code Module Map

- Status: Current; released as v1.5.0; desktop distribution retired
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
quantrule-gateway ─┘
        └──> quantrule-runtime ──> quantrule-api, quantrule-service-api

quantrule-data-service ────┐
quantrule-model-service ───┼──> quantrule-app ──> quantrule-backtest
quantrule-backtest-service ┘                         └──> quantrule-model
                                                           └──> quantrule-data

quantrule-evidence-service ──> quantrule-research
quantrule-simulation-service ──> quantrule-simulation
```

| Crate | Owns | Must not own |
|---|---|---|
| `quantrule-data` | Candle validation, bounded window, timeframe construction, gaps, UTC | Files, indicators, events, strategy |
| `quantrule-model` | Readings, states, levels, events, progression, context, report, spec | Files, strategy, fills, cash |
| `quantrule-backtest` | Strategy, execution, portfolio, benchmark | Files, transport, rendering |
| `quantrule-app` | Workspace, dataset, configuration, research artifact packs, run orchestration, artifact I/O, shared use cases | Indicators, rules, fills, P&L, transport, research memory |
| `quantrule-api` | Contract types, error envelope, state enum, stage types, version constants | Any engine dependency |
| `quantrule-service-api` | `quantrule.internal/v1` records, bundles, run lifecycle, stage manifest, canonical digest | Any engine, runtime, or service dependency |
| `quantrule-research` | Hypothesis evaluation: expectancy, significance, drawdown, comparison | I/O, clock, engine dependency |
| `quantrule-simulation` | Research plane: deterministic windows, marks, oracle, check, exit lab | I/O, clock, run artifacts |
| `quantrule-runtime` | Configuration, health, logging, shutdown, adapters, migrations, service client | An engine or `quantrule-app` |
| `quantrule-gateway` | Profiles (`gateway` schema), static client, proxy table, health | Any app, engine, or domain service crate |
| `quantrule-data-service` | `catalog` schema, ingest, datasets, configs, inspection, ladder | Another schema or a model run |
| `quantrule-model-service` | Model compute and content-addressed model bundles | A strategy, execution, portfolio, or database schema |
| `quantrule-backtest-service` | Worker orchestration, strategy, execution, portfolio, stage worker, artifact objects | A catalog, run, or research schema |
| `quantrule-evidence-service` | `runs`, `research`, and `stage` schemas, run records, stage lifecycle, provenance, digest, memory, hypotheses, research ledger | An engine or `quantrule-app` |
| `quantrule-simulation-service` | `simulation` schema, research objects, eleven endpoints | A model, run artifact, strategy, or queue message |
| `quantrule-shell` | Parsing, terminal rendering, HTTP calls | Domain logic, engine dependency |
| `apps/web` | Svelte screens, stores, SVG chart, rendering | Domain logic, computation |

The compile/source guard in `crates/quantrule-gateway/tests/service_boundary.rs`
enforces the graph: the gateway links no app, engine, or domain service crate; a
service reads only its own schema; every service ships one binary; simulation
cannot read a strategy, write a run, publish to the queue, or compute a model.

## 2. Service crates

| Crate | Read first |
|---|---|
| `quantrule-gateway` | `src/server.rs` for the route and proxy table; `src/profile/` for `gateway.profiles`; `src/static_files.rs` for the SPA |
| `quantrule-data-service` | `src/routes.rs`, `src/ingest.rs`, `src/catalog.rs`, `src/postgres.rs` |
| `quantrule-model-service` | `src/routes.rs`, `src/bundle.rs`, `src/state.rs` |
| `quantrule-backtest-service` | `src/backtest/worker.rs`, `src/backtest/workspace.rs`, `src/backtest/retry.rs`, `src/stage/` |
| `quantrule-evidence-service` | `src/state.rs`, `src/postgres.rs`, `src/migrations.rs`, `src/stage/` |
| `quantrule-simulation-service` | `src/routes/`, `src/postgres.rs`, `src/bundle.rs` |
| `quantrule-runtime` | `src/config.rs`, `src/health.rs`, `src/adapters/`, `src/client.rs`, `src/migrations.rs` |

## 3. Where to add a change

| Change | Start here | Also update |
|---|---|---|
| Base reading | `model/src/readings/<family>.rs` | registry, config parser, writer, golden test |
| Event | `model/src/events/<family>.rs` | codes, registry, dispatch, writer, test |
| Endpoint | `gateway/src/server.rs` | owner service, `quantrule-api`, `http-api.md`, client store, test |
| Service responsibility | the owning `*-service` crate | `services.md`, `service_boundary.rs` |
| Migration | the owning service's `src/migrations.rs` and `migrations/*.sql` | `deploy/postgres/migration-ledger.json`, [`migrations.md`](migrations.md) |
| Worker behaviour | `backtest-service/src/backtest/worker.rs` | `control-plane.md`, retry test |
| Stage endpoint | `evidence-service/src/stage/` | `gateway/src/server.rs`, `quantrule-api`, `stage-surface.md`, test |
| Stage execution | `backtest-service/src/stage/` | `quantrule-service-api`, retention, no-partial-publish test |
| Client screen | `apps/web/src/lib/<Name>Panel.svelte` | store, `client.md`, Playwright test |
| Command | `shell/src/command/parse/<family>.rs`, `dispatch/<family>.rs`, `src/output/<family>.rs` | `command-surface.md`, help, tutorial, `tests/cases/`, checker |
| Config field | `app/src/config/<file>.rs` | input type, provenance, docs, registry |
| Pack field | `app/src/pack/<kind>.rs`; shared identity in `pack/core.rs` | config contract, canonical hash, lifecycle test |
| Artifact field | `evidence-service/src/` | artifact contract, golden bundle |

## 4. Repository tooling

`crates/quantrule-governance` is the `qr-governance` binary for historical
repository evidence. It is developer tooling, not a product service: it depends
only on `serde_json` and `sha2`, is not in the product image, and never reads a
runtime store.

## 5. Reading order

1. `AGENTS.md` sections 2–4.
2. `docs/architecture/README.md`, `kata.md`, `characteristics.md`.
3. `services.md`, `data-plane.md`, `control-plane.md`.
4. `crates/quantrule-data/src/lib.rs`.
5. `crates/quantrule-model/src/lib.rs`, `timeframe.rs`, `run.rs`.
6. `crates/quantrule-backtest/src/lib.rs`, `strategy/mod.rs`.
7. `crates/quantrule-app/src/lib.rs`, `pipeline.rs`, `service.rs`, `pack/`.
8. `crates/quantrule-gateway/src/server.rs`.
9. `crates/quantrule-shell/src/command.rs`.
10. `apps/web/README.md`.

## 6. Maintenance

Update this map in the same pull request as any module move. A module that is not
on the map is either new and incomplete or stale.
