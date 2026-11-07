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
