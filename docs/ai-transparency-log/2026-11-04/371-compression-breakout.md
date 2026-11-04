# AI Transparency Log — Compression-Breakout Range Expansion

- Date: 2026-11-04 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Plan: [`docs/execution-plans/2026-11-04/371-compression-breakout.md`](../../execution-plans/2026-11-04/371-compression-breakout.md)
- Issue: [#371](https://github.com/quant-rule/quant-rule/issues/371)

## Intent and boundary

Test the model's non-directional claim: a coiling volatility regime followed by
a confirmed compression breakout predicts a larger forward range than coiling
alone. The work is model-only. No strategy, portfolio, or cost claim is made
until a model baseline is accepted.

## Design decisions

- The event is a **signal** event with a new subtype 7, source ATR 5. It uses
  the published context code for the coiling window, ATR for the break distance,
  and RVOL for participation.
- The outcome is a forward 20-bar range of at least 2 ATR, not a direction.
- The baselines are all bars, coiling bars, and a placebo context transition.
  The placebo isolates the breakout from the volatility transition.
- The primary timeframe is chosen from event counts only; no outcome is
  inspected before the choice.
- The event is opt-in in the v4 default config and enabled in the new
  compression config, so existing v4 runs are unchanged.

## Process note

The thesis was chosen after the v3 and v4 directional gates recorded no edge.
The prior results and the model's non-directional validation boundary are the
basis for this hypothesis. No threshold or timeframe was selected from a prior
outcome; the protocol was written before the event was implemented and before
the development matrix was run.

## Result

The development gate is **inconclusive** because the pre-registered outcome
saturates. The 1h primary cell has 302 events, a 0.9967 hit rate, a 0.9888
coiling base, a +0.79pp lift, a +0.04pp corrected lower bound, and a 0.1443
random p-value. The 15m secondary cell has 1,355 events, a 0.9786 hit rate, a
0.9583 coiling base, a +2.03pp lift, and a +1.12pp corrected lower bound; it is
not a primary cell and is not promoted. The maximum possible lift is 1.12pp at
1h and 4.17pp at 15m, below the 5pp threshold.

The validation partition was not opened. No baseline, strategy, or portfolio
work followed. The event and thresholds were not changed after the result. The
harness gained a saturation flag after the first cell showed the coiling base
above 98%; the flag changes only the verdict label (`inconclusive` rather than
`no edge`) and does not change the event, thresholds, or pass rule.

## Verification

- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test
  --workspace`, and `cargo doc --workspace --no-deps` passed.
- The compression event has explicit, causality, determinism, and edge-case
  tests; the range harness has unit and end-to-end tests. `cargo test
  --workspace` passed, and the improvement tooling ran 94 tests.
- Registry and validation checks passed: 10 indicators, 17 events, 65
  parameters, 26 claims.
- Links and terms checks passed.
- A regenerated BTCUSD 2018 bundle is byte-identical to the matrix bundle for
  `events.csv`, `readings.csv`, and `result.json`.
- The development harness is deterministic with seed 0.

## Learning

- The pre-registered 2 ATR / 20-bar outcome is too easy: on coiling bars it is
  reached 95.8–98.9% of the time, so the 5pp gate cannot be met. The result is
  inconclusive, not a no-edge finding.
- The 15m secondary cell is suggestive (+2.03pp, corrected LCB +1.12pp, random
  p 0.005), but promoting it after the result would be post-hoc. A new protocol
  must pre-register a harder outcome and a timeframe before looking.
- The v4 signal settings were parsed but `TimeframeModel` still used the default
  `SignalConfig`; the compression work exposed this. The wiring now passes
  `SignalSettings` to the signal, DMI, and trend detectors, and the compression
  event is gated by its own setting.
