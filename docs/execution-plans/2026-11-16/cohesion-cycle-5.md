# Cohesion Refactor — Cycle 5

- Date: 2026-11-16 (UTC)
- Status: **Verified** (human approved 2026-11-16; PRs #836–#840 merged; verification pass recorded)
- Claim type: **Behaviour Claim** (structure changes; commands, artifacts, and client behaviour stay byte-identical)
- Owner: Human
- Workflow: [Execution Plan](../../workflows/execution-plan.md) + [Refactor Workflow](../../workflows/refactor-workflow.md)
- Refactor assessment: [`docs/refactors/2026-11-16/cohesion-refactor.md`](../../refactors/2026-11-16/cohesion-refactor.md)
- Issue: — (no issue number assigned; filename agreed when the plan is approved)
- Plan path: `docs/execution-plans/2026-11-16/cohesion-cycle-5.md`
- AI log (same base name): `docs/ai-transparency-log/2026-11-16/cohesion-cycle-5.md`

> Proposed by the Refactor Planner after the read-only assessment. No
> implementation starts until the human approves. The Structure Auditor and the
> Refactor Planner were performed by the same agent in sequential passes; this
> is not independent review.

## 1. Intent

Remove the four duplicated definitions that make the next change more expensive
than it should be, without changing one command, artifact byte, or screen:

1. the in-memory object-store fake copied into five service test suites;
2. the command usage text written twice, in the help registry and the parser;
3. the profile database bootstrap copied into four service binaries;
4. the simulation detail screen's inline state, loaders, and actions.

The intended outcome is the same behaviour with one place to change each thing.
Affected users are the human reviewing the code and the agent adding a service,
a command, a test suite, or a simulation screen action.

## 2. Current state

The assessment records the evidence, the options, and the metrics. The baseline:

| Item | Observed |
|---|---|
| Branch | `refactor/cohesion-cycle-5`, based on `main` at `439a0639` |
| Toolchain | `rustc 1.99.1`, `cargo 1.99.1`; `Python 3.14.10` |
| Structural report | 16 crates; 345 files; 103,175 lines; 0 over 1,000; 13 over 800; 56 over 600 |
| Test inventory | 2,132 listed; baseline run 2,118 passed, 0 failed, 14 ignored (119 test binaries) |
| Golden fixtures | 135 files; combined SHA256 `325d0c4a…` |
| Client-parity fixture | `api.json` `0b960a3e…`; `view.json` `e94e9d2b…` |
| Checker gate | 24 checkers pass (`bash tools/checkers.sh`) |
| Full gate | Exit 0; `target/gate` inside the worktree |

The findings selected for this plan are `REF-2026-11-16-01`, `-02`, `-03`, and
`-07`. The four Low findings (`-04`, `-05`, `-06`, `-08`) stay `Open`; the plan
does not authorize them.

## 3. Scope

Included:

- P1: one `MemoryObjects` definition for the five service test suites.
- P2: one command-usage text source for the help registry and the parser.
- P3: one profile database bootstrap in `quantrule-runtime`, used by the four
  database-owning service binaries.
- P4: a `simulationRun.svelte.ts` store and a smaller
  `SimulationRunDetail.svelte`.
- The documentation that names the changed structure, in the same PR as the
  change.

Explicitly excluded:

- `REF-2026-11-16-04`, `-05`, `-06`, `-08` (Low; stay `Open`).
- Any model, strategy, execution, portfolio, or analytics behaviour.
- Any schema, migration content, artifact header, numeric value, command name,
  argument, printed text, or error string.
- Any new external dependency, desktop packaging change, or release tag.
- `REF-2026-11-12-06`, which stays `Deferred` by the human.

## 4. Assumptions and uncertainty

- The five fakes expose a union of helpers (`keys`, `remove`, `store`, `read`).
  The shared type must expose all four so no suite loses a helper; unused ones
  are covered by `#[allow(dead_code)]` only where the suite already needs it.
- `quantrule-runtime` already ships public memory clients (`MemoryClient`,
  `MemoryDataClient`), so an unfeatured `MemoryObjects` follows the precedent
  and needs no Cargo feature. If the human prefers a test-only surface, option B
  (a `testing` feature) is the fallback.
- The database helper can preserve the exact health component names (`sqlite`,
  `postgres`), the migrate switch, and the startup log fields; the services
  build their stores from the returned handle. If a store constructor needs a
  concrete pool type, the handle exposes it.
- The client extraction changes no request order and no rendered text; the
  Playwright simulation specs are the equivalence guard.
- If any cycle needs a logic edit, a new dependency, a schema change, or a
  changed expected value, the cycle stops and returns here.

## 5. Decisions

| ID | Decision | Options | Recommendation | Status |
|---|---|---|---|---|
| D1 | Where the shared `MemoryObjects` lives | A runtime module (unfeatured); B runtime `testing` feature; C new dev-only crate | A | Awaiting approval |
| D2 | Shape of the database helper | A one `open_repository` returning a handle the service matches once; B two per-backend helpers; C docs-only correction | A | Awaiting approval |
| D3 | Usage text | A `help::usage(name)` accessor plus a parity test; B a checker over two copies; C leave | A | Awaiting approval |
| D4 | Client seam | A store module plus a decision panel if over 400 lines; B panel only; C leave | A | Awaiting approval |
| D5 | Plan filename without an issue | `cohesion-cycle-5.md` | as written | Awaiting approval |

## 6. Acceptance criteria

- One `MemoryObjects` definition in the workspace; the five suites use it; no
  test is removed, skipped, or weakened.
- No command usage literal remains in the parser; the shell prints the same
  bytes on every error path.
- No service binary branches on `DeploymentProfile`; the health component names,
  migrate switch, and startup log fields are unchanged.
- `SimulationRunDetail.svelte` is under 400 lines and keeps its DOM contract.
- The full gate is green before and after each cycle; test count does not
  decrease; fixture hashes and client-parity fixture hashes are unchanged; the
  dependency set is unchanged.
- Each finding moves `Open -> Triaged -> Planned -> In progress -> Refactored`
  and only the Refactor Verifier moves it to `Verified`.

## 7. Work breakdown

| Step | Finding | Seam | Files (expected) | Metric | Diff budget |
|---|---|---|---|---|---|
| P1 | REF-…-01 | One object-store fake | `runtime/src/adapters/mod.rs`, new `runtime/src/adapters/memory.rs`, five `tests/common/mod.rs` | 5 -> 1 definitions | ≤ 7 files; +120/−300 |
| P2 | REF-…-03 | One usage text source | `shell/src/command/help.rs`, `parse.rs`, `command/tests.rs` | 25 -> 0 duplicated literals | ≤ 4 files; +80/−80 |
| P3 | REF-…-02 | One profile database bootstrap | `runtime/src/backends.rs`, four service mains, runtime tests | 4 -> 0 binary branches | ≤ 8 files; +220/−220 |
| P4 | REF-…-07 | Client state seam | new `apps/web/src/lib/simulationRun.svelte.ts`, `SimulationRunDetail.svelte`, possibly one child component | 762 -> under 400 lines | ≤ 5 files; +400/−500 |

P2 carries one deliberate visible correction, approved by the human on
2026-11-16: the parser's `simulation marks` hint drops the false `|all`. Every
other command keeps its printed bytes.

Order: P1, P2, P3, P4. P1 and P2 are independent; P3 depends on the runtime
surface P1 touches only as a file location, not as a contract. One branch and
one PR per cycle:

- `refactor/cohesion-cycle-5` — this assessment, plan, and AI log.
- `refactor/cohesion-cycle-5-p1-test-store`
- `refactor/cohesion-cycle-5-p2-command-usage`
- `refactor/cohesion-cycle-5-p3-database-bootstrap`
- `refactor/cohesion-cycle-5-p4-simulation-state`

Roles: Refactor Implementer per cycle; Refactor Verifier as a separate pass after
each merge. Docs-only work needs no Quant Validator record; no cycle changes
model or strategy behavior.

## 8. Verification

Per cycle: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
-D warnings`, `cargo test --workspace --no-fail-fast`, `RUSTDOCFLAGS="-D
warnings" cargo doc --workspace --no-deps`, `bash tools/checkers.sh`, and the
cycle-specific commands in the table. P4 adds `bun run check`, `bun test`, and
the simulation Playwright specs under `apps/web`.

Limitation: the fast tier does not start the Compose stack. The composed tier and
`tools/parity_desktop.sh` belong to release verification, not to this refactor.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Baseline commit | The plan is measured against one commit | `git rev-parse HEAD` | `439a0639…` | `439a0639f95ab93cd2f74fceb3af8df31532293d` | Pass | Planner | 2026-11-16 | Worktree `quant-rule--refactor-cohesion-cycle-5` |
| V2 | Baseline tests | The test inventory is recorded before any change | `cargo test --workspace --no-fail-fast` | All pass; count recorded | 2,118 passed, 0 failed, 14 ignored, 119 binaries (2,132 listed) | Pass | Planner | 2026-11-16 | `target/gate` inside the worktree |
| V3 | Baseline gate | The full gate is green before the first seam | `bash tools/gate.sh` | Exit 0 | Exit 0; fmt, clippy, test, doc, 24 checkers | Pass | Planner | 2026-11-16 | Uses `target/gate` inside the worktree |
| V4 | Baseline fixtures | Fixture bytes are recorded | `find crates -path '*/fixtures/*' -type f \| sort \| xargs sha256sum \| sha256sum` | `325d0c4a…` | `325d0c4a…` | Pass | Planner | 2026-11-16 | 135 files |
| V5 | Baseline metrics | The four named metrics are recorded | `rg -c 'pub struct MemoryObjects' crates`; usage-literal count; `rg -c 'DeploymentProfile::Desktop'` binaries; `wc -l SimulationRunDetail.svelte` | 5 / 25 / 4 / 762 | 5 / 25 / 4 / 762 | Pass | Planner | 2026-11-16 | Commands in the assessment |
| V6 | P1 shared fake | One definition; five suites import it | `rg 'pub struct MemoryObjects' crates` | 1 | 1 (`runtime/src/adapters/memory.rs`) | Pass | Implementer | 2026-11-16 | Five common modules re-export it |
| V7 | P1 tests | The suites still prove their behaviour | `cargo test --workspace` | All pass; count ≥ V2 | 2,120 passed, 0 failed, 14 ignored (baseline 2,118) | Pass | Implementer | 2026-11-16 | Two new runtime unit tests |
| V8 | P2 usage source | No parser literal duplicates a help usage | `rg -n 'usage\("' crates/quantrule-shell/src/command/parse.rs`; `rg -n '"usage: ' crates/quantrule-shell/src/command/parse.rs` | 25 registry calls; only family hints and `quit` remain | 25 registry calls; 7 family hints and `quit` remain | Pass | Implementer | 2026-11-16 | The false `|all` hint was corrected with human approval |
| V9 | P2 output | The shell prints the same bytes | `cargo test -p quantrule-shell` | All pass | 85 passed, 0 failed | Pass | Implementer | 2026-11-16 | New parity test covers every registered command |
| V10 | P3 bootstrap | No binary branches on the profile | `rg -c 'DeploymentProfile::Desktop' crates/*/src/main.rs crates/*/src/bin/*.rs` | 0 | 0 (only `runtime/src/backends.rs` and `runtime/src/config.rs` branch) | Pass | Implementer | 2026-11-16 | One helper in `backends.rs` |
| V11 | P3 health | Health names and log text are unchanged | `cargo test -p quantrule-data-service -p quantrule-evidence-service -p quantrule-gateway -p quantrule-simulation-service` | All pass | 349 passed, 0 failed, 5 ignored across the four services | Pass | Implementer | 2026-11-16 | Health names `sqlite`, `postgres`; gateway log field `gateway` becomes `applied` |
| V12 | P4 client | The component is under 400 lines | `wc -l apps/web/src/lib/SimulationRunDetail.svelte` | < 400 | 344 | Pass | Implementer | 2026-11-16 | Store plus three child components |
| V13 | P4 client tests | Web unit and e2e behaviour is unchanged | `cd apps/web && bun run check && bun test`; full Playwright suite | All pass | check 0 errors; 121 unit tests; 41 e2e tests | Pass | Implementer | 2026-11-16 | — |
| V14 | Equivalence | No artifact or fixture byte changed | `find crates -path '*/fixtures/*' -type f \| sort \| xargs sha256sum \| sha256sum`; `sha256sum tests/fixtures/client-parity/*.json` | `325d0c4a…`; `0b960a3e…`; `e94e9d2b…` | `325d0c4a…`; `0b960a3e…`; `e94e9d2b…` | Pass | Verifier | 2026-11-16 | Unchanged after the merge |
| V15 | Equivalence | No dependency or schema change | `git diff --stat 439a0639 -- Cargo.toml Cargo.lock 'crates/*/migrations/*'` | Empty | Empty diff against `439a0639` | Pass | Verifier | 2026-11-16 | No dependency or migration change |
| V16 | Equivalence | No test removed or weakened | `git diff 439a0639 -- '*tests*'` review; test count | Count ≥ V2; no skipped test | 2,122 passed (baseline 2,118); no test removed or skipped | Pass | Verifier | 2026-11-16 | — |
| V17 | Gate | The full gate is green after each merge | `bash tools/gate.sh` | Exit 0 | Exit 0; 2,122 passed; 24 checkers | Pass | Verifier | 2026-11-16 | Merged `main` at `9324a581` |
| V18 | Metric | Each named metric moved in the expected direction | V5 commands | 1 / 0 / 0 / < 400 | 1 / 0 / 0 / 344 | Pass | Verifier | 2026-11-16 | All four moved as planned |
| V19 | Docs | The documents match the changed structure | `bash tools/checkers.sh` | All pass | 24 checkers pass; web check 0 errors; `client.md` names the new files | Pass | Verifier | 2026-11-16 | No module-map row changed |
| V20 | Closure | Findings are `Refactored`, then `Verified` or returned | Assessment status column | 4 `Verified` or returned with evidence | REF-01, -02, -03, -07 `Verified` | Pass | Verifier | 2026-11-16 | Sequential pass, not independent review |

## 9. Risks and recovery

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| P3 changes a health name, log field, or startup error | Medium | Observable startup behaviour | Row V11; message text and health names are copied verbatim; the gateway's log field is now `applied` (log-only, recorded) |
| P2 changes an error message | Low | Printed contract | Row V9 pins the bytes; any diff stops the cycle |
| P1 changes what a suite exercises | Low | Weaker tests | The shared fake is the union of the five; row V16 |
| P4 changes a request order or rendered text | Medium | Simulation screen regression | The Playwright specs are the guard; the store is a mechanical move |
| A cycle needs a logic edit | Medium | It is not a refactor | Stop and route to the owning workflow |

Rollback: each cycle is one revertible commit on its own branch. No schema, data,
or dependency migration is involved; the assessment and plan documents remain if
a cycle is abandoned.

## 10. Stop conditions

- A cycle requires a logic edit, a new dependency, a schema change, or a changed
  expected value.
- A golden fixture, client-parity fixture, command byte, or bundle byte changes.
- The test count decreases or a test is skipped or weakened.
- A finding proves stale or wrong against the current tree.
- The human stops the campaign or the cycle cap is reached.
