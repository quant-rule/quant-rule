# AI Transparency Log — Portfolio Accounting

- Date: 2026-10-28 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#171](https://github.com/quant-rule/quant-rule/issues/171)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md)
- Branch/worktree: `feat/171-portfolio-accounting` in `quant-rule--171-portfolio-accounting`
- PR: [#183](https://github.com/quant-rule/quant-rule/pull/183)
- Status: PR open; merge after checks pass.

## Intent and boundary

P2 turns execution fills into capital: cash, the open position, equity, trades,
costs, funding, drawdown, and the summary metrics the portfolio report publishes.
The runner (#172) supplies each strategy bar and its fills; the ledger is a pure
state machine with no I/O, clock, randomness, or command change.

The boundary is the `portfolio` module. It now consumes execution's `Fill` as
input; execution does not know the portfolio exists. The ledger publishes
records and metrics, not labels; the shell renders them.

## Work performed

1. Added `PortfolioLedger` with entry, exit, mark, funding, and sizing methods.
2. Added `PortfolioMetrics` and `MonthlyReturn`, and the ledger's trade,
   equity-point, funding-charge, and risk-event accessors.
3. Added commission on equity before each operation, the fill's own execution
   fee, funding at aligned UTC boundaries, and the latching drawdown halt.
4. Added the longest-underwater duration, exposure, win rate, profit factor,
   average trade, average hold, and monthly returns.
5. Added a checked-in golden fixture for a short trade with funding and a stop,
   plus tests for the long path, halt, empty run, open position, zero equity,
   determinism, causality, and invalid fill sequences.

## Decisions and corrections

- **Cash account, not a margin account.** A long spends the notional and a short
  receives it, so `cash` and `position_value` are meaningful in the equity CSV
  and `exposure = sum(position_value) / sum(equity)` measures the invested
  fraction. A futures-style account would leave cash unchanged and make
  `position_value` the unrealized P&L, which would make exposure near zero. The
  two models produce the same equity curve; the report's cash/position split
  decides between them.
- **Commission on the last known equity.** A fill at the open is charged on the
  previous close's equity (adjusted by funding), not on the fill price. The
  runner marks at the close after the fill, so the fill price is not the last
  known equity. This matches "equity before the operation".
- **Funding before fills, and only at an exact boundary.** A zero-volume bar and
  an unaligned bar produce no charge. A missing boundary bar is never passed to
  the ledger, so the boundary is skipped rather than caught up.
- **The halt latches at or above the limit.** It records one risk event, blocks
  `size_entry` with `SizingRejection::Halted`, and leaves the open position to
  its strategy, stop, or target exit. It does not liquidate.
- **`max_drawdown_duration_bars` is the longest run of marked bars with a
  positive drawdown.** The contract names the field but not the definition. A
  recovery to the peak resets the run; an unrecovered run at the end still
  counts. This definition is causal and needs no future bar.
- **`realized_pnl` is closed-trade net P&L; `unrealized_pnl` is the open
  position's gross P&L minus its entry costs and accrued funding.** `net_pnl` is
  their sum, and the ledger invariant is `final_equity = initial_cash + net_pnl`.
  Counting an open position's entry costs nowhere would break that invariant.
- **Profit factor is null with no trades or no losses.** A loss-only run has a
  defined profit factor of `0.0`. Win rate is `wins / trades`; a breakeven trade
  is neither a win nor a loss but stays in the denominator.
- **Monthly returns publish a numeric year and month.** The engine publishes no
  label; the shell renders `YYYY-MM`. A month with no equity point has a null
  return, and a gap breaks the prior-equity chain rather than reusing an older
  month's end.
- **The plan's two implementation commits were consolidated.** Accounting,
  commission, funding, and the halt are one state machine; an intermediate
  ledger that could record a trade but not its funding would compile but not be
  a meaningful review unit. The work item has a feature commit, a test commit,
  and a docs commit.
- The first long-scenario exposure expectation used only the first two marks
  (`21,000 / 20,980`). The exit bar also marks, so the denominator is
  `31,959.01`; the assertion now includes it.
- The test commit briefly declared `mod ledger_tests` twice; clippy's
  `duplicate_mod` caught it, and the duplicate was removed before the commit was
  published.
- The invalid-sequence test shadowed the `ledger()` helper with a local of the
  same name; the local is now `open`.

## Verification and evidence

Commands were run in `quant-rule--171-portfolio-accounting` on 2026-10-28.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; no warning |
| `cargo test` | 1,314 passed, 0 failed, 1 ignored (`store_100k`) |
| `cargo test -p quantrule-backtest` | 140 unit, 6 + 9 integration, 38 doctests passed |
| `cargo doc --no-deps` | Passed; exit 0; no warning |

The full-suite count rose from 1,302 (C1 baseline) to 1,314. The increase is the
twelve ledger tests; no existing test was changed or removed.

## Learning

- A golden fixture is most useful when it fixes the costs separately from the
  prices. The fixture pins the entry and exit commission, the funding amount, and
  the resulting cash, so a change to any one of them fails on its own line.
- The cash-account choice is not cosmetic: it decides what `cash` and
  `position_value` mean in the report. Writing the invariant
  `final_equity = initial_cash + net_pnl` down before the code made the open
  position's cost treatment unambiguous.
- A latching risk flag is simpler than a state enum: the ledger only needs to
  know whether a new entry is allowed, and the position's exit path is unchanged.
