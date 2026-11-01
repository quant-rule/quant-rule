# Audit Workflow

- Date: 2026-11-01 (UTC)
- Status: Current; approved by the 2026-11-01 workflow reset
- Scope: program defects, contract drift, test gaps, documentation defects, and
  workflow defects

## Purpose

The Audit Workflow finds a defect in the program, turns it into a reviewed plan,
implements the fix, and verifies that the finding is actually closed. It answers
one question: **is the program correct and consistent with its contracts?**

It never tests a trading hypothesis. A change that alters model or strategy
behavior is a Quant change and needs the Quant roles, even when an audit found
the defect. Keeping the two routes separate is what makes each result
attributable: an audit fix is judged by correctness, not by backtest profit.

## Stages

```text
Audit -> Remediation Plan -> Execution -> Verification
```

| Stage | Owner | Input | Output | Gate |
|---|---|---|---|---|
| 1. Audit | Audit Agent | Fixed baseline commit, contracts, tests, current behavior | `docs/audits/<YYYY-MM-DD>/<scope>-audit.md` with findings in `Open` | Read-only; no code, test, issue, or PR changes |
| 2. Remediation Plan | Planner | Audit document and current code | Approved execution plan with finding-to-issue-to-verification traceability | Human approves the plan; no implementation before approval |
| 3. Execution | Engineering; Quant roles when behavior changes | Approved plan and fix issues | Branch, implementation, tests, PR; finding moves to `Fixed` after merge | Scope limited to the plan; no test weakening |
| 4. Verification | Improvement Verifier | Merged fix, regression guard, validation record | Finding moves to `Verified`, or returns to remediation | Read-only; cannot verify its own audit or plan |

### 1. Audit

The Audit Agent reads a fixed baseline and records what is wrong with reproducible
evidence. Each finding carries an ID, severity, area, evidence, impact, options,
recommendation, verification method, status, and links. Critical and Major
findings carry at least two remediation options.

The audit does not modify the system. It does not create fix issues; the Planner
does that after the plan is approved.

### 2. Remediation Plan

The Planner reproduces or code-reads every finding, marks stale or duplicate
findings, finds the root cause, and compares remediation options. The result is
one execution plan at `docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md` with a
traceability table:

```text
finding -> issue/sub-issue -> PR -> verification
```

The human approves the plan before any implementation starts. The Planner does
not implement and does not approve.

### 3. Execution

The Engineering roles implement only the approved scope. A model or strategy
behavior change also requires the Quant Researcher, Quant Developer, and Quant
Validator under the quant role rules. Every fix carries a regression test that
fails before the fix and passes after it, or a written reason why a fail-before
test is impossible. The finding moves to `Fixed` only after the fix is merged.

### 4. Verification

The Improvement Verifier independently checks that the merged fix closes the
finding. It confirms the regression guard, the diff, the tests, the absence of
unrelated changes, and the required Quant Validator record. It then moves the
finding to `Verified` or returns it for remediation. Only the Improvement
Verifier moves a finding to `Verified`; only the human decides `Won't fix`.

## Roles

| Role | Responsibility | Boundary |
|---|---|---|
| Audit Agent | Finds and records problems with reproducible evidence | Read-only; no code, tests, issues, or PRs; no self-verification |
| Planner | Verifies findings, finds the root cause, writes the plan and fix issues | No implementation; no approval |
| Implementation / Engineering | Implements the approved fix and its regression test | No scope expansion; no test weakening |
| Improvement Verifier | Independently verifies closure without regressions | Read-only; cannot verify its own audit or plan |

The Quant Researcher, Quant Developer, and Quant Validator join when the finding
changes model or strategy behavior. The Audit Workflow cannot close such a
finding alone.

## Finding statuses

| Status | Meaning | Moved by |
|---|---|---|
| `Open` | Recorded, not yet reviewed for action | Audit Agent |
| `Triaged` | Verified against the current code; severity and route agreed | Planner |
| `Planned` | Covered by an approved execution plan and fix issue | Planner after human approval |
| `In progress` | Implementation is underway | Execution |
| `Fixed` | Fix merged and regression guard present | Execution / PR |
| `Verified` | Closure independently checked | Improvement Verifier only |
| `Won't fix` | Accepted residual risk | Human only |
| `Duplicate` | Covered by another finding | Planner |
| `Stale` | No longer reproduces or no longer applies | Planner |

`Fixed` is not `Verified`. A merge is evidence that a change landed; it is not
evidence that the finding is closed.

## Green criteria

The Audit Workflow is green when:

- every Critical and Major finding is `Verified`, or the human has recorded a
  dated `Won't fix` with residual-risk acceptance;
- every fix has a regression guard, or a written reason why a fail-before test is
  impossible;
- `cargo fmt`, `cargo clippy --all-targets`, `cargo test`, `cargo doc`, the link
  check, and the term check pass on the merged tree;
- no test was weakened, skipped, or removed to obtain a pass;
- the diff contains no unrelated change;
- every model or strategy behavior change has the required Quant Validator
  record;
- documentation and command references match the changed behavior.

A release with an open Critical finding requires explicit human residual-risk
acceptance before it can proceed.

## Artifacts

| Artifact | Path | Owner |
|---|---|---|
| Audit document | `docs/audits/<YYYY-MM-DD>/<scope>-audit.md` | Audit Agent |
| Execution plan | `docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md` | Planner |
| Fix issues and sub-issues | Project board | Planner after approval |
| Pull request and verification record | Repository and execution plan | Engineering / Quant |
| AI transparency log | `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md` | Orchestrator |
| Finding status and links | Audit document | Planner, Execution, Improvement Verifier |

The audit document is the status record. The project board tracks the
remediation issues and pull requests, not the audit document itself.

## Rules

- Evidence must be reproducible: a command or a code reference, not an
  impression.
- Do not rewrite published evidence; correct it with a dated note.
- Distinguish confirmed defects from hypotheses and unexamined areas.
- A program fix and a trading change are never mixed in one plan, branch, or
  experiment.
- If a defect invalidated an improvement run, record the run in
  [`docs/improvement/INVALIDATIONS.md`](../improvement/INVALIDATIONS.md) and
  retire any baseline that used it.

## Related documents

- [Audit format](../audits/README.md)
- [Audit Workflow roles](../roles/2026-10-30/audit-workflow-roles.md)
- [Improvement Workflow](improvement-workflow.md)
- [`AGENTS.md` section 11](../../AGENTS.md)
