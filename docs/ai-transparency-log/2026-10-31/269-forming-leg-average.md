# AI Transparency Log — Dashboard Forming-Leg Average

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential documentation pass
- Work item: #269 (W6b1), parent #268
- Finding: AUD-2026-10-31-11
- Plan: `docs/execution-plans/2026-10-31/audit-unified-ladder.md` (W6)

## Intent and boundary

The implementation and golden fixture compute `average_net_displacement_atr_per_bar`
for a forming leg, while the dashboard contract said a confirmed-leg measurement
is not published before `origin_available_at`. This change amends the contract
rather than the code: the average is detail-only and is not availability-gated;
pace, radius, and percentile measurements remain gated.

No model formula, golden, or command output changed.

## Verification

| Check | Observed |
|---|---|
| `cargo test -p quantrule-model dashboard` | Golden unchanged and passing |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | 45 suites, 0 failures |
| `git diff --check` | Clean |

## Learning and rejected suggestions

- Rejected gating the average and regenerating the golden. The average is a
  descriptive detail, and the existing golden already defines it for forming
  legs; changing the model would be a behavior change without a hypothesis.
- Kept the pace, radius, and percentile availability rules unchanged.
