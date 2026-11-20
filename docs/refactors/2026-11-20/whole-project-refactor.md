# Whole-Project Refactor Assessment

- Date: 2026-11-20 (UTC)
- Status: Assessment complete; REF-2026-11-20-01 and REF-2026-11-20-02 are `In progress`; decisions remain open
- Scope: repository layout, documentation, Rust modules, tests, tooling, and CI
- Workflow: [Refactor Workflow](../../workflows/refactor-workflow.md)
- Baseline commit: `8443697fec9e20a555b2fe900a36c19dae03b8cb`
- Claim type: Behaviour Claim

## Method

One agent performed the Structure Auditor pass, then the Refactor Planner pass.
The passes were sequential, not independent. No code, test, issue, or pull request
was changed while assessing. The only writes are this assessment and its plan.

Reproduce the inventory with:

```sh
git rev-parse HEAD
find crates -name '*.rs' -exec wc -l {} + | sort -rn | head -30
git ls-files 'docs/**/*.md' | wc -l
git ls-files 'docs/**/*.md' -z | xargs -0 cat | wc -l
find config -type f | wc -l
CARGO_TARGET_DIR="$PWD/target/gate" cargo test --workspace -- --list | grep -c ': test$'
```

## Baseline

| Item | Content |
|---|---|
| Commit | `8443697fec9e20a555b2fe900a36c19dae03b8cb` |
| Toolchain | `rustc 1.99.1`, `cargo 1.99.1`, `python3 3.14.10` |
| Test inventory | 2,394 tests listed by `cargo test --workspace -- --list`; 0 `#[ignore]` |
| Golden fixtures | Aggregate SHA-256 `ea8bacd83197c7c0f5b6f42abc64d73454f01742a415804a453c326e7af775a6` over `crates/**/tests/fixtures/*`, sorted by path |
| Representative output | `quantrule-shell help`: 61 lines, SHA-256 `c1c5a7c2f70704651bcfe41236c185104ed5b37bf599288de96dfd99d60891c5` |
| Gates | Full `tools/gate.sh` PASS, exit 0: fmt, clippy, 2,394 tests across 133 test binaries, rustdoc, and 27 checkers (84.44 s). `check_structure.py`, `check_file_size.py`, and `check_doc_length.py` PASS separately. |
| Repository pack | 124.89 MiB; 19,492 objects |

### Scorecard

| Metric | Before |
|---|---|
| Rust source files | 509 |
| Rust source lines | 159,527 |
| Files over 600 lines | 87 |
| Files over 800 lines | 23 |
| Files over 900 lines | 10 |
| Files over 990 lines | 2 |
| `pub` items | 2,707 |
| `pub(crate)` items | 345 |
| Tracked Markdown documents | 1,026 |
| Tracked Markdown lines | 125,293 |
| Tracked config files | 2,178 |
| AI transparency logs | 442 |
| Execution plans | 198 |
| Improvement records | 107 |
| Commit touches to `docs/README.md` in six months | 225 |

## Findings

| ID | Impact | Area | Finding | Evidence | Principle | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| REF-2026-11-20-01 | High | shell | The command pipeline is three near-cap monoliths with mixed responsibilities | `wc -l`: `output.rs` 995, `dispatch.rs` 992, `parse.rs` 933; `research create` crosses `types.rs:272`, `parse/parse_research.rs:52`, `dispatch.rs:135`, `help.rs:215`, `output.rs:678` | cohesion, boundary, atomicity | A / B / C | A | File-size gate, 2,394 tests, byte-identical help and command output | In progress | Plan |
| REF-2026-11-20-02 | High | docs | The global status index sits at its cap and is machine-coupled to four checkers | `docs/README.md` 328 lines, cap 330 (`check_doc_length.py:27`); 226 commits in six months; `c3c10cc1` compressed a 20-row table to fit; `check_workflows.py:126` requires every workflow | cohesion, readability, coupling | A / B / C | A | Index length, checker pass, every workflow still discoverable | In progress | Plan |
| REF-2026-11-20-03 | Medium | tools | Every PR runs retired and historical Python suites | `.github/workflows/ci.yml:58-69`; 153 improvement tests, 96 research tests, plus programme coverage; `tools/improvement/README.md` calls its adapters historical | coupling, testability | A / B / C | A | PR CI time, current checks unchanged, on-demand historical workflow | Open | Decision |
| REF-2026-11-20-04 | Medium | naming | `research` and `improvement` name the same record at different layers | Commands/routes `research`; types/module `Improvement*`; `AGENTS.md:71` and `docs/manual/07-improvement.md:25` say "research run"; `docs/workflows/run-contract.md:366` says "improvement run is a research record" | readability, cohesion | A / B / C | A | Term inventory, command surface, no wire change | Open | Decision |
| REF-2026-11-20-05 | Medium | docs | `AGENTS.md` repeats lists owned by other documents | 103,345 bytes, 1,263 lines; section 2 enumerates 46 commands; section 11 enumerates 20 workflows; section 10 is 375 lines of roles | readability, coupling | A / B / C | A | Agent context size, no drift, index authorities unchanged | Open | Decision |
| REF-2026-11-20-06 | Info | Rust | The 600-line advisory is crossed widely, but most large files are cohesive | 87 files over 600; 23 over 800; 10 over 900. Largest non-shell: `adapters/filesystem.rs` 964 (one struct/impl), `progression/leg.rs` 955 (one `Leg` impl), `api/improvement.rs` 928 (one contract aggregate) | size, cohesion | A / B | A | Review trigger, not a target | Open | None |
| REF-2026-11-20-07 | Info | layout | Historical experiment config is interleaved with current config | 2,178 files; `config/research/` 1,119; `config/sr*` and `config/sbe-01` 838; `config/README.md` calls them evidence, not current | layout, readability | A / B / C | Defer | Data Lifecycle decision; no path change in this campaign | Open | Data Lifecycle |
| REF-2026-11-20-08 | Info | docs | Historical records are 70% of the documentation mass | 442 logs + 198 plans + 107 improvement + 41 reports = 788 files, 88,504 lines of 125,293 | readability | A / B | Defer | Recordkeeping decision; no deletion | Open | Recordkeeping |
| REF-2026-11-20-09 | Info | tools | Port-parity fixtures are 24 MB of the 125 MiB pack | `PORT-001.datasets.json` 16.9 MB, `PORT-001.golden.json` 7.0 MB | layout | A / B / C | Defer | Clone and gate time | Open | None |
| REF-2026-11-20-10 | Low | deps | The gateway declares `quantrule-service-api` as a production dependency but only its tests use it | `crates/quantrule-gateway/Cargo.toml` `[dependencies]`; `rg` finds uses only in `tests/surface.rs:26` and `tests/gateway.rs:27`; `src/` has none | coupling, boundary | A / B / C | A | `cargo tree -p quantrule-gateway`, tests, boundary guard | Open | Follow-up |

## Findings in detail

### REF-2026-11-20-01 — Shell command pipeline is three near-cap monoliths

- Expected invariant: one command family is one cohesive module; changing a
  command touches a small, local surface; rendering is separate from transport
  and local file I/O.
- Observed: `output.rs` 995 lines, `dispatch.rs` 992, `parse.rs` 933,
  `help.rs` 618, and the command integration test 989. `output.rs` groups
  improvement 179 lines, stage 175, run 87, simulation 81, dataset 56, config 51,
  and baseline 51; `dispatch.rs` groups run 382 and simulation 192; `parse.rs`
  holds 534 lines of command parsing. The `research create` path alone crosses
  at least five files.
- Structural cause: only the stage and research families were extracted into
  their own modules. Dataset, config, run, inspect, profile, simulation,
  baseline, and verify remain in the central files. `output.rs` also mixes the
  generic JSON writers (`write_json_line`, `write_records`) with human
  rendering, and `dispatch.rs` mixes HTTP calls with dataset upload and run
  fetch file I/O.
- Impact: High. A command change edits files at the 1,000-line cap, so review
  and merge conflicts are paid on every change. The module map says a command
  starts in `shell/src/command/<family>.rs`, which is untrue for most families.
- Affected contracts/docs: `docs/architecture/command-surface.md`,
  `docs/architecture/module-map.md`, shell `help`, and the command tests.
- Behavior risk: Low if the cycle is move-only. `Command` variants, parse
  results, render functions, and output bytes must not change.
- Dependencies: none.
- Effort: Medium, one cycle and one PR.
- Metric before: largest touched source file 995 lines; one command change
  crosses at least five files.
- Metric target: no touched shell module over 400 lines; one family module per
  command family; the same five-file contract, but each file small and named.

#### Options

A. Split `parse.rs`, `dispatch.rs`, and `output.rs` into family modules
(`dataset`, `config`, `run`, `stage`, `research`, `baseline`, `inspect`,
`simulation`, `profile`, `verify`, `health`) with a thin `mod.rs` facade. Keep
generic JSON writers in `output/json.rs` and local file I/O in a separate
module. Move each function and its tests without editing logic.
B. Introduce a command descriptor table that co-locates name, parse, dispatch,
and render. This is a new abstraction across all 46 commands and a larger
behavior risk.
C. Split only `output.rs`, the largest file. Lower effort, but parse and
dispatch stay at the cap and the command contract remains scattered.

#### Recommendation

A. It is a pure move along seams that already exist, and it makes the module
map true again. B is a design change, not a refactor. C does not remove the
merge-conflict surface.

#### Verification

`cargo fmt`, `cargo clippy --all-targets`, `cargo test` with 2,394 tests,
`cargo doc`, `check_file_size.py`, and byte-identical `help` and representative
command output before and after. Review the diff with move detection and stop
on any logic edit.

#### Links

Plan: `docs/execution-plans/2026-11-20/whole-project-refactor.md`.

### REF-2026-11-20-02 — The global status index is at its cap and machine-coupled

- Expected invariant: the global index is a map. Each status has one owning
  document, and adding a workflow does not require compressing unrelated
  history.
- Observed: `docs/README.md` is 328 lines against a 330-line cap. It has been
  touched by 226 commits in six months. Commit `c3c10cc1` replaced a 20-row
  workflow table with a five-line paragraph to stay under the cap. Four
  checkers read the file: `check_workflows.py` requires every workflow to be
  listed, `check_client_docs.py` checks counts, `check_releases.py` checks the
  release state, and `check_legacy.py` scans it.
- Structural cause: navigation, current status, model status, run contracts,
  and release state are conflated in one capped file, while the owning indexes
  already exist under `docs/architecture/`, `docs/model/`,
  `docs/workflows/`, and `docs/releases/`.
- Impact: High for documentation work. The most contended file is also the one
  with the least room, so a status update can force the deletion of unrelated
  rows.
- Affected contracts/docs: `docs/README.md`, `docs/workflows/README.md`,
  `docs/architecture/README.md`, `docs/model/`, `docs/releases/README.md`,
  and the four checkers above.
- Behavior risk: Low for the product, but the checkers are a gate. The same
  assertions must remain; only the file each checker reads may move.
- Dependencies: none.
- Effort: Medium, one cycle and one PR.
- Metric before: 328 lines; four checkers read the index.
- Metric target: at most 120 lines; each checker reads the owning index; a
  workflow addition touches only `docs/workflows/README.md`.

#### Options

A. Reduce `docs/README.md` to navigation: kind directories, living entry
points, and a short "where status lives" table. Move each status table to its
owning index. Update the four checkers to read the owning index while keeping
the same assertions.
B. Keep the status tables but generate them from one machine-readable
`docs/status.json`. The index becomes generated output and checkers read the
JSON. This adds a generator and a new source of truth.
C. Raise the cap to 500 and keep the current structure. This defers the
problem and increases duplication.

#### Recommendation

A. It removes the coupling without inventing a new artifact. B is acceptable
only if the status is already machine-readable, which it is not.

#### Verification

`check_doc_length.py`, `check_workflows.py`, `check_client_docs.py`,
`check_releases.py`, `check_legacy.py`, and `check_links.py`. A mapping table in
the PR proves every moved status row has an owner and no row was deleted.

#### Links

Plan: `docs/execution-plans/2026-11-20/whole-project-refactor.md`.

### REF-2026-11-20-03 — Historical Python suites run on every PR

- Expected invariant: the pull-request gate runs the checks that protect the
  change; historical reproduction runs on demand or at release.
- Observed: `.github/workflows/ci.yml` runs the historical improvement suite
  (153 tests), the historical research suite (96 tests), and a programme
  coverage check on every push and pull request. `tools/improvement/README.md`
  says its run adapters are historical and retained for pre-cutover
  reproduction. The tree carries 144 Python files, 56 of them at the `tools/` root.
- Structural cause: current and historical tooling share one workflow and one
  test discovery step.
- Impact: Medium. Every PR pays for code that is not on the product surface,
  and a historical tool failure can block an unrelated change.
- Affected contracts/docs: `docs/workflows/release-operations.md`,
  `docs/workflows/improvement-workflow.md`, and the run contract.
- Behavior risk: None to the product; this changes what PR CI enforces, which
  is a methodology decision, not a refactor.
- Dependencies: a human decision on which research tooling is current.
- Effort: Small.
- Metric before: 249 historical Python tests per PR.
- Metric target: current checks in PR CI; historical checks on
  `workflow_dispatch` and before a release.

#### Options

A. Split the CI steps: current research tooling stays in PR CI; historical
improvement adapters and the M2.x-M4.x reproduction checks move to an
on-demand workflow and the release checklist.
B. Keep the steps but path-filter them to `tools/improvement/**`,
`tools/research/**`, and `config/research/**`.
C. Retire the historical tools after archiving their outputs. This is a
recordkeeping and data-lifecycle decision, not a refactor.

#### Recommendation

A, with B as the fallback if the human wants a single workflow. C is not
recommended without a separate retention decision.

#### Verification

Compare PR CI time and the job list before and after. The current checks must
still run on every PR; the historical workflow must pass on demand.

#### Links

Decision requested in the plan; no code change in this campaign.

### REF-2026-11-20-04 — `research` and `improvement` name the same record

- Expected invariant: one term per concept. A layer may qualify a term, but the
  same object is not called two names without a written mapping.
- Observed: the command surface, routes, and screens say `research`; the
  contract module and types say `Improvement*`; `help` says "Create one v3
  improvement-run pre-registration" and prints `improvement_id`;
  `AGENTS.md:71` and `docs/manual/07-improvement.md:25` say "A research run";
  `docs/workflows/run-contract.md:366` says "An improvement run is a research
  record"; the architecture glossary says "A research record is always written
  **improvement run**".
- Structural cause: the client surface was named `research` while the domain
  record was named `improvement run`, and the distinction was never written
  down as a hierarchy.
- Impact: Medium. A new contributor or agent cannot tell whether research and
  improvement are one concept, two, or a surface and its record.
- Affected contracts/docs: `AGENTS.md`, `docs/architecture/README.md`,
  `docs/architecture/command-surface.md`, `docs/manual/07-improvement.md`,
  `docs/workflows/run-contract.md`, shell `help`.
- Behavior risk: A code-only type rename is neutral. Help and manual wording is
  user-visible output, so changing it is not a pure refactor.
- Dependencies: a human decision on the canonical term.
- Effort: Small for the wording fix; medium for a code rename.
- Metric before: four phrasings for one record.
- Metric target: one glossary entry and one sentence per layer, with no
  ambiguous "research run" in current docs or help.

#### Options

A. Keep the hierarchy and write it down: `research` is the client surface and
the verb; `improvement run` is the record and the domain object. Replace
"research run" with "improvement run" in current docs and help.
B. Rename the domain to `research run` everywhere: `ResearchRun*` types,
`api/research.rs`, `docs/research/`, and the `research` schema. This is a large
rename and a terminology decision.
C. Rename the commands to `improvement ...` to match the types. This breaks the
public command contract and needs a deprecation path.

#### Recommendation

A. It matches the existing manifest schema (`quantrule.improvement-run/v3`)
and the Improvement Workflow, and it needs no wire or command change. B is a
second-best if the human prefers the surface term everywhere. C is not
recommended.

#### Verification

A term inventory over current docs, help, and routes; the command surface and
`help` output reviewed as an intentional documentation change; no JSON field or
command name changes in the code-only part.

#### Links

Decision requested in the plan; wording change is outside the refactor
equivalence contract because help text is output.

### REF-2026-11-20-05 — `AGENTS.md` repeats lists owned elsewhere

- Expected invariant: the authority for a list is one document; `AGENTS.md`
  states the normative rule and links to the authority.
- Observed: `AGENTS.md` is 103,345 bytes and 1,263 lines. Section 2 enumerates
  all 46 commands, section 11 enumerates 20 workflows, and section 10 is 375
  lines of role definitions. The command list is checked against `AGENTS.md`
  (`check_command_surface.py` reads it as normative), but the workflow and role
  lists are not all checked there.
- Structural cause: the agent contract grew into a second copy of the command,
  workflow, and role catalogs.
- Impact: Medium. Every agent session pays the context cost, and the copies can
  drift from the authorities (`docs/architecture/command-surface.md` and
  `docs/workflows/README.md`).
- Affected contracts/docs: `AGENTS.md`, `docs/workflows/README.md`,
  `docs/architecture/command-surface.md`, `docs/roles/`.
- Behavior risk: None to the product; this is a methodology text change and
  needs human approval.
- Dependencies: approval of the documentation hierarchy.
- Effort: Medium.
- Metric before: 1,263 lines, three duplicated catalogs.
- Metric target: the catalogs are links; `AGENTS.md` keeps the normative rules
  and the decision boundaries.

#### Options

A. Replace the command, workflow, and role enumerations with links to their
authorities, keeping the normative rules and the "what must never happen"
statements. Save roughly 150 lines and remove drift.
B. Keep the enumerations and add checkers for every copy. This raises the
maintenance cost.
C. Split `AGENTS.md` into a short core contract plus linked role and workflow
annexes. This is the largest change and needs a methodology decision.

#### Recommendation

A. It preserves the contract while making the authorities single-source.

#### Verification

`check_command_surface.py`, `check_workflows.py`, and `check_links.py`; a line
and byte count; a review that every normative rule survived.

#### Links

Decision requested in the plan; methodology change.

### REF-2026-11-20-06 — File-size distribution is a review trigger

- Expected invariant: the 1,000-line cap is a prompt to look, not a packing
  target. Cohesive modules are not split for the count.
- Observed: 87 of 509 files exceed 600 lines, 23 exceed 800, 10 exceed 900, and
  2 exceed 990. Outside the shell, the largest files are cohesive:
  `runtime/adapters/filesystem.rs` is one struct and one trait impl,
  `model/progression/leg.rs` is one `Leg` impl, and `api/improvement.rs` is one
  contract aggregate.
- Impact: Info. Only the shell files in REF-01 show mixed responsibilities.
- Recommendation: treat 600 as a review trigger; do not split the cohesive
  files. No action beyond REF-01.

### REF-2026-11-20-07 — Historical config is interleaved with current config

- Expected invariant: current run config is easy to distinguish from archived
  evidence.
- Observed: 2,178 tracked files. `config/research/` alone holds 1,119 M2.x
  programme files; `config/sr*` and `config/sbe-01` hold 838 more. `config/README.md`
  already labels them historical evidence and says the current shell does not
  load them.
- Impact: Info. Search and review noise, not product behavior.
- Recommendation: defer. Moving or deleting evidence is a Data Lifecycle
  decision, not a refactor. If pursued, add a machine-readable current/archive
  manifest first.

### REF-2026-11-20-08 — Historical records dominate the documentation mass

- Expected invariant: the working documentation is easy to search; historical
  evidence is retained without competing with current guidance.
- Observed: 1,026 Markdown files and 125,293 lines. AI transparency logs (442),
  execution plans (198), improvement records (107), and reports (41) account for
  788 files and 88,504 lines, about 70% of the lines.
- Impact: Info. This is governed evidence, not dead weight. Deleting it would
  violate the recordkeeping rules.
- Recommendation: defer to Recordkeeping. The lever is process (one log per
  work item, plans only for substantial work, per-year indexes), not a bulk
  deletion or a path move.

### REF-2026-11-20-09 — Port-parity fixtures are 24 MB

- Expected invariant: parity evidence stays reproducible without dominating
  the repository.
- Observed: `PORT-001.datasets.json` is 16.9 MB; `PORT-001.golden.json` is
  7.0 MB; the pack is 124.89 MiB.
- Impact: Info. Clone and pack size, not day-to-day development.
- Recommendation: defer. If clone time becomes a problem, compress the
  fixtures and decompress in the checker; do not replace them with a release
  asset, because the gate must run offline.

### REF-2026-11-20-10 — The gateway declares a test-only dependency as production

- Expected invariant: a crate's production dependency list is the set it needs
  to compile its library and binaries.
- Observed: `crates/quantrule-gateway/Cargo.toml` lists
  `quantrule-service-api` under `[dependencies]`. `src/` never names it. The
  integration tests `tests/surface.rs:26` and `tests/gateway.rs:27` import
  `DatasetObject` from it. The boundary test permits the import but does not
  require it.
- Structural cause: the dependency was declared for the integration tests and
  never moved to `[dev-dependencies]`.
- Impact: Low. It widens the gateway's declared production surface and
  contradicts the boundary story; it does not affect runtime behavior.
- Affected contracts/docs: `crates/quantrule-gateway/src/lib.rs` and
  `docs/architecture/module-map.md` both say the gateway links the crate.
- Behavior risk: None. Dev-dependencies are available to integration tests,
  and `Cargo.lock` already carries the crate through `quantrule-runtime`.
- Dependencies: none.
- Effort: Small, one line plus two doc sentences.
- Metric before: one test-only crate in a production dependency list.
- Metric target: zero.

#### Options

A. Move the dependency to `[dev-dependencies]` and update the two sentences
that describe the gateway's links.
B. Leave it as a declared permission. The cost is a wider production surface
and a misleading boundary statement.
C. Remove it and import through a runtime re-export. This adds indirection to
save one line and is not recommended.

#### Recommendation

A. It is the smallest change that makes the manifest tell the truth.

#### Verification

`cargo tree -p quantrule-gateway`, `cargo test -p quantrule-gateway`,
`tests/service_boundary.rs`, and `check_structure.py`.

#### Links

Separate follow-up PR; not in this campaign because it touches `Cargo.toml` and
the gateway, not the shell or the docs index.

## Examined and not findings

| Area | Observation |
|---|---|
| Crate dependency graph | One-way and acyclic. `quantrule-gateway` depends only on `quantrule-api`, `quantrule-runtime`, and `quantrule-service-api`; engine crates do not reach the services. `service_boundary.rs` enforces it. |
| Engine purity | `check_structure.py` passes the purity, naming, layout, and data-allowlist rules. |
| Test discipline | 2,394 tests, 0 ignored, no `test_1`-style names, golden fixtures present. |
| Panic hygiene | 15 non-test `unwrap`/`expect` occurrences, all in `quantrule-runtime` memory test doubles and lock guards; no production panic path found. |
| Banned module names | None in `crates/`. The two `shared.rs` files carry module docs and one purpose. |
| Dead code | 8 `#[allow(dead_code)]`, all in integration-test support modules. |
| Naming, candle vs bar | The glossary use is consistent: a candle is the OHLCV record, a bar is the time position. The confusing term is `research` vs `improvement` (REF-04), not candle/bar. |
| Dependencies | No new dependency is proposed. One test-only production dependency in the gateway is recorded as REF-2026-11-20-10 for a separate follow-up. |
| Browser client | 13,147 lines. Largest file `SimulationChart.svelte` 731; the generated label table is 354. No mixed-responsibility file at the scale of the shell monoliths. |

## Limitations

- Timings are from one workstation; Cargo cache state was not controlled.
- CI duration was not measured. The 249 historical Python tests are counted
  from source, not from a CI run.
- "Development slow" is inferred from file size, files touched per command, and
  CI content. No developer-time study was performed.
- The baseline gate was observed green at commit `8443697f`; the assessment does
  not claim any gate after a change.
- Historical records and config were assessed for structure, not for content.
  No deletion or path move is proposed.
- One agent performed the audit and planning passes. No independent review is
  claimed.

## Campaign recommendation

A two-cycle campaign, one seam per cycle and one PR per seam:

| Cycle | Finding | Seam | Named metric |
|---|---|---|---|
| 1 | REF-2026-11-20-01 | Shell parse, dispatch, output, and command tests split by family | Largest touched file 995 to at most 400 lines |
| 2 | REF-2026-11-20-02 | Global docs index reduced to navigation; checkers read the owning indexes | 328 to at most 120 lines; workflow addition touches one index |

Cycle cap: 2. Stop when both findings are `Verified`, the budget is exhausted,
or a behavior change is required. Naming, CI, `AGENTS.md`, config, record,
fixture, and dependency findings are outside this campaign because they either
change user-visible output, change what the gate enforces, or belong to Data
Lifecycle, Recordkeeping, or a separate one-line dependency fix.
