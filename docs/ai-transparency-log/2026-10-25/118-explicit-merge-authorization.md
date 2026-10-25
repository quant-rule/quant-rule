# AI Transparency Log: Explicit Human Authorization for Approval and Merge

- Work item: `118-explicit-merge-authorization`
- Issue: [#118](https://github.com/quant-rule/quant-rule/issues/118)
- Date: 2026-10-25 (UTC)

## Entry: explicit-instruction exception

- **Date and tool:** 2026-10-25 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** The human instructed: "CHANGE THE RULE IF I TELL YOU TO DO IT EXPLICTLY THEN DO IT". Section 9 now allows an AI to approve or merge its own pull request only with an explicit, pull-request-specific human instruction. Section 10 now states the same exception. No other approval gate changed.
- **Changes:**
  - Added "Approving a pull request" to the gated operations in section 9.
  - Replaced the blanket self-approval and self-merge prohibition with the explicit-instruction exception and the recording requirement.
  - Added approvals to the human's authorization list and aligned the Code Reviewer Agent boundary in section 10.
- **Verification and evidence:**
  - `git diff --check`: clean.
  - `cargo fmt --all -- --check`: clean.
  - `cargo clippy --all-targets -- -D warnings`: clean.
  - `cargo test`: 890 passed, 0 failed, 4 ignored.
  - `cargo doc --no-deps` with `-D warnings`: clean.
- **Learning:**
  - The earlier blanket prohibition blocked an explicitly authorized merge. The human corrected it and asked for the exception. The exception is scoped to one pull request, so a general approval or a standing instruction cannot authorize a future self-merge.
  - The instruction is recorded in the pull request before the merge, as the new rule requires.
- **Remaining:** merge the rule-change pull request and the explicitly authorized PR #117; the human owns acceptance.
