# W5a Core Execution and Portfolio Validation

- Date: 2026-10-31 (UTC)
- Work item: #261 (W5a)
- Findings: AUD-2026-10-31-05, -15, -16, -17, -25, -27
- Plan: `docs/execution-plans/2026-10-31/audit-unified-ladder.md` (W5)
- Validator: Quant Validator (AI), sequential read-only pass
- Baseline: `main` before W5a; fixed branch `fix/251-execution-portfolio`
- Claim type: program correctness fix; no edge claim

## Scope

The validator checked that a partial exit is refused without changing the
account, a second pending order is refused, peak equity and drawdown follow
fills and funding, a capped order records the filled quantity, slippage is a
quote-currency total, and the prior-mark commission rule is pinned.

## Reproduction

| Check | Baseline | Fixed |
|---|---|---|
| 100-unit entry + 50-unit exit | Accepted; cash/equity corrupted | `ExitQuantityMismatch`; state and trades unchanged |
| Two entry submits on one bar | Second overwrote the first | `PendingOrderExists`; first fills |
| Entry fill after a mark | Drawdown and peak stale | `drawdown = 1 - equity/peak` |
| Capped entry | Order kept the original quantity | Order quantity equals fill quantity |
| Quantity-100 fill with 0.02 per-unit slippage | Total 0.02 | Total 2.0 |
| Gap exit after a prior mark | Prior-mark commission | Prior-mark commission; contract text matches |

The fail-before check disabled each guard while keeping the tests. All five
checks failed; with the guards restored, all passed.

## Commands and observed results

| Command | Observed |
|---|---|
| `cargo test -p quantrule-backtest --lib a_partial_exit_is_rejected_without_changing_the_account` | Passed |
| `cargo test -p quantrule-backtest --lib a_fill_updates_peak_equity_and_drawdown` | Passed |
| `cargo test -p quantrule-backtest --lib a_second_pending_order_is_rejected` | Passed |
| `cargo test -p quantrule-backtest --lib an_entry_above_the_risk_limit_is_capped_at_the_fill` | Passed |
| `cargo test -p quantrule-backtest --lib a_gap_exit_commission_uses_the_prior_mark` | Passed |
| `cargo test -p quantrule-shell --bin quantrule the_execution_summary_slippage_is_a_quote_currency_total` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | 45 test-result-ok suites, 0 failures |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Passed |
| `git diff --check` | Clean |

The existing full-close tests assert `final_equity == initial_cash + net_pnl`
(for example 10,000 + 979.01 = 10,979.01).

## Risks and limitations

- The validator and implementer are the same agent and model. This is a
  sequential pass, not an independent review or independent model judgment.
- The full-year SOLUSD source is not committed; the evidence is the ledger and
  engine fixtures and unit tests.
- The step-aware risk cap and the sizing-rejection cause remain open in W5.
- The prior-mark commission rule is documented rather than changed; a change
  would require a new accounting decision and regenerated trade goldens.

## Verdict

The evidence supports moving AUD-2026-10-31-05, -15, -16, -17, -25, and -27 to
`Fixed`. It is not a profitability or edge claim.
