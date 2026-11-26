# AI Transparency — Frozen Pack Lifecycle and Upload Resolution

- Date: 2026-11-26 (UTC)
- Work item: `REM-RA-008` ([#1239](https://github.com/quant-rule/quant-rule/issues/1239))
- Origin: RA-007 D1 (Major) and D2 (Moderate), [validation record](../../reports/2026-11-26/rem-ra-quant-validation.md)
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim; no model, strategy, signal, or edge claim.

## 1. Intent and boundary

Make the pack lifecycle a real new-run gate. The data service must resolve every
v5 reference at upload, and both upload and submission must refuse a pack that
is not frozen. The check is on new runs only: a run that already recorded a
reference keeps its lineage readable after the pack is retired.

## 2. Implemented

- The data-service upload route calls the existing `config_references` resolver
  before the idempotent lookup and refuses a pack whose state does not
  `allows_new_runs`. A missing pack/signal or a missing pack object is refused
  at upload, not later at submission.
- The evidence submission route checks every pack reference's provenance state
  before it writes the run row. A missing provenance, draft, validated or
  retired pack returns `pack.invalid`.
- The composed v5 verifier validates and freezes the three fixture packs, and
  reuses an already frozen pack on the replica pass.
- Tests: data-service upload covers draft/validated/retired, missing pack,
  missing pack object and the three-reference order; evidence covers all three
  non-frozen states and asserts no run row is written.

## 3. Verification

Commands actually run in the RA-008 worktree:

- `cargo test -p quantrule-data-service --test configs_v5` → 4 passed.
- `cargo test -p quantrule-evidence-service --test promoted_runs a_v5_run_refuses` → 1 passed.
- `cargo test -p quantrule-data-service -p quantrule-evidence-service` → all
  suites passed, including the existing v4 and lineage cases.
- `cargo fmt --all` clean.
- The broader gate and clippy are run before the PR.

No composed, lab, live-Postgres, or release run was performed.

## 4. Learning

- The pack parser starts every document in `Draft`; the parser alone cannot
  enforce the lifecycle. The registry provenance is the only place the current
  state exists, so both upload and submission need the resolved provenance.
- Validation must run before the upload's idempotent lookup. Otherwise a config
  uploaded while frozen and re-uploaded after retirement would return the old
  valid record and bypass the gate.
- The composed fixture's second pass reuses the same frozen packs; the pack
  validate/freeze commands are not idempotent across a frozen state, so the
  script checks the returned state and skips the transition.

## 5. Next action

D3 is #1240. A fresh independent validation is required before `V-RA-07` can move
from `Fail` to `Pass`. Commit, push, PR, merge and cleanup remain gated.
