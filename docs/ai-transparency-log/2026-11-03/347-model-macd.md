# AI Transparency Log — MACD and Exhaustive Model Artifacts

- Date: 2026-11-03 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#347](https://github.com/quant-rule/quant-rule/issues/347)
- Plan:
  [`docs/execution-plans/2026-11-03/342-model-v3.md`](../../execution-plans/2026-11-03/342-model-v3.md)
- Claim type: model implementation; no edge claim

## Intent and boundary

Add MACD 12/26/9 with line/signal, histogram, and zero-cross events, and write
the full per-bar readings, event batch, levels, and progression legs as CSV
artifacts. The legacy axis path keeps MACD disabled unless a config enables it;
the v3/default path enables it.

## Work

- Added `MacdSeries` with the line, signal, and histogram, seeded from the
  existing EMA and from the line's own EMA.
- Added MACD source codes 13/14/15 and Signal subtypes 4 (histogram slope flip)
  and 5 (zero cross). The legacy axis sources keep 9/10/11.
- Added the MACD config block (`fast`, `slow`, `signal`, `active`) and the
  per-timeframe enable switch.
- Added `readings.csv`, `events.csv`, `levels.csv`, and `legs.csv` to the model
  bundle.
- Added MACD states/readings to the flat report.
- Extended the research export registry with the new subtypes and sources, and
  updated the fixed column widths.
- Enabled MACD in `config/defaults/model.json`; kept it opt-in for historical
  configs so their independent goldens still test the legacy path.

## Verification and evidence

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --all-targets -- -D warnings`: pass.
- `cargo test --workspace`: pass.
- `cargo doc` with `-D warnings`: pass.
- A MACD-enabled fixture run produced non-null MACD readings, states, and
  events, plus the four CSV artifacts.
- The independent dashboard and market-report goldens run with MACD disabled;
  the v3 flat report test runs with MACD enabled.

## Learning

- Source code 9/10/11 was already used by the legacy axis state events. MACD was
  assigned 13/14/15 so the registry has no collision; codes are never reused.
- The research export is a fixed-width contract. Adding signal subtypes and
  sources changes the dictionary and the layer3 width; the tests were updated
  with the new 84 aggregate columns.
- Enabling MACD by default on the legacy path would have changed every old
  golden for a feature the old strategies never read. The explicit `active`
  switch keeps the migration honest.

## Rejected suggestions

- Reusing source codes 9/10/11 for MACD: it would make two different events
  indistinguishable in the record.
- Treating a null line value as zero when seeding the signal: it fabricates a
  reading.
- Emitting both a line/signal cross and a histogram zero cross: they are the same
  occurrence.
- Regenerating the independent axis goldens with Rust output: it would weaken
  the oracle; the legacy tests keep MACD disabled instead.

## Limitations

- MACD is opt-in on the legacy v2 path; the v3 config parser and default-on
  behavior are part of the remaining #346 work.
- The `event_strength` field remains null until an event record carries its ATR
  evidence.
- Same-session review, not an independent Quant Validator record.
