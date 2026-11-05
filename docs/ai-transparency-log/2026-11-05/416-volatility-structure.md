# AI Transparency Log — Volatility and Structure Readings (#416)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: volatility and structure readings
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Checklist: [`docs/improvement/2026-11-05/INDICATOR-VERIFICATION.md`](../../improvement/2026-11-05/INDICATOR-VERIFICATION.md)
- Status: readings implemented and tested; events are tracked in #419–#422

## Intent and boundary

Continue the indicator programme with the volatility and structure readings. This
work item adds readings only; the event layer is a separate work item. No
strategy or baseline change.

## Implemented work

| Item | Status |
|---|---|
| ATR term ratio, slope, regime | Done |
| Range expansion | Done |
| Mean absolute return fast/slow/ratio | Done |
| Keltner upper/lower/position | Done |
| Donchian upper/lower/position | Done |
| Swing structure state and last pivots | Done |
| Level touches and age | Done |
| Aroon up/down/oscillator | Done |
| BOS/CHOCH share the swing-structure outputs | Reading inputs done; events pending |
| Registry entries and parameters | Done |

## Verification and evidence

Observed on 2026-11-05:

| Command | Result |
|---|---|
| `cargo test -p quantrule-model extended` | 28 extended tests pass |
| `cargo test --workspace` | all 49 result groups pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `python3 tools/check_registry.py` | 28 indicators, 74 events, 96 parameters |

The new tests cover the ATR term, range and MAR windows, Keltner and Donchian
bands, Aroon on a rising series, and the structure/level outputs on a wave
series.

## Learning and rejected suggestions

- The ATR term needs the slow ATR window; the ratio is null before it.
- Range and MAR are window means; the zero-denominator cases publish null.
- Swing state needs two confirmed pivots of each kind; before that it is null.
- Level touches scan confirmed major levels within the lookback; the count is a
  measured zero when no level is touched, not null.
- Aroon on a rising series gives a low but nonzero Aroon-down (the oldest bar in
  the window); the test asserts the up side above 90 and the oscillator present,
  rather than assuming an exact 100/0 split.
- Implementing the event layer in this work item was rejected: #419–#422 own the
  event formulas, availability, and precedence. The checklist marks Events
  unchecked for these rows.
