# Code Module Map

- Status: Current; P3.4 target layout noted
- Part of: [`docs/architecture/README.md`](README.md)
- Normative rules: [`AGENTS.md`](../../AGENTS.md) section 2
- Related: [`clarity-and-p34-shared-pipeline.md`](../execution-plans/2026-11-12/clarity-and-p34-shared-pipeline.md)

This is the code navigation map. It answers three questions: which crate owns a
responsibility, which module to read first, and where a new reading, event,
command, or use case belongs. It is not a contract; the model, configuration,
and artifact contracts remain authoritative.

## 1. Dependency direction

```
quantrule-shell ──┐
                  ├──> quantrule-app ──> quantrule-backtest ──> quantrule-model ──> quantrule-data
quantrule-desktop ┘                         (strategy, execution, portfolio)
```

| Crate | Owns | Must not own |
|---|---|---|
| `quantrule-data` | Candle validation, bounded window, timeframe construction, gaps, UTC timestamps. | Files, indicators, events, strategy. |
| `quantrule-model` | Readings, discrete states, confirmed levels, events, progression, context, flat report, model spec evaluation. | Files, strategy, fills, cash, P&L. |
| `quantrule-backtest` | Strategy vocabulary and engines, execution fills, portfolio accounting, evaluation benchmarks. | Files, source access, rendering, model readings. |
| `quantrule-app` | Workspace, dataset, configuration, run orchestration, artifact I/O, research memory, shared use cases. | Indicators, rules, fills, P&L, terminal rendering. |
| `quantrule-shell` | Command parsing, terminal I/O, explicit fetch, rendering. | Model, strategy, execution, portfolio, or analytics logic. |
| `quantrule-desktop` | Window, IPC, path scope, webview rendering. | Domain logic or a second use-case implementation. |

The crate graph is guarded by
[`crates/quantrule-shell/tests/architecture.rs`](../../crates/quantrule-shell/tests/architecture.rs).

## 2. Model engine (`crates/quantrule-model/src`)

| Module | Responsibility | Read this when |
|---|---|---|
| `lib.rs` | Crate map and `REGISTRY_VERSION`. | You need the model entry point. |
| `input.rs` | `MarketStateConfig` and `TimeframeInput`: the validated run configuration and candle input. | You add a configurable reading or event parameter. |
| `timeframe.rs` | One timeframe's complete series: readings, extended readings, states, events, levels, progression, context. | You need to see how a stage is composed. |
| `run.rs` | The timeframe ladder orchestration. | You add a ladder-level behavior. |
| `readings/` | Base readings: EMA, RSI, ATR, RVOL, MACD, ADX/DI, confirmed pivots, `ReadingSeries`. | You add a base indicator or change a formula. |
| `readings/extended/` | Extended reading families: VWAP, profile, participation, flow, volatility/structure, Fibonacci, band, anatomy. | You add a family or change an extended formula. |
| `readings/extended.rs` | The `ExtendedReadings` aggregate and family dispatch. | You add an extended reading field. |
| `events/` | Shared event record, event types, codes, and base detectors: level, divergence, signal, compression, DMI, retest, trend. | You add or change a base event. |
| `events/extended/` | Extended event families: VWAP, profile, participation, flow, levels, structure, Fibonacci, band, divergence. | You add or change an extended event. |
| `events/extended/signals.rs` | The extended single-instrument signal batch, called by `events/extended.rs`. | You add an ATR/range/channel/Aroon/path signal. |
| `events/codes.rs` | Stable event codes. | You register a new event. |
| `events/source_codes.rs` | Canonical source codes, defined once and re-exported by the event modules. | You add or rename a source code. |
| `levels.rs` | Confirmed-level registry from confirmed pivots. | You change level identity or confirmation. |
| `progression/` | Progression legs, targets, lifecycle, and tests. | You change the leg model. |
| `context.rs` | Volatility and participation context. | You add a context measurement. |
| `report/` | The flat per-timeframe report projection. | You change the report envelope or JSON view. |
| `spec.rs`, `spec/v2.rs` | Resolved model-spec contracts and causal evaluators. | You change the condition language or prediction. |
| `calendar.rs`, `sessions.rs`, `panel.rs` | Calendar, session, and cross-asset inputs. | You change session or panel context. |
| `kernel.rs` | Shared numeric primitives, unavailable reasons, readiness, percentiles. | You need a shared deterministic primitive. |
| `retention.rs` | Bounded event history. | You change event retention. |

**Base vs extended.** The base surface is the v4 reading set and the event
families wired directly in `timeframe.rs`. The `extended` surface is the
indicator-completeness batch. A new family goes under `extended` only when it is
part of that batch and does not belong to a base family; the module doc must say
which family it owns. The deeper family split is tracked by
REF-2026-11-12-06 and is deferred.

## 3. Backtest engine (`crates/quantrule-backtest/src`)

| Module | Responsibility |
|---|---|
| `strategy/` | Strategy vocabulary, event-composite engine, compression-bracket engine, event/state input snapshots, decision record. |
| `execution/` | Order intent, order/fill/position records, causal fill engine, bracket policy, execution errors. |
| `portfolio/` | Capital and sizing configuration, state, ledger, trade/equity/funding/risk records. |
| `benchmark/` | Buy-and-hold and seeded random-timing evaluation benchmarks. Only analytics reads them. |
| `intent.rs` | The strategy-to-execution risk intent boundary. |
| `side.rs` | Long/short vocabulary. |

## 4. Application service (`crates/quantrule-app/src`)

| Module | Responsibility |
|---|---|
| `lib.rs` | Crate map and the shared boundary statement. |
| `service.rs` | `AppService`: workspace, dataset, run check, model run, export, record. |
| `workspace.rs` | Workspace root discovery and the `config/` and `data/sources/` listings. |
| `dataset.rs` | One M1 CSV summary. |
| `csv.rs` | Strict and tolerant CSV source loading; the only local source reader. |
| `pipeline.rs` | The source-to-model composition: load M1, build the ladder, run the model. |
| `config/` | Run, data, model v4, execution, portfolio, strategy, qualification, and model-spec parsing. |
| `export.rs` | Deterministic run-summary JSON. |
| `fingerprint.rs` | FNV-1a content identity. |
| `date.rs` | Explicit UTC date windows. |
| `error.rs` | The application-service error. |
| `artifact/provenance.rs` | Shared model/stage provenance reader and identity fields. |
| `memory/` | SQLite research memory: schema, store, types, transfer, tests. |

P3.4 adds `artifact/` (bundle writer, provenance, labels, model/report artifacts,
backtest manifest) and `stages/` (data, model, strategy, execution, portfolio,
backtest, qualification, analytics). The target layout is in section 7.

## 5. Shell (`crates/quantrule-shell/src`)

| Module | Responsibility |
|---|---|
| `main.rs` | Binary entry point and interactive loop. |
| `command.rs`, `command/` | Parsed command vocabulary, argument grammar, dispatch, help, and one module per command family. |
| `request.rs` | Parsed request structs shared by command parsers and stage runs. |
| `render.rs` | Candle and timeframe NDJSON. |
| `render_flat.rs` | Event NDJSON. |
| `labels.rs` | Numeric-code-to-word mapping for terminal output. |
| `source/`, `fetch.rs`, `export.rs` | Explicit Dukascopy fetch and its CSV writer. |
| `model_bundle/`, `render_report/` | Current model-bundle writer and report artifacts. |
| `spec.rs`, `spec/v2.rs` | Current model-spec JSON parser. |
| `strategy/`, `execution/`, `portfolio/`, `backtest/`, `qualification/`, `analytics/` | Current stage orchestration and artifact readers/writers. |
| `backtest_bundle.rs` | Current backtest manifest and stage-provenance reader. |

The stage modules and `model_bundle/`/`render_report/`/`spec*` are transitional;
P3.4 moves them to the app. Do not add a new stage capability to the shell.

## 6. Desktop (`apps/desktop`)

| Path | Responsibility |
|---|---|
| `src-tauri/src/lib.rs` | Tauri registration and the native boundary. |
| `src-tauri/src/commands/` | Tauri command boundary: `views`, `scope`, `workspace`, `memory`, and tests. |
| `src/App.svelte` | Workspace shell and section navigation. |
| `src/lib/*Panel.svelte` | Workspace, data, config, run, memory, settings views. |
| `src/lib/api.ts` | Tauri invoke or deterministic fixture backend. |
| `src/lib/types.ts` | IPC view types. |
| `src/lib/mock.ts` | Deterministic browser fixture backend. |

P4 splits `commands.rs` into `commands/{mod,views,scope,workspace,memory,tests}`.
P3.4g adds `commands/pipeline.rs`; P3.5 adds the pipeline section to the UI.

## 7. Target layout after P3.4

```
quantrule-app/src/
  artifact/      bundle, provenance, labels, model, report, backtest
  stages/        data, model, strategy, execution, portfolio,
                 backtest, qualification, analytics
quantrule-shell/src/
  command/       parse + render only
  source/        explicit fetch only
  export.rs      fetch CSV writer
quantrule-desktop/src-tauri/src/
  commands/      mod, views, scope, workspace, memory, pipeline, tests
```

The model and backtest crates keep their layout. The shell is an adapter; the
desktop is an adapter; the app is the use-case boundary.

## 8. Reading order

1. `AGENTS.md` sections 2–4 for the invariants.
2. `docs/architecture/README.md` and `docs/architecture/characteristics.md`.
3. `crates/quantrule-data/src/lib.rs`.
4. `crates/quantrule-model/src/lib.rs`, `input.rs`, `timeframe.rs`, `run.rs`.
5. `crates/quantrule-model/src/readings/mod.rs` and `events/mod.rs`.
6. `crates/quantrule-backtest/src/lib.rs` and `strategy/mod.rs`.
7. `crates/quantrule-app/src/lib.rs`, `pipeline.rs`, `service.rs`.
8. `crates/quantrule-shell/src/command.rs` and `main.rs`.
9. `apps/desktop/README.md`.

## 9. Where to add a change

| Change | Start here | Also update |
|---|---|---|
| Base reading | `model/src/readings/<family>.rs` | `readings/mod.rs`, `input.rs`, config parser, CSV writer, registry, tests. |
| Extended reading | `model/src/readings/extended/<family>.rs` | `readings/extended.rs` aggregate, config, CSV writer, registry, golden test. |
| Base event | `model/src/events/<family>.rs` | `events/codes.rs`, registry, `timeframe.rs` dispatch, CSV writer, row/golden tests. |
| Extended event | `model/src/events/extended/<family>.rs` | `events/extended.rs` dispatch, codes, registry, tests. |
| Model state | `model/src/timeframe.rs` | `report/flat.rs`, spec state fields, strategy state enum, CSV writer, tests. |
| Command | `shell/src/command/<family>.rs` | `command.rs`, `dispatch.rs`, `request.rs`, help, README, command-surface, tutorial, checker. |
| Shared use case | `app/src/service.rs` + `app/src/stages/<stage>.rs` | shell command adapter, desktop command, parity test, docs. |
| Desktop command | `apps/desktop/src-tauri/src/commands/<domain>.rs` | `lib.rs` handler, `types.ts`, frontend panel, tests. |
| Config field | `app/src/config/<file>.rs` | model/execution/portfolio input type, provenance, registry if a model parameter, docs. |
| Artifact field | `app/src/artifact/` (after P3.4) | stage writer, reader, artifact contract, golden bundle. |

## 10. Maintenance

Update this map in the same pull request as any module move. A module that is not
on the map is either new and incomplete or stale and ready for removal; resolve
which before merging.
