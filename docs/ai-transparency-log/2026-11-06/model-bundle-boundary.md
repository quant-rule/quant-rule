# AI Transparency Log — Model Bundle Boundary

- Date: 2026-11-06 (UTC)
- Tool: DeepSeek V4.1 Flash through the OpenCode agent harness
- Work item: #528 `Execute the third behavior-neutral refactor cycle`
- Plan: [`docs/execution-plans/2026-11-06/model-bundle-boundary.md`](../../execution-plans/2026-11-06/model-bundle-boundary.md)
- Status: approved; implementation and local verification complete; PR and merge pending

## Intent and boundary

The agent was asked to proceed with cycle 3 after cycle 2 was verified and
closed. It performed the read-only Structure Auditor pass at `172b65a`,
re-verified REF-2026-11-06-05, captured the baseline, and wrote a third-cycle
plan. After human approval, it moved the model-bundle writer to
`crate::model_bundle`, added `ModelBundleError`, updated the report and backtest
callers, and broadened the structural guard. It did not change a fixture,
command, schema, or behavior. It did not push, open a pull request, or merge.

## Work performed

- Inspected the merged `main` at `172b65a`, the cycle-2 assessment and closure,
  the bundle writer, the command error types, and the structural guard.
- Confirmed the remaining stage-to-command reference:
  `backtest/run.rs:140` calls `crate::command::model_bundle_from`.
- Confirmed `RunError::Bundle` is used only by the moved writer and
  `ReportError::EmptyPeriod` is used only by the writer and `spec.rs`.
- Computed the report bundle baseline `59124f...` and a new deterministic
  backtest bundle baseline `4173b505...`; two output directories produced
  identical bytes for each.
- Ran `python3 tools/check_structure.py --report`.
- Wrote the cycle-3 assessment and the proposed plan.
- After approval, created issue #528, moved the bundle writer, provenance, and
  artifacts to `src/model_bundle/`, added `ModelBundleError`, updated the
  command and backtest callers, and added the two command-coupling guards.
- Ran the full gate and a clean-worktree verification pass at the implementation
  commit.

## Verification and evidence

Commands actually run and observed results:

- `cargo test --workspace --no-fail-fast` at `172b65a` — 1,309 passed, 0 failed,
  8 ignored.
- Full gate at `1dd13ca` and the docs closure at `172b65a` — fmt, clippy, test,
  doc, file size, structure, registry, validation, verification matrix,
  verification evidence, emitters, command surface, links, terms, legacy, oracle,
  progression, improvement, research, program, coverage, and diff check pass.
- `find crates -path '*/fixtures/*' -type f | sort | xargs sha256sum | sha256sum`
  — `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1`.
- `report model` bundle — `59124f63a2741697c533e699eeaf191684d84811d354b7778518b4d03ef82512`;
  unchanged.
- `backtest run` bundle —
  `4173b5055814048597ee5e946da9b117e34dadf5f867de750aaa2f2cf15f085e`;
  two output directories produced the same hash and `diff -r` reported identical
  bytes.
- `python3 tools/check_structure.py --report` — 197 `src` files / 63,774 lines;
  34 files over 600, 7 over 800, 0 over 1,000.
- `cargo test --workspace --no-fail-fast` — 1,312 passed, 0 failed, 8 ignored
  (1,309 plus 3 new model-bundle error tests).
- `python3 -m unittest discover -s tools -p 'test_check_structure.py'` — 17
  tests pass, including the 6 stage/artifact coupling tests.
- Stage and artifact command references — 0.
- Golden fixture hash `da7389...`, report bundle hash `59124f...`, and backtest
  bundle hash `4173b505...` — unchanged.
- Clean-worktree verifier pass at `9dc160a`: every gate passes; 1,312 passed, 0
  failed, 8 ignored; all three hashes unchanged; both coupling guards pass.

## Learning and rejected suggestions

- Moving only `model_bundle_from` into `render_report` was rejected. It would
  leave the spec path in `command` and create a second half-boundary; the whole
  artifact contract should move together.
- Keeping `RunError` as the writer's error was rejected. It would make the
  neutral `model_bundle` module depend on `command`, recreating the reverse edge
  under a new name.
- A guard that only checks `model_bundle_from` was rejected. The stage invariant
  is "no stage module imports the command layer", so the guard is broadened to
  any `crate::command` reference in the six stage directories.
- Reusing the cycle-2 report hash as the only artifact check was rejected. The
  backtest is the caller that currently depends on the command writer, so the
  plan adds a deterministic backtest-bundle hash.
- A single cycle combining REF-05 and REF-03 was rejected. The date move is a
  separate, low-risk seam and belongs in its own cycle.
- The first compile failed because `model_bundle_with_spec` was still
  `pub(super)` after the move. Widening it to `pub(crate)` was the only
  visibility change the command caller needed; the compiler caught it before
  any test.

## Limitations

- One agent performed the Structure Auditor, Refactor Planner, Refactor
  Implementer, and verifier passes, including the clean-worktree pass. These
  were sequential passes, not independent agents or independent model
  judgments.
- The pull request, merge, and post-merge independent verification have not
  happened. The finding stays `In progress`; the plan's verification record
  covers the local gate and the clean-worktree pass.
- The backtest bundle hash is new; it is reproducible but does not replace the
  command-level shell tests.
- The structural report is a heuristic; the findings are supported by code
  references and review.
