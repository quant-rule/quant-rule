# AI Transparency Log — Execution Types

- Date: 2026-10-28 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#163](https://github.com/quant-rule/quant-rule/issues/163)
- Container: [#10](https://github.com/quant-rule/quant-rule/issues/10)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md`](../../execution-plans/2026-10-27/141-market-analysis-and-backtest.md)
- Boundary: [`docs/adr/2026-10-27/analysis-and-backtest-boundaries.md`](../../adr/2026-10-27/analysis-and-backtest-boundaries.md)
- Branch/worktree: `feat/163-execution-types` in `quant-rule--163-execution-types`
- PR: [#164](https://github.com/quant-rule/quant-rule/pull/164)
- Status: Merged in PR #164.

## Intent and boundary

E1 defines the execution vocabulary: the transaction side, order states, fill
reasons, fill records, position lifecycle, and execution costs. It is the type
layer that E2 will drive against candles.

The module creates no order from a strategy decision, simulates no fill, and
computes no cash, equity, trade, or P&L. Those belong to E2 and the portfolio
work items. Model Signal events remain a separate event type. No existing
command, crate dependency, or Cargo manifest changed.

## Work performed

1. Added `crates/quantrule-backtest/src/execution/` with `mod.rs`, `order.rs`,
   `fill.rs`, `position.rs`, `config.rs`, and `tests.rs`.
2. Added `OrderSide`, `Trigger`, `OrderState`, `RejectReason`, `Order`, and
   `OrderError` in `order.rs`.
3. Added `FillReason`, `Fill`, and `FillError` in `fill.rs`. `Fill::new` derives
   the executed price from the reference price and the adverse slippage rather
   than accepting an arbitrary price.
4. Added `Position`, `PositionState`, and `PositionError` in `position.rs`, with
   side-aware stop/target bracketing validation.
5. Added `ExecutionConfig` and `ExecutionConfigError` in `config.rs`, with no
   default so a run cannot silently assume zero fees or slippage.
6. Documented the fill policy in `execution/mod.rs`: next-bar reference prices,
   gap handling, stop-first tie-break, and the per-unit slippage convention.
7. Added 17 unit tests covering every order state, fill reason, rejection case,
   position state, validation path, and repeated-construction determinism.
8. Exported `execution` from `lib.rs` and updated the crate docs.

## Verification and evidence

Commands were run in `quant-rule--163-execution-types` on 2026-10-28. `cargo`
is not on `PATH` in this environment, so each command was invoked through
`~/.cargo/bin/cargo`.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --all-targets -- -D warnings` | passed |
| `cargo test` | 1,182 passed, 0 failed, 6 ignored |
| `cargo doc --no-deps` | passed, no warnings |

The total is the previous 1,156 plus 17 new unit tests and 9 new doc tests. No
test was removed, skipped, or weakened.

The execution tests assert the fill price rule directly: a buy pays
`reference + slippage`, a sell receives `reference - slippage`, and a sell whose
slippage reaches the reference price is rejected. A long position requires
`stop < entry < target`; a short requires `target < entry < stop`.

## Decisions and learning

- **The fill price is derived, not supplied.** An earlier draft accepted a
  `price` argument alongside `reference_price` and `slippage`, which allowed a
  caller to record a price inconsistent with the documented rule. Deriving it
  removes that invalid state.
- **`OrderSide` is separate from `strategy::Side`.** The order side is the
  transaction direction; the strategy side is the position direction. Sharing
  one enum would let a long entry read as a buy and obscure the exit rule.
- **Costs have no default.** `ExecutionConfig::new` requires both the fee rate
  and the slippage, so a later run records them explicitly instead of inheriting
  a silent zero.
- **Rejections are execution conditions.** `RejectReason` covers the no-next-bar,
  no-position, already-open, and zero-volume cases; invalid quantities and
  trigger prices are construction errors and never reach an order.
- **`Fill::new` keeps nine arguments.** A builder would add a parallel type
  without enforcing a new invariant. The `too_many_arguments` allow is scoped to
  the constructor with a comment saying why.
- **Rejected:** adding cash, equity, or P&L fields to `Fill`. The fill records
  costs; the portfolio module owns accounting.
- **Rejected:** a `Default` for `ExecutionConfig`. A default would hide the cost
  assumption from the command output.
