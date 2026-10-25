# AI Transparency Log: Architecture Principles in AGENTS.md

- Work item: `116-architecture-principles`
- Issue: [#116](https://github.com/quant-rule/quant-rule/issues/116)
- Pull request: [#117](https://github.com/quant-rule/quant-rule/pull/117)
- Date: 2026-10-25 (UTC)

## Entry: review and packaging of the architecture principles

- **Date and tool:** 2026-10-25 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Review the human-authored architecture principles
  added to `AGENTS.md`, package them as issue #116, and prepare the
  documentation-only pull request. The principles, their wording, and their
  placement were authored and decided by the human; the review did not change
  their meaning.
- **Review evidence:**
  - Checked the new section against sections 2, 3, and 12 of `AGENTS.md`: no
    contradiction with the pipeline, layer isolation, axis independence,
    purity, causality, or determinism rules.
  - Confirmed the precedence sentence keeps the existing architecture,
    invariants, and constraints authoritative over the summary principles.
  - Corrected only the double blank lines around the subsection, matching the
    document's single-blank-line spacing.
  - `git diff --check`: clean.
  - `cargo fmt --all -- --check`: clean.
  - `cargo clippy --all-targets -- -D warnings`: clean.
  - `cargo test`: 890 passed, 0 failed, 4 ignored.
  - `cargo doc --no-deps` with `-D warnings`: clean.
- **Learning:**
  - Moving the subsection under section 2 was considered because the content
    is architectural, but rejected: section 3 already holds the module-level
    conventions, and the preamble scopes the principles to design, review, and
    refactoring.
  - The principles restate some existing rules. That duplication is retained
    as a consolidated checklist; the precedence sentence prevents the summary
    from becoming a second, competing source of rules.
- **Remaining:** human review and merge; no code, command, test, dependency,
  or behavior changes.
