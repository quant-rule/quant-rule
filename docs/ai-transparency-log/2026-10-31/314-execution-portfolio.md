# AI Transparency Log — Execution, Portfolio, and Lifecycle Fixes

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #314, branch `fix/314-execution-portfolio`; PR [#326](https://github.com/quant-rule/quant-rule/pull/326)
- Plan: `docs/execution-plans/2026-10-31/full-system-audit.md` (W7)
- Audit: `docs/audits/2026-10-31/full-system-audit.md` (AUD-48, -53, -54, -64, -66)
- Validator record: `docs/reports/2026-10-31/w7-execution-portfolio-validation.md`

## Intent and boundary

The human approved the full-system audit plan on 2026-10-31. W7 fixes the
entry-bar stop reference, the unavailable-Progression exit, the risk-cap step
floor, and the drawdown/halt mismatch, and removes two unreachable public
variants. It does not change the strategy hypothesis, thresholds, cost rates,
model values, or configuration schemas. The Quant Developer and Quant Validator
passes were performed sequentially by the same agent; no independent review is
claimed.

## Implemented

- **AUD-48.** `apply_bracket` now branches on the entry bar. On that bar the
  fill is the reference and the bracket is evaluated from the bar's high and
  low; the open gap rule remains for later bars. Long and short mirrors are
  covered.
- **AUD-53.** `context_side` returns `None` when the Progression agreement is
  `Unavailable`. A measured `Mixed` agreement still returns `Some(false)`, so
  `context_lost` exits only on a measurement.
- **AUD-54.** `floored_steps` is `pub(crate)`, re-exported by `portfolio`, and
  used by the risk-cap path. The cap now fits 0.7 with step 0.1 at price 1.0
  and one step at price 7.0.
- **AUD-64.** The ledger checks the halt after an entry, an exit, and a funding
  charge, in addition to a mark. The drawdown metric and the halt now observe
  the same equity refreshes. `records.rs` documents the broader observation
  points.
- **AUD-66.** Removed `PositionState::Closed`, `is_closed`, and
  `LadderContextError::MissingContextTimeframe`; corrected the position and
  context rustdoc and `trade-intent.md`. A missing context timeframe is not an
  error: it is reported in `missing_timeframes`.
- Updated `execution-v3.md` for the entry-bar rule and
  `policy-and-strategy.md` for the halt observation points.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo test -p quantrule-backtest --lib` | 189 passed, 0 failed, 1 ignored |
| `cargo test -p quantrule-backtest` | 189 unit, 3 + 9 + 3 integration, 40 doctests passed |
| `cargo test --workspace` | 1,563 passed, 0 failed, 7 ignored; 47 suites |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

Fail-before: with the pre-fix source files checked out from `main` while the new
tests were kept, eight of the nine new tests failed (both entry-bar references,
both cap boundaries, the unavailable-Progression hold, and the three halt
breaches). The measured-Mixed guard passed before and after by design. The full
record is in the validator document.

## Learning and rejected suggestions

- The first funding-breach test used the default rate and expected a 100-unit
  charge; the observed charge was 1. The test now sets the rate explicitly, so
  the fixture states the charge it relies on.
- I considered treating the entry bar as a normal bar and only changing the
  stop reference, but that leaves a target gap impossible by construction and
  adds a branch with no reachable case. The entry-bar branch reads only the
  high, low, and level prices.
- I considered implementing the just-closed state instead of removing it, but
  no consumer reads it and the portfolio owns the trade record. Removal is the
  smaller contract.
- The `RiskEvent` is recorded at the fill/funding bar rather than at the next
  mark, because that is the equity refresh that breached. The event keeps the
  configured limit and peak, so the diagnosis can reproduce it.

## Limitations

- Sequential same-agent Quant Developer and Quant Validator passes; no
  independent model judgment.
- No new experiment or out-of-sample run. This is a correctness fix, not an
  edge claim.
- The stop fill beyond the bar low/high under slippage remains the execution
  model's documented approximation; it is not modelled against an order book.
