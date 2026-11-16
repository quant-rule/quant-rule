# AI Transparency Log — Cohesion Refactor Cycle 5

- Date: 2026-11-16 (UTC)
- Tool: OpenCode (DeepSeek V4.1 Flash 1017)
- Plan: [`docs/execution-plans/2026-11-16/cohesion-cycle-5.md`](../../execution-plans/2026-11-16/cohesion-cycle-5.md)
- Claim type: Behaviour Claim

## Intent and boundary

The human asked: "run the refactor workflow". AI work covers the read-only
structure assessment (Stage 1) and the proposed refactor plan (Stage 2). No
implementation has started. The human owns the plan approval, the choice among
the recorded options, the campaign scope, and final acceptance.

## Work recorded

### 2026-11-16 — Assessment

- Proposed: `REF-2026-11-16-01..08`, their evidence, options, and metrics in
  `docs/refactors/2026-11-16/cohesion-refactor.md`.
- Baseline: commit `439a0639`; 16 crates; 345 Rust source files; 103,175 lines;
  0 files over 1,000; 13 over 800; 56 over 600; `pub` 4,295; `pub(crate)` 245.
- Fixtures: 135 files, combined SHA256 `325d0c4a…`.
- Checker gate: `bash tools/checkers.sh` observed to pass 24 checkers.
- Boundary: the assessment changed no code, test, issue, or PR. It is a new
  document only.

### 2026-11-16 — Plan

- Proposed: a four-cycle plan covering the four Medium findings, with options,
  diff budgets, named metrics, and a 20-row verification table.
- Boundary: no implementation, no commit, no push, no PR.
- Verification: baseline rows V1–V5 were observed directly. `cargo test
  --workspace --no-fail-fast` ran 2,118 passed, 0 failed, 14 ignored across 119
  binaries (2,132 listed); `bash tools/gate.sh` exited 0 and the checker gate
  reported 24 checkers. Both ran in the worktree with `target/gate`.

## Rejected or corrected suggestions

- Considered a new `quantrule-testing` workspace member for the shared
  object-store fake; rejected in favour of a `runtime::adapters::memory` module
  because `MemoryClient` already lives in runtime and the new member would exist
  for about sixty test-only lines.
- Considered bundling the four Low findings into the same four cycles; rejected
  because one PR carries one seam and each Low has its own root cause.
- Considered a checker instead of a single usage source for the shell; rejected
  because a checker keeps the duplication and moves the failure to a later
  review rather than removing the copy.
- Recorded that the Structure Auditor, Planner, and (later) Implementer passes
  are performed by the same agent; no independent review is claimed.

### 2026-11-16 — P1 shared object store

- Implemented: `runtime::adapters::memory::MemoryObjects`; the five service
  test-support modules re-export it. 350 lines removed, 168 added.
- Verification: full `tools/gate.sh` exit 0; 2,120 passed, 0 failed, 14 ignored
  (baseline 2,118).
- Commit: `ad8e4fc1`.

### 2026-11-16 — P2 usage source

- Implemented: `help::usage`, a parser helper that formats the registry line,
  and a parity test over every registered command. 25 command literals removed.
- Learned: the parity test found that the `simulation marks` error advertised
  `--origin human|oracle|all` while the parser accepts `human|oracle`. The human
  approved correcting the hint; it is the one changed printed line and has a
  changelog entry.
- Verification: full `tools/gate.sh` exit 0; 2,119 passed; shell tests 85
  passed.
- Commit: `ac17287a`.

### 2026-11-16 — P3 database bootstrap

- Implemented: `backends::open_service_database` and four binaries matching one
  result; `check_schema_prefixes.py` now checks the helper and the call site.
- Trade-off recorded: the gateway's structured log field becomes `applied`;
  message text and health names are unchanged.
- Verification: full `tools/gate.sh` exit 0; 2,119 passed; the four services 349
  passed.
- Commit: `3c74e953`.

### 2026-11-16 — P4 simulation screen

- Implemented: `simulationRun.svelte.ts`, `SimulationMarksTable`,
  `SimulationCheckPanel`, and `SimulationDecisionPanel`; the screen is 344
  lines. The chart gained `onMarksChanged` and `onMarkPosted` callbacks.
- Learned: the Playwright suite found a lost update in the first store draft,
  where an object spread across an `await` overwrote marks fetched while the run
  read was pending. The store now awaits before spreading.
- Verification: `bun run check` 0 errors; 121 unit tests; 41 e2e tests; full
  `tools/gate.sh` exit 0.
- Commit: `6c0e4917`.
