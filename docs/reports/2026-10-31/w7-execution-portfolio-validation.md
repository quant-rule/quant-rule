# W7 Execution, Portfolio, and Lifecycle Validation

- Date: 2026-10-31 (UTC)
- Work item: #314
- Plan: `docs/execution-plans/2026-10-31/full-system-audit.md`
- Findings: AUD-2026-10-31-48, -53, -54, -64, -66
- Validator: Quant Validator role, same agent as the implementation, sequential pass
- Verdict: Pass for the recorded scope; no profitability claim

## Scope

The change fixes four behaviours and removes two dead public variants:

1. On the entry bar, the bracket uses the fill and the bar's high/low; the open
   gap rule applies only on later bars.
2. An unavailable Progression direction returns no context side, so an open
   position holds; a measured Mixed direction still exits on `context_lost`.
3. The risk-cap quantity floor reuses the tolerant `floored_steps`.
4. The drawdown halt is checked after an entry, an exit, and a funding charge,
   as well as after a mark, so a breach latches and records one `RiskEvent`.
5. `PositionState::Closed`/`is_closed` and
   `LadderContextError::MissingContextTimeframe` are removed; the trade-intent
   and context rustdoc describe the actual states.

No model axis value, strategy threshold, cost rate, or configuration schema
changes. Golden fixtures are unchanged.

## Reproduction

Commands run in this worktree, with observed results:

| Command | Observed |
|---|---|
| `cargo test -p quantrule-backtest --lib` | 189 passed, 0 failed, 1 ignored |
| `cargo test -p quantrule-backtest` | 189 unit, 3 + 9 + 3 integration, 40 doctests passed |
| `cargo test --workspace` | 1,563 passed, 0 failed, 7 ignored; 47 suites |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

## Fail-before evidence

The pre-fix source files were checked out from `main` while the new tests were
kept, then each test was run by name. Eight failed; the measured-Mixed guard
passed, which is expected because it protects the existing exit path from being
over-corrected.

| Test | Pre-fix result |
|---|---|
| `an_entry_bar_stop_uses_the_fill_not_the_pre_entry_open` | Failed: expected reference 100.01, got 100 |
| `a_short_entry_bar_stop_uses_the_fill_not_the_pre_entry_open` | Failed: expected reference 99.99, got 100 |
| `a_risk_cap_fits_the_exact_step_boundary` | Failed on the quantity assertion |
| `a_risk_cap_fits_one_step_when_the_price_is_seven_times_the_cap` | Failed: the entry was rejected instead of filling one step |
| `an_unavailable_progression_holds_an_open_position` | Failed: Exit instead of Hold |
| `an_entry_cost_breach_halts_and_records_one_risk_event` | Failed: `halted` stayed false |
| `a_funding_breach_halts_and_records_one_risk_event` | Failed: `halted` stayed false |
| `a_closing_loss_breach_halts_and_records_one_risk_event` | Failed: `halted` stayed false |
| `a_measured_mixed_progression_still_exits_on_context_lost` | Passed, by design |

After restoring the fixed source files, the same tests pass.

## Checks

- **Look-ahead and causality.** The entry-bar bracket reads only the bar that
  filled the entry. It cannot read a later bar; the later-bar gap rule is
  unchanged. The unavailable-Progression path reads the current complete
  context and returns no decision when the reading is missing.
- **Fill cost.** The stop now fills at the stop level plus the configured
  slippage against the position, instead of at the pre-entry open. Fees and
  slippage rates are unchanged. The probe case (fill 100.02, stop 100.01,
  open 100.0, low 99.99) now records reference 100.01 and price 99.99 to four
  decimals; before it recorded 100.0 and 99.98.
- **Risk cap.** The cap is the largest step multiple that fits the fill price
  under the same representation tolerance as portfolio sizing. Caps at 0.7 with
  step 0.1 now fit seven steps at price 1.0 and one step at price 7.0; before,
  the first used six steps and the second rejected the entry.
- **Drawdown and halt.** The same equity refresh drives the peak, drawdown,
  maximum drawdown, and halt. A breach at entry, funding, or exit latches once,
  records one risk event, and blocks later entries. The metric and halt now
  observe the same points.
- **Lifecycle.** The engine returns to `Flat` on the closing bar. No public
  state or error variant remains that no shipped path can reach.
- **Determinism.** All new computations are arithmetic over passed-in values;
  no clock, randomness, I/O, or unordered-map iteration is introduced.

## Limitations and residual risk

- This is a same-agent sequential validation, not an independent model or
  reviewer. The human remains the acceptance authority.
- The entry-bar fix changes stop/target fill prices for brackets tighter than
  the configured slippage. Existing fixtures did not exercise that case, so
  they are unchanged; a live run with such a bracket will now report a slightly
  different (correct) P&L.
- The check is over the implementation and its tests, not a new profitability
  experiment. No edge is claimed.
- The `RiskEvent` bar and time are those of the fill or funding charge that
  breached, not the later mark. This is the point the metric observes.
