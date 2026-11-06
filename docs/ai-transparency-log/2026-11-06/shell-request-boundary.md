# AI Transparency Log — Shell Request Boundary

- Date: 2026-11-06 (UTC)
- Tool: DeepSeek V4.1 Flash through the OpenCode agent harness
- Work item: #525 `Execute the second behavior-neutral refactor cycle`
- Plan: [`docs/execution-plans/2026-11-06/shell-request-boundary.md`](../../execution-plans/2026-11-06/shell-request-boundary.md)
- Status: approved; implementation started

## Intent and boundary

The agent was asked to run the Refactor Workflow. It performed the read-only
Structure Auditor pass on a fresh baseline, re-verified the carried findings,
found one new High finding, and wrote a proposed second-cycle plan. It did not
change code, tests, fixtures, commands, or schemas. It did not commit, push, or
open a pull request. The plan is `Proposed` and awaits human approval before
implementation.

## Work performed

- Inspected the repository, worktrees, branch state, previous assessment, first
  cycle plan, workflow contract, and structural checker.
- Captured the baseline at commit `7e6ab31`.
- Re-ran the full gate in the project directory before creating the task
  worktree.
- Ran `python3 tools/check_structure.py --report` and a production-only
  `unwrap`/`expect` scan to check whether the heuristic count is a real finding.
- Re-verified REF-2026-11-06-02, REF-03, and REF-04 against the current code.
- Found REF-2026-11-06-05: `backtest/run.rs` calls the command-layer
  `model_bundle_from` writer.
- Found REF-2026-11-06-06: the largest test suites are close to the 1,000-line
  gate, while the production files are below it.
- Created the task worktree and branch `refactor/shell-request-boundary`.
- Wrote the re-scan assessment and the proposed cycle-2 plan.

## Verification and evidence

Commands actually run and observed results:

- `cargo test --workspace --no-fail-fast` — 1,309 passed, 0 failed, 8 ignored.
- `cargo fmt --all -- --check` — pass.
- `cargo clippy --all-targets -- -D warnings` — pass.
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` — pass.
- `python3 tools/check_file_size.py` — pass.
- `python3 tools/check_structure.py` — pass.
- `python3 -m unittest discover -s tools -p 'test_check_structure.py'` — 11
  tests pass.
- `python3 tools/check_registry.py` — pass.
- `python3 tools/check_validation.py` — pass.
- `python3 tools/check_verification_matrix.py` — pass.
- `python3 tools/check_verification.py` — pass.
- `python3 tools/check_event_emitters.py` — pass.
- `python3 tools/check_command_surface.py` — pass.
- `python3 tools/check_links.py` — 588 Markdown files, no broken relative links.
- `python3 tools/check_terms.py` — 356 files, no banned terms.
- `python3 tools/check_legacy.py` — pass.
- `python3 -m unittest discover -s tools/oracle -p 'test_*.py'` — 54 tests pass.
- `python3 tools/oracle/progression_replay.py --check` — pass.
- Historical improvement and research tooling — pass.
- `find crates -path '*/fixtures/*' -type f | sort | xargs sha256sum | sha256sum`
  — `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1`,
  unchanged from the first baseline.
- `report model` over `crates/quantrule-shell/tests/fixtures/config-run/run.json`
  — sorted-file bundle hash
  `59124f63a2741697c533e699eeaf191684d84811d354b7778518b4d03ef82512`;
  a second output directory produced the same hash and `diff -r` reported
  identical bytes.

The production-only panic scan was a read-only Python check. It excluded
`*_tests.rs`, files under `tests`, and lines after an inline `#[cfg(test)]`
marker. It found no executable `unwrap`/`expect` in the shell, no executable
hits in the data and model production code beyond doc examples, and only test
modules in the backtest crate. The structural report's 315-line heuristic is
therefore not evidence of a panic problem.

## Learning and rejected suggestions

- The first assessment recorded a bundle hash but not the command that produced
  it. The old value cannot be reproduced from the repository. The re-scan
  defines a sorted-file hash and records the method instead of reusing an
  unverifiable number.
- Folding REF-05 into this cycle was rejected. It needs a neutral
  `ModelBundleError` so the error strings stay identical; that is a second seam
  and a second plan, not a move-only extension.
- Moving only the seven crossing request types was rejected. All eleven are the
  same concept, and keeping four in `command/types.rs` would leave the module
  with the same mixed role.
- Reusing the first assessment's status table was rejected. A new campaign needs
  a fresh status record; the old document remains the historical cycle-1 record.
- A guard that rejects every `crate::command` reference was rejected for this
  cycle because `backtest/run.rs:140` still calls the writer. The guard targets
  the request vocabulary and records REF-05 as the next seam.

## Limitations

- One agent performed the Structure Auditor and Refactor Planner passes. These
  were sequential passes, not independent agents or independent model
  judgments.
- The implementation and independent verification have not happened. The plan's
  verification record is empty.
- The re-scan covers the structural report, the shell module imports, the test
  sizes, and the documented boundaries. It is not a line-by-line review of every
  engine module; findings outside the campaign may still be discovered later.
- The structural report is a heuristic; the findings are supported by code
  references and review.
