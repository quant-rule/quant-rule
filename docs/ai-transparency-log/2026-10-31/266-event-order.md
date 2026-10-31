# AI Transparency Log — Event Order and Aggregate Guard

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential implementation and verification
- Work item: #266 (W6a), parent #252
- Findings: AUD-2026-10-31-09, -23
- Plan: `docs/execution-plans/2026-10-31/audit-unified-ladder.md` (W6)

## Intent and boundary

Preserve the model contract's event order within one availability bar and stop
the public cross-timeframe aggregate from indexing an out-of-range divergence
direction. The change is limited to the model event sort and the aggregate
count guard, plus a shell regression test.

## Implemented

- Events are sorted by `(available_bar_index, occurred_bar_index)` only; the
  detector append order is preserved within a bar. Bar 28 now emits the EMA20
  break before the minor-pivot test, and bar 37 emits Momentum before Structure.
- The divergence-direction index is bounds-checked with `usize::try_from` and a
  length check.
- Added `events_keep_the_contract_order_on_one_bar`.

## Verification

| Check | Observed |
|---|---|
| Fail-before, old full sort restored | Event-order test failed at bar 28 |
| Restored | Event-order test passed |
| `cargo test -p quantrule-model` | Passed |
| `cargo test -p quantrule-shell --test cli events_keep_the_contract_order_on_one_bar` | Passed |
| `cargo fmt`, `clippy`, `test --workspace`, `doc` | Pending final PR run |

## Learning and rejected suggestions

- Rejected sorting by an explicit contract rank. The detectors already append
  in the contract order; a second rank table would duplicate that order.
- Rejected a panic-on-invalid-direction assertion. A public snapshot may carry
  an out-of-contract code; the aggregate should ignore it, not panic.
- The out-of-range direction test is covered by the bounds check but not by a
  dedicated fixture in this pass; that remains a test gap noted for W6.
