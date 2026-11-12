# Structure Re-Scan — Clarity Refactor

- Date: 2026-11-12 (UTC)
- Baseline commit: `d2ed0c7`
- Scope: current Rust workspace, desktop backend, tests, documentation, and module layout
- Workflow: [`docs/workflows/refactor-workflow.md`](../../workflows/refactor-workflow.md)
- Format: [`docs/refactors/README.md`](../README.md)
- Plan: [`docs/execution-plans/2026-11-12/clarity-and-p34-shared-pipeline.md`](../../execution-plans/2026-11-12/clarity-and-p34-shared-pipeline.md)
- Status: Assessment published; findings verified by the Planner and marked `Planned`

## Baseline

| Item | Observed |
|---|---|
| Commit | `d2ed0c7` |
| Toolchain | `rustc 1.99.1`, `cargo 1.99.1` (pinned by `rust-toolchain.toml`) |
| Structural report | 223 Rust source files; 73,107 lines; 0 files over 1,000; 9 over 800; 40 over 600 |
| Crate size (including tests) | data 3,517 / 9 files; model 34,974 / 99; backtest 13,864 / 47; app 10,716 / 30; shell 22,965 / 79 |
| Largest production files | `progression/leg.rs` 955; `app/config/strategy.rs` 878; `memory/store.rs` 857; `events/level.rs` 843; `source/dukascopy.rs` 829 |
| Desktop backend | 985 lines; `commands.rs` 922 |
| Test inventory | Recorded in P0; last documented baseline is 1,476 passed, 0 failed, 8 ignored |
| Golden fixtures | Recorded in P0; last documented set is 105 files |
| Report and backtest bundles | Recorded in P0; last documented hashes are `59124f63...` and `4173b505...` |
| Gates | `tools/check_structure.py --report`, `check_file_size.py`, and the full gate are the reproducible checks |

Reproduction:

```bash
git rev-parse HEAD
rustc --version && cargo --version
cargo test --workspace -- --list
find crates -path '*/fixtures/*' -type f | sort | xargs sha256sum | sha256sum
python3 tools/check_structure.py --report
```

## Findings

| ID | Impact | Area | Finding | Evidence | Principle | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| REF-2026-11-12-01 | High | model | Model names encode delivery history, not domain | `events/signals.rs` is the extended-only detector called from `events/extended.rs:189`; `events/signal.rs` is the base detector. `events/divergence.rs` vs `events/extended/divergence.rs`; `events/level.rs` vs `events/extended/levels.rs`; 19 `use super::*` imports in the extended submodules | cohesion, readability | A move `signals.rs` under `extended` and replace glob imports; B rename every extended module; C leave | A now; B only with a domain-family assessment | `test ! -f events/signals.rs`; `rg 'use super::\*'` returns 0; model tests pass | Refactored | plan P2 |
| REF-2026-11-12-02 | Medium | docs | Stale boundary doc comments contradict the one-program decision | `shell/src/main.rs:3` "only surface"; `backtest/src/lib.rs:11` and `strategy/config.rs:4` "S3.x work"; `app/src/lib.rs:5` "neither adapter parses config"; `desktop/src-tauri/src/lib.rs:3` "no path resolution" while `commands.rs:300-374` does | readability, boundary | A truth pass with the module map; B leave the comments | A | `rg` stale phrases returns none; links/terms pass | Refactored | plan P1 |
| REF-2026-11-12-03 | Low | docs | Small factual drift in model and help docs | `timeframe.rs:31,108` says six scalar readings but there are nine; `report/flat.rs:3` says M1.0 view; `help/topics.rs:50` says M1.0 current while `docs/README.md` says M1.3 | readability | A correct the comments and add the CLI help assertion; B leave | A | `rg` stale phrases returns none; help test passes | Refactored | plan P1 |
| REF-2026-11-12-04 | Medium | shell, app | Stage-provenance readers and error types are duplicated | `strategy/bundle.rs:253`, `execution/decisions.rs:142`, `portfolio/fills.rs:96` parse the same schema; `ModelBundleError` exists twice; `BundleError` exists twice | duplication, coupling | A one app artifact reader with one error; B leave | A | three readers use one parser; duplicate enum names gone; tests pass | Refactored | plan P3 |
| REF-2026-11-12-05 | Medium | desktop | The Tauri command boundary is one 922-line file | `commands.rs` holds 24 commands, view types, path scope, path validation, memory CRUD, and import/export | cohesion, atomicity | A split by responsibility; B split views only; C leave | A | largest command module under 350 lines; desktop tests/build pass | Refactored | plan P4 |
| REF-2026-11-12-06 | Medium | model | `ExtendedReadings` has no visible family boundaries | 88 public fields; `empty` and `compute` repeat all 88; 567-line file | cohesion, readability | A family structs; B sectioned docs only; C leave | B now; A only with a separate assessment | golden and `readings.csv` bytes unchanged | Deferred | plan P12 |
| REF-2026-11-12-07 | High | shell, app | The shell still orchestrates the pipeline stages | `backtest/run.rs:150`, `report_run/mod.rs:60,86`, `execution/run.rs`, `portfolio/run.rs`; P3.4 is pending | boundary, coupling | A extract one stage at a time behind `AppService`; B leave | A | no stage orchestration in shell; parity test; bundle hashes identical | Refactored | plan P5–P10 |
| REF-2026-11-12-08 | Medium | docs | There is no code navigation map | 223 source files; docs are contract-oriented; no page maps modules and "where to add" | readability | A one `docs/architecture/module-map.md`; B per-crate READMEs; C leave | A | map exists and is linked from both indexes | Refactored | plan P1 |
| REF-2026-11-12-09 | Low | model | Source-code constants are duplicated | `EMA20/50/200_SOURCE_CODE` in `events/level.rs` and `events/signal.rs`; `RSI_SOURCE_CODE` in `events/divergence.rs` and `events/signal.rs` | duplication | A one `source_codes` module and registry check; B leave | A | one definition per source; `check_registry.py` passes | Refactored | plan P2 |

## Detail blocks

### REF-2026-11-12-01 — Model names encode delivery history

- Expected invariant: a module name predicts its contents; a reader can tell base from extended.
- Observed: `events/signals.rs` is an extended-only detector with a near-identical name to the base `events/signal.rs`; two `divergence` and two `level` modules sit in different conceptual layers; the extended submodules hide their imports behind `use super::*`.
- Structural cause: the indicator-completeness programme added families beside the base model and named the batch `extended`.
- Impact: a new reader cannot predict where a reading or event belongs; a move or rename is harder because dependencies are invisible.
- Behavior risk: low. The change is a file move, explicit imports, and doc text; no event code or numeric value changes.
- Verification: module file check, glob-import scan, `cargo test -p quantrule-model`, report bundle hash.

### REF-2026-11-12-02 / -03 — Stale boundary and factual docs

- Expected invariant: the first doc comment a reader sees matches the shipped architecture and current model identity.
- Observed: the shell is described as the only surface; the backtest runner is described as future work; the app is described as the only config parser while the shell still owns `spec.rs`; the desktop host is described as doing no path resolution; the model is described as six readings and M1.0.
- Impact: the docs teach the wrong boundary, so a reader either duplicates work or edits the wrong layer.
- Behavior risk: low for comments; the help glossary line is a visible text correction and gets a test and changelog entry.
- Verification: stale-phrase scan, `check_links`, `check_terms`, CLI help test.

### REF-2026-11-12-04 — Duplicated stage-provenance readers

- Expected invariant: one artifact schema has one parser and one error contract.
- Observed: strategy, execution, and portfolio each parse `quantrule.stage-provenance/v1` with local field checks and `read_text`; the model bundle reader has its own `ModelBundleError`.
- Impact: a schema or message change must be made in three places; the duplicate `ModelBundleError`/`BundleError` names make imports ambiguous.
- Behavior risk: medium. The shared reader must preserve every Display string exactly.
- Verification: one parser in use, error-string tests, stage bundle hashes.

### REF-2026-11-12-07 — Shell still orchestrates the pipeline

- Expected invariant: the app service owns the use cases; the shell parses, fetches, and renders.
- Observed: the shell computes the model, drives strategy/execution/portfolio, writes bundles, and parses model specs.
- Structural cause: the stages were built command-first; the app service was introduced later and the extraction was planned as P3.4.
- Impact: two places to change a stage; desktop parity is blocked; the shell's module layout is transitional.
- Behavior risk: high if moved in one step; the plan uses one stage per PR and byte-identical bundle hashes.
- Verification: no stage orchestration in shell, parity test, bundle hashes.

### REF-2026-11-12-05 — Desktop command boundary

- Expected invariant: a command module has one reason to change.
- Observed: one 922-line file holds registration, view types, path scope, path validation, memory CRUD, and import/export.
- Impact: P3.4 adds more commands to the same file; review and conflict cost grows.
- Behavior risk: low. The split keeps command names, arg names, and error strings.
- Verification: file size, desktop tests, clippy, Tauri build.

### REF-2026-11-12-06 — `ExtendedReadings` aggregate

- Expected invariant: a reader can see the reading families without scanning 88 fields.
- Observed: the flat struct repeats all 88 fields in `empty` and `compute`.
- Impact: high cognitive load; adding a field touches three places.
- Behavior risk: medium. Family structs touch every detector and the CSV writer but must keep artifact bytes.
- Decision: `Deferred` for this campaign. P2 lands the naming and module-map fixes; a separate assessment can propose family structs when the next reading family lands.
- Verification: if reopened, golden and `readings.csv` bytes unchanged.

### REF-2026-11-12-08 / -09 — Navigation and source constants

- Expected invariant: one code map and one definition per source code.
- Observed: no module map; EMA/RSI source constants exist in two modules each.
- Impact: a reader cannot find the right module; a constant can drift from the registry.
- Verification: map links pass; `check_registry.py` parses the canonical constants.

## Observations

- The hard structural gates are green: crate graph guard, module docs, file-size gate, and the structural checker.
- The largest files are within the 1,000-line gate; this campaign is about naming, boundaries, and duplication, not file size.
- The app service is the correct home for artifact I/O and stage use cases; the shell keeps terminal rendering and explicit fetch.
- The `fetch` network path is out of scope and stays shell-only.
- The model `extended` family split is deferred; the small naming traps are fixed now.

## Out of scope

- Any model, strategy, execution, portfolio, qualification, or analytics behavior change.
- Any schema, artifact-header, or numeric change.
- Any new dependency, desktop `fetch` path, or release tag.
- Any deletion of historical `tools/`, `data/`, `config/`, or `docs/` records.

## Campaign

| Item | Value |
|---|---|
| Baseline commit | `d2ed0c7` |
| Finding list | REF-2026-11-12-01..09 |
| Named metric | duplicate module names 3 -> 0; provenance parsers 3 -> 1; largest desktop command file 922 -> under 350; stale boundary phrases -> 0 |
| Cycle cap | Six cycles (P1–P4, P10, P12 if reopened) |
| Stop rule | Stop after P10 if P12 is deferred; no High finding remains open except the deferred REF-06 |

## Verification

The verification table and evidence commands are in the plan
[`clarity-and-p34-shared-pipeline.md`](../../execution-plans/2026-11-12/clarity-and-p34-shared-pipeline.md),
rows V2–V18. The Refactor Verifier checks the merged diff against the named
metrics, the equivalence contract, and the artifact hashes.
