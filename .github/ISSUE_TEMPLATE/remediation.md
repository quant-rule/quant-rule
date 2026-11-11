Goal: Fix the audit findings named below with the chosen remediation, preserving determinism, causality, and layer boundaries.

Scope:
- Audit IDs:
- Chosen option:
- Included:
- Deliberately excluded:

Done when:
- [ ] Regression guard fails before and passes after (or the reason a fail-before test is impossible is recorded).
- [ ] `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `cargo doc --no-deps` pass.
- [ ] The audit finding is `Fixed` and the Improvement Verifier can mark it `Verified`.
- [ ] The `CHANGELOG.md` release entry (or the reason none is needed) and the AI transparency entry are present.
- [ ] No unrelated changes.

Claim type: one of Problem Claim, Behaviour Claim, Outcome Claim, or ADR (Architecture Decision)

Workflow: [`docs/workflows/audit-workflow.md`](../../docs/workflows/audit-workflow.md)

Prior records reviewed:
-
