# Current Structure Refactor Assessment

- Date: 2026-11-06 (UTC)
- Baseline commit: `73fff19`
- Scope: current Rust workspace, tests, documentation, tools, and repository layout
- Workflow: [`docs/workflows/refactor-workflow.md`](../../workflows/refactor-workflow.md)
- Format: [`docs/refactors/README.md`](../README.md)

## Baseline

| Item | Observed |
|---|---|
| Commit | `73fff19` |
| Toolchain | `rustc 1.99.1`, `cargo 1.99.1` |
| Test inventory | 1,309 passed, 0 failed, 8 ignored |
| Golden fixtures | 105 files; combined SHA256 `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1` |
| Representative bundle | `report model` over `crates/quantrule-shell/tests/fixtures/config-run/run.json`; artifact manifest SHA256 `9f4c9d263293a76350f30720830d1a32037deaec1f6a1981f256bb375aca1e54` |
| Structural report | 192 `src` files / 63,683 lines; 230 files / 75,412 lines including integration tests |
| Largest file | `crates/quantrule-backtest/src/portfolio/ledger_tests.rs`, 973 lines |
| Files over 800 lines | 8 |
| Files over 600 lines | 35 |
| `pub` / `pub(crate)` items | 1,653 / 789 |
| `unwrap`/`expect` lines in non-test source | 315, heuristic |
| Gates | fmt, clippy, test, doc, file size, structure, registry, validation, verification, emitters, command surface, links, terms, legacy, and oracle pass |

The structural report is reproducible with
`python3 tools/check_structure.py --report`. The baseline above is the
before-state for the first cycle.

## Findings

| ID | Impact | Area | Finding | Evidence | Principle | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| REF-2026-11-06-01 | High | shell | `report_run.rs` mixes argument parsing, run orchestration, bundle assembly, provenance assembly, and panel loading | `wc -l crates/quantrule-shell/src/command/report_run.rs` = 856; `run_report`, `model_bundle_with`, `spec_provenance`, `panel_provenance`, `load_panel_series`, `parse_report_arguments` in one module | cohesion, atomicity | A split by existing seam; B move provenance only; C leave | A | file sizes, full gate, fixture and bundle hashes unchanged | In progress | #521 |
| REF-2026-11-06-02 | High | shell | Domain modules import `crate::command::types`, so the adapter flow runs both ways | `analytics/run.rs:13`, `backtest/run.rs:21`, `execution/run.rs:23`, `portfolio/run.rs:22`, `qualification/run.rs:15`, `strategy/run.rs:24` | boundary, coupling | A neutral request module; B pass primitives; C leave | A, separate cycle | import guard and shell tests | Triaged | — |
| REF-2026-11-06-03 | Medium | shell | `config` imports `source` date types, coupling two shell adapters | `crates/quantrule-shell/src/config/data.rs:13-14` imports `DateBounds` and `DateRangeError` | coupling, boundary | A move the date bounds to config; B shared date module; C leave | B, separate cycle | import guard and config tests | Triaged | — |
| REF-2026-11-06-04 | Info | shell | The public surface is wide and the panic count is heuristic | structural report: 1,653 `pub` items, 789 `pub(crate)`, 315 `unwrap`/`expect` lines in non-test source | surface, panic hygiene | A compiler-based count and ratchet; B leave | A, later | clippy `unwrap_used`/`expect_used` on library targets or a reviewed ratchet | Triaged | — |

### REF-2026-11-06-01 — `report_run.rs` has five responsibilities

- Expected invariant: a command module parses its arguments, orchestrates the
  engine calls, and emits the result. Metadata assembly and panel loading have
  their own modules.
- Observed: the 856-line module contains argument parsing, spec selection and
  run orchestration, bundle assembly, provenance assembly, panel loading, and
  output emission.
- Structural cause: the command grew by accretion around `report model`; the
  seams already exist as function groups but were never promoted to modules.
- Impact: a change to provenance or panel loading touches the same file as the
  command flow; the module has several reasons to change.
- Affected contracts/docs: `report model` command output and bundle artifacts.
- Behavior risk: low. The shell tests assert the bundle shape, provenance
  boundary, event ordering, and byte determinism.
- Dependencies: none.
- Effort: medium.
- Metric before: 856 lines, 824 production.
- Metric target: no file over 400 lines in the split; command orchestration in
  `mod.rs` under 250 lines.

#### Options

A. Split by the existing function groups into `report_run/{mod,parse,bundle,provenance,spec}.rs`.
B. Move only provenance into a child module.
C. Leave it; revisit when the next `report model` change lands.

#### Recommendation

A. The seam is already visible in the function groups, and the shell tests make
the move verifiable without new behavior.

#### Verification

Full gate; test count unchanged at 1,309 passed and 8 ignored; fixture hash
`da7389...`; bundle manifest hash `9f4c9d...`; no fixture or expected-value
change; `report model` shell tests pass.

#### Links

- Cycle plan: [`docs/execution-plans/2026-11-06/521-refactor-report-run.md`](../../execution-plans/2026-11-06/521-refactor-report-run.md)
- Issue: #521

### REF-2026-11-06-02 — Domain modules import command argument types

- Expected invariant: `command` depends on the domain modules; domain modules
  do not import the command layer.
- Observed: six domain run modules import argument structs from
  `crate::command::types`.
- Structural cause: argument structs were placed in the command layer while the
  run functions were extracted into domain modules.
- Impact: a command-layer change can force a domain-module rebuild and couples
  the adapter to its callers.
- Behavior risk: medium; the fix moves types and call sites.
- Metric before: 6 domain modules import `crate::command::types`.
- Metric target: 0.

#### Options

A. Create a neutral `crate::request` module for the argument structs.
B. Pass primitives from `command` to the domain `run` functions.
C. Leave it.

#### Recommendation

A, in a separate architecture-sensitive cycle after the first split. The type
move touches many call sites and deserves its own plan.

#### Verification

A source-level import guard plus the full shell test suite.

### REF-2026-11-06-03 — Config imports source date types

- Expected invariant: shell adapters do not import each other's internals; a
  shared value type lives in the module that owns its meaning.
- Observed: `config/data.rs` imports `DateBounds` and `DateRangeError` from
  `source`.
- Structural cause: date bounds were defined with the source parser and reused
  by configuration parsing.
- Impact: a source-parser change can reach configuration parsing.
- Behavior risk: low.
- Metric before: 2 imports.
- Metric target: 0.

#### Options

A. Move the bounds to `config`; B. create a shared `date` module; C. leave it.

#### Recommendation

B, in a separate cycle. The date type is a shared value, not a source detail.

#### Verification

A source-level import guard and the config tests.

### REF-2026-11-06-04 — Public surface and panic count

- Expected invariant: the public surface is intentional and library code does
  not panic on invalid input.
- Observed: the structural report counts 1,653 `pub` items, 789 `pub(crate)`,
  and 315 `unwrap`/`expect` lines in non-test source. The count includes inline
  test modules, so it is a scan trigger, not a finding.
- Structural cause: no compiler-based measurement yet.
- Impact: unknown until measured; do not refactor on a heuristic.
- Recommendation: add a clippy `unwrap_used`/`expect_used` pass on library
  targets, or a reviewed ratchet, before opening a cycle.

## Out of scope

- Stale documentation content, such as `data/README.md` naming retired local
  directories, is an Audit finding, not a refactor.
- A behavior, formula, schema, or command change is not a refactor.
- A performance change requires a benchmark and a separate plan.

## Selected cycle

REF-2026-11-06-01: split `report_run.rs` by its existing seams. The cycle is
planned in
[`docs/execution-plans/2026-11-06/521-refactor-report-run.md`](../../execution-plans/2026-11-06/521-refactor-report-run.md).

## Loop decision

Pre-merge verification is complete. The implementation split `report_run` into
five modules, with the largest at 275 lines and `mod.rs` at 144. A clean-worktree
pass re-ran the full gate: 1,309 passed, 0 failed, 8 ignored; the fixture hash and
the bundle manifest hash are unchanged. The pass was performed by the same agent
and is disclosed as sequential, not independent.

The finding remains `In progress` until the pull request is merged. It then
moves to `Refactored`; a separate Refactor Verifier moves it to `Verified` and
the assessment re-scans for the next cycle or a new campaign.

## Limitations

- The structural report is a heuristic. Cohesion and coupling findings are
  supported by code references and review, not by a metric alone.
- One agent produced this assessment. The independent verifier role is exercised
  after the first cycle.
