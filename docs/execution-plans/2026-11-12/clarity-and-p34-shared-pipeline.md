# Clarity Refactor and P3.4 Shared Pipeline Extraction

- Date: 2026-11-12 (UTC)
- Status: **In Review** (human, 2026-11-12); P0–P11 implemented and verified, P12 deferred, PR #628 open
- Claim type: **Behaviour Claim** (internal boundary and parity; the only visible change is the corrected help model-version line)
- Owner: Human
- Workflow: Execution Plan + Refactor Workflow; Audit Workflow closure for AUD-2026-11-11-11
- Parent plan: [`../2026-11-11/audit-app-service-native-remediation.md`](../2026-11-11/audit-app-service-native-remediation.md)
- Child plan: [`audit-parity-phase-3.md`](audit-parity-phase-3.md)
- ADR: [`../../adr/2026-11-11/one-program-first-class-desktop.md`](../../adr/2026-11-11/one-program-first-class-desktop.md)
- Refactor assessment (P0): `docs/refactors/2026-11-12/clarity-refactor.md`
- Issue: — (no issue number assigned; approved as the clarity/P3.4 campaign plan)
- Plan path: `docs/execution-plans/2026-11-12/clarity-and-p34-shared-pipeline.md`
- AI log (same base name): `docs/ai-transparency-log/2026-11-12/clarity-and-p34-shared-pipeline.md`

> Approved by the human on 2026-11-12. The plan is the plan of record for the
> clarity refactor and the P3.4 extraction. P0 records the formal refactor
> assessment before the first code move.

## 1. Intent

Make the codebase easier to navigate and finish the one-program architecture.
The work has two halves:

1. **Clarity**: remove stale boundary docs, module-name traps, duplicated
   artifact readers, and the single 922-line desktop command file. Add one code
   map that says where each kind of change belongs.
2. **P3.4**: move the remaining pipeline orchestration out of the shell and
   behind `quantrule-app`, one stage at a time, so the shell and desktop UI call
   one shared use-case layer. The `fetch` network source stays shell-only until
   a separate human decision.

The intended outcome is not a new feature. It is the same commands, the same
artifact bytes, and the same command output, with one obvious place to change
each thing and one implementation behind both surfaces. The one deliberate
visible correction is the help glossary line that says M1.0 is current when
M1.3 is current; that line and its test are updated in P1.

Affected users:

- The human reading and reviewing the code.
- Agents adding a reading, event, command, or stage.
- The desktop UI, which currently cannot reach the pipeline stages.
- Future releases, which should not need to fix the same drift twice.

## 2. Current state

### 2.1 Baseline

| Item | Value or source |
|---|---|
| Branch | `main` |
| Commit | `d2ed0c7` (`Merge pull request #627 ...`) |
| Rust toolchain | `1.99.1` pinned by `rust-toolchain.toml` |
| Structural report | 223 Rust source files; 73,107 lines; 0 files over 1,000; 9 files over 800; 40 over 600 |
| Crate size (including tests) | data 3,517 / 9 files; model 34,974 / 99; backtest 13,864 / 47; app 10,716 / 30; shell 22,965 / 79 |
| Largest production file | `progression/leg.rs` 955; `app/config/strategy.rs` 878; `memory/store.rs` 857; `events/level.rs` 843; `source/dukascopy.rs` 829 |
| Desktop backend | 985 lines total; `commands.rs` 922 |
| Test inventory | Record exactly in P0; the last documented baseline is 1,476 passed, 0 failed, 8 ignored |
| Golden fixtures | Record SHA256 in P0; the last documented set is 105 files |
| Report bundle | Record SHA256 in P0; the last documented hash is `59124f63...` |
| Backtest bundle | Record SHA256 in P0; the last documented hash is `4173b505...` |

Reproduction commands are in section 11.1. A baseline is evidence, not a
guarantee; P0 records the observed values before any file moves.

### 2.2 Why this work exists

The architecture is sound at the crate level:

- `crates/quantrule-shell/tests/architecture.rs` guards the allowed crate graph.
- `tools/check_structure.py` requires a module doc on every non-test source file.
- `check_file_size.py` catches files over 1,000 lines.
- Determinism, causality, golden, and boundary tests are extensive.
- The artifact/provenance and command-surface contracts are documented.

The clarity debt is concentrated:

| ID | Finding | Evidence |
|---|---|---|
| CLAR-01 | Model names encode delivery history, not domain | `events/signal.rs` is the base detector; `events/signals.rs` is an extended-only detector called from `events/extended.rs:189`. `events/divergence.rs` vs `events/extended/divergence.rs`; `events/level.rs` vs `events/extended/levels.rs`. 19 `use super::*` imports in the extended submodules. |
| CLAR-02 | Stale boundary docs | `shell/src/main.rs:3` says "the shell is the product's only surface". `backtest/src/lib.rs:11` and `strategy/config.rs:4` say the runner/context are "S3.x work" though they exist. `app/src/lib.rs:5` says neither adapter parses configuration, but `shell/src/spec.rs` and `spec/v2.rs` do. `apps/desktop/src-tauri/src/lib.rs:3` says "no path resolution", but `commands.rs:300-374` normalizes and scopes paths. |
| CLAR-03 | Small factual drift | `model/src/timeframe.rs:31,108` says "six scalar readings" but there are nine. `model/src/report/flat.rs:3` says "M1.0 view"; `shell/command/help/topics.rs:50` says "M1.0 current", while `docs/README.md` says M1.3 is current. |
| CLAR-04 | Duplicated stage-provenance readers | `strategy/bundle.rs:253`, `execution/decisions.rs:142`, and `portfolio/fills.rs:96` parse `quantrule.stage-provenance/v1` with separate errors and `read_text`. `ModelBundleError` exists twice; `BundleError` exists twice. |
| CLAR-05 | Desktop command boundary is one file | `apps/desktop/src-tauri/src/commands.rs` is 922 lines and holds 24 Tauri commands, view types, path scope, path validation, memory CRUD, and import/export. |
| CLAR-06 | `ExtendedReadings` has no visible family boundaries | 88 public fields; `empty` and `compute` repeat all 88; the file is 567 lines. |
| CLAR-07 | Shell still orchestrates pipeline stages | `backtest/run.rs:150`, `report_run/mod.rs:60,86`, `execution/run.rs`, and `portfolio/run.rs` drive the engines. The shared-pipeline extraction (P3.4/P3c) is pending. |
| CLAR-08 | No code navigation map | 223 source files; the docs are contract-oriented. There is no single page that maps modules to responsibilities and says where to add a reading, event, command, or use case. |
| CLAR-09 | Source-code constants are duplicated | `EMA20/50/200_SOURCE_CODE` exist in `events/level.rs` and `events/signal.rs`; `RSI_SOURCE_CODE` exists in `events/divergence.rs` and `events/signal.rs`. |

### 2.3 Current boundary

```
shell (CLI)
  ├─ command/ parse + render
  ├─ stage modules: analytics, backtest, execution, portfolio, qualification, strategy
  ├─ model_bundle/ + render_report/ artifact writers
  └─ spec/ model-spec JSON parser
        │
        ├─ quantrule-app: workspace, dataset, config, model run, export, memory, pipeline
        └─ direct calls into quantrule-model and quantrule-backtest

desktop (Tauri)
  └─ commands.rs → quantrule-app: workspace, dataset, config, model, memory, export
```

The app service already owns the source-to-model path (`crates/quantrule-app/src/pipeline.rs`), but the shell still owns the later stage orchestration and much of the artifact I/O. The desktop cannot reach those stages. That is the parity gap AUD-2026-11-11-11 records.

## 3. Scope

### 3.1 Included

1. **Documentation truth pass** — fix the stale doc comments and the help model
   version; add a code module map.
2. **Model naming and constants** — move `events/signals.rs` under the extended
   batch; replace glob imports in the extended submodules; centralize source-code
   constants; update module docs.
3. **Shared stage-bundle reader** — one provenance reader and one error type for
   the strategy, execution, and portfolio stage bundles; resolve duplicate error
   names.
4. **Desktop command split** — split `commands.rs` into `commands/` by
   responsibility without changing command names, arguments, or behavior.
5. **P3.4 shared pipeline use cases** — move data inspection, events, report,
   strategy, execution, portfolio, backtest, qualification, and analytics
   orchestration behind `quantrule-app`, one stage per PR.
6. **Spec parser move** — move the model-spec JSON parser from the shell to the
   app configuration layer as part of P3.4a.
7. **Shell thinning and parity** — remove stage orchestration from the shell,
   add architecture guards and a parity test, and keep the shell as parse +
   terminal I/O + fetch + render.
8. **Desktop pipeline commands** — expose every shared pipeline use case as a
   Tauri command. The visible UI panels are P3.5.
9. **P3.5 desktop pipeline UI** — expose the stages in the desktop UI; `fetch`
   remains shell-only with a recorded exception. This phase requires explicit
   human approval of the UI scope.
10. **Conditional model extended split** — group `ExtendedReadings` into family
    structs and rename colliding extended modules. This is a separate,
    behavior-neutral refactor campaign; it starts only if a separate assessment
    and human approval accept the diff budget.
11. **Records and closure** — changelog, AI transparency log, plan updates,
    full gate, worktree/branch cleanup, board `Done`.

### 3.2 Excluded

- No model, strategy, execution, portfolio, qualification, or analytics
  behavior change.
- No signal, parameter, event, baseline, or edge claim.
- No schema, artifact header, or error-string change. Bundles must be
  byte-identical; command output and exit codes must be identical.
- No new dependency without a separate human approval. The Tauri dialog plugin
  is already approved; this plan adds none.
- No desktop `fetch` network path. The local-first promise is not weakened.
- No release tag, GitHub release, or deployment.
- No deletion or retirement of historical `tools/`, `data/`, `config/`, or
  `docs/` records.
- No big-bang rewrite. Every stage moves in its own PR with its own evidence.

## 4. Assumptions and uncertainty

- The parent plan and the one-program ADR are approved; P3.4 is the remaining
  piece of AUD-2026-11-11-11.
- The command set and artifact contracts are stable; a move must not change
  them. If a move exposes a needed contract change, stop and re-plan.
- The app service can own artifact I/O without becoming a presentation layer:
  artifacts are files, not terminal output. The shell keeps terminal rendering.
- The desktop can call the new app methods through Tauri commands without a new
  dependency. The existing path-scope rules apply to every new path.
- The native E2E environment (`tauri-driver` + `WebKitWebDriver`) may be
  unavailable. A recorded exception is allowed only with human approval.
- P3.5 UI work may be larger than the backend extraction. It is a separate
  approval gate; the backend can land first.
- The model extended family split may exceed the campaign budget. The module
  map and naming fixes still land if the deeper split is deferred.
- The human may decide that `fetch` should eventually be exposed in the desktop.
  That is a separate network-boundary decision and is not assumed here.
- The issue number and final plan filename are not assigned yet. The suggested
  path is `docs/execution-plans/2026-11-12/clarity-and-p34-shared-pipeline.md`.

## 5. Decisions

| ID | Decision | Why |
|---|---|---|
| D1 | Move artifact read/write to `quantrule-app`; the shell keeps terminal rendering only. | Artifacts are file I/O and belong to the service boundary; the ADR says the service owns export and the shell owns rendering. |
| D2 | One P3.4 slice per stage, one PR per slice. | Each move is reviewable, reversible, and independently verified against a bundle hash. |
| D3 | Put the shared provenance reader in `quantrule-app`, not the shell. | It moves with the stage readers and becomes the single artifact contract implementation. |
| D4 | Keep `fetch` shell-only; record the desktop exception. | The network boundary is a product/security decision, not a refactor. |
| D5 | Keep `extended` as the documented batch name for now; move `signals.rs` under it and fix collisions where they confuse. | A full domain-family split is a separate, high-churn campaign; the immediate traps are small and worth fixing now. |
| D6 | Group `ExtendedReadings` into family structs only after a separate refactor assessment. | It is behavior-neutral but high churn; it should not be mixed with P3.4. |
| D7 | P3.5 desktop UI is included, but its visible scope needs human approval. | The backend can land first; the UI is the larger, less certain half. |
| D8 | Stage use cases return typed outcomes; adapters write stdout/stderr. | This is the boundary that lets both surfaces share one implementation without a writer parameter in the domain layer. |
| D9 | Preserve stage-specific error types rather than forcing one giant `AppError`. | Error strings are observable behavior; explicit types keep the messages stable. |
| D10 | No schema or header changes; byte-identical artifacts are the equivalence gate. | The refactor workflow forbids behavior drift. |
| D11 | One leaf issue `In Progress` at a time unless the human authorizes parallel work. | Moves touch overlapping files; serial execution avoids conflicts. |
| D12 | The issue tree is a parent container plus one sub-issue per phase; each PR closes one sub-issue. | Matches the board and execution-plan lifecycle. |
| D13 | Move the shared presentation-label mapping (`shell/src/labels.rs`) to the app; the app uses it for human-readable artifact text and the shell uses it for terminal text. | Both surfaces must render the same words, and a bundle writer cannot be split across layers. The model still publishes numbers; labels remain outside the model crate. |

## 6. Target architecture

### 6.1 Crate flow

```
shell (CLI)                       desktop (Tauri + Svelte)
   \                                /
    \                              /
     quantrule-app::AppService
       ├─ workspace, dataset, date, csv, config
       ├─ artifact/ (bundle writer, provenance, model bundle, backtest bundle)
       ├─ stages/ (data, model, strategy, execution, portfolio, backtest, qualification, analytics)
       └─ memory
             │
             ├─ quantrule-data
             ├─ quantrule-model
             └─ quantrule-backtest
```

The shell and desktop are adapters. Neither holds stage orchestration. The
engines remain pure.

### 6.2 App crate layout after P3.4

```
crates/quantrule-app/src/
  lib.rs
  service.rs                  // thin AppService methods; no stage math
  workspace.rs
  dataset.rs
  date.rs
  csv.rs
  pipeline.rs
  fingerprint.rs
  error.rs
  export.rs
  config/
    mod.rs, data.rs, model.rs, model_v4.rs, execution.rs, portfolio.rs,
    run.rs, strategy.rs, qualification/, spec/   // model-spec parser moves here
  artifact/
    mod.rs                    // ensure_available, write_stage_bundle, write_bundle
    bundle.rs                 // atomic bundle writer
    provenance.rs             // StageProvenance reader/writer, one error type
    labels.rs                 // shared numeric-code-to-word mapping
    model.rs                  // model bundle assembly + CSV/JSON artifacts
    report.rs                 // summary.txt and result JSON
    backtest.rs               // backtest manifest + stage provenance reader
  stages/
    mod.rs
    data.rs                   // candles, candles-tolerant, timeframes
    model.rs                  // events, report, spec run
    strategy/                 // bundle, run, artifacts, context
    execution/                // decisions, run
    portfolio/                // fills, run
    backtest.rs
    qualification/            // evaluate, evidence, render, run
    analytics/                // benchmarks, evidence, evidence_parse, metrics, render, run
  memory/
```

### 6.3 Shell layout after P3.4

```
crates/quantrule-shell/src/
  main.rs
  command/
    mod.rs, dispatch.rs, parse.rs, types.rs, help.rs, help/
    data.rs, model.rs, strategy.rs, execution.rs, portfolio.rs,
    backtest.rs, qualify.rs, analytics.rs, memory.rs
  request.rs                  // parsed CLI arguments only
  render.rs                   // candles / timeframes NDJSON
  render_flat.rs              // events
  source/                     // explicit fetch only
  export.rs                   // fetch CSV atomic writer
                              // labels come from quantrule-app::artifact::labels
```

The shell's stage directories (`analytics/`, `backtest/`, `execution/`,
`portfolio/`, `qualification/`, `strategy/`) and artifact modules
(`model_bundle/`, `render_report/`, `backtest_bundle.rs`, `spec.rs`) are removed
when their last consumer moves.

### 6.4 Desktop backend layout after P4

```
apps/desktop/src-tauri/src/
  lib.rs
  commands/
    mod.rs                    // register/re-export
    views.rs                  // serializable view types + From impls
    scope.rs                  // path scope, normalization, validation
    workspace.rs              // workspace, dataset, run, export, record
    memory.rs                 // memory status/lists/CRUD/export/import/backup
    pipeline.rs               // P3.4 stage commands
    tests.rs                  // moved command tests
```

### 6.5 Model layout after P2/P12

P2 moves `events/signals.rs` to `events/extended/signals.rs` and replaces glob
imports with explicit imports. P12 is conditional: if approved, `ExtendedReadings`
becomes a set of family structs (`vwap`, `profile`, `participation`, `flow`,
`volatility`, `structure`, `fibonacci`, `band`, `anatomy`), and the extended
event modules are renamed only where the name collides with a base module.

## 7. Work breakdown

Each phase is one sub-issue and one PR unless noted. Roles are the minimum; the
orchestrator discloses sequential passes when one agent performs several roles.

**Move rule:** a move updates every consumer and the module map in the same PR.
A downstream shell stage that still exists calls the new app module directly;
no temporary duplicate module is left behind. When that downstream stage moves
in a later slice, it carries the already-updated call with it.

### P0 — Baseline and guardrails

**Role:** Planner / Test.

- Create the formal refactor assessment
  `docs/refactors/2026-11-12/clarity-refactor.md` from the findings CLAR-01..09,
  with the baseline table, evidence, options, and status `Open`. Map the working
  IDs to `REF-2026-11-12-01..09`. The Planner verifies each finding against the
  current code and marks it `Triaged` or `Planned`. The assessment is read-only.
- Create the parent issue with claim type `Behaviour Claim` (the one-program
  boundary). Give each sub-issue the claim type that fits its deliverable:
  P1 `Problem Claim` (documentation friction); P2–P12 `Behaviour Claim`
  (behavior-neutral structure or parity); P13 `Behaviour Claim` (closure).
- Create one worktree per sub-issue, named `quant-rule--<task-slug>`, on a
  `refactor/...`, `docs/...`, or `feat/...` branch.
- Preserve the pre-existing untracked `data/improvement/2026-11-08/`,
  `2026-11-09/`, and `2026-11-10/` directories in the main worktree. Do not
  stash, reset, clean, or move them.
- Record the baseline in this plan: commit, toolchain, test inventory, golden
  fixture hash, report bundle hash, backtest bundle hash, structural report.
- Confirm `tools/gate.sh` is green before any move.
- Add the AI transparency log skeleton with the same base name.

**Paths:** `docs/execution-plans/`, `docs/ai-transparency-log/`, repository
root, `crates/`, `apps/desktop/`.

**Depends on:** human approval.

**Verification:** V1.

### P1 — Documentation truth pass and module map

**Role:** Architect / Implementation / Test.

- Fix the doc comments listed under CLAR-02 and CLAR-03:
  - `crates/quantrule-shell/src/main.rs` — one of two first-class surfaces.
  - `crates/quantrule-backtest/src/lib.rs` — the runner and context exist.
  - `crates/quantrule-backtest/src/strategy/config.rs` — the context exists.
  - `crates/quantrule-backtest/src/strategy/mod.rs` — the two engines are the
    current generic engines, not future work.
  - `crates/quantrule-app/src/lib.rs` — name the remaining shell spec-parser
    exception until P3.4a lands.
  - `apps/desktop/src-tauri/src/lib.rs` — the host does path-scope validation;
    it does not resolve domain paths or parse configs.
  - `crates/quantrule-model/src/timeframe.rs` — "the scalar readings", not
    "six".
  - `crates/quantrule-model/src/report/flat.rs` — the flat projection, without
    a stale model-version label.
  - `crates/quantrule-shell/src/command/help/topics.rs` — M1.3 is current.
  - `crates/quantrule-data/src/lib.rs` and `crates/quantrule-backtest/src/lib.rs`
    — source access is app + shell fetch, not shell only.
- Add `docs/architecture/module-map.md`:
  - Crate roles and the one-way graph.
  - Model module map: base readings/events, extended readings/events,
    progression, context, report, spec, timeframe, run.
  - Shell module map: command adapters, request vocabulary, renderers, fetch.
  - App module map: config, pipeline, artifact, stages, memory.
  - Desktop module map: commands, views, scope, panels.
  - Reading order for a new contributor.
  - "Where to add" checklist for a reading, event, indicator state, command,
    configuration field, stage use case, and desktop command.
- Link the map from `docs/README.md` (Current architecture) and
  `docs/architecture/README.md` (Documents table and reading order).
- Update the help glossary line from M1.0 to M1.3 and add or update the CLI
  help test assertion. This is the one intentional visible text change in the
  campaign; record it in the changelog.
- Update the command-surface docs only if the command contract changes (it
  should not).

**Paths:** the files above, `docs/architecture/module-map.md`,
`docs/README.md`, `docs/architecture/README.md`.

**Depends on:** P0.

**Verification:** V2, V3, V21.

### P2 — Model naming and source-code constants

**Role:** Architect / Implementation / Test / Code Reviewer.

- Use `git mv` to move `crates/quantrule-model/src/events/signals.rs` to
  `crates/quantrule-model/src/events/extended/signals.rs`.
- Update `events/extended.rs` to declare `mod signals;` and call
  `signals::compute(...)`.
- Remove `pub mod signals;` from `events/mod.rs`.
- Replace `use super::*` in `events/extended/*.rs` and
  `readings/extended/*.rs` with explicit imports. Keep `shared.rs` only if it
  still has one cohesive purpose and a module doc.
- Update the module docs:
  - `events/extended.rs` — list the actual event families, not "first batch:
    VWAP and volume profile".
  - `readings/extended.rs` — list the actual reading families, not the first
    batch.
  - `events/signals.rs` — "extended single-instrument signal events", not
    "new".
- Create `crates/quantrule-model/src/events/source_codes.rs`:
  - One canonical constant for every source the model emits, in numeric order,
    with a doc line.
  - Re-export from the existing event modules so public imports and integration
    tests keep working during the transition.
  - Add a unit test that every `*_SOURCE_CODE` value is unique and within the
    registry's source range.
  - Extend `tools/check_registry.py` to parse `events/source_codes.rs` and
    compare the numeric set to `codes.sources`; fail on a missing or extra
    source. This prevents the constants and the registry from drifting.
- Optionally add a `check_structure.py` advisory that reports near-duplicate
  module names in one parent (`signal`/`signals`); do not make it a hard gate
  until the tree passes.
- Do not rename CSV headers, artifact fields, or event codes.

**Paths:** `crates/quantrule-model/src/events/`,
`crates/quantrule-model/src/readings/extended/`, `tools/check_registry.py`,
`tools/check_structure.py` (optional).

**Depends on:** P1.

**Verification:** V4, V5, V23.

### P3 — Shared stage-bundle reader and error cleanup

**Role:** Architect / Implementation / Test / Code Reviewer.

- Create `crates/quantrule-app/src/artifact/provenance.rs`:
  - `StageProvenance` with `schema`, `stage`, `artifact`, `data_version`,
    `config_id`, `code_commit`, `source_artifacts`, `seed`, and `period`.
  - `StageIdentity` or `ExpectedStage { stage, artifact }`.
  - `StageBundleError` with `Read`, `Invalid`, and `UnsupportedSchema`. The
    error carries a bundle label (`model`, `strategy`, `execution`, `portfolio`)
    so the existing Display text stays exact.
  - `load_model_provenance(path)` for `quantrule.provenance/v1` (the model
    bundle's `report`, `config_hash`, `period`).
  - `load_stage_provenance(path, expected_stage, expected_artifact)` for
    `quantrule.stage-provenance/v1`.
  - `read_json`, `read_text`, and optional `config_id`/`config_hash` helpers.
- Preserve every existing Display string. The app error is wrapped by the shell
  stage errors, so the printed text must not change.
- Refactor `crates/quantrule-shell/src/strategy/bundle.rs`,
  `crates/quantrule-shell/src/execution/decisions.rs`, and
  `crates/quantrule-shell/src/portfolio/fills.rs` to use the shared reader.
- Rename the duplicate error types:
  - `strategy/bundle.rs::ModelBundleError` → the shared `StageBundleError`.
  - `backtest_bundle.rs::BundleError` → `ManifestError`.
  - `render_report/bundle.rs::BundleError` → `BundleWriteError`.
- Add unit tests in the app crate for:
  - missing file, invalid JSON, non-object root;
  - wrong schema, wrong stage, wrong artifact;
  - missing/empty `data_version`, missing/empty `config_id`;
  - model provenance `config_hash` and period parsing.
- Do not move the stage readers to the app yet; P5–P9 do that. The shared reader
  is already in the app so the move does not duplicate it.

**Paths:** `crates/quantrule-app/src/artifact/`,
`crates/quantrule-shell/src/strategy/bundle.rs`,
`crates/quantrule-shell/src/execution/decisions.rs`,
`crates/quantrule-shell/src/portfolio/fills.rs`,
`crates/quantrule-shell/src/backtest_bundle.rs`,
`crates/quantrule-shell/src/render_report/`.

**Depends on:** P2 (no file overlap, but keep the model build stable).

**Verification:** V6, V7.

### P4 — Desktop command split

**Role:** Architect / Implementation / Test / Code Reviewer / Security.

- Create `apps/desktop/src-tauri/src/commands/`:
  - `mod.rs` — module declarations and re-exports. Keep the 24 command names
    and `#[tauri::command(rename_all = "snake_case")]` attributes.
  - `views.rs` — every serializable view struct and its `From` impl.
  - `scope.rs` — `Scope`, the process statics, `scope()`, `set_workspace`,
    `set_memory`, `normalize`, `validate_workspace_path`,
    `validate_output_path`, `validate_memory_db`, `open_memory`, `text`, and a
    `reset_for_test()` helper.
  - `workspace.rs` — `open_workspace`, `dataset_summary`, `check_run`,
    `run_model`, `export_run_summary`, `record_run`.
  - `memory.rs` — all memory status/list/CRUD/export/import/backup/restore
    commands.
  - `tests.rs` — the existing `#[cfg(test)] mod tests`, with imports updated.
- Update `apps/desktop/src-tauri/src/lib.rs` `generate_handler!` to the new
  paths. If Tauri's macro requires the original module path, keep thin
  `pub use` wrappers in `commands/mod.rs`.
- Keep behavior identical:
  - same command names and argument names;
  - same path-scope errors;
  - same JSON field names and ordering;
  - same database open/close rules.
- The `Security` review checks that path scope still covers every new module and
  that no file outside the workspace/memory roots is reachable.

**Paths:** `apps/desktop/src-tauri/src/commands.rs` (remove after move),
`apps/desktop/src-tauri/src/commands/`, `apps/desktop/src-tauri/src/lib.rs`.

**Depends on:** P0. Independent of P1–P3, but run after P3 to avoid touching
the same shell files.

**Verification:** V8, V20.

### P5 — P3.4a: data and model use cases

**Role:** Architect / Implementation / Test / Code Reviewer.

- Move the shared presentation-label mapping `shell/src/labels.rs` to
  `app/src/artifact/labels.rs`; both the artifact writer and the shell terminal
  renderers use it. The model still publishes numbers only.
- Create the app artifact layer:
  - `artifact/bundle.rs` — `ensure_available`, `write_bundle`,
    `write_stage_bundle`, `write_atomic`, `BundleWriteError` (moved from
    `shell/render_report/bundle.rs` and `shell/export.rs`). The shell keeps its
    fetch-specific `PendingCsv`; the generic atomic writer has one home.
  - `artifact/provenance.rs` — the shared reader from P3 plus the existing
    provenance writer(s).
  - `artifact/report.rs` — `summary_bytes`, `json_bytes`, `model_result_bytes`
    (moved from `shell/render_report/{mod,json}.rs`).
  - `artifact/model.rs` — the model bundle assembly and CSV/JSON artifact
    writers (moved from `shell/model_bundle/`).
- Create `stages/data.rs`:
  - `inspect_candles(path, window)` returns the retained window and quality
    report.
  - `inspect_candles_tolerant(path)` returns the window plus the quality
    report.
  - `inspect_timeframes(path)` returns the timeframe set.
- Create `stages/model.rs`:
  - `compute_events(path, window)` returns the model run.
  - `run_report(request)` loads the run, parses the optional spec, runs the
    model over the spec ladder, writes the model bundle, and returns a
    `ReportOutcome`.
- Move the model-spec parser:
  - `shell/src/spec.rs` → `app/src/config/spec.rs`.
  - `shell/src/spec/v2.rs` → `app/src/config/spec/v2.rs`.
  - Move `shell/src/command/report_run/spec.rs` helpers (`spec_timeframes`,
    `load_panel_series`, `load_model`) into the app stage. Keep the shell's
    `pipeline_error`/`spec_error` mapping in the command adapter; the app
    returns its own stage error.
  - Fix the `include_str!` path for `docs/registry/indicators.json`.
- Add `AppService` methods:
  - `inspect_candles`, `inspect_candles_tolerant`, `inspect_timeframes`,
    `compute_events`, `run_model_report`.
- Make the shell commands thin:
  - `command/data.rs` calls the service and renders.
  - `command.rs::run_events` calls the service and renders.
  - `command/report_run/mod.rs` parses args, calls the service, and prints the
    artifact paths or the `--json` payload.
- Delete the shell `model_bundle/`, `render_report/` artifact code, and
  `spec.rs`/`spec/v2.rs` in the same PR that updates their consumers. The
  backtest stage, which is still in the shell at this point, calls the new app
  model-bundle writer directly; it carries that call when it moves in P8. Keep
  any terminal rendering helper in the shell.
- Preserve:
  - the report bundle bytes;
  - the `events` NDJSON bytes;
  - the `candles`/`candles-tolerant`/`timeframes` stdout and stderr;
  - every spec parse and validation error string;
  - the `--json` stdout bytes.

**Paths:** `crates/quantrule-app/src/{artifact,stages,config}/`,
`crates/quantrule-app/src/service.rs`, `crates/quantrule-shell/src/command/`,
`crates/quantrule-shell/src/labels.rs` (move),
`crates/quantrule-shell/src/model_bundle/` (remove),
`crates/quantrule-shell/src/render_report/` (remove artifact parts),
`crates/quantrule-shell/src/spec.rs` (remove), `crates/quantrule-shell/src/spec/`
(remove).

**Depends on:** P1–P4.

**Verification:** V9, V15, V23.

### P6 — P3.4b: strategy use cases

**Role:** Architect / Implementation / Test / Code Reviewer.

- Move the strategy stage into the app:
  - `shell/src/strategy/bundle.rs` → `app/src/stages/strategy/bundle.rs`.
  - `shell/src/strategy/run.rs` → `app/src/stages/strategy/run.rs`.
  - `shell/src/strategy/artifacts.rs` → `app/src/stages/strategy/artifacts.rs`.
  - `shell/src/strategy/context.rs` → `app/src/stages/strategy/context.rs`.
- Split computation from terminal rendering:
  - The app builds the decisions, records, summary, NDJSON, provenance, and
    bundle. It returns stable numeric/wire codes and typed records, never
    free-form presentation labels.
  - The app returns `StrategyOutcome { root, artifacts, json }` and a typed
    `StrategyExplanation` for `explain`.
  - The shell `command/strategy.rs` maps parsed args to app requests, maps
    numeric codes to labels, and prints the explanation or the artifact
    paths/JSON.
- Add `AppService::explain_strategy` and `AppService::run_strategy`.
- Keep the model-bundle reader in the app stage (it uses the shared provenance
  reader from P3).
- Preserve:
  - `strategy run` decisions.ndjson and summary.txt bytes;
  - `strategy explain` stdout text and exit code;
  - the unsupported compression-bracket error;
  - the context validation errors.

**Paths:** `crates/quantrule-app/src/stages/strategy/`,
`crates/quantrule-app/src/service.rs`, `crates/quantrule-shell/src/command/`,
`crates/quantrule-shell/src/strategy/` (remove after move).

**Depends on:** P5 (artifact writer + model bundle reader in app).

**Verification:** V10, V23.

### P7 — P3.4c: execution and portfolio use cases

**Role:** Architect / Implementation / Test / Code Reviewer.

- Move:
  - `shell/src/execution/decisions.rs` → `app/src/stages/execution/decisions.rs`.
  - `shell/src/execution/run.rs` → `app/src/stages/execution/run.rs`.
  - `shell/src/portfolio/fills.rs` → `app/src/stages/portfolio/fills.rs`.
  - `shell/src/portfolio/run.rs` → `app/src/stages/portfolio/run.rs`.
- Add `AppService::run_execution` and `AppService::replay_portfolio`.
- Return `ExecutionOutcome` and `PortfolioOutcome` with root, artifacts, and
  the `--json` payload. The shell prints.
- Preserve:
  - orders.csv, fills.csv, rejections.csv, summary.txt, provenance.json bytes;
  - trades.csv, equity.csv, funding.csv, risk_events.csv, metrics.json,
    summary.txt, provenance.json bytes;
  - the data-version, timeframe, bar-count, and bracket errors;
  - the `--json` output.

**Paths:** `crates/quantrule-app/src/stages/{execution,portfolio}/`,
`crates/quantrule-app/src/service.rs`, `crates/quantrule-shell/src/command/`,
`crates/quantrule-shell/src/{execution,portfolio}/` (remove after move).

**Depends on:** P5–P6.

**Verification:** V11, V23.

### P8 — P3.4d: backtest use case

**Role:** Architect / Implementation / Test / Code Reviewer.

- Move:
  - `shell/src/backtest/run.rs` → `app/src/stages/backtest.rs`.
  - `shell/src/backtest_bundle.rs` → `app/src/artifact/backtest.rs`.
- Add `AppService::run_backtest`.
- The app backtest calls the app strategy, execution, and portfolio stage
  modules directly; it imports no shell module.
- Return `BacktestOutcome { root, manifest, stages, json }`; the shell prints
  the bundle paths or JSON.
- Preserve:
  - the model, strategy, execution, and portfolio stage bundles;
  - manifest.json and its stage provenance hashes;
  - the canonical stage order;
  - the empty-period and missing-timeframe errors;
  - the `--json` output.

**Paths:** `crates/quantrule-app/src/stages/backtest.rs`,
`crates/quantrule-app/src/artifact/backtest.rs`,
`crates/quantrule-app/src/service.rs`, `crates/quantrule-shell/src/command/`,
`crates/quantrule-shell/src/backtest/` and `backtest_bundle.rs` (remove after
move).

**Depends on:** P5–P7.

**Verification:** V12, V23.

### P9 — P3.4e: qualification and analytics use cases

**Role:** Architect / Implementation / Test / Code Reviewer.

- Move `shell/src/qualification/*` to `app/src/stages/qualification/`.
- Move `shell/src/analytics/*` to `app/src/stages/analytics/`.
- Add `AppService::qualify` and `AppService::evaluate_analytics`.
- Return `QualificationOutcome` and `AnalyticsOutcome`; the shell prints.
- Preserve:
  - qualification JSON, text, and provenance bytes;
  - analytics metrics, benchmark, evaluation, and provenance bytes;
  - the `human_decision: null` rule and the no-verdict boundary;
  - unsupported-schema and unsupported-field errors;
  - the `--json` output.

**Paths:** `crates/quantrule-app/src/stages/{qualification,analytics}/`,
`crates/quantrule-app/src/service.rs`, `crates/quantrule-shell/src/command/`,
`crates/quantrule-shell/src/{qualification,analytics}/` (remove after move).

**Depends on:** P8 (both consume the backtest artifact reader).

**Verification:** V13, V23.

### P10 — P3.4f: parity, shell thinning, architecture guards

**Role:** Architect / Implementation / Test / Code Reviewer.

- Remove the now-empty shell stage modules and artifact modules.
- Keep `shell/src/source/` and `shell/src/export.rs` for explicit `fetch`.
- Update `crates/quantrule-shell/tests/architecture.rs`:
  - allowed crate edges remain one-way;
  - app must not depend on shell;
  - no shell stage module may call `pipeline::run_model`, construct an
    `ExecutionEngine`, or drive a `PortfolioLedger`;
  - no app module may import `crate::command`.
- Add a parity test:
  - one fixed scenario (the fixture run + event-composite strategy) executed
    through `AppService` and through the shell command path;
  - compare stdout/stderr/exit code and the artifact bytes;
  - use the same relative output root or normalize the path prefix before
    comparing text, because printed artifact paths include the output directory;
  - if a full duplicate run is too slow, run the model report and one stage
    bundle from both paths; document the limitation.
- Add a `use_case_inventory` test or checker that lists every stage command and
  its `AppService` method, so a new command without a shared use case fails.
- Update `docs/architecture/README.md`, `docs/architecture/command-surface.md`,
  and `docs/README.md` to show the app service as the owner of every stage use
  case.
- Update `apps/desktop/README.md` after P3.4g.

**Paths:** `crates/quantrule-shell/src/`, `crates/quantrule-shell/tests/`,
`crates/quantrule-app/src/`, `docs/architecture/`.

**Depends on:** P5–P9.

**Verification:** V14, V15, V16.

### P10b — P3.4g: desktop backend pipeline commands

**Role:** Implementation / Test / Security.

- Add `apps/desktop/src-tauri/src/commands/pipeline.rs`:
  - one Tauri command per `AppService` stage method;
  - serializable outcome views in `commands/views.rs`;
  - all path arguments validated by `commands/scope.rs`;
  - `fetch` is not added.
- Register the commands in `lib.rs`.
- Add backend tests:
  - one model report command against the fixture;
  - one stage command (strategy or execution) against a fixture bundle;
  - out-of-scope input and output paths rejected.
- The UI can call these commands; the visible panels land in P11.

**Paths:** `apps/desktop/src-tauri/src/commands/`,
`apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src/lib/types.ts` (types
only, no UI yet).

**Depends on:** P10.

**Verification:** V16, V20.

### P11 — P3.5: desktop pipeline UI

**Role:** Requirements / Architect / Implementation / Test / Security / Human.

- Add a `Pipeline` section to the desktop UI with stage tabs:
  - Data: inspect candles, tolerant candles, timeframes.
  - Model: events, report.
  - Strategy: explain, run.
  - Execution: run.
  - Portfolio: replay.
  - Backtest: run.
  - Qualification: evaluate.
  - Analytics: evaluate.
- Reuse one generic stage form/result component where the inputs are the same;
  do not build nine bespoke designs.
- Each stage shows the artifact directory and the artifact names returned by the
  service. The UI does not parse an artifact or compute a metric.
- Keep `fetch` shell-only. Show a recorded exception in the Settings/help text:
  "Source fetch is shell-only until the network-boundary decision is approved."
- Add Vitest tests for the stage forms and one Playwright E2E for a model report
  plus one later stage.
- This phase requires explicit human approval of the UI scope. If it is not
  approved, record the parity exception and stop after P10b.

**Paths:** `apps/desktop/src/App.svelte`,
`apps/desktop/src/lib/PipelinePanel.svelte` (or equivalent),
`apps/desktop/src/lib/{api,types}.ts`, `apps/desktop/e2e/`,
`apps/desktop/README.md`.

**Depends on:** P10b.

**Verification:** V17, V20.

### P12 — Conditional model extended clarity

**Role:** Structure Auditor / Refactor Planner / Implementation / Test /
Refactor Verifier.

- This is a separate behavior-neutral refactor campaign. It starts only after a
  separate assessment and human approval. P2 already lands the small fixes.
- 12a: group `ExtendedReadings` into family structs:
  - `VwapReadings`, `ProfileReadings`, `ParticipationReadings`, `FlowReadings`,
    `VolatilityReadings`, `StructureReadings`, `FibonacciReadings`,
    `BandReadings`, `AnatomyReadings`.
  - Move each family's `compute`/`empty` into its module.
  - Update the event detectors and the app CSV writer to use the nested paths.
  - Keep `readings.csv` headers and bytes identical.
- 12b: rename extended event modules only where the name collides:
  - `events/extended/divergence.rs` → `vwap_divergence.rs` (or a name the
    assessment agrees);
  - `events/extended/levels.rs` → `channels.rs` (or a name the assessment
    agrees).
- Add a field-family table to the module docs and the module map.
- If the diff budget is exceeded, stop after 12a or defer the whole phase and
  record the finding as `Deferred`; P3.4 is not blocked.

**Paths:** `crates/quantrule-model/src/readings/extended*`,
`crates/quantrule-model/src/events/extended*`,
`crates/quantrule-app/src/artifact/model.rs`.

**Depends on:** P10 (the CSV writer is in the app).

**Verification:** V18, V23.

### P13 — Closure

**Role:** Test / Code Reviewer / Release / Human.

- Run the full gate and the desktop gate; record observed results.
- Recompute the baseline values and compare:
  - golden fixture hash;
  - report bundle hash;
  - backtest bundle hash;
  - representative CLI stdout/stderr/exit codes;
  - test count (must not decrease).
- Update `CHANGELOG.md` under `## [Unreleased]`.
- Update the AI transparency log with actual work, verification, and rejected
  suggestions.
- Update the plan status and verification table.
- Update `docs/README.md` status only if the architecture status changes.
- Record residual risks and exceptions:
  - `fetch` remains shell-only;
  - native E2E exception, if the environment cannot run it;
  - P12 deferred, if not approved.
- Merge after human acceptance; clean up every worktree and local/remote branch;
  move the sub-issues and parent to `Done` with `Closed date`.
- Do not create a release tag.

**Paths:** `CHANGELOG.md`, `docs/`, plan file, board.

**Depends on:** all approved phases.

**Verification:** V19–V26.

## 8. Issue tree and PR sequence

| Order | Branch | Issue | Deliverable | Depends on |
|---:|---|---|---|---|
| 1 | `docs/clarity-truth-pass` | P1 | Stale docs fixed + module map | P0 |
| 2 | `refactor/model-module-names` | P2 | `signals.rs` moved; explicit imports; source codes | P1 |
| 3 | `refactor/stage-bundle-reader` | P3 | Shared provenance reader; error cleanup | P2 |
| 4 | `refactor/desktop-command-split` | P4 | `commands/` split | P3 |
| 5 | `refactor/p34a-data-model-use-cases` | P5 | App data/model use cases; spec move | P4 |
| 6 | `refactor/p34b-strategy-use-cases` | P6 | App strategy use cases | P5 |
| 7 | `refactor/p34c-execution-portfolio-use-cases` | P7 | App execution/portfolio use cases | P6 |
| 8 | `refactor/p34d-backtest-use-case` | P8 | App backtest use case | P7 |
| 9 | `refactor/p34e-qualification-analytics-use-cases` | P9 | App qualification/analytics use cases | P8 |
| 10 | `refactor/p34f-parity-shell-thinning` | P10 | Guards and parity test | P9 |
| 11 | `feat/p34g-desktop-pipeline-commands` | P10b | Desktop backend stage commands | P10 |
| 12 | `feat/desktop-pipeline-parity` | P11 | Desktop stage UI (approval gate) | P10b |
| 13 | `refactor/model-extended-families` | P12 | Family structs (conditional) | P10 + approval |
| 14 | `chore/clarity-p34-closure` | P13 | Gate, records, cleanup | All approved |

One seam per PR. Moves use `git mv`. A PR that needs a logic edit, a schema
change, or a new dependency stops and returns to the plan.

## 9. Acceptance criteria

Observable conditions:

1. No stale boundary claim remains in the code doc comments; the module map is
   linked from the docs index and covers every crate and stage.
2. `events/signals.rs` no longer exists; `events/extended/signals.rs` exists;
   the extended submodules have explicit imports; source-code constants have one
   definition.
3. One shared stage-provenance reader serves the strategy, execution, and
   portfolio bundles; the duplicate `ModelBundleError`/`BundleError` names are
   gone.
4. The desktop command file is split into cohesive modules; all 24 command
   names, argument names, and error strings are unchanged.
5. Every pipeline stage has exactly one `AppService` use case; the shell and
   desktop call it; no shell stage module drives an engine.
6. The model-spec parser lives in the app configuration layer; the shell has no
   `crate::spec` parser.
7. The report, strategy, execution, portfolio, backtest, qualification, and
   analytics bundle bytes are identical before and after each move.
8. The CLI stdout/stderr/exit codes are identical before and after each move,
   except the documented help glossary correction from M1.0 to M1.3.
9. The desktop exposes the shared stages through Tauri commands; P3.5 exposes
   them in the UI or records an approved exception.
10. The full gate, desktop checks, and native E2E (or an approved exception)
    pass.
11. The changelog, AI log, plan table, board, and cleanup records are complete.
12. No model, strategy, execution, portfolio, or analytics result changes; no
    Quant Validator record is required, and the golden fixtures are unchanged.

## 10. Verification

### 10.1 Commands

Baseline and every slice:

```bash
git rev-parse HEAD
rustc --version && cargo --version && python3 --version
cargo test --workspace --no-fail-fast
find crates -path '*/fixtures/*' -type f | sort | xargs sha256sum | sha256sum
python3 tools/check_structure.py --report
tools/gate.sh
```

Report bundle:

```bash
rm -rf target/clarity-report
target/debug/quantrule report model \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/clarity-report
(cd target/clarity-report/model && LC_ALL=C sha256sum * | LC_ALL=C sort -k2 | sha256sum)
```

Backtest bundle:

```bash
rm -rf target/clarity-backtest
target/debug/quantrule backtest run \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/clarity-backtest
(cd target/clarity-backtest/backtest && find . -type f | sort | xargs sha256sum | sha256sum)
```

Stage bundles: capture the output of each stage command before and after the
slice; compare the directory tree hashes.

CLI output: run the commands in `crates/quantrule-shell/tests/cli.rs` with a
fixed fixture, capture stdout, stderr, and exit code, and compare SHA256.

Desktop:

```bash
cd apps/desktop
npm run check
npm test
npm run test:e2e
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run tauri build
```

Native E2E: `tauri-driver` + `WebKitWebDriver` if available; otherwise record a
dated exception and keep the launch smoke plus the bridge test.

Checkers:

```bash
python3 tools/check_file_size.py
python3 tools/check_structure.py
python3 tools/check_registry.py
python3 tools/check_command_surface.py
python3 tools/check_links.py
python3 tools/check_terms.py
python3 tools/check_event_emitters.py
python3 tools/check_verification_matrix.py
python3 tools/check_verification.py
python3 tools/check_validation.py
```

## Verification table

The implementer fills `Observed`; the reviewer fills `Status`, `Reviewer`, and
`Date`. `Status` is `Pass`, `Fail`, `Blocked`, `Pending`, or `Accepted risk`.

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | P0 baseline | Baseline recorded and gate green | `git rev-parse HEAD`; `cargo test --workspace -- --list`; fixture SHA256; `tools/gate.sh` | exact values recorded | commit `d2ed0c7`; 1,505 tests listed; fixture SHA256 `fdcf3118…`; gate pass | Pass | Agent | 2026-11-12 | plan and assessment committed |
| V2 | P1 stale docs | No stale boundary claims | `rg -n 'only surface\|S3\\.x work\|six scalar\|M1\\.0 view\|M1\\.0 +The current' crates apps`; `check_links`; `check_terms` | no stale matches; checks pass | no stale matches; links and terms pass | Pass | Agent | 2026-11-12 | help glossary corrected |
| V3 | P1 module map | One code map exists and is linked | `test -f docs/architecture/module-map.md`; `rg -n 'module-map' docs/README.md docs/architecture/README.md` | file exists and is linked | file exists; both indexes link it | Pass | Agent | 2026-11-12 | updated for the moves |
| V4 | P2 module names | `signals.rs` moved; no glob imports | `test ! -f crates/quantrule-model/src/events/signals.rs`; `test -f crates/quantrule-model/src/events/extended/signals.rs`; `rg -n 'use super::\\*' crates/quantrule-model/src/events/extended crates/quantrule-model/src/readings/extended` | old file gone; zero glob imports | old file gone; zero glob imports; model tests pass | Pass | Agent | 2026-11-12 | `events/extended/signals.rs` |
| V5 | P2 source codes | One definition per source code | `rg -n 'SOURCE_CODE: u16' crates/quantrule-model/src`; `cargo test -p quantrule-model`; `python3 tools/check_registry.py` | no duplicate values; tests and registry pass | constants only in `events/source_codes.rs`; registry passes | Pass | Agent | 2026-11-12 | registry checks the numeric set |
| V6 | P3 shared reader | Three stage readers use one provenance reader | `rg -n 'quantrule\\.stage-provenance/v1' crates/quantrule-app/src crates/quantrule-shell/src`; `cargo test -p quantrule-app`; `cargo test -p quantrule-shell` | one parser; tests pass | one app parser; strategy/execution/portfolio use it | Pass | Agent | 2026-11-12 | app tests added |
| V7 | P3 error names | No duplicate `ModelBundleError`/`BundleError` in shell | `rg -n 'enum ModelBundleError\|enum BundleError' crates/quantrule-shell/src` | no duplicate names | `StrategyBundleError`, `BundleWriteError`, `ManifestError`; no duplicate `BundleError` | Pass | Agent | 2026-11-12 |  |
| V8 | P4 desktop split | Commands split; behavior unchanged | `wc -l apps/desktop/src-tauri/src/commands/*.rs`; `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`; `cargo clippy --manifest-path ...`; `npm test` | largest file under 350; tests pass | largest file 247; 5 backend tests pass; clippy pass | Pass | Agent | 2026-11-12 | frontend untouched |
| V9 | P5 data/model | Data/model use cases in app; outputs identical | `target/debug/quantrule report model ...`; `events`; `candles`; `timeframes`; bundle hash | bundle/stdout identical | report/CLI tests pass; data/model use cases in `stages/{data,model}.rs` | Pass | Agent | 2026-11-12 | artifact/spec moved to app |
| V10 | P6 strategy | Strategy use cases in app; outputs identical | `strategy run`; `strategy explain`; decisions hash | identical | 19 strategy/backtest command tests pass | Pass | Agent | 2026-11-12 | app `stages/strategy` |
| V11 | P7 execution/portfolio | Execution/portfolio use cases in app; outputs identical | `execution run`; `portfolio replay`; bundle hashes | identical | 16 execution/portfolio tests pass | Pass | Agent | 2026-11-12 | app stage modules |
| V12 | P8 backtest | Backtest use case in app; outputs identical | `backtest run`; manifest hash | identical | 9 backtest tests pass | Pass | Agent | 2026-11-12 | app `stages/backtest` |
| V13 | P9 qualification/analytics | Qualification/analytics use cases in app; outputs identical | `qualify`; `analytics evaluate`; bundle hashes | identical | 19 qualification/analytics tests pass | Pass | Agent | 2026-11-12 | app stage modules |
| V14 | P10 shell thinning | No stage orchestration in shell | `rg -n 'pipeline::run_model\|ExecutionEngine::new\|PortfolioLedger' crates/quantrule-shell/src`; `cargo test -p quantrule-shell --test architecture` | no orchestration; guard passes | shell holds only command, fetch, render, request; guard passes | Pass | Agent | 2026-11-12 |  |
| V15 | P10 spec move | Spec parser only in app | `test ! -f crates/quantrule-shell/src/spec.rs`; `rg -n 'crate::spec' crates/quantrule-shell/src`; report hash | parser gone; report hash identical | parser in `app/config/spec`; shell clean | Pass | Agent | 2026-11-12 |  |
| V16 | P10b desktop commands | Every stage use case has a Tauri command | `cargo test --manifest-path ...`; `cargo clippy ...`; `npm test`; parity test | commands call app; tests pass | 12 stage commands; bridge and parity tests pass | Pass | Agent | 2026-11-12 |  |
| V17 | P11 desktop UI | Stages visible and usable, or approved exception | `npm run check`; `npm test`; `npm run test:e2e` | pass or accepted risk | Pipeline panel; check/test/build pass; E2E not run | Pass | Agent | 2026-11-12 | browser E2E not run |
| V18 | P12 extended split (conditional) | Family structs, identical artifacts | `cargo test -p quantrule-model`; report bundle hash; `readings.csv` header/bytes | identical | deferred by the approved conditional scope; REF-06 remains Deferred | Accepted risk | Human | 2026-11-12 | not required for P3.4 |
| V19 | P13 full gate | Workspace gate passes | `tools/gate.sh` | pass | gate passed with `CARGO_TARGET_DIR=target/gate` | Pass | Agent | 2026-11-12 | fmt, clippy, test, doc, checkers |
| V20 | P13 desktop gate | Desktop checks pass | commands in 10.1 | pass | backend tests/clippy, frontend check/test, browser E2E, native build, and launch smoke pass | Pass | Agent | 2026-11-12 | WebDriver E2E not run; launch smoke exits 124 as expected |
| V21 | P13 docs/checkers | Command surface and docs checkers pass | `check_command_surface`, `check_links`, `check_terms`, `check_workflows`, `check_changelog`, `check_releases`, `check_claims` | pass | all checkers passed in `tools/gate.sh` | Pass | Agent | 2026-11-12 |  |
| V22 | P13 records | Changelog, AI log, plan table complete | file review | complete | changelog and AI log updated; table filled | Pass | Agent | 2026-11-12 |  |
| V23 | All slices | No behavior drift except the documented help correction | golden fixture hash; report/backtest/CLI hashes; help output diff | identical except the M1.0→M1.3 help line | parity test and all stage command tests pass; help fixture updated | Pass | Agent | 2026-11-12 |  |
| V24 | Closure | Worktrees/branches removed; board Done | `python3 tools/worktree_cleanup.py check <worktree>`; `git worktree list`; `git branch -a` | clean; board Done | pending merge/acceptance | Pending | Agent | — | PR #628 open |
| V25 | All slices | No new dependency | `git diff -- Cargo.toml Cargo.lock apps/desktop/src-tauri/Cargo.toml` | no new dependency | no dependency changed | Pass | Agent | 2026-11-12 |  |
| V26 | All slices | No test weakened; count not down | `cargo test --workspace -- --list`; fixture diff | count equal or higher; fixtures unchanged | 1,505 tests listed; no test removed; golden fixtures unchanged | Pass | Agent | 2026-11-12 |  |

## 11. Risks and recovery

| Risk | Impact | Control | Recovery |
|---|---|---|---|
| Large file moves cause merge conflicts | Rework and lost time | One stage per PR; `git mv`; rebase before review; no parallel moves | Rebase the slice; stop the queue until the tree is green |
| Artifact bytes drift | Auditability and determinism claim break | Bundle hashes before/after every slice; golden fixtures unchanged | Revert the slice; fix the move before continuing |
| CLI output/error strings drift | Observable behavior change | Capture stdout/stderr/exit codes; preserve Display strings | Revert the slice; restore the exact strings |
| App crate grows too large | Readability and compile time | Split `artifact/` and `stages/`; file-size gate; one stage per module | Split the module before adding the next stage |
| App starts to own presentation | Boundary erosion | App returns typed outcomes; shell owns terminal rendering and labels | Move the presentation code back to the adapter |
| Desktop command split breaks Tauri registration | Desktop build fails | Keep the 24 names; test `generate_handler!`; clippy/build | Revert to thin re-export wrappers; fix the paths |
| P3.5 UI scope grows | Schedule and design risk | One generic stage form; human approval gate; backend lands first | Stop after P10b; record the parity exception |
| Native E2E environment missing | Gate gap | Try `tauri-driver` + `WebKitWebDriver`; keep launch smoke + bridge test | Record a dated human-approved exception |
| Extended family split is too large | Churn without behavior value | Separate assessment; diff budget; stop after 12a | Defer P12; keep the module map and naming fixes |
| Fetch network path requested in desktop | Local-first promise weakens | Keep shell-only; separate human decision | Stop and request the network-boundary decision |
| Test count drops | Coverage claim weakens | Ratchet V26; no test deletion or skip | Restore the test; do not merge |
| New dependency needed | Approval gate | No dependency in this plan | Stop and request approval |

## 12. Stop conditions

Stop and request human direction when:

- a move requires a logic change, a schema change, or a behavior change;
- a bundle hash, CLI output, exit code, or error string changes and the fix is
  not a pure move;
- a test must be removed, skipped, weakened, or a golden fixture regenerated;
- a new dependency, public crate, or module-boundary decision is needed;
- the app would depend on the shell, or an engine would gain I/O;
- the desktop UI needs a product/design decision or a new dependency;
- the desktop `fetch` path is requested without a separate decision;
- P3.5 or P12 approval is missing;
- the native E2E cannot run and no exception is approved;
- a worktree or branch cannot be cleaned safely;
- a board item would need to be deleted, archived, or moved back from `Done`.

## 13. Quant validation and security review

**Quant validation:** No Quant Validator record is required for the plan as a
whole because no model, strategy, execution, portfolio, qualification, or
analytics result changes. The evidence is the opposite: golden fixtures, bundle
hashes, and CLI outputs must be identical. If a slice would change a numeric
value, the slice is not a refactor and stops. The plan records this in every PR
under `Quant validation:`.

**Security review:** Required for P4, P10b, and P11. The review checks:

- path scope for every new desktop command and every new output path;
- no arbitrary filesystem access from the webview;
- no new network path;
- no secret in a log, artifact, or command output;
- the `fetch` exception remains explicit.

## 14. Records and closure

- **Issues:** parent container + one sub-issue per phase; claim types as in
  section 7/P0; dependencies as in section 8.
- **Board:** `Todo` → `In Progress` (with start date) → `In Review` (PR open,
  checks green) → `Done` (accepted, merged, verified, cleaned up, closed date).
- **Plan:** Proposed → Approved → In Progress → In Review → Verified.
- **AI log:** one file per work item, same base name; actual use, verification,
  rejected suggestions.
- **Changelog:** under `## [Unreleased]`:
  - `Changed`: pipeline stages now run through one `AppService` use-case layer;
    shell and desktop are thin adapters.
  - `Changed`: shared stage-provenance reader and split desktop command modules.
  - `Added`: code module map and parity test.
  - `Fixed`: stale module docs and the help model-version reference.
  - No edge claim, no model/strategy/execution/portfolio behavior change, no new
    dependency. The help glossary correction is recorded under `Fixed`.
- **Release:** no tag. A release note is created only if the human authorizes a
  release after the gate.
- **Cleanup:** every merged sub-issue removes its worktree and local/remote
  branch; the `Done` board items remain.

## 15. Traceability

| Finding | Phase | Verification |
|---|---|---|
| CLAR-01 model naming | P2, P12 | V4, V18 |
| CLAR-02 stale boundary docs | P1 | V2 |
| CLAR-03 factual drift | P1 | V2 |
| CLAR-04 duplicated readers | P3 | V6, V7 |
| CLAR-05 desktop command file | P4 | V8 |
| CLAR-06 `ExtendedReadings` | P12 | V18 |
| CLAR-07 shell orchestration | P5–P10 | V9–V15 |
| CLAR-08 no code map | P1 | V3 |
| CLAR-09 source constants | P2 | V5 |
| AUD-2026-11-11-11 one-program parity | P5–P11 | V9–V17 |
| AUD-2026-11-11-16 native E2E | P11, P13 | V17, V20 |
| Parent plan P3.4 | P5–P10b | V9–V16 |
| Parent plan P3.5 | P11 | V17 |

P0 records the formal refactor assessment and maps the working CLAR IDs to
`REF-2026-11-12-01..09`. The traceability above is the working map; the
assessment is the status record.

## 16. Approval

**Requested decisions:**

1. Approve the scope and order in this plan.
2. Approve the issue tree and the suggested filename
   `docs/execution-plans/2026-11-12/clarity-and-p34-shared-pipeline.md`
   (issue number TBD).
3. Confirm P3.5 desktop UI is in scope, or record it as a follow-on parity
   exception.
4. Confirm the `fetch` exception (shell-only) for this campaign.
5. Confirm D13: the shared label mapping moves to the app so artifact text and
   terminal text use one source. If rejected, the plan falls back to a
   shell-owned `summary.txt` renderer and an app writer callback.
6. Confirm P12 is a conditional separate refactor, or defer it explicitly.

**Gate:** no implementation until this plan is approved, the P0 refactor
assessment is published, the issue tree exists, and the plan is committed to
the repository. The implementation session must run outside Plan mode.
