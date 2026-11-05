# AI Transparency Log — Single-Instrument Signal Events (#421)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: single-instrument signal events
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Status: single-instrument signals implemented and tested; panel and session
  signals remain in #421

## Intent and boundary

Add the signal events that use one instrument and no session calendar: ATR,
range, MAR, Keltner, Donchian, structure, strength, Aroon, candle anatomy,
patterns, gaps, path, extremes, drawdown, and range position.

Panel signals (relative strength, lead-lag, breadth, co-movement, dispersion) and
session signals (session open/close, opening-range breakout/fade, overnight,
session range) remain in #421; they need the panel and session APIs in the event
run.

## Implemented work

| Group | Events |
|---|---|
| ATR / MAR / range | compression, expansion, regime, range expansion/contraction, MAR regime |
| Channels | Keltner/Downchian breakout and position |
| Structure | structure change, strength change |
| Aroon | Aroon cross and oscillator |
| Candle | momentum, rejection, doji, marubozu |
| Patterns | engulfing, inside, outside, hammer, star, three soldiers, three crows |
| Gap | continuation, fade |
| Path | efficiency, direction changes, extremes, drawdown, range position |

The registry now carries 46 indicators, 125 events, and 110 parameters.

## Verification and evidence

Observed on 2026-11-05:

| Command | Result |
|---|---|
| `cargo test -p quantrule-model extended` | 38 extended tests pass |
| `cargo test --workspace` | all 49 result groups pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `python3 tools/check_registry.py` | 46 indicators, 125 events, 110 parameters |

The tests cover a momentum-bar signal and a gap-continuation signal; the
occurrence-key dedup and availability rules are the same as the earlier batches.

## Learning and rejected suggestions

- The signal batch is one module, `events/signals.rs`, called once after the
  per-bar event detectors; this keeps the signal formulas separate from the
  level and divergence records.
- Pattern events share source 38 and subtype 10; the pattern code is the
  primary value, so one pattern event is published per bar.
- Panel and session signals were not faked on a single instrument or without a
  session calendar; they remain open with the panel and session APIs.
- `cargo clippy --fix` collapsed the nested let-chains; no logic changed.
