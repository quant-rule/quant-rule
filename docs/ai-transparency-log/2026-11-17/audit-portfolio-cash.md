# AI transparency log — Portfolio cash remediation (P1.4, AUD-20)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 3
  (Execution) for `AUD-2026-11-17-20`.
- Claim type: Behaviour Claim.
- Intent: close the portfolio-cash Critical: a fixed notional could exceed the
  account equity, an equity fraction of `1.0` plus costs could make cash
  negative, the ledger accepted the fill, a halted account could still open a
  position, and funding measured drawdown against the previous close.
- Boundary: the work is in the P1.4 worktree, branch
  `fix/audit-portfolio-cash`, based on current `main` `7ae3a984`. No model or
  strategy signal changed; no push, PR, merge, or board change was made. The
  Quant Validator pass is a sequential pass by the same agent, not an
  independent validator.

## What changed

- `SizingMode::max_notional` caps a fixed notional at `equity * leverage`, and
  `SizingMode::size` caps a fixed notional at the current equity. The docs say a
  fixed notional is capped, not ignored.
- `PortfolioLedger::open` refuses an entry when the account is halted
  (`PortfolioLedgerError::Halted`) and refuses a fill whose cash would be
  negative (`InsufficientCash { required, available }`).
- `PortfolioLedger::apply_funding` marks the position at the funding price:
  `equity = cash + position_value(price)`, not `state.equity - signed`.
- The portfolio golden fixture is a **short** entry. It is over-collateralized
  and remains byte-identical; the fix does not change it.
- The long-entry ledger tests now use a 100,000 funded config (10% allocation)
  instead of the all-in 10,000 config that the new cash check correctly refuses.
  The `an_all_in_entry_is_refused` test asserts the refusal; the
  `funding_marks_the_position_at_the_boundary_price` test asserts the new
  funding mark; the halt test asserts `open` refuses while halted.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-backtest --lib` | 210 passed, 0 failed |
| `cargo test -p quantrule-app --lib` | 164 passed, 0 failed |
| `cargo fmt --all -- --check` | pass |
| `cargo test -p quantrule-backtest --lib portfolio::ledger_tests` | 21 passed, 0 failed |
| `tools/gate.sh` | pass: 2,322 passed, 0 failed, 16 ignored; 25 checkers |

## Quant validation (sequential, not independent)

- **Reproduction:** the old `SizingMode::size` returned `notional / price`
  without equity; an all-in 10,000/10,000/0.1% entry produced cash `-10` and
  the ledger accepted it. The new test `an_all_in_entry_is_refused` returns
  `InsufficientCash { required: 10_010, available: 10_000 }` and leaves the
  state unchanged.
- **P&L:** the golden short fixture is unchanged; the long-entry tests use a
  funded config and their expected commission/P&L/equity values were updated
  deliberately. No published strategy or portfolio baseline is accepted
  (`docs/README.md` status line), so no accepted baseline is invalidated and no
  `INVALIDATIONS.md` row is added. Historical strategy records remain
  historical; a later accepted strategy/portfolio baseline must use the new
  accounting.
- **Causality/determinism:** the portfolio ledger is a fold over bars; the new
  checks are pure functions of the fill, cash, and position. The sizing and
  ledger tests are deterministic.
- **Residual risk:** the ledger now fails a run that would overdraw on a gap-up
  fill, rather than allowing negative cash. That is the intended refusal; a
  strategy whose sizing relies on the old overdraw will fail and must be
  re-sized. This is a deliberate behavior change, not a silent one.
- **Independence:** the Quant Researcher/Developer/Validator roles were
  performed sequentially by the same agent. This is not an independent
  validation and is not claimed as one.

## Learning

- The audit's warning was correct: the existing portfolio golden is a short and
  is not invalidated by the cash check; the **long-entry unit tests** encoded
  the all-in negative-cash model. The golden stays, the tests move to a funded
  config, and the refusal is a new test.
- The old `zero_equity_has_no_exposure_and_a_full_loss` test was replaced, not
  deleted to obtain a pass: its scenario required a negative cash balance,
  which the fix now refuses. `an_all_in_entry_is_refused` asserts the refusal
  and leaves the state unchanged; a zero-equity state is no longer reachable
  through a valid long entry.
- The halt test needed a 2% drawdown limit rather than the default 20% once the
  account was funded; the test's purpose is the halt latch, not a fixed
  percentage.
- The funding test is the direct proof of the equity-mark change; it uses a
  funding price below the last mark, where the old formula would have reported a
  much higher equity.
- The full gate passed on the fix branch (2,322 tests, 16 ignored, 25
  checkers). The composed tier and the shipped `verify-mini` configuration were
  not run here: the example still uses a 10,000/10,000 all-in portfolio, so the
  composed run must re-size it and re-record the canonical digest before the
  next release. That is a P7.2/P3 release task, not a claim of this log.
