# AI Transparency Log — Program Consistency Remediation

- Date: 2026-11-16 (UTC)
- Tool: OpenCode agent (DeepSeek V4.1 Flash 1017)
- Work item: `docs/execution-plans/2026-11-16/program-consistency-remediation.md`
- Claim type: Behaviour Claim

## Intent and boundary

The human asked for a read-only review of the released tree, then approved the
resulting execution plan and requested full implementation. The agent wrote the
plan and this log, then implemented the approved P1-P4 scope and the P5 gate
hardening. The changes are limited to documentation, the browser client, the
contract rustdoc, example config labels, and repository checkers/tests. No model,
strategy, execution, portfolio, or signal behavior was changed. No commit, push,
pull request, merge, tag, or deployment was made; those gates remain separate.

The implementation covers the 24 `CONS-2026-11-16-NN` findings recorded in the
plan: the executable runbooks, the current config contracts, the policy-layer
removal, the run execution path, browser detail views and SVG export, storage and
HTTP/control-plane/deployment contracts, release/status/model identity, and the
gate checks.

## Verification and evidence

- `tools/gate.sh` on the implemented tree: passed; 2,118 tests passed, 0 failed,
  14 ignored; fmt, clippy `-D warnings`, rustdoc, and 24/24 checkers clean.
- `bun test` in `apps/web`: 121 passed, 0 failed.
- `bun test` in `apps/desktop-shell`: 35 passed, 0 failed.
- `cargo test` in `apps/desktop-shell/supervisor`: 9 passed, 0 failed.
- `bun run check` in `apps/web`: 0 errors, 0 warnings.
- `bun run build` in `apps/web`: production build succeeded.
- `bun run e2e` in `apps/web`: 41 passed, 0 failed, including dataset/config
  detail views and the SVG chart download.
- Direct parser checks before the fix:
  - `run submit --data-version … --config …` → `error: unknown option '--data-version'`
  - `run fetch … --out …` → `error: unknown option '--out'`
  - `inspect events --data-version …` → `error: unknown option '--data-version'`
- New checker tests: command grammar (`unknown option`, positional arity), retired
  command in a Current config, Current-to-Historical workflow link, capability
  test id, and a living-doc retired tag.
- The example config `model_id` was changed from `M1.0` to `M1.3`; the one test
  that asserted the old label was updated to the current identity. The gate
  passes with the new value.

## Learning

- The green gate did not mean the current contracts were true. The checkers
  validated names and indexes; they did not parse documented invocations, scan
  `docs/config` for retired commands, reject Current-to-Historical workflow
  links, verify that a claimed browser control exists, or cross-check living tags.
  Those checks now exist and have seeded tests.
- The 2026-11-15 tag renumbering was applied to the release map and some status
  rows but not to living architecture/workflow documents or the reusable release
  procedure. The living documents now use the current map, and the release step
  uses a version variable.
- The service split moved run submission and cancellation to `evidence`, but the
  architecture documents still named `gateway` and `backtest`. The service
  catalog, HTTP contract, and control plane now agree.
- Adding a detail button changed an existing Playwright accessible-name match.
  The control now has the stable accessible name `Details` and the dataset name
  in its title, and the profiles tests pass again.
- The implementation needed one test expectation update because the example
  config label changed. This is a deliberate fixture change, not a weakened test.

## Corrections and rejected suggestions

- The first pass treated the 106-parameter line as a possible historical value.
  The registry and the 610 de-prop-firm record confirm the current count is 100;
  the line is now 100.
- The first pass did not include the browser detail and SVG-export gaps. A second
  read-only scan verified them against the Svelte components and the client
  contract; they are now implemented and tested.
- A rename of the `model_version` wire field was rejected for this plan because
  it changes the API contract. The plan corrects the rustdoc, prose, examples,
  and UI label, and records the three distinct contexts. A rename remains a
  separate human decision.
- The composed Compose tier and the Windows package were not run in this
  worktree; the gate, web, desktop, supervisor, and Playwright tiers were run.
  This is recorded as a verification limitation, not a pass.
- No independent Improvement Verifier or Quant Validator was available. The
  verification table records a sequential implementer pass; independent review
  and human acceptance remain pending.
