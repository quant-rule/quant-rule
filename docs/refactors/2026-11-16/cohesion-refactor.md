# Structure Re-Scan — Cohesion Refactor

- Date: 2026-11-16 (UTC)
- Baseline commit: `439a0639`
- Scope: current Rust workspace, service binaries, browser client, tests,
  documentation, and module layout
- Workflow: [`docs/workflows/refactor-workflow.md`](../../workflows/refactor-workflow.md)
- Format: [`docs/refactors/README.md`](../README.md)
- Predecessor: [`2026-11-12/clarity-refactor.md`](../2026-11-12/clarity-refactor.md)
- Status: Assessment published; findings `Open`

## Baseline

| Item | Observed |
|---|---|
| Commit | `439a0639` (`Merge pull request #835 from quant-rule/docs/course-methodology-10-10`) |
| Toolchain | `rustc 1.99.1`, `cargo 1.99.1` (pinned by `rust-toolchain.toml`); `Python 3.14.10` |
| Crate inventory | 16 workspace crates; 345 Rust source files; 103,175 lines |
| Structural report | 0 files over 1,000; 13 over 800; 56 over 600; `pub` items 4,295; `pub(crate)` items 245 |
| Largest files | `portfolio/ledger_tests.rs` 969 (test-only); `progression/leg.rs` 955; `runtime/adapters/filesystem.rs` 951; `progression/tests.rs` 929; `app/config/strategy.rs` 878; `shell/command/dispatch.rs` 862; `shell/command/parse.rs` 858; `simulation/check.rs` 853 |
| Test inventory | 2,132 listed by `cargo test --workspace -- --list`; baseline run 2,118 passed, 0 failed, 14 ignored (119 test binaries) |
| Golden fixtures | 135 files under `crates/**/fixtures/**`; combined SHA256 `325d0c4aa9498a7b4a91566c6704a3b42358dc0f16162b9952358061f251b1dd` |
| Contract fixture | `tests/fixtures/client-parity/api.json` `0b960a3e94575eeb24711ecd09d5e9e4717d87e105e00b5a41493e0a1f01fa92`; `view.json` `e94e9d2b98ab4beaf5bbb6d882a6400efa677963bf9766a7d08d9d724dce1cc2` |
| Gates | `bash tools/checkers.sh`: 24 checkers pass. `bash tools/gate.sh`: exit 0 (fmt, clippy, test, doc) |
| Scorecard | 4 Medium and 4 Low findings; no High finding; hard structural gates green |

Reproduction:

```bash
git rev-parse HEAD
rustc --version && cargo --version && python3 --version
python3 tools/check_structure.py --report
find crates -path '*/fixtures/*' -type f | sort | xargs sha256sum | sha256sum
bash tools/checkers.sh
```

## Findings

| ID | Impact | Area | Finding | Evidence | Principle | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| REF-2026-11-16-01 | Medium | tests | Five service test suites copy the same in-memory object store | `rg -n 'pub struct MemoryObjects' crates` returns 5 copies, 57-69 lines each, 0.89-0.96 similar | duplication, testability | A / B / C | A | one definition; five suites import it; test count unchanged | Refactored | plan P1 |
| REF-2026-11-16-02 | Medium | runtime, services | Four service binaries repeat the profile database bootstrap | `data-service/src/main.rs:39`, `evidence-service/src/main.rs:52`, `gateway/src/bin/qr_gateway.rs:36`, `simulation-service/src/main.rs:38`; `backtest-service/src/main.rs:38` already calls `backends::open_database` | duplication, coupling | A / B / C | A | one branch in runtime; four binaries call one helper; health tests pass | Open | plan pending |
| REF-2026-11-16-03 | Medium | shell | Command usage text is duplicated between the help registry and the parser | 25 of 27 `help.rs` usages are byte-identical literals in `parse.rs`; example `help.rs:51` vs `parse.rs:319` | duplication, coupling | A / B / C | A | 0 duplicated usage literals; shell output tests unchanged | Open | plan pending |
| REF-2026-11-16-04 | Low | app | The timeframe code is mapped in three places | `artifact/labels.rs:8` (hand-written), `artifact/model_bundle/bundle.rs:278` and `stages/analytics/metrics.rs:323` (both from `Timeframe::ALL`) | duplication | A / B / C | A | one lookup; report and metrics tests unchanged | Open | plan pending |
| REF-2026-11-16-05 | Low | app | Stage artifact writers duplicate the engine-code mappings | `sizing_label` at `execution/run.rs:561` and `portfolio/run.rs:513`; `summary_number` at 604 and 528; `collect` at `backtest.rs:556` and `execution/run.rs:357` | duplication | A / B | A | one definition each; bundle bytes identical | Open | plan pending |
| REF-2026-11-16-06 | Low | docs | The normative crate list is stale in AGENTS.md and README.md | `AGENTS.md:48-63` and `README.md:237-250` list 14 crates; `Cargo.toml:3-18` lists 16; `quantrule-research` and `quantrule-simulation` missing | readability | A / B / C | A | both tables match `cargo metadata`; link and term checks pass | Open | plan pending |
| REF-2026-11-16-07 | Medium | client | `SimulationRunDetail.svelte` owns state, loaders, actions, and markup | 762 lines; 23 `$state` variables; 7 async flows; the other panels use `*.svelte.ts` stores | cohesion, atomicity | A / B / C | A | component under 400 lines; web unit and e2e tests pass | Refactored | plan P4 |
| REF-2026-11-16-08 | Low | app | Two different types are named `StageProvenance` in one module tree | `artifact/provenance.rs:22` (identity fields) and `artifact/backtest.rs:121` (manifest stage record) | readability | A / B | A | no duplicate type name; gate passes | Open | plan pending |

## Detail blocks

### REF-2026-11-16-01 — Five copies of the in-memory object store

- Expected invariant: one fake per trait; a trait change edits one file.
- Observed: `crates/quantrule-{model,data,evidence,simulation}-service/tests/common/mod.rs`
  and `crates/quantrule-backtest-service/tests/common/mod.rs` each define
  `MemoryObjects` over `BTreeMap<String, Vec<u8>>`. The bodies differ only in
  convenience helpers. The simulation copy's module doc says "the object store is
  the same in-memory store the other service tests use" while no shared home
  exists, and `quantrule-runtime` already ships `MemoryClient` and
  `MemoryDataClient` for the same purpose.
- Structural cause: each service test suite was written before a shared
  test-support location existed; the fake is small enough that copying looked
  cheaper than a cross-crate home.
- Impact: an `ObjectStore` change (a method, an error, a key rule) touches five
  files, and the copies have already diverged (`remove`, `store`, `read`, `keys`
  differ per suite), so two tests can exercise different fakes.
- Affected contracts/docs: `docs/architecture/module-map.md` read-first row for
  `quantrule-runtime`.
- Behavior risk: low; test-only code.
- Dependencies: none new.
- Effort: small.
- Metric before: 5 definitions; metric target: 1 definition.

#### Options
A. Add `runtime::adapters::memory::MemoryObjects` beside the trait and the
   existing memory clients; the five suites import it.
B. Add a `quantrule-testing` workspace member and depend on it as a dev-dependency.
C. Leave the copies and document them.

#### Recommendation
A. It follows the existing `MemoryClient` precedent, adds no crate and no
dependency, and keeps the fake beside the trait it implements. B adds a workspace
member for sixty lines; C leaves the drift.

#### Verification
`rg 'pub struct MemoryObjects' crates` returns one definition; the five suites
compile against it; `cargo test --workspace`; test count does not decrease.

#### Links
Plan pending.

### REF-2026-11-16-02 — Four copies of the profile database bootstrap

- Expected invariant: one place selects the database backend, applies the
  service's migration ledger, and registers the health ping.
- Observed: `data`, `evidence`, `gateway`, and `simulation` each open
  `DeploymentProfile::Desktop`, join `QR_DATA_DIR` with `DATABASE_FILE`, open
  SQLite or connect Postgres, apply the schema migrations under `config.migrate`,
  and register a `sqlite` or `postgres` ping. `backtest` calls
  `backends::open_database` instead. The module doc at
  `crates/quantrule-runtime/src/backends.rs:3` promises "Every service branches
  once, here, instead of in a route or a use case."
- Drift already present: the gateway logs the applied count as `gateway` where
  the others log `applied`; the gateway and evidence alias `apply_migrations_in`
  to `apply_postgres_migrations` while data and simulation call it directly.
- Structural cause: the database branch was written per service when each schema
  landed; the shared runtime helper arrived later for the worker and was never
  adopted by the HTTP services. `open_database`'s own doc explains why a service
  names its ledger; the duplicated part is the open, migrate, and health
  sequence, not the choice of ledger.
- Impact: a desktop storage change (WAL mode, a path rule, an error mapping, a
  new health component name) touches four binaries, and a seventh service would
  copy the block again.
- Behavior risk: medium. The health component names, the startup log fields, and
  the migrate switch are observable; the refactor must keep them identical.
- Dependencies: none new.
- Effort: medium.
- Metric before: four profile branches outside runtime; target: one helper, and
  a service binary that matches the result once.

#### Options
A. `runtime::backends::open_repository(config, schema, sqlite_migrations,
   postgres_migrations, &mut health)` returns a small enum the service matches
   once to build its store.
B. Two helpers, `open_sqlite_repository` and `open_postgres_repository`, each
   taking the schema and migrations; the service still chooses the branch.
C. Leave the copies and correct the `backends` module doc.

#### Recommendation
A. The schema name and the two migration sets are the only per-service inputs;
everything else is identical. B keeps four branches but halves each; C leaves a
documented promise the code does not keep.

#### Verification
`rg -c 'DeploymentProfile::Desktop'` on the six binaries returns zero; one
definition of the migrate/health sequence in `backends.rs`; the service health
tests and `cargo test --workspace` pass; the startup log lines are unchanged.

#### Links
Plan pending.

### REF-2026-11-16-03 — Usage text exists twice for every command

- Expected invariant: one text source for a command's usage; help and an error
  message cannot disagree.
- Observed: `crates/quantrule-shell/src/command/help.rs` holds 27 `usage:`
  strings and is the checked source of truth for the command surface (README,
  `docs/architecture/command-surface.md`, tutorials, and `AGENTS.md` are compared
  against it by `tools/check_command_surface.py`). `parse.rs` holds 33 usage
  literals, 25 of them byte-identical to the registry entry. Nothing compares the
  parser's copy with the registry.
- Structural cause: the parser was written first with per-command error text; the
  help registry was added later as the single published contract and the parser
  was not rewired.
- Impact: changing an argument in `help.rs` leaves the parser's error message
  stale, so the shell prints two contracts for one command and the checker stays
  green.
- Behavior risk: low; the parser must return the same bytes, which a test pins.
- Dependencies: none.
- Effort: small.
- Metric before: 25 duplicated literals; target: 0.

#### Options
A. Add a `help::usage(name)` (or `CommandHelp` lookup) accessor used by the parser
   and a test that every parser usage equals the registry usage.
B. Add a checker that compares both files and keep the duplication.
C. Leave.

#### Recommendation
A. It removes the copy rather than policing it, and the test moves the failure
from a future review to the gate.

#### Verification
No `usage: <command>` literal remains in `parse.rs`; the shell command tests
print the same bytes; a new test maps each command to its registry usage.

#### Links
Plan pending.

### REF-2026-11-16-04 — Three timeframe label mappings

- Expected invariant: one lookup from a timeframe in seconds to its code.
- Observed: `artifact/labels.rs:8` is a hand-written `match` over five
  timeframes; `artifact/model_bundle/bundle.rs:278` and
  `stages/analytics/metrics.rs:323` both search `Timeframe::ALL` and differ only
  in their fallback (`"15m"` and `"{seconds}s"`). The three agree today.
- Structural cause: the label table predates the `Timeframe` constants; the two
  later callers used the authoritative ladder instead of the table.
- Impact: a sixth timeframe or a renamed code updates three places; the
  hand-written table would render `unknown` in the report while the bundle writes
  the code.
- Behavior risk: low; each caller keeps its own fallback.
- Dependencies: none.
- Effort: small.
- Metric before: 3 mappings; target: 1 lookup plus caller-specific fallbacks.

#### Options
A. One seconds-to-`Timeframe` lookup in `quantrule-data`; each caller keeps its
   fallback.
B. One `app::artifact::labels::timeframe_label` with the `unknown` fallback and
   adapt the metrics caller.
C. Leave.

#### Recommendation
A. The data engine owns the timeframe set; the fallback is a presentation choice
   and stays with the caller.

#### Verification
`rg '=> "1h"'` has one occurrence; report and metrics tests pass; fixture bytes
unchanged.

#### Links
Plan pending.

### REF-2026-11-16-05 — Duplicated engine-code mappings in the stage writers

- Expected invariant: one mapping from an engine enum to its artifact code.
- Observed: execution and portfolio each define an identical `sizing_label` and
  `summary_number`; `collect` is identical in `stages/backtest.rs:556` and
  `stages/execution/run.rs:357`; `buy`/`sell`, `long`/`short`, and
  `fixed_notional`/`equity_fraction` are written in three stage modules.
- Structural cause: each stage writer landed with its own artifact and its own
  small mapping; the shared provenance reader was consolidated later but the
  value mapping was not.
- Impact: the execution and portfolio bundles can disagree on the same engine
  state, and a new sizing mode is a two-file edit.
- Behavior risk: low; the strings are artifact contract and are covered by the
  stage tests.
- Dependencies: none.
- Effort: small.
- Metric before: 3 duplicate helper pairs; target: 0.

#### Options
A. One `app::artifact::codes` module (engine enum to artifact code, number
   formatting) used by the stage writers.
B. Leave; the bundle tests guard the strings.

#### Recommendation
A. The mapping is one published vocabulary shared by two bundles, and the module
is the natural home next to `artifact::labels`.

#### Verification
One `sizing_label`, one `summary_number`, one `collect`; execution, portfolio, and
backtest bundle bytes unchanged.

#### Links
Plan pending.

### REF-2026-11-16-06 — Stale crate list in the two normative documents

- Expected invariant: the normative crate list equals the workspace members.
- Observed: `AGENTS.md:48-63` and the README table at `README.md:237-250` list
  14 crates. `Cargo.toml:3-18` lists 16; `quantrule-research` and
  `quantrule-simulation` are absent from both tables, while
  `docs/architecture/module-map.md:39-40` already documents them.
- Structural cause: the two crates were added and documented in the module map,
  and the two older tables were not revisited.
- Impact: a reader who checks the boundary from AGENTS.md or the README concludes
  the tree holds two undocumented crates; the module map is the only correct
  list.
- Behavior risk: none; documentation only.
- Effort: small.
- Metric before: 2 stale tables; target: 0.

#### Options
A. Add the two rows to both tables.
B. Replace both tables with a pointer to `module-map.md`.
C. Add a checker that compares the table rows with `cargo metadata`.

#### Recommendation
A, with C as a follow-up if the duplication returns. B removes a useful
at-a-glance list from the two documents a reader opens first.

#### Verification
Both tables name all 16 members; `check_links` and `check_terms` pass.

#### Links
Plan pending.

### REF-2026-11-16-07 — The simulation detail screen owns its whole state

- Expected invariant: a screen component renders; its state, loaders, and actions
  live in a `*.svelte.ts` store, as every other panel does.
- Observed: `apps/web/src/lib/SimulationRunDetail.svelte` is 762 lines with 23
  `$state` variables, four loaders (run, windows, series, marks), three actions
  (oracle, check, decide), and the markup. `runs`, `configs`, `datasets`,
  `profiles`, `connection`, and `simulation` each have a `*.svelte.ts` store.
- Structural cause: the screen grew from the first simulation prototype; each
  request was added inline, and the store pattern landed after it.
- Impact: the largest client file is also the only one where a request race, a
  busy flag, and markup change together; a review of one cannot be scoped to one.
- Behavior risk: low-medium; request order, DOM contract, and status text must
  not change, and the e2e suite is the guard.
- Dependencies: none.
- Effort: medium.
- Metric before: 762 lines and 23 inline state variables; target: under 400 lines
  and a `simulationRun.svelte.ts` store.

#### Options
A. Extract a `simulationRun.svelte.ts` store (state, loaders, actions) and split
   the decision panel if the component stays over 400 lines.
B. Extract only the decision/check panel, leaving the loaders inline.
C. Leave.

#### Recommendation
A. It follows the pattern the other six panels already use and separates the
request lifecycle from the markup.

#### Verification
`bun run check`, `bun test`, and the simulation Playwright specs pass; the
component is under 400 lines; no request or status text changes.

#### Links
Plan pending.

### REF-2026-11-16-08 — Two types named `StageProvenance`

- Expected invariant: a type name predicts the type.
- Observed: `artifact/provenance.rs:22` is the four identity fields read from a
  provenance file; `artifact/backtest.rs:121` is one manifest stage record with
  its directory, hash, schema, and parsed value. Two shapes, one name, one module
  tree.
- Structural cause: the reader and the manifest were written independently and
  both adopted the stage-provenance contract's name.
- Impact: an import site must alias one of them; a signature cannot be read
  without the path.
- Behavior risk: none; a rename changes no bytes.
- Effort: small.
- Metric before: 1 name collision; target: 0.

#### Options
A. Rename the reader type to `StageIdentity` and the manifest record to
   `ManifestStage`.
B. Leave and rely on the module path.

#### Recommendation
A.

#### Verification
`rg 'struct StageProvenance' crates` returns nothing; `cargo test -p
quantrule-app` and the gate pass.

#### Links
Plan pending.

- Revision (2026-11-16, P4): the screen is 344 lines and reads one store. The
  marks table, the check, and the decision moved to their own components; the
  chart gained `onMarksChanged` and `onMarkPosted` callbacks so the store owns
  the mark list without a two-way binding. The e2e suite found one lost update
  during implementation (a state spread across an `await`), which was fixed
  before the cycle closed.

## Observations

- The hard structural gates are green: crate-graph guard, module docs, the
  1,000-line cap, the structural checker, and 24 checker scripts.
- No High finding was recorded. The four Medium findings are duplication or
  cohesion friction with a bounded, behavior-neutral fix; the four Low findings
  are naming, documentation, and small duplication.
- The 600-line scan trigger now covers 56 files, up from 40 at the previous
  baseline, because six crates landed. The largest production file is
  `progression/leg.rs` at 955; the largest file overall is a test module. No file
  crosses the 1,000-line gate, so file size is a trend to watch, not a finding of
  this cycle.
- Database-implementation parity is tested, not assumed: `catalog_conformance`,
  `sqlite_conformance`, and `sqlite_parity` run the same store contract against
  SQLite and, when `QR_TEST_DATABASE_URL` is set, Postgres.
- The heuristic `unwrap`/`expect` count in non-test source is 942 lines across
  the 1,000-line-checked tree. It is a ratchet (no new instances) rather than a
  finding here; removing an existing one without changing error behavior needs
  its own assessment.
- `REF-2026-11-12-06` (`ExtendedReadings` family structs) remains `Deferred` by
  the human and is not re-opened by this scan.

## Out of scope

- Any model, strategy, execution, portfolio, qualification, or analytics
  behavior change.
- Any schema, migration content, artifact-header, or numeric change.
- Any new external dependency, desktop `fetch` path, or release tag.
- Deleting historical `tools/`, `data/`, `config/`, or `docs/` records.

## Campaign

| Item | Value |
|---|---|
| Baseline commit | `439a0639` |
| Finding list | REF-2026-11-16-01..08 |
| Named metric | duplicate `ObjectStore` fakes 5 -> 1; duplicated usage literals 25 -> 0; profile database branches in binaries 4 -> 0; largest client component 762 -> under 400 lines |
| Cycle cap | Four cycles planned (the four Medium findings); campaign cap five |
| Stop rule | Stop after the four Medium findings are `Verified`; the four Low findings stay `Open` for a later campaign unless the human adds one to this plan |

## Verification

The plan carries the verification table. The Refactor Verifier recomputes the
baseline commands above plus `bash tools/gate.sh`, the test inventory, the
fixture hash, and the two client-parity fixture hashes. A change that alters a
numeric value, a bundle byte, a command's printed text, or an error string is not
a refactor and stops the cycle.
