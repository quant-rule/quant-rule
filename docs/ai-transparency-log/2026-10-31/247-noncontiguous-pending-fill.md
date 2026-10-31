# AI Transparency Log — Non-Contiguous Pending Fills

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential implementation and verification passes
- Work item: #247
- Plan: `docs/execution-plans/2026-10-31/audit-unified-ladder.md` (W1)
- Finding: AUD-2026-10-31-01
- Validation: `docs/reports/2026-10-31/w1-gap-fill-validation.md`

## Intent and boundary

The human approved the unified-ladder remediation plan and decisions D1–D12.
W1 fixes the Critical finding that a pending entry fills on the first supplied
bar after a data gap, using the pre-gap ATR. Decision D1 chose rejection of a
non-contiguous pending fill.

The change stays inside the execution and runner boundary: it adds an explicit
contiguity input to `ExecutionEngine::advance`, rejects a pending market order
when the next bar is not contiguous, and records the rejection. It does not
change the strategy rule, the model, the portfolio, or any threshold. It does
not add a dependency or `unsafe` code. The human authorized commits and PRs on
2026-10-31. The work was committed as `7d29dcc`, pushed to
`fix/247-noncontiguous-pending-fill`, and opened as PR #257; it is not merged.

## Implemented

- Added `RejectReason::NonContiguousBar` and its report code
  `non_contiguous_bar`.
- Changed `ExecutionEngine::advance` to take a `contiguous` flag. The engine
  rejects a pending order on a false flag, records the rejection, and leaves the
  position unchanged.
- Kept the bracket evaluation on a non-contiguous bar, so a gap through a stop
  or target still exits at the bracket price.
- Moved the runner's contiguity calculation before `advance` and passed the
  same value to the strategy input, so execution and strategy see one decision.
- Added engine tests for a pending entry, a pending exit, and a bracket exit on
  a non-contiguous bar.
- Added a runner test with a removed execution bar and the corresponding 4h
  bucket removed, so the entry bar's context is unchanged.
- Updated the execution module docs, the `advance` rustdoc, the order rejection
  rustdoc, and the trade-intent contract.

## Verification and evidence

| Command | Observed |
|---|---|
| Gap probe, fixed engine | `EX-01 fixed fill count=0 rejections=[NonContiguousBar] position=Flat` |
| Contiguous control | `EX-01 control fill count=1` |
| Fail-before, branch disabled | 2 engine rejection tests and the runner sparse-source test failed |
| Restored, branch enabled | All 4 `non_contiguous` tests and the runner test passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | 45 test-result-ok suites, 0 failures |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Passed |
| `git diff --check` | Clean |
| Commit and PR | `7d29dcc`; PR #257; not merged |

The fail-before run is the regression guard evidence. It temporarily replaced
the `if contiguous` branch with `if true` while keeping the new signature; the
three behavior tests failed. The branch was restored from a backup, and the
tests passed again.

## Learning and rejected suggestions

- Rejected inferring the cadence inside the engine from consecutive candle
  timestamps. The engine does not own the timeframe; the runner does. An
  inferred interval would misclassify a legitimately sparse series and would
  duplicate the timeframe contract.
- Rejected adding a `bar_interval_secs` field to `ExecutionConfig`. It would
  make a fill-mechanics config own series cadence and would require a schema
  change for a caller-supplied fact.
- Rejected canceling the pending order in the runner before `advance`. The
  engine owns order records and rejections; a caller-side cancel would leave the
  order and the rejection in two places and could drift.
- Rejected rejecting only entries. An exit that crosses a gap would silently
  close at the next available open, contrary to decision D1. Both pending
  market sides are rejected; the bracket path still handles risk exits.
- Rejected suppressing the bracket on the gap bar. A stop or target that is
  reached on that bar must still be evaluated; only the pending market order is
  invalid.
- Kept the runner test in memory rather than adding a large CSV fixture. It
  removes one execution bar and the 4h bucket that contains it, so the entry
  context is unchanged and the test is deterministic.
