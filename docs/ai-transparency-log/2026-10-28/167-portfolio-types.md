# AI Transparency Log — Portfolio Types and Sizing

- Date: 2026-10-28 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#167](https://github.com/quant-rule/quant-rule/issues/167)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md)
- Branch/worktree: `feat/167-portfolio-types` in `quant-rule--167-portfolio-types`
- PR: [#180](https://github.com/quant-rule/quant-rule/pull/180)
- Status: PR open; merge after checks pass.

## Intent and boundary

P1 defines the portfolio type layer that the accounting engine (#171) and the
portfolio report (#174) will use: configuration, sizing, state, and the record
types a completed run publishes.

The boundary is the type layer only. P1 adds no accounting: cash, equity, and
P&L are not updated by any code in this work item. It adds no fill consumption,
no I/O, no clock, no randomness, no serde, and no command change. The new module
depends on `strategy::Side` and not on `execution`.

## Work performed

1. Added `portfolio/config.rs`: `PortfolioConfig` with public fields and a
   `validate()` method, plus `Currency`, `CommissionConfig`, `CommissionBase`,
   `CommissionPer`, `FundingConfig`, `FundingInterval`, `MaxDrawdownConfig`,
   `DrawdownAction`, and `MarkToMarket`.
2. Added `portfolio/sizing.rs`: `SizingMode::FixedNotional` and
   `SizingMode::EquityFraction`, `size()`, and the typed `SizingRejection`.
3. Added `portfolio/state.rs`: `PortfolioState` and `OpenPosition` with their
   validation errors.
4. Added `portfolio/records.rs`: `TradeRecord`, `EquityPoint`, `FundingCharge`,
   `RiskEvent`, and `ExitReason`.
5. Wired `pub mod portfolio` in `lib.rs` and re-exported the public types from
   `portfolio/mod.rs`.
6. Added `portfolio/tests.rs`: validation, sizing, state, record-code, display,
   and determinism tests.

## Decisions and corrections

- `PortfolioConfig` uses public fields plus `validate()`, as approved for P1,
  instead of a ten-argument constructor or a builder. The nested configs
  (`CommissionConfig`, `FundingConfig`, `MaxDrawdownConfig`) keep private fields
  and validated constructors because each enforces an invariant of its own.
- Leverage is a configuration check only. `validate()` rejects any value other
  than `1.0`; sizing applies no affordability cap, per the contract.
- A `halt` action with no drawdown limit is rejected. The alternative — halting
  on a limit that was never set — has no defined threshold.
- A quantity below one step returns `SizingRejection::BelowOneStep` rather than
  `0.0`, so a rejected entry cannot be mistaken for a valid zero-sized entry.
- `cargo doc` reported one unresolved link: `config.rs` referenced
  `SizingRejection` without importing it. The link now uses
  `super::sizing::SizingRejection`; no behavior changed.
- The first test draft asserted only that invalid limits return an error. The
  final tests assert the specific error variant, so a validation rule cannot
  regress into a different rejection unnoticed.

## Verification and evidence

Commands were run in `quant-rule--167-portfolio-types` on 2026-10-28.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; `Finished dev profile` with no warning |
| `cargo test` | 1,283 passed, 0 failed, 1 ignored (`store_100k`) |
| `cargo test -p quantrule-backtest` | 126 unit, 6 + 6 integration, 36 doctests passed |
| `cargo doc --no-deps` | Passed; exit 0; no warning |

The full-suite count rose from 1,217 (C0 baseline) to 1,283. The increase is the
new portfolio unit tests and doc examples; no existing test was changed or
removed.

## Learning

- Splitting a module into a commit that is not yet referenced by `mod.rs` keeps
  each commit compiling while the type layer is built up. The final commit wires
  the test module together with its tests.
- A type layer is where a bad configuration is cheapest to reject. Validating
  funding intervals, commission base and period, and drawdown action here keeps
  those checks out of the accounting loop.
- The rustdoc warning is a build failure in spirit: `cargo doc` exits 0 with a
  warning, so the check must inspect the output rather than the exit code alone.
