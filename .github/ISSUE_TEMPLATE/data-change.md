---
name: Data change
about: A source, version, partition, or admissibility change
title: "[Data] "
labels: ""
assignees: ""
---

Goal: change the input data without hiding a defect or rescuing a failed
hypothesis.

Scope:
- Source:
- Instrument:
- Timeframe:
- Coverage:
- Data version:
- Partition:
- Affected baselines:
- Admissibility rule:

Done when:
- [ ] The quality report has no unexplained gap, duplicate, or malformed row.
- [ ] The data version is explicit and immutable.
- [ ] The instrument and partition are declared.
- [ ] The admissibility rule passed before any outcome.
- [ ] The affected baselines are re-established or retired.
- [ ] The changelog and AI log are present.

Claim type: one of Problem Claim, Behaviour Claim, Outcome Claim, or ADR (Architecture Decision)

Workflow: [`docs/workflows/data-lifecycle.md`](../../docs/workflows/data-lifecycle.md)

Prior records reviewed:
- 

Changelog row:
- 
