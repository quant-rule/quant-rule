# AI Transparency Log — Risk Cap and Sizing Rejection Causes

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential implementation and verification passes
- Work item: #263 (W5b)
- Parent: #251 (W5)
- Plan: `docs/execution-plans/2026-10-31/audit-unified-ladder.md` (W5)
- Findings: AUD-2026-10-31-14, -26

## Intent and boundary

The human approved the unified-ladder remediation plan. W5b closes the two
remaining W5 findings: a risk-capped fill must be a multiple of the configured
quantity step, and the report must distinguish the sizing rejection cause
instead of collapsing every cause to `missing_quantity`.

The change adds an explicit step to the execution cap path and a sizing-rejection
channel on the completed run. It does not change the strategy rule or the
uncapped sizing path. It regenerates the runner trade and metric goldens because
step-flooring a capped quantity changes the trade quantities and downstream
equity by design.

## Implemented

- Added `ExecutionEngine::submit_with_step`; `submit` keeps the old signature and
  passes `None`.
- The runner passes `ledger.quantity_step()` on every submission. A capped fill
  is floored to a step multiple, and a cap below one step is rejected with
  `RiskLimit`.
- Added `SizingRejection::code`.
- Added `BacktestRun::sizing_rejections`, populated when `size_entry` fails.
- Extended the execution report text and JSON with `sizing_rejections` counts.
- Added the step-cap and short-disabled cause tests.
- Regenerated `runner_trades_golden.csv` and `runner_metrics_golden.csv`
  deliberately: quantities now end on the step grid, and final equity, commission,
  and net P&L move by the corresponding small amount. The decisions fixture is
  unchanged.

## Verification and evidence

| Check | Observed |
|---|---|
| Fail-before, cap step ignored | `a_risk_capped_fill_respects_the_quantity_step` failed |
| Fail-before, sizing cause not recorded | `a_rejected_short_is_never_an_open_strategy_position` failed |
| Restored | Both passed |
| Runner golden | 6 trades and all metrics match the regenerated fixture |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | 45 test-result-ok suites, 0 failures |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Passed |
| `git diff --check` | Clean |

The golden delta is deliberate: for example, trade 2 quantity
`91.09367536313772` became `91.09360000000001`, and final equity
`9409.959422030162` became `9409.959666975237`. The decision sequence did not
change.

## Learning and rejected suggestions

- Rejected adding the step to `ExecutionConfig` or the uncapped sizing path. The
  step belongs to the portfolio sizing contract; only the risk cap needs it.
- Rejected making `submit` take a fifth argument. A separate
  `submit_with_step` keeps existing callers and doc examples stable.
- Rejected a separate sizing-rejection order record. The execution report already
  has a rejection section; a parallel sizing count is clearer and does not
  pretend the entry became an order.
- Rejected hand-editing the runner goldens. They were regenerated from the run
  and the decision fixture was confirmed unchanged.
- Kept the execution fee and commission rules unchanged. W5a already documented
  the prior-mark commission rule.
