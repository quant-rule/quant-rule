# AI Transparency Log — Strategy Context Contract

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential implementation and verification passes
- Work item: #250
- Plan: `docs/execution-plans/2026-10-31/audit-unified-ladder.md` (W4)
- Findings: AUD-2026-10-31-04, -12, -13, -20, -22
- Decisions: D3=A, D5=A, D10=B

## Intent and boundary

The human approved the unified-ladder remediation plan. W4 makes the strategy
context contract truthful and safe: the inert diagnostic fields are documented,
a missing or unavailable context no longer forces an exit, overflowing weight
sums are rejected, a context from another bar is ignored, and the neutral band
uses the model's shared threshold. The shipped strategy's intent expiry and
slippage behavior is documented rather than expanded.

The change stays inside the context builder, the ladder strategy, the strategy
contract/explain output, and their tests. It does not change the entry or exit
hypothesis, the model formulas, execution, or portfolio accounting. It does not
add a dependency or `unsafe` code. The human authorized commits and PRs; the work
was committed as `4e2b357` and pushed to `fix/250-context-contract`. The
validator record is `docs/reports/2026-10-31/w4-context-contract-validation.md`.
The work was opened as PR #260.

## Implemented

- Documented `weights.structure`, the context `kind` literals, the stage
  histogram, and the agreement codes as diagnostic-only in `strategy-v3.md`.
- Added a `strategy explain` diagnostics line.
- `context_lost` now requires a measured context flip. A missing context or a
  null share holds an open position instead of exiting.
- `LadderWeights::new` rejects a per-axis weight sum that overflows the finite
  range.
- The context neutral band now uses `COMPACT_STATE_THRESHOLD` instead of a
  duplicated `0.1`.
- `decide` verifies the context's execution timeframe, bar index, and open time
  against the input; a mismatch returns Hold.
- The golden fixture's bar 19 is now an open-position missing-context row that
  must hold.
- The trade-intent contract and ADR now state that the shipped strategy leaves
  `valid_until` and `max_slippage_bps` unset and that the context is not
  persisted as a separate artifact.

## Verification and evidence

| Check | Observed |
|---|---|
| Fail-before, weight sum check disabled | `invalid_ladders_and_weights_are_rejected` failed: `1e308` weights were accepted |
| Fail-before, measured-context rule disabled | Golden bar 19 expected `hold`, got `context_lost` |
| Fail-before, identity check disabled | A context from bar 0 entered on bar 1 |
| Restored | All three checks passed |
| Golden fixture | 21 rows, including the open-position missing-context row |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | 45 test-result-ok suites, 0 failures |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Passed |
| `git diff --check` | Clean |
| Commit and PR | `4e2b357`, `a68ea8c`; PR #260; not merged |

## Learning and rejected suggestions

- Rejected using the Structure score or a context `kind` literal in the rule.
  That would be a behavior change and needs a pre-registered hypothesis and
  Quant Validator, not a documentation fix.
- Rejected treating a missing context as an exit. The exit is named
  `context_lost`, and an unmeasured context is not a measured flip; the position
  holds.
- Rejected adding `valid_until`/`max_slippage_bps` fields to strategy v3 in this
  work item. That is a schema change; the shipped strategy is documented as not
  setting them, and custom callers retain the engine checks.
- Rejected adding a context CSV to the strategy bundle. The ADR claim is
  narrowed instead: the in-memory context is the decision input, and the
  strategy bundle does not persist a separate context artifact.
- Kept the context identity check in the strategy rather than the builder. The
  builder already sets the fields; the strategy is the boundary that must refuse
  a mispaired context.
