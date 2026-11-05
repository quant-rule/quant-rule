# AI Transparency Log — Level Events (#419)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: level events for the new indicators
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Checklist: [`docs/improvement/2026-11-05/INDICATOR-VERIFICATION.md`](../../improvement/2026-11-05/INDICATOR-VERIFICATION.md)
- Status: implemented and tested

## Intent and boundary

Add the remaining level events for Keltner, Donchian, swing structure, level
strength, candle anatomy, and gaps. VWAP, profile, and absorption level events
were already in the first batch. Divergences and signals for these indicators
remain in #420–#421.

## Implemented work

| Event | Status |
|---|---|
| `level.keltner_test`, `level.keltner_break` | Done |
| `level.donchian_test`, `level.donchian_break` | Done |
| `level.structure_break` | Done |
| `level.strength_test` | Done |
| `level.wick_test` | Done |
| `level.gap_fill`, `level.gap_test` | Done |
| Session VWAP and opening-range level events | Deferred: session readings are a separate API |
| BOS/CHOCH | Deferred: the level/break code would collide with structure break |

## Verification and evidence

Observed on 2026-11-05:

| Command | Result |
|---|---|
| `cargo test -p quantrule-model extended` | 35 extended tests pass |
| `cargo test --workspace` | all 49 result groups pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `python3 tools/check_registry.py` | 46 indicators, 83 events, 110 parameters |

The new tests cover a Keltner break, a Donchian break, and a filled gap.

## Learning and rejected suggestions

- A Donchian break must compare the close to the **previous** channel, because
  the current channel includes the current bar's high; the first test failed
  until the comparison used the prior upper/lower.
- The Keltner upper includes the current ATR, so the test bar's ATR raised the
  band; the fixture close was raised to clear it.
- BOS/CHOCH was deferred: `level.structure_break` already publishes the same
  break with source 33, and a second source-36 break with the same occurrence
  would duplicate the evidence. The registry keeps the indicator without events
  until a distinct CHOCH definition is agreed.
- Session VWAP and opening-range level events need the session API wired into the
  event run; they are left for the spec-engine integration.
- Clippy auto-fixed eight collapsible let-chains; no logic changed.
