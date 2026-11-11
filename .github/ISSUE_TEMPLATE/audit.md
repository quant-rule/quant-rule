---
name: Audit finding
about: A read-only finding and its remediation
title: "[Audit] "
labels: ""
assignees: ""
---

Goal: close the audit finding without changing model or strategy behavior unless
the finding explicitly requires it.

Scope:
- Audit ID:
- Finding:
- Evidence:
- Chosen remediation:
- Included:
- Deliberately excluded:

Done when:
- [ ] The finding is `Fixed` and the Improvement Verifier can mark it `Verified`.
- [ ] A regression guard fails before and passes after, or the reason a fail-before test is impossible is recorded.
- [ ] The full gate passes.
- [ ] No unrelated change is included.

Claim type: one of Problem Claim, Behaviour Claim, Outcome Claim, or ADR (Architecture Decision)

Workflow: [`docs/workflows/audit-workflow.md`](../../docs/workflows/audit-workflow.md)

Prior records reviewed:
- 

Changelog row:
- 
