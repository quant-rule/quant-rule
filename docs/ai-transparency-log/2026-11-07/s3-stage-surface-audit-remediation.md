# AI Transparency Log — S3.x Stage-Surface Audit Remediation Plan

- Date: 2026-11-07 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: S3.x stage-surface audit remediation plan
- Plan: `docs/execution-plans/2026-11-07/s3-stage-surface-audit-remediation.md`
- Boundary: Planner stage only. Read-only verification of the audit findings;
  one proposed plan document and this log. No code, test, issue, branch, commit,
  push, PR, or merge for the remaining findings.

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

Publication: PR #536 (`fix/s3-audit-remediation`) is stacked on PR #535 (the
plan) and PR #534 (the removal). All three CI checks pass. No merge was
performed.
