# AI Transparency Log — Multi-Instrument Panel (#436)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: multi-instrument panel input
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Prerequisite for: [#418](https://github.com/quant-rule/quant-rule/issues/418)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Status: panel API implemented and tested; shell wiring is part of the spec engine

## Intent and boundary

Add the multi-instrument panel that #418 needs for the cross-asset readings. The
panel aligns instruments on common closed-bar timestamps and computes relative
strength, lead-lag, breadth, co-movement, and dispersion from past bars only.

This work item does not add the session calendar (#437) or the shell panel
command. The single-instrument model run is unchanged.

## Implemented work

| Item | Status |
|---|---|
| `PanelInstrument` and `PanelConfig` | Done |
| Common-timestamp intersection, no forward fill | Done |
| ATR-normalized returns | Done |
| Relative strength, lead-lag, breadth, co-movement, dispersion | Done |
| Registry entries for the five readings | Done |
| Causality and determinism tests | Done |

## Verification and evidence

Observed on 2026-11-05:

| Command | Result |
|---|---|
| `cargo test -p quantrule-model panel` | 6 panel tests pass |
| `cargo test --workspace` | all 49 result groups pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `python3 tools/check_registry.py` | 41 indicators, 74 events, 105 parameters |

The tests cover relative-strength ranking, common breadth and dispersion,
co-movement on aligned series, the lead-lag reading, timestamp intersection, and
causality when a future bar is appended.

## Learning and rejected suggestions

- Alignment is the intersection of timestamps; forward-filling a missing bar was
  rejected because it would invent a price.
- Returns are normalized by the instrument's ATR at the current common bar; a
  missing ATR leaves the row null.
- The first instrument is the lead-lag leader. A configurable leader was
  rejected for now: the panel config can add it when a strategy needs it.
- Breadth and dispersion are common values, stored per instrument for a uniform
  output shape.
- Shell wiring was left to the spec-engine work; the panel API is the required
  model input, and the single-instrument run is unchanged.
- The calendar area remains blocked on #437.
