# AI Transparency Log — ATR Divergence (#420)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: ATR divergence events
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Status: ATR divergence implemented and tested; panel divergences remain in #420

## Intent and boundary

Add the remaining single-instrument divergence: price vs the ATR term ratio.
VWAP, profile, RVOL, climax, OBV, CMF, MFI, and AD divergences were already in
the extended batch. Panel-level relative-strength, lead-lag, and breadth
divergences need a panel event API and remain open.

## Implemented work

| Event | Status |
|---|---|
| `divergence.atr_bullish` | Done |
| `divergence.atr_bearish` | Done |
| Shared `divergence_subtype` helper | Done |
| Registry entries | Done |

## Verification and evidence

Observed on 2026-11-05:

| Command | Result |
|---|---|
| `cargo test -p quantrule-model extended` | 36 extended tests pass |
| `cargo test --workspace` | all 49 result groups pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `python3 tools/check_registry.py` | 46 indicators, 85 events, 110 parameters |

The test covers the four directional cases of the shared subtype helper.

## Learning and rejected suggestions

- The divergence comparison is a shared helper so the ATR and future panel
  events use the same matrix.
- The ATR divergence is regular-only; a hidden variant would need a separate
  claim and is not registered.
- Panel divergences were not faked on a single instrument: relative strength,
  lead-lag, and breadth are panel readings, and their divergence events need a
  panel event series with pivot-to-timestamp mapping.
- The issue remains open for the panel event series and the panel divergence
  tests.

## Panel divergence follow-up

The panel-level divergences are implemented in `events/panel.rs`:

- `PanelEventSeries` groups events by instrument.
- It maps each confirmed pivot bar to the panel timestamp and reads relative
  strength, lead-lag, or breadth at that point.
- It emits regular bullish/bearish divergences for sources 45, 46, and 47.
- Events are sorted by availability and deduplicated by occurrence key.

Verification: `cargo test -p quantrule-model events::panel` passes; the
workspace suite passes; the registry reports 46 indicators, 91 events, and 110
parameters.

The panel event record has no instrument field, so the series returns a map
keyed by instrument; collapsing the panel into one event list would lose the
instrument identity.
