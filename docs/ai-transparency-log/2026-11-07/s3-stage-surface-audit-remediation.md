# AI Transparency Log — S3.x Stage-Surface Audit Remediation Plan

- Date: 2026-11-07 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: S3.x stage-surface audit remediation plan
- Plan: `docs/execution-plans/2026-11-07/s3-stage-surface-audit-remediation.md`
- Boundary: Initial Planner entry only. Read-only verification of the audit
  findings; one proposed plan document and this log. The later addenda record the
  approved implementation, verification, and closure.

## Intent and boundary

The human approved the `allow_short` removal and asked to begin the audit work
again. The audit found three Critical, two Major, and five Minor findings.
`AUD-2026-11-07-03` was removed by the human decision; the remaining nine
findings were re-read against the tree based on that removal. This plan proposes
the remediation, the sub-issue split, the regression guards, and the decisions
the human must make before implementation.

The plan does not implement a fix and does not create fix issues. It names the
Quant Researcher, Quant Developer, and Quant Validator for the findings that
change strategy behavior (01, 02, 04, 05).

## Verification and evidence

- The plan branch is based on `feat/remove-allow-short` at `e5fe7de`; that
  branch passed `cargo fmt`, `cargo clippy`, `cargo test --workspace` (1,312
  passed, 0 failed, 8 ignored), `cargo doc`, all contract checks, the structure
  check, and `git diff --check`.
- The remaining findings were re-read against the current code. The removal
  changed the portfolio sizing path and the shared run-config bytes; it did not
  touch the strategy input, the backtest timeframe/root logic, the manifest
  writer, the registry parser, or the evidence table paths named by the
  findings.
- The audit's reproduction recipes remain valid. The plan references the audit
  document and the removal plan.
- The plan adds documentation only. `check_links.py` and `check_terms.py` were
  run on the plan branch.

## Learning and corrections

- The Planner did not re-open the `allow_short` finding: the human removed the
  rule, so the finding is `Won't fix`, not a candidate for a code fix.
- The event-window finding was kept as a **human decision** rather than silently
  choosing between converting the window and changing the contract. The
  implementation and the published contract genuinely disagree, and either
  correction changes strategy behavior.
- The event-stream and registry findings were kept as separate sub-issues. The
  registry/role gate (S2) is a parser/input boundary; the stream unification
  (S3) is a stage-handoff correction. Restricting the bracket to the compression
  range removes the observed diagnostic case, but the two event streams still
  need one definition.
- No fix was implemented. The plan's `Proposed` status is deliberate; the
  remaining findings stay `Open` until the plan is approved.

## Limits

- One sequential Planner pass; no independent Code Reviewer, Quant Validator, or
  human review is claimed.
- The plan depends on the human choosing D1-D4. The recommended options are
  proposals, not decisions.
- The plan does not create or predict GitHub issue numbers; the fix issues are
  created only after approval.

## Addendum — implementation (2026-11-07)

The human approved the plan with D1-D4 = A and asked to proceed with the full
remediation. The implementation is on `fix/s3-audit-remediation`, based on the
approved plan commit. It is committed locally as `cb08c11`; it is not pushed or
merged.

What was implemented:

- S1: a shared strategy context validator used by `strategy explain`, `strategy
  run`, and `backtest run`; the coupled runner rejects a declared/run timeframe
  mismatch and a condition outside execution plus context.
- S2: a build-time registry table generated from `docs/registry/indicators.json`;
  unregistered and diagnostic identities are rejected; the compression bracket is
  restricted to the registered `(3,8,5)` range event.
- S3: `EventStrategyInput` now reads `timeframe.all_events()`, matching the
  standalone bundle reader.
- S4: the execution-bar window is converted to each context timeframe's bars.
- S5: the backtest manifest records `strategy_config_id`.
- S6: a later-bar causality test for the strategy snapshot.
- S7: the stale status lines and the evidence table were corrected.
- S8: the backtest root is checked before the model bundle is written.

Verification: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`,
`cargo test --workspace` (1,323 passed, 0 failed, 8 ignored), `cargo doc` with
`-D warnings`, the contract checks, the structure check, and `git diff --check`
pass. The analytics and qualification goldens moved only in the manifest hash.

Corrections during implementation:

- The first registry build assumed `(event_type, subtype, source)` was unique. It
  is not: the registry groups several event codes under one family identity. The
  build now merges a family's roles with diagnostic dominance, then a signal, then
  context; the table remains one row per identity.
- The first registry lookup passed a `u8` source where the table uses `u16`; the
  engine converts before the lookup.
- The old diagnostic/unregistered configs were re-run against the new shell: both
  are rejected before a model read.

No independent Quant Validator or Code Reviewer pass is claimed. The plan's
Quant Validator requirement for S1-S4 and the merged-tree gate remain open.

Publication: PR #534 (removal), PR #535 (plan), and PR #536 (remediation) were
merged on 2026-11-07 at `812d58c`, `0c1c265`, and `e8f99cc`. The merged `main`
tree passes the full gate: 1,323 tests, 0 failed, 8 ignored. Independent Quant
Validator, Code Reviewer, and Improvement Verifier passes remain outstanding.

## Addendum — verification remediation (2026-11-07)

A separate Improvement Verifier pass on merged `main` at `9ec6ef9` returned
`Not verified`. The AUD-02 parity test used a core event and passed on the
pre-fix tree; the AUD-04 diagnostic-input test was a boundary assertion; the
AUD-08 guard was snapshot-only; and the plan's Quant Validator and Code Reviewer
passes were sequential same-agent passes, not independent records.

A separate read-only Quant Validator subagent and a separate read-only Code
Reviewer subagent reviewed the merged fixes and the follow-up guards. They are
separate sessions, not human reviews, and not independent model judgments. The
Quant Validator returned `pass with residual risks`; the Code Reviewer returned
`changes requested` for the uncommitted state, two weak guards, documentation
defects, and a per-bar allocation in `all_events()`.

What changed in the follow-up:

- `an_extended_event_reaches_both_strategy_paths` exercises the extended
  `(1,3,19)` level event. With the pre-fix core-only stream it fails `enter` vs
  `hold`; the fixed stream matches both paths.
- `the_context_view_window_is_converted_from_execution_bars` asserts the 5m/15m
  view reaches back only 7 context bars. With the conversion removed it fails at
  oldest event 14; the fixed code starts at bar 19.
- `the_snapshot_boundary_keeps_both_engine_decisions_causal` runs the
  event-composite and compression-bracket engines on the bar-40 snapshot from
  60 bars, 80 bars, and a changed later bar, and asserts an `Enter` decision.
- `a_diagnostic_identity_is_not_strategy_readable` checks the registered
  diagnostic role is not a strategy role.
- `TimeframeModel` caches the merged core/extended event batch; `all_events()`
  returns a slice, removing the per-bar allocation in the coupled loop. The
  `render_flat` caller was adjusted for the slice.
- The redundant per-context `MissingTimeframe` loop in `backtest run` was removed
  because `validate_context` already validates every context timeframe.
- The strategy table, execution-timeframe coupling, window rounding,
  artifact-boundary wording, audit historical note, and the audit/plan evidence
  were corrected.

Verification on the follow-up tree: `cargo fmt --all -- --check`, `cargo clippy
--all-targets -- -D warnings`, `cargo test --workspace` (1,328 passed, 0 failed,
8 ignored), `cargo doc` with `-D warnings`, the contract checks, the structure
check, and `git diff --check` pass. The two fail-before probes above were run by
temporarily reverting the specific code path and then restoring it.

At the time of this addendum the findings remained `Fixed`, not `Verified`. This
change set was merged in PR
[#538](https://github.com/quant-rule/quant-rule/pull/538) at `08dd260`.

## Addendum — fresh review on the merged tree (2026-11-07)

A fresh read-only Quant Validator subagent reviewed `08dd260` and returned `pass
with residual risks`: causality, determinism, the input boundary, fixture
decisions and accounting, and the full gate pass; the two fail-before probes were
reproduced. Residual risks: the bracket causality arm is `Hold` on the fixture,
the standalone context-window path has no direct fail-before test, the
`EventDiagnostic` branch is defensive, and the view-order documentation is
imprecise.

A fresh read-only Code Reviewer subagent returned `changes requested`. It found
no correctness defect in the merged code, but the governance records still
described the follow-up as unmerged, and the missing execution/context timeframe
branches had no regression test.

Corrections:

- Four `strategy/context.rs` unit tests cover the missing execution timeframe,
  missing context timeframe, execution mismatch, and condition-outside-context
  branches.
- `backtest_rejects_a_missing_context_timeframe` builds a run whose data ladder
  omits `1h`; it fails when `validate_context` is removed.
- `backtest_rejects_a_strategy_execution_timeframe_mismatch` now uses a valid
  15m/1h strategy so the mismatch is the only defect.
- `backtest_refuses_an_unrelated_file_in_the_root` asserts no stage directory or
  manifest exists after the refusal.
- The bracket causality arm is documented as `Hold` on the fixture; the
  `EventDiagnostic` branch carries a defensive-code comment.
- The strategy contract states the registry/role rejection and the bracket
  `(3,8,5)` restriction; the artifact contract says evidence re-sorting is
  display-only.
- The audit, plan, and this log now record the PR #538 merge.

At the time of the review pass the findings remained `Fixed`.

## Addendum — verification closure (2026-11-07)

- Date and tool: 2026-11-07 (UTC), OpenCode with DeepSeek V4.1 Flash.
- Intent and boundary: record the final independent records and the closure of
  the S3.x stage-surface audit remediation. The final passes were read-only; the
  status update is documentation only. No model or strategy behavior changed.
- Evidence: the final Quant Validator pass on merged `main` at `99c4308`
  returned `pass with residual risks`; the final Code Reviewer pass returned
  `approve with residual risks`; the Improvement Verifier returned `Verified` for
  `AUD-2026-11-07-01`, `-02`, `-04`, `-05`, `-06`, `-07`, `-08`, `-09`, and
  `-10`, and confirmed `AUD-2026-11-07-03` remains `Won't fix`. The verifier
  reproduced the fail-before probes and ran the full gate: 1,333 passed, 0
  failed, 8 ignored; fmt, clippy, doc, the contract checks, the structure check,
  and `git diff --check` pass.
- Learning: the first verification pass was right to reject guards that passed
  pre-fix. The extended-event and converted-window probes are the direct evidence
  that the guards exercise the defects. The remaining residual risks are the
  bracket causality arm being `Hold` on the fixture, the standalone
  context-window path sharing the tested helper without its own fail-before test,
  the defensive `EventDiagnostic` branch, the unit-only missing-execution branch,
  and the synthetic holdout label.
- Limits: the Quant Validator, Code Reviewer, and Improvement Verifier passes
  were separate read-only subagent sessions, not human reviews and not
  independent model judgments. The human remains the final acceptance authority;
  no edge or profitability claim is made.
