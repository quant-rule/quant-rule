# AI Transparency Log — DST-Aware Session Calendar (#437)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: DST-aware session and calendar readings
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Prerequisite for: [#418](https://github.com/quant-rule/quant-rule/issues/418)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Status: calendar module implemented and tested; shell wiring is part of the spec engine

## Intent and boundary

Add the DST-aware session calendar that #418 needs, without adding a timezone
dependency. The module uses explicit London and New York rules and a UTC zone.
It adds readings only; no strategy or baseline change.

## Implemented work

| Item | Status |
|---|---|
| London DST: last Sunday March/October at 01:00 UTC | Done |
| New York DST: second Sunday March 07:00 UTC, first Sunday November 06:00 UTC | Done |
| UTC zone | Done |
| Session code and age | Done |
| Opening range high/low/ATR | Done |
| Overnight and intraday returns | Done |
| Session range in ATR | Done |
| Calendar bucket: weekday * 100 + month | Done |
| Registry entries for five indicators | Done |

## Verification and evidence

Observed on 2026-11-05:

| Command | Result |
|---|---|
| `cargo test -p quantrule-model sessions` | 3 calendar tests pass |
| `cargo test --workspace` | all 49 result groups pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `python3 tools/check_registry.py` | 46 indicators, 74 events, 110 parameters |

The tests cover both DST transitions at the exact UTC hour and a London session
with its opening range and calendar bucket.

## Learning and rejected suggestions

- Adding `chrono-tz` was rejected because a new dependency requires explicit
  human approval and the rules needed are small and stable for 2018–2025.
- Manual DST rules are deterministic and testable, but they are maintained in
  this module; a dependency would be safer if the date range grows or historical
  exceptions matter.
- The 1h model uses hour-aligned sessions: London 08:00–16:00 and New York
  09:00–16:00 local. A 09:30 NY start does not align with 1h bars; the 15m
  session work remains separate.
- The session VWAP is not part of this batch; it belongs to the VWAP anchor
  configuration.
- Shell wiring is left to the spec engine; the calendar API is the required
  model input.
