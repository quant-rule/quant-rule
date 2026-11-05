# AI Transparency Log — Edge Cases and Precedence (#405)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: per-indicator edge cases and extended precedence
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Status: checklist `E` and `P` checked; rows move from `code` to `tested`

## Intent and boundary

Close the last two test columns. The edge suite proves every extended, panel, and
session reading is finite or null on an empty series, a single candle, a short
series, a flat range, zero volume, and equal highs and lows. The precedence
evidence is the fixed detector order and the occurrence key in the event
validation suite, together with the v4 level-reference order test.

The checks are family-level. They prove the shared edge behavior and the event
ordering, not a separate fixture for each indicator.

## Implemented work

| Item | Detail |
|---|---|
| Edge suite | `crates/quantrule-model/tests/edge_cases.rs`, 7 tests |
| Extended fields | Every extended reading asserted finite or null |
| Session fields | Every session reading asserted finite or null |
| Panel | Empty and one-bar panel safety |
| Checklist | `E` and `P` checked for all 37 rows; status `tested` |

## Verification and evidence

| Command | Result |
|---|---|
| `cargo test -p quantrule-model --test edge_cases` | 7 tests pass |
| `cargo test --workspace` | all result groups pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |

## Learning and rejected suggestions

- A per-indicator edge fixture would be 37 near-identical files. The family
  suite asserts every published field, which is the invariant that matters:
  no NaN, no infinity, and no fabricated value on an unavailable input.
- `P` for the extended families is not the v4 level reference order. It is the
  fixed detector append order plus the occurrence key; the v4 order test stays
  separate and is cited, not replaced.
- `Sign` stays open. `tested` is the test status; `verified` still requires the
  human or Quant Validator signature.
