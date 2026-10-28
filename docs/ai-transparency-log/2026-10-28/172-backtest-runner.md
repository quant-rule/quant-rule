# AI Transparency Log — Backtest Runner

- Date: 2026-10-28 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#172](https://github.com/quant-rule/quant-rule/issues/172)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md)
- Branch/worktree: `feat/172-backtest-runner` in `quant-rule--172-backtest-runner`
- PR: [#184](https://github.com/quant-rule/quant-rule/pull/184)
- Status: PR open; merge after checks pass.

## Intent and boundary

B2 composes the model, strategy, execution, and portfolio in one deterministic
pass. The runner computes the model once, walks the strategy timeframe bar by
bar in the contract's order, and publishes the model run, decisions, execution
batches, and the completed ledger. The reports consume that result without
recomputing the model.

The runner is pure. It performs no I/O, reads no clock, uses no randomness, and
adds no command or shell code. The shell's config loader already owns the file
boundary.

## Work performed

1. Added `BacktestConfig` and `run_backtest(inputs, config) -> BacktestRun`.
2. Added the per-bar order: funding, `advance`, fills to the ledger, mark,
   decision and sizing, `submit`; then `finish`.
3. Added `BacktestRun` with the model run, decisions, execution batches, the
   ledger, and a metrics accessor.
4. Extended `ExecutionConfig` with `stop_atr_multiple` and
   `target_atr_multiple` and made the engine use them. The shell's config loader
   now builds `with_bracket`, closing the C1 gap where the file exposed the
   multiples but the engine ignored them.
5. Added a deterministic triangle-wave M15 series and checked-in fixtures for
   the decisions, trades, and summary metrics; covered determinism, causality,
   empty inputs, a missing strategy timeframe, no trades, an open position at the
   end, the halt, a wide bracket, and a short run.

## Decisions and corrections

- **The bracket multiples are part of `ExecutionConfig`.** The plan's B2 paths
  did not include the execution config, but leaving the engine on its 2/3
  constants would silently ignore `execution.json`. `new` keeps the 2/3
  defaults, so E2/E3 and every existing caller are unchanged; `with_bracket`
  validates the configured values, and the engine reads them from its config.
  The shell loader's `ExecutionRunConfig` now exposes them through its
  `ExecutionConfig`.
- **A refused sizing is submitted as a missing quantity.** The ledger returns
  `SizingRejection::Halted` (or a below-step rejection), and the runner submits
  `None`. Execution records `MissingQuantity`, so the blocked entry is visible in
  the rejection stream; the ledger's risk event and `halted` flag record the
  halt itself. A `None` quantity is the E3 contract's "sizing produced nothing".
- **The strategy's position is not rewritten by execution.** When a bracket
  closes a position, the strategy still believes it is open and later emits an
  exit; execution rejects it with `NoPosition`. The golden run shows three such
  rejections. This is the existing strategy boundary, not a runner defect: a
  rejected or externally closed position is evidence in the record, and the
  strategy is not silently rewritten.
- **An empty run has no batches.** The per-bar batches are always kept, even
  when empty, to preserve the call order; an empty `finish` batch is omitted. An
  empty input therefore has no decisions and no batches.
- **The golden series is a triangle wave, not a sine.** `sin` is not guaranteed
  bit-identical across libm implementations, so the test data is built from
  integer phases and exact binary fractions. The first version used the previous
  close as the open; every post-peak high then equalled the peak and no pivot was
  confirmed. Setting `open = close` gives strict highs and lows, and the major
  pivots appear.
- The first test imported the strategy's `ExitReason` instead of the portfolio's;
  the trade's exit reason is the portfolio enum.
- The causality comparison initially compared three prefix equity points with
  two full points. The prefix ends mid-day, so only the completed dates are
  comparable; the test now compares the first two points.
- The wide-target test initially expected an open position. With 320 bars the
  third trade closes by agreement, so the expectation is three agreement exits
  and no open position.

## Verification and evidence

Commands were run in `quant-rule--172-backtest-runner` on 2026-10-28.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; no warning |
| `cargo test` | 1,325 passed, 0 failed, 1 ignored (`store_100k`) |
| `cargo test -p quantrule-backtest` | 151 unit, 6 + 9 integration, 38 doctests passed |
| `cargo doc --no-deps` | Passed; exit 0; no warning |

The full-suite count rose from 1,314 (P2 baseline) to 1,325: one execution
config test and ten runner tests. No existing test was changed or removed; the
E2/E3 goldens still reproduce under the default bracket.

## Learning

- The per-bar order is the whole design. Funding must precede fills so the
  commission sees the post-funding equity; the mark must precede the decision so
  the entry is sized from the close; the submit must follow the mark so the order
  is scheduled for the next open.
- A config field that the engine cannot apply is a wiring task, not a parsing
  task. Carrying the ATR multiples in `ExecutionConfig` was smaller than keeping
  a parallel wrapper and a documented gap.
- A checked-in fixture generated from the implementation is a regression guard,
  not an independent derivation. The synthetic series is simple enough to review
  (a triangle wave on a trend, entries where the three axes agree, targets at
  3×ATR), and the invariants `final_equity = initial_cash + net_pnl` and the
  decision/order/fill id sequences are asserted separately.
