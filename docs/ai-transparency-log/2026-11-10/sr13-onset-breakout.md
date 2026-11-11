# AI Transparency Log — SR13 Onset Breakout

- Date: 2026-11-10 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash
- Work item: [`sr13-onset-breakout`](../../execution-plans/2026-11-10/sr13-onset-breakout.md)
- Result: [`SR13`](../../improvement/2026-11-10/SR13.md)
- Claim type: strategy research; no edge claim.

## Intent and boundary

After SR12, the human chose a new premise: a 5m fresh-onset compression breakout
continues in the direction of the first confirmed close outside the range. The
plan fixed four protocol items before touching data: the 3-bar breakout window,
the 6-bar pooled primary with descriptive secondaries, 2024 as the one-shot
validation, and a model event as the production path only after a passing gate.
The screen was to stop at the development gate; no 2024, model event, strategy,
execution, or portfolio work was authorized.

## Implemented

- `config/sr13/`: model config with `compression_range_onset` enabled, 5m
  policy/execution/portfolio, five data configs, and five run configs.
- Five BTCUSD 5m model bundles for 2018–2022 under
  `data/improvement/2026-11-10/SR13/artifacts/model/`.
- `tools/research/breakout_edge.py`: a deterministic raw-edge screen for the
  first-close rule, with the baseline, fade, random-direction, back-lag,
  placebo-range, and compression-only controls.
- `tools/research/test_breakout_edge.py`: 9 fixture tests covering long, short,
  boundary-equal, window expiry, next-onset supersession, gap-open entry,
  end-of-series, non-overlap, and statistics.
- `docs/governance/2026-11-10/SR13-CHARTER.md`,
  `docs/config/2026-11-10/onset-breakout-5m.md`,
  `docs/execution-plans/2026-11-10/sr13-onset-breakout.md`, and
  `docs/improvement/2026-11-10/SR13.md`.
- The run record under `data/improvement/2026-11-10/SR13/`.

## Evidence

The primary (W=3, 6 bars, pooled, non-overlapping) is −2.1790 bps with a 95%
interval of [−3.5608, −0.7972], a 44.5% hit rate, and 2/5 positive years. The
signal is worse than the unconditional baseline (−0.0017 bps) and the non-coiling
placebo range (−1.7215 bps). The 1-bar mean is +0.2801 bps, below the 6 bps cost;
the 3- and 12-bar means are negative. W=1 and W=6 are −1.5597 and −2.1536 bps.
The 2024 partition was not opened.

## Verification

- `python3 -m unittest tools/research/test_breakout_edge.py -v`: 9 tests pass.
- `cargo build --release -p quantrule-shell`: success.
- `quantrule policy check`: pass for the 5m run config.
- Five `report model` runs: success.
- Ladder check: 18,267 common 5m rows, 0 differences; 201,431 5m events
  identical to the full 1m/5m/15m/1h quarter run.
- Raw-edge runs: W=1/W=3/W=6 complete; all fail the gates.

## Learning

- The first-close breakout direction does not predict continuation at 6 bars;
  the result is negative and stable across the three pre-registered windows.
- The compression condition adds nothing: the signal is worse than the matched
  non-coiling placebo.
- The 1-bar result is the only positive point and is below the 6 bps cost; it is
  not evidence of an edge.
- The fade is the mechanical mirror and is not a candidate. The temptation to
  flip the rule is exactly the post-hoc change the charter forbids.
- The 5m-only ladder was verified on a quarter sample, not the full year: the
  full-year run was cancelled by a server restart. The common-sample check is
  sufficient to show the per-timeframe rows are independent.
- The diagnostic initially indexed past the end of the series on placebo onsets
  near the last bars; the fix clamps the candidate range and a fixture test
  covers the end-of-series case.
- The 2018 5m coverage is 67.17%, a data limitation. The negative result is not
  driven by the missing bars; 2018 is negative and the later years are mostly
  negative.

## Decision

No edge. The premise is rejected; the 2024 validation remains closed; no model
event, strategy, execution, or portfolio work follows. The rule is not
re-registered with a different window, horizon, side, cost, or filter. The
human's final acceptance is pending.

## Rejected alternatives

- **Flip to the fade.** Rejected: mechanical mirror, below cost, not
  pre-registered.
- **Use the 1-bar horizon.** Rejected: below cost and not significant.
- **Change the window to 1 or 6.** Rejected: both are negative.
- **Use the existing `signal.compression_breakout` event.** Rejected: it embeds
  a 0.2 ATR buffer and RVOL ≥ 1.2, a different rule.
- **Put the close comparison in the strategy.** Rejected: it is a model
  measurement; the strategy may not read the candle close.
- **Run 2024 "to see".** Rejected: the development gate failed; the partition
  stays closed.
