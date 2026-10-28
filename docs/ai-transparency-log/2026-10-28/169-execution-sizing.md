# AI Transparency Log — Execution Sizing and Order Lifecycle

- Date: 2026-10-28 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#169](https://github.com/quant-rule/quant-rule/issues/169)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md)
- Branch/worktree: `feat/169-execution-sizing` in `quant-rule--169-execution-sizing`
- PR: [#181](https://github.com/quant-rule/quant-rule/pull/181)
- Status: PR open; merge after checks pass.

## Intent and boundary

E3 makes the execution engine size each entry independently and publish the
order lifecycle. The runner (#172) needs to apply a bar's fills, mark the
position to market, and size the new entry from the same close, so the engine
must accept a quantity per decision and return the orders it created and
resolved.

The boundary is the execution module. E3 adds no portfolio accounting, no cash
or equity, no fill consumption, no I/O, no clock, no randomness, no serde, and
no command change. `Fill` is unchanged: the fill id belongs to the order record,
not to the fill.

## Work performed

1. Replaced the run-wide quantity and `on_bar` with `advance(candle)` and
   `submit(decision, quantity, atr14)`. `advance` resolves the previous bar's
   market order and the open bracket; `submit` schedules the bar's decision.
2. Added `OrderRecord` and `ExecutionBatch { orders, fills, rejections }`.
   `advance`, `submit`, and `finish` each return a batch.
3. Added deterministic `order_id` and `fill_id` sequences starting at zero.
   A pending record is emitted at submission and a filled or rejected record
   with the same id when the order resolves.
4. Made `OrderState::Rejected` a unit variant, removed `OrderState::Cancelled`,
   removed `Order::state`, and added `Rejection::order_id`.
5. Added `RejectReason::MissingQuantity` and
   `RejectReason::NonPositiveQuantity`.
6. Added the checked-in `tests/fixtures/execution_quantities.csv` and extended
   the golden tests to the order records, per-entry quantities, causality, and
   `pending_at_end`.

## Decisions and corrections

- **Deviation from the plan's signature.** The plan wrote
  `submit(decision, quantity)`. The implementation adds a third parameter,
  `atr14`, because the bracket is anchored to the decision bar's ATR and the
  runner already has that value when it submits. Passing it later would need a
  second channel or a re-lookup, and the plan's own per-bar sequence has the
  runner compute the decision from the bar's readings.
- **Bracket orders are recorded when they trigger.** There is no `Cancelled`
  state in v1, so creating both bracket levels as pending orders would leave the
  untriggered level pending forever and break `pending_at_end == 0`. The record
  is created at the trigger with the entry fill's bar and time as its
  submission, because that is when the bracket became live. The level that
  never triggers is not an order.
- **A rejected decision still consumes an order id.** `Rejection` carries
  `order_id`, and the report joins orders and rejections by it. A decision
  refused before an order exists (entry while open, exit while flat, missing or
  non-positive quantity) therefore emits a rejected order record with the same
  id, so the join never dangles. Its `quantity` is `None` only when the decision
  carried none.
- **`OrderState::Rejected` carries no payload.** The reason lives on
  `OrderRecord::rejection_reason`. Keeping it in both places would allow the two
  to disagree.
- The first `reject_decision` draft had eight parameters; clippy rejected it.
  The decision fields are now grouped in a private `RejectedDecision`.
- A quantity test compared `Some(f64::NAN)` with `==`, which is false by
  definition. It now compares the bit patterns, so the test checks the carried
  value rather than relying on float equality.
- The first `last_record_per_order` helper borrowed the result vector mutably
  while pushing in the `None` arm. It now finds the position first, then assigns
  or pushes.

## Verification and evidence

Commands were run in `quant-rule--169-execution-sizing` on 2026-10-28.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; no warning |
| `cargo test` | 1,290 passed, 0 failed, 1 ignored (`store_100k`) |
| `cargo test -p quantrule-backtest` | 128 unit, 6 + 9 integration, 38 doctests passed |
| `cargo doc --no-deps` | Passed; exit 0; no warning |

The full-suite count rose from 1,283 (P1 baseline) to 1,290. The increase is the
new execution tests and doc examples; the E2 golden fills still reproduce, and
no existing test was removed or weakened.

## Learning

- Emitting a pending record and then a resolved record with the same id keeps
  the lifecycle observable without mutating a record in place. The report's
  "last record per id" rule is the join, and it needs no manifest.
- A lazy bracket record is the simplest way to keep one position, one bracket,
  and no `Cancelled` state consistent: the order exists exactly when it can
  fill.
- A per-entry quantity is only proven if the bracket fill inherits it. The
  fixture checks the entry and its stop or target, and the order record that
  carries each fill id.
- Returning a batch from `finish` instead of mutating a shared rejection list
  makes the end-of-data rejection impossible for a caller to miss.
