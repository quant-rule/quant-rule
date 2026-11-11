---
name: Release
about: Release readiness, tagging, rollback, or an operational incident
title: "[Release] "
labels: ""
assignees: ""
---

Goal: release or recover a change with recorded readiness and verification.

Scope:
- Tag:
- Included:
- Excluded:
- Rollback:
- Recovery:
- Open findings:

Done when:
- [ ] The full gate passes on the release commit.
- [ ] The changelog release entry and release note are present.
- [ ] The verification record is present.
- [ ] No Critical finding is open on the changed path.
- [ ] The human authorizes the release.
- [ ] The recovery path is stated.

Claim type: one of Problem Claim, Behaviour Claim, Outcome Claim, or ADR (Architecture Decision)

Workflow: [`docs/workflows/release-operations.md`](../../docs/workflows/release-operations.md)

Prior records reviewed:
- 

Changelog row:
- 
