# AI Transparency Log — Whole-Project Refactor

- Date: 2026-11-20 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: whole-project refactor assessment and plan
- Plan: [`docs/execution-plans/2026-11-20/whole-project-refactor.md`](../../execution-plans/2026-11-20/whole-project-refactor.md)
- Assessment: [`docs/refactors/2026-11-20/whole-project-refactor.md`](../../refactors/2026-11-20/whole-project-refactor.md)
- Claim type: Behaviour Claim

## Intent and boundary

The human asked to run a whole-project refactor workflow across docs, code,
tests, and tooling, removing unneeded weight and confusing names. The Refactor
Workflow requires a read-only assessment, then a plan, then human approval before
implementation.

This session performed the Structure Auditor and Refactor Planner passes. It did
not implement a refactor, edit code or tests, create a board issue, commit, push,
open a pull request, or merge. The only writes are the assessment, this plan, and
this log, on the task branch in the worktree
`/home/zantgo/Proyectos/quant-rule--whole-project-refactor`.

The human asked for "everything". The scope was corrected to a bounded two-cycle
campaign because the workflow allows one seam per cycle and a campaign with a
finite finding list and stop rule. Historical records, experiment config, CI
enforcement, `AGENTS.md`, and the public command surface were assessed but
excluded from the behavior-neutral campaign.

## Verification and evidence

Observed before planning:

| Command | Result |
|---|---|
| `git rev-parse HEAD` | `8443697fec9e20a555b2fe900a36c19dae03b8cb` |
| `rustc --version`, `cargo --version` | 1.99.1, 1.99.1 |
| `cargo test --workspace -- --list \| grep -c ': test$'` | 2,394 tests, 0 ignored |
| `./tools/gate.sh` | PASS, exit 0: fmt, clippy, tests, rustdoc, 27 checkers |
| `bash tools/checkers.sh` | PASS, 27 checkers, 84.44 s |
| `python3 tools/check_links.py` | 1,063 Markdown files, no broken relative links |
| `python3 tools/check_terms.py` | 629 files, no banned terms |
| `cargo run -q -p quantrule-shell -- help \| sha256sum` | `c1c5a7c2f70704651bcfe41236c185104ed5b37bf599288de96dfd99d60891c5` |
| Fixture aggregate | `ea8bacd83197c7c0f5b6f42abc64d73454f01742a415804a453c326e7af775a6` |

Reproducible structural evidence is in the assessment. The main measurements:
509 Rust files and 159,527 lines; 87 files over 600 lines and 2 over 990;
1,026 tracked Markdown files and 125,293 lines; 2,178 tracked config files;
`docs/README.md` 328 lines against a 330 cap and 225 commits in six months.

No claim is made that a refactor was implemented or verified. The plan's
verification table is `Pending`.

## Learning

- The repository is green and the crate graph is clean. The cost is
  accumulation and a few high-contention structures, not broken gates. A
  refactor that hunts for violations would have found nothing.
- The strongest code finding is the shell command pipeline, not the largest
  files overall. `runtime/adapters/filesystem.rs` (964) and
  `model/progression/leg.rs` (955) are cohesive single-purpose files; splitting
  them on line count would reduce cohesion. The shell files mix families,
  rendering, transport, and file I/O.
- The strongest documentation finding is the global index. It is at its cap,
  machine-coupled to four checkers, and was compressed by commit `c3c10cc1` to
  fit. This is a structural cause of documentation churn.
- Historical records are 70% of the documentation lines, but they are governed
  evidence. The correct route is Recordkeeping, not deletion. The same applies
  to 2,178 historical config files and the 24 MB port-parity fixtures.
- The "research" and "improvement" names are a real ambiguity, but the fix
  likely touches help text and docs, which is user-visible output. It was kept
  out of the behavior-neutral campaign and presented as decision D2.
- A first non-test `unwrap`/`expect` scan reported 473 occurrences. It included
  test modules and `tests.rs` files. A refined scan found 15, all in runtime
  memory test doubles and lock guards. The first number was discarded rather
  than used as evidence.
- A dependency-use scan found one test-only crate in a production dependency
  list: `quantrule-service-api` in `quantrule-gateway`. It was added as a Low
  finding with a separate follow-up, not folded into the campaign, because the
  campaign excludes `Cargo.toml` changes.
- A first file-size review suggested splitting every file over 600 lines. That
  was rejected: the workflow says line count is a prompt to look, not a target.

## Rejected or corrected suggestions

| Suggestion | Disposition |
|---|---|
| Delete or archive historical AI logs, plans, and config to reduce tree size | Rejected. Governed evidence; route to Recordkeeping and Data Lifecycle |
| Rename `research` commands to `improvement` to match the types | Rejected. Breaks the public command contract and needs a deprecation path |
| Split all Rust files over 600 lines | Rejected. Would break cohesive modules; only the shell seam is justified |
| Remove historical Python tests from the tree | Rejected without a retention decision; the plan proposes moving them out of PR CI only |
| Treat the line-count target as the goal | Rejected. The named metric is a review trigger; module boundaries decide |
| Implement the shell split immediately because the human said "run the workflow" | Rejected. The workflow requires plan approval before implementation |

## Disclosure

One agent performed the assessment and planning passes sequentially. No
independent agent review, Quant Validator, or Code Reviewer pass was performed.
The plan was `Proposed` during the assessment; implementation evidence is
recorded below.

## Implementation and verification (2026-11-20)

The human approved the plan with "approved proceed". The plan moved to
`In Progress`. Both cycles were implemented in the task worktree and left
uncommitted. No issue, commit, push, pull request, merge, tag, or deployment was
created.

### Cycle 1 — shell command pipeline

- Moved `output.rs` (995 lines) into `output/` (12 files, largest 270),
  `command/dispatch.rs` (992) into `command/dispatch/` (10 files, largest 304),
  and `command/parse.rs` (933) into `command/parse/` (8 files, largest 393).
- Moved the 45 HTTP output tests into `tests/cases/` under the existing
  `http_commands` target. All 45 test function names are preserved; the shell
  test count is 110 before and after.
- Updated the architecture guard to scan the output directory and the module
  map to name the new family modules.
- Verified: `cargo test -p quantrule-shell` (110 passed), `cargo clippy -p
  quantrule-shell --all-targets -- -D warnings`, `cargo fmt --check`, `help`
  SHA-256 `c1c5a7c2f70704651bcfe41236c185104ed5b37bf599288de96dfd99d60891c5`
  unchanged, and the full `tools/gate.sh` PASS with 2,394 workspace tests.
- No logic edit, no dependency, no `Cargo.lock` change, no test removed or
  weakened.

### Cycle 2 — documentation index

- Reduced `docs/README.md` from 328 to 81 lines. Moved status to
  `docs/architecture/README.md`, `docs/model/README.md`,
  `docs/config/README.md`, `docs/improvement/programmes.md`,
  `docs/roles/README.md`, `docs/releases/README.md`, `docs/adr/README.md`, and
  `tools/README.md`.
- Updated `check_client_docs.py` to read the model index, `check_workflows.py`
  and `check_structure.py` to use the workflow index as the only listing
  authority, and the corresponding checker tests.
- Verified: row-count mapping preserved (17/35/20/58/11/18/15/19/13), 27
  checkers pass, 1,071 Markdown links resolve, 635 files pass the term scan,
  and the full `tools/gate.sh` PASS.

### Corrections

- The HTTP test file could not be split into `tests/http_commands/` without
  `#[path]` (forbidden) or a second Cargo target (changes discovery). Used
  `tests/cases/` as a module directory included by the existing target: one
  test target, same test count.
- `check_structure.py` carried a second workflow-index coupling that
  `check_workflows.py` did not. Both were updated, and the structure test was
  corrected to assert one violation, not two.
- The first documentation move wrote the evidence table without applying the
  link rewrite. `check_links.py` found 37 broken links; the rewrite was applied
  and the file rechecked.

### Boundary

The implementation checkpoint above was uncommitted. The human then authorized
commit and PR; the commit and pull-request record follows.

### Commit and PR (2026-11-20)

- Issues: #929 (cycle 1) and #930 (cycle 2).
- Commits: `ee8162d3` (assessment and plan), `358df5da` (shell split),
  `ac5d14f8` (documentation indexes).
- Pull requests: #931 (cycle 1, base `main`) and #932 (cycle 2, stacked on
  #931).
- Both pull-request CI runs are green (verify, web, web-e2e); issues #929 and
  #930 are `In Review`. No merge, tag, or deployment was performed. The plan is
  `In Review`; V23 (done and cleanup) is pending the merge.
