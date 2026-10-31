# AI Transparency Log — Core Execution and Portfolio Invariants

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential implementation and verification passes
- Work item: #261 (W5a)
- Parent: #251 (W5)
- Plan: `docs/execution-plans/2026-10-31/audit-unified-ladder.md` (W5)
- Findings: AUD-2026-10-31-05, -15, -16, -17, -25, -27
- Decisions: D4=A, D11 (deferred), D12=A for the reporting items here

## Intent and boundary

The human approved the unified-ladder remediation plan. This sub-issue fixes the
Major exit-quantity defect and the core execution/portfolio invariants that share
the same ledger and order-record boundary: a partial exit cannot corrupt the
account, a second pending order is refused, equity state follows fills and
funding, a capped order records the filled quantity, and slippage is reported in
quote currency. The prior-mark commission rule is documented and pinned.

It does not add partial-exit support, change the strategy or model, or implement
the step-aware risk cap (AUD-14) and the sizing-rejection cause (AUD-26), which
remain in W5.

## Implemented

- Added `PortfolioLedgerError::ExitQuantityMismatch` and rejected an exit fill
  whose quantity differs from the open position; the state and trades are
  unchanged.
- Added `ExecutionError::PendingOrderExists`; a second non-hold submit on one bar
  is refused and the first order remains.
- Added `refresh_equity_state`; `open`, `close`, `apply_funding`, and `mark` now
  keep peak equity and drawdown in step with equity.
- `filled_record` now records the fill quantity rather than the original order
  quantity, so a risk-capped fill reconciles.
- The execution summary multiplies per-unit slippage by fill quantity.
- Documented the prior-mark commission rule and added a gap-exit test for it.

## Verification and evidence

| Check | Observed |
|---|---|
| Fail-before, quantity check disabled | Partial-exit test failed; the 50-unit exit was accepted |
| Fail-before, equity refresh disabled | Post-fill drawdown test failed; drawdown stayed 0 |
| Fail-before, pending guard disabled | Second-submit test failed; the first order was overwritten |
| Fail-before, filled quantity reverted | Cap test failed; the order kept the original quantity |
| Fail-before, slippage reverted | Summary test failed; total was per-unit |
| Restored | All five checks passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | 45 test-result-ok suites, 0 failures |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Passed |
| `git diff --check` | Clean |

## Learning and rejected suggestions

- Rejected supporting partial exits in this sub-issue. The ledger is a
  single-position model; a mismatch is a typed rejection until a partial-exit
  contract is approved.
- Rejected a silent clamp of the exit quantity. A clamp would corrupt the trade
  record and hide the caller error.
- Rejected a supersede record for a second submit. The first pending order is
  still live for the next bar; returning a typed error is the simplest honest
  behavior.
- Rejected changing the commission base to the fill price. The prior-mark rule is
  now explicit in the current contract; changing it would alter every trade
  golden and is a separate accounting decision.
- Kept the step-aware cap and sizing-cause reporting in W5; both need an
  execution/portfolio boundary change and remain open.
