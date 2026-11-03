# AI Transparency Log — Model v3 Flat Report

- Date: 2026-11-03 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#346](https://github.com/quant-rule/quant-rule/issues/346)
- Plan:
  [`docs/execution-plans/2026-11-03/342-model-v3.md`](../../execution-plans/2026-11-03/342-model-v3.md)
- Claim type: model output implementation; no edge claim

## Intent and boundary

Add the flat per-timeframe indicator/event projection to the model report:
readings, states, levels, events, progression, context, and coverage. The
existing axis report remains for the retired v3–v7 commands until the final
removal; the flat section is the v3 output and the only one a v8 strategy may
read.

## Work

- Added `FlatReport`, `FlatTimeframe`, `FlatLatest`, `FlatLevel`,
  `FlatEventSeries`, and `FlatProgression` in `quantrule-model`.
- Added the `flat` section to `MarketReport` and its JSON envelope.
- Exposed the computed readings from `TimeframeModel`; the flat report reads the
  existing series and never recomputes an indicator.
- Added unit and JSON tests, including an absent timeframe, reading parity with
  the axis report, event-count consistency, and the empty-report envelope.
- Regenerated the market-report JSON golden fixture deliberately because the
  report gained a section.

## Verification and evidence

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --all-targets -- -D warnings`: pass.
- `cargo test --workspace`: pass.
- `cargo doc` with `-D warnings`: pass.
- `python3 tools/check_links.py`: 359 files, 0 broken.
- `python3 tools/check_terms.py`: 219 files, 0 banned.
- A real fixture report shows four flat timeframes, readings, states, context,
  confirmed levels, event counts, recent events, and progression legs.

## Learning

- The existing `MarketReport` is axis-shaped; adding a flat projection is safer
  than rewriting every consumer at once. The old axis report is now explicitly
  legacy and the v3 section is self-contained.
- The event batch can be large; the JSON keeps counts, latest, and a bounded
  recent list, while the full series belongs in the CSV artifacts (#347).

## Rejected suggestions

- Replacing the entire `MarketReport` in one change: it would break the old
  report, diagnostics, and golden fixtures before the v8 path exists.
- Emitting every event in the JSON: the full batch can be hundreds of thousands
  of records; the CSV is the right carrier.
- Faking `event_strength` without the ATR value: the field stays null until the
  event record carries the ATR evidence in #347.

## Limitations

- MACD is not in this PR; it follows in #347.
- The legacy axis section remains until the final removal.
- Same-session review, not an independent Quant Validator record.
