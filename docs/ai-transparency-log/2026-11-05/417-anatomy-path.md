# AI Transparency Log — Anatomy and Path Readings (#417)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: candle anatomy, patterns, gaps, and path readings
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Checklist: [`docs/improvement/2026-11-05/INDICATOR-VERIFICATION.md`](../../improvement/2026-11-05/INDICATOR-VERIFICATION.md)
- Status: readings implemented and tested; events are tracked in #419–#422

## Intent and boundary

Add the anatomy and path readings. This work item adds readings only; the event
formulas are a separate work item. No strategy or baseline change.

## Implemented work

| Item | Status |
|---|---|
| Body, upper/lower wick, close-position ratios | Done |
| Pattern code and direction: engulfing, inside, outside, hammer, star, three soldiers, three crows | Done |
| Gap in ATR and same-bar fill | Done |
| Path efficiency, direction changes | Done |
| Bars since high/low, drawdown, range position | Done |
| Registry entries and parameters | Done |
| CSV columns | Done |

## Verification and evidence

Observed on 2026-11-05:

| Command | Result |
|---|---|
| `cargo test -p quantrule-model extended` | 32 extended tests pass |
| `cargo test --workspace` | all 49 result groups pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `python3 tools/check_registry.py` | 36 indicators, 74 events, 101 parameters |

The new tests cover the bar ratios, an engulfing pattern, the gap reading, and a
rising path with efficiency 1, a current extreme, a high range position, and a
non-positive drawdown.

## Learning and rejected suggestions

- Patterns start at bar 1 for engulfing/inside/outside; the three-bar patterns
  are guarded by bar >= 2 so the second prior bar is never indexed early.
- Gap and drawdown need ATR; the gap test explicitly checks the null without ATR
  and the value with a short ATR config.
- The path high is the highest high, not the close; range position is below 1
  on a rising close series, and the test asserts above 0.9 rather than equality.
- Event formulas for these readings were left to #419–#422; the checklist marks
  Events unchecked.
