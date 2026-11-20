# Whole-Project Refactor Plan

- Date: 2026-11-20 (UTC)
- Status: In Review
- Workflow: [Refactor Workflow](../../workflows/refactor-workflow.md)
- Assessment: [`docs/refactors/2026-11-20/whole-project-refactor.md`](../../refactors/2026-11-20/whole-project-refactor.md)
- Baseline commit: `8443697fec9e20a555b2fe900a36c19dae03b8cb`
- Worktree: `/home/zantgo/Proyectos/quant-rule--whole-project-refactor`
- Branch: `refactor/whole-project-shell` (cycle 1); `refactor/whole-project-docs` (cycle 2, stacked)
- Claim type: Behaviour Claim
- Issues: #929 (cycle 1), #930 (cycle 2).

## 1. Intent

The repository is functionally green: the full gate passes, 2,394 tests are
listed, and the crate graph is clean. The cost is structural. Two files in the
shell command pipeline sit at the 1,000-line cap, one command change crosses at
least five files, and the global documentation index is at 328 of 330 allowed
lines while four checkers depend on it. Every workflow or status change edits the
most contended document in the repository.

This plan removes those two costs without changing product behavior. It is a
two-cycle refactor campaign, one seam per cycle and one PR per seam. It does not
delete historical records or experiment config, does not change the CI gate, and
does not rename the public command surface. Those are decisions in section 5.

Affected users: the maintainer reviewing and merging changes, and the agents that
navigate the shell and documentation.

## 2. Current state

| Area | Observed |
|---|---|
| Shell source | `output.rs` 995 lines, `command/dispatch.rs` 992, `command/parse.rs` 933, `command/help.rs` 618 |
| Shell tests | `tests/http_commands.rs` 989, `tests/support/mod.rs` 912 |
| Command fan-out | `research create` appears in `types.rs:272`, `parse/parse_research.rs:52`, `dispatch.rs:135`, `help.rs:215`, and `output.rs:678` |
| Documentation index | `docs/README.md` 328 lines against a 330 cap; 225 commits in six months |
| Checker coupling | `check_workflows.py:126`, `check_client_docs.py`, `check_releases.py`, and `check_legacy.py` read `docs/README.md` |
| Baseline gate | `tools/gate.sh` PASS at `8443697f`: fmt, clippy, 2,394 tests, rustdoc, 27 checkers |
| Help output | 61 lines, SHA-256 `c1c5a7c2f70704651bcfe41236c185104ed5b37bf599288de96dfd99d60891c5` |

The full evidence is in the assessment. This plan does not repeat it.

## 3. Scope

### Included

- Cycle 1, REF-2026-11-20-01: split the shell command pipeline by command family
  into `parse/`, `dispatch/`, `output/`, and the matching integration tests. Move
  code and tests; do not edit logic.
- Cycle 2, REF-2026-11-20-02: reduce `docs/README.md` to a navigation index and
  move each status table to its owning index. Update the four checkers to read
  the owning index while preserving every assertion.

### Excluded

- No model, strategy, execution, portfolio, analytics, schema, API, or command
  contract change.
- No command name, argument, output, exit code, or error string change.
- No dependency, `Cargo.toml`, or `Cargo.lock` change.
- No test removal, skip, rename-only churn, or fixture regeneration.
- No deletion or move of historical records (`docs/ai-transparency-log/`,
  `docs/execution-plans/`, `docs/improvement/`, `docs/reports/`).
- No deletion or move of historical config (`config/research/`, `config/sr*`).
- No CI gate change (REF-2026-11-20-03).
- No `AGENTS.md` restructuring (REF-2026-11-20-05).
- No `research`/`improvement` rename or help-text rewrite (REF-2026-11-20-04).
- No port-parity fixture format change (REF-2026-11-20-09).

## 4. Assumptions and uncertainty

- The two seams are independent. Cycle 2 does not depend on cycle 1.
- The shell split can be move-only. If a move needs a logic edit, the cycle stops
  and returns to planning.
- The integration test file can be split without changing Cargo test discovery
  or the test count. The exact layout is decided in cycle 1 and recorded in the
  PR.
- The four checkers can read the owning index without weakening an assertion.
  If an assertion cannot be preserved, cycle 2 stops.
- The human approves the plan filename without an issue number, or supplies an
  issue number before execution.
- No production user is affected; the only observable artifacts are the shell
  help and command output, which must stay byte-identical.

## 5. Decisions

| ID | Decision | Options | Recommendation | Approval |
|---|---|---|---|---|
| D1 | Approve this two-cycle campaign and the slug `whole-project-refactor` | A: approve as written. B: approve cycle 1 only. C: request a different scope | A | Approved 2026-11-20 |
| D2 | Canonical `research` vs `improvement` term (REF-04) | A: `research` is the surface, `improvement run` is the record; fix ambiguous prose. B: rename the domain to `research run`. C: rename commands to `improvement` | A | Pending |
| D3 | Historical Python suites in PR CI (REF-03) | A: current in PR CI, historical on demand and release. B: path-filter the historical steps. C: retire after archiving | A | Pending |
| D4 | `AGENTS.md` catalog duplication (REF-05) | A: link to the authorities, keep the norms. B: add drift checkers for every copy. C: split into annexes | A | Pending |
| D5 | Historical config and record volume (REF-07, REF-08) | A: defer to Data Lifecycle and Recordkeeping. B: add indexes without moving files. C: archive externally | A | Pending |

D2, D3, and D4 change user-visible help text, what the gate enforces, or
methodology text. They are not part of this behavior-neutral campaign and are
not implemented under this plan.

Plan approval authorizes the work in section 7. It does not authorize commits,
pushes, pull requests, or merges; those are requested separately under
`AGENTS.md` section 9.

## 6. Acceptance criteria

- Cycle 1: no shell source or test file touched by the split exceeds 400 lines;
  every command family has one parse, dispatch, and render module; the `Command`
  enum and all parse results are unchanged; `help` and all command output are
  byte-identical; the test count stays 2,394.
- Cycle 2: `docs/README.md` is at most 120 lines; every status table has one
  owning index; every checker assertion is preserved; every workflow is still
  discoverable from the workflow index; no status row is lost.
- Both cycles: full `tools/gate.sh` passes; the fixture aggregate hash is
  unchanged; no new dependency; no unrelated change; the AI transparency log and
  changelog are updated; the verification table is fully `Pass` before the plan
  moves to `Verified`.

## 7. Work breakdown

| Phase | Step | Role | Paths | Depends on |
|---|---|---|---|---|
| P0 | Approve the plan and the D1 scope; create the board issue with `Behaviour Claim` | Planner + Human | Issue, this plan | Human gate |
| P0 | Commit the approved plan, assessment, and AI log after explicit authorization | Planner | `docs/execution-plans/2026-11-20/`, `docs/refactors/2026-11-20/`, `docs/ai-transparency-log/2026-11-20/` | P0 approval |
| C1.1 | Move family parsers out of `parse.rs` into `parse/<family>.rs`; keep `parse_argv`, `parse_line`, tokenization, globals, and the shared option helpers as the facade | Implementer | `crates/quantrule-shell/src/command/parse.rs`, `parse/` | P0 |
| C1.2 | Move family dispatch functions into `dispatch/<family>.rs`; keep one `run` match whose arms delegate; move dataset upload and run-fetch file I/O out of the dispatcher | Implementer | `command/dispatch.rs`, `dispatch/` | C1.1 |
| C1.3 | Move family renderers into `output/<family>.rs`; keep generic JSON writers in `output/json.rs`; keep `emit`/`emit_raw` as the public facade | Implementer | `src/output.rs`, `output/` | C1.1 |
| C1.4 | Split `tests/http_commands.rs` by family under one integration entry point; move shared helpers into `tests/support/` without duplication | Implementer | `crates/quantrule-shell/tests/http_commands.rs`, `tests/http_commands/`, `tests/support/` | C1.3 |
| C1.5 | Update the module map and shell module docs to name the new family modules | Implementer | `docs/architecture/module-map.md`, shell module docs | C1.4 |
| C1.6 | Run the cycle-1 verification set, review the diff with move detection, and open the cycle-1 PR | Implementer + Reviewer | Branch, PR | C1.5 |
| C2.1 | Move the architecture, model, run-contract, course, and decision status rows to the owning indexes; create `docs/model/README.md`, `docs/config/README.md`, and `docs/adr/README.md` as owning indexes | Implementer | `docs/README.md`, `docs/architecture/README.md`, `docs/model/README.md`, `docs/config/README.md`, `docs/adr/README.md`, `docs/course/README.md`, `docs/releases/README.md` | P0 |
| C2.2 | Reduce `docs/README.md` to a navigation map of kinds and living entry points | Implementer | `docs/README.md` | C2.1 |
| C2.3 | Update `check_workflows.py`, `check_client_docs.py`, `check_releases.py`, and `check_legacy.py` to read the owning index; preserve every assertion and add a regression test for any checker behavior that moves | Implementer | `tools/check_*.py`, `tools/test_check_*.py` | C2.2 |
| C2.4 | Run the cycle-2 verification set, review the diff, and open the cycle-2 PR | Implementer + Reviewer | Branch, PR | C2.3 |
| V | Independent verification of both cycles against the baseline and the named metrics | Refactor Verifier | Plan table, merged PRs | C1.6, C2.4 |
| Z | Update the changelog and AI log, move each issue to `In Review`, then `Done` after merge, verify acceptance, and clean up the worktree and branches | Implementer + Human | `CHANGELOG.md`, AI log, board, Git | V |

The two cycles are sequential in this campaign because they share the branch
policy and the verification budget. They have no code dependency; the human may
split them into two issues or two campaigns.

## 8. Verification

Commands, in order:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
bash tools/checkers.sh
./tools/gate.sh
```

Evidence to capture before and after each cycle:

- `find crates/quantrule-shell -name '*.rs' -exec wc -l {} + | sort -rn | head`
- `CARGO_TARGET_DIR="$PWD/target/gate" cargo test --workspace -- --list | grep -c ': test$'`
- `cargo run -q -p quantrule-shell -- help | sha256sum`
- `find crates -path '*/tests/fixtures/*' -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum`
- `git diff --stat` and `git diff --color-moved` review
- `python3 tools/check_doc_length.py`, `check_links.py`, `check_terms.py`,
  `check_workflows.py`, `check_client_docs.py`, `check_releases.py`,
  `check_legacy.py`

Limitations:

- Command output that requires a live gateway is covered by the in-process
  integration tests, not by a live stack. The fast tier is the gate; the composed
  tier is a release check.
- The line-count metric is a proxy for cohesion, not a proof of it. The reviewer
  must inspect the module boundaries, not only the numbers.
- CI duration for REF-03 was not measured; D3 is a decision, not a claim.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | P0 plan | Plan, assessment, and AI log exist and are linked | `ls docs/execution-plans/2026-11-20/ docs/refactors/2026-11-20/ docs/ai-transparency-log/2026-11-20/` | Three files present | Present | Pass | Implementer (self-check) | 2026-11-20 | Independent verification pending |
| V02 | P0 scope | D1 approved; issue carries `Behaviour Claim` | Issue link and board field | Approved | Issues #929 and #930 carry `Behaviour Claim` | Pass | Implementer (self-check) | 2026-11-20 |  |
| V03 | P0 worktree | Task worktree and branch exist and are based on `8443697f` | `git worktree list`, `git status --short --branch` | Worktree on `refactor/whole-project-shell` | Present at `8443697f`; commits `ee8162d3`, `358df5da`, `ac5d14f8` | Pass | Implementer (self-check) | 2026-11-20 | Cycle 2 branch is stacked |
| V04 | C1.1 parse | Family parsers live in family modules; facade is small | `wc -l` on `parse.rs` and `parse/*.rs` | No touched file over 400 lines | `parse/mod.rs` 393; largest family 222 | Pass | Implementer (self-check) | 2026-11-20 |  |
| V05 | C1.2 dispatch | Family dispatch lives in family modules; file I/O is separate; one `run` match remains | `wc -l`, `rg -e 'fn dataset_' -e 'fn run_' -e 'fn simulation_' -e 'fn profile_'` | No touched file over 400 lines | `dispatch/mod.rs` 304; simulation 214 | Pass | Implementer (self-check) | 2026-11-20 |  |
| V06 | C1.3 output | Family renderers live in family modules; generic JSON writers are separate | `wc -l` on `output.rs` and `output/*.rs` | No touched file over 400 lines | `output/mod.rs` 118; research 270 | Pass | Implementer (self-check) | 2026-11-20 |  |
| V07 | C1.4 tests | Integration tests split by family; test names preserved | `cargo test --workspace -- --list \| grep -c ': test$'` | 2,394 | 2,394; `http_commands` 45/45 names preserved | Pass | Implementer (self-check) | 2026-11-20 |  |
| V08 | C1 contract | `Command` variants, parse results, and public shell behavior unchanged | `git diff -- crates/quantrule-shell/src/command/types.rs`; test suite | No semantic diff | `types.rs` unchanged; 110 shell tests pass | Pass | Implementer (self-check) | 2026-11-20 |  |
| V09 | C1 output | `help` is byte-identical | `cargo run -q -p quantrule-shell -- help \| sha256sum` | `c1c5a7c2...60891c5` | `c1c5a7c2...60891c5` | Pass | Implementer (self-check) | 2026-11-20 |  |
| V10 | C1 purity | No logic edit, new dependency, or new `#[allow]` | `git diff --stat`, `git diff -- Cargo.toml Cargo.lock`, `rg '#\[allow'` | No change | No Cargo change; no new allow; function names preserved | Pass | Implementer (self-check) | 2026-11-20 |  |
| V10a | C1.5 docs | The module map names the new family modules and the where-to-add-command row | `rg -e 'parse/' -e 'dispatch/' -e 'output/' docs/architecture/module-map.md` | Updated | Updated | Pass | Implementer (self-check) | 2026-11-20 |  |
| V11 | C1 gates | fmt, clippy, test, doc, checkers pass | `./tools/gate.sh` | PASS, exit 0 | PASS, exit 0 | Pass | Implementer (self-check) | 2026-11-20 | Cycle 1 gate |
| V12 | C2.1 status | Each status table has one owning index | Mapping table in the PR | No orphan row | Row counts preserved: 17/35/20/58/11/18/15/19/13; no old headings | Pass | Implementer (self-check) | 2026-11-20 |  |
| V13 | C2.2 index | `docs/README.md` is at most 120 lines | `wc -l docs/README.md` | At most 120 | 81 | Pass | Implementer (self-check) | 2026-11-20 |  |
| V14 | C2.3 checkers | Every checker assertion is preserved; owning-index reads are tested | `bash tools/checkers.sh`; checker unit tests | 27 checkers pass | 27 checkers pass; 30 structure tests pass | Pass | Implementer (self-check) | 2026-11-20 |  |
| V15 | C2 discovery | Every workflow remains listed in the workflow index and reachable from the global map | `python3 tools/check_workflows.py`; `rg` over `docs/README.md` and `docs/workflows/README.md` | All 20 workflows discoverable | 20 workflows, 8 templates | Pass | Implementer (self-check) | 2026-11-20 |  |
| V16 | C2 links | No broken relative link and no banned term | `python3 tools/check_links.py`; `python3 tools/check_terms.py` | PASS | 1,071 Markdown files; 635 files scanned | Pass | Implementer (self-check) | 2026-11-20 |  |
| V17 | V fixtures | Golden fixtures are unchanged | Fixture aggregate `sha256sum` | `ea8bacd8...f775a6` | No fixture file touched | Pass | Implementer (self-check) | 2026-11-20 |  |
| V18 | V diff | No unrelated change and no unapproved scope | `git diff --stat`; reviewer checklist | Only the two seams | Shell split, docs indexes, checker coupling, closure records | Pass | Implementer (self-check) | 2026-11-20 |  |
| V19 | V gate | Full gate passes after both cycles | `./tools/gate.sh` | PASS, exit 0 | PASS, exit 0 | Pass | Implementer (self-check) | 2026-11-20 | Cycle 2 gate |
| V20 | Z changelog | `CHANGELOG.md` records the structural change, or states why no entry is needed | `git diff -- CHANGELOG.md` | Entry under `Unreleased` | `Changed` entry added; `check_changelog.py` passes | Pass | Implementer (self-check) | 2026-11-20 |  |
| V21 | Z AI log | The AI log records assessment, planning, implementation, and verification | `docs/ai-transparency-log/2026-11-20/whole-project-refactor.md` | Complete | Updated with implementation evidence | Pass | Implementer (self-check) | 2026-11-20 |  |
| V22 | Z review | Each issue is `In Review` only with green checks and the PR open | Board status, PR checks | In Review | PRs #931 and #932 green; issues #929 and #930 In Review | Pass | Human | 2026-11-20 | Human review gate |
| V23 | Z done | Human acceptance, merge, acceptance verification, worktree and branch cleanup | Board `Done`, `git worktree list`, `git branch -a` | Done, cleaned | Not merged; worktree retained | Pending | Human |  | Cleanup after merge |

## 9. Risks and recovery

| Risk | Likelihood | Impact | Mitigation | Recovery |
|---|---|---|---|---|
| A move changes module visibility or an import path | Medium | Compile or test failure | Move-only commits, compiler-driven import fixes, no logic edits | Fix within the cycle; if a logic edit is needed, stop and re-plan |
| The integration test split changes test discovery | Medium | Test count drop or duplicate test names | One integration entry point with family submodules; compare the 2,394 names | Revert the test split commit; keep the production split |
| Checker change weakens a gate | Low | False green | Keep every assertion; add a failing fixture test for each moved check; reviewer reads the checker diff | Revert cycle 2; the old index remains |
| The docs index loses a status row | Medium | Lost status | Mapping table in the PR; `rg` each row before and after | Restore the row from Git; stop the cycle |
| Merge conflicts while the branch is open | Medium | Rework | Short-lived branch; one seam per PR; rebase only with authorization | Rebase or reapply the move; never force-push |
| Line-count target drives a bad split | Low | Lower cohesion | The reviewer checks module boundaries, not only counts | Merge modules back if the split is artificial |

Rollback is a revert of the cycle PR. There is no schema, data, or deployment
change, so recovery is the previous commit.

## 10. Stop conditions

Stop and request human direction when:

- a move needs a logic edit, a new dependency, a schema change, or a new
  abstraction;
- the `Command` enum, parse result, error string, exit code, or output changes;
- a test would be removed, skipped, weakened, or renamed without preserving the
  behavior it proves;
- a golden fixture would change;
- a checker assertion cannot be preserved in cycle 2;
- a status row cannot be mapped to an owning index;
- the plan needs a new claim type or scope;
- a board item would be deleted, archived, or reopened;
- a worktree or branch cannot be cleaned safely.

## 11. Traceability

| Finding | Issue | PR | Verification |
|---|---|---|---|
| REF-2026-11-20-01 | #929 | Cycle 1 PR | V04-V11 |
| REF-2026-11-20-02 | #930 | Cycle 2 PR | V12-V16 |
| REF-2026-11-20-03 | Decision D3 only | None in this plan | D3 |
| REF-2026-11-20-04 | Decision D2 only | None in this plan | D2 |
| REF-2026-11-20-05 | Decision D4 only | None in this plan | D4 |
| REF-2026-11-20-06 | No action | None | Assessment |
| REF-2026-11-20-07 | Decision D5 only | None in this plan | D5 |
| REF-2026-11-20-08 | Decision D5 only | None in this plan | D5 |
| REF-2026-11-20-09 | Decision D5 only | None in this plan | D5 |
| REF-2026-11-20-10 | Follow-up issue after approval | Separate PR | `cargo tree -p quantrule-gateway`; gateway tests; boundary guard |

## 12. Related documents

- [Refactor Workflow](../../workflows/refactor-workflow.md)
- [Refactor assessment format](../../refactors/README.md)
- [Execution Plan Workflow](../../workflows/execution-plan.md)
- [Whole-project assessment](../../refactors/2026-11-20/whole-project-refactor.md)
- [AI transparency log](../../ai-transparency-log/2026-11-20/whole-project-refactor.md)
