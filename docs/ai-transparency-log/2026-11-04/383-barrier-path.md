# AI Transparency Log — 5m Onset Barrier Path

- Date: 2026-11-04 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Plan: [`docs/execution-plans/2026-11-04/383-barrier-path.md`](../../execution-plans/2026-11-04/383-barrier-path.md)
- Issue: [#383](https://github.com/quant-rule/quant-rule/issues/383)

## Intent and boundary

Measure the barrier path of the accepted 5m fresh-onset event after the 5m
bracket failed with a negative gross edge. The diagnostic compares the momentum
and fade sides for 1–4 ATR targets against a 1 ATR stop over 20 bars. It is an
evaluation, not a strategy; it does not change the event, the execution engine,
or the bracket rule.

## Design decisions

- Use the same entry population as the strategy: gap-resolved side, exactly one
  boundary touched, both-touch bars skipped.
- Measure both directions from the same entry price, so the momentum and fade
  results are paired.
- Include the time-stop mark and an estimated 12 bps round-trip cost in R.
- Use nine instrument-year bundle means for a cluster-level lower bound rather
  than treating every trade as independent.

## Result

No directional edge. Across 10,349 entries, the momentum and fade sides are
almost mirror images; the 1 ATR target hit rate is 47.6% against 50% break-even
and the 2 ATR target is 31.5% against 33.3%. Gross R is near zero or negative
for every side/target, and the 12 bps cost is about 1.21 R per trade. No
side/target is promising. The compression bracket program stops.

## Verification

- `barrier_path.py` unit tests cover target-first, stop-first, same-bar tie,
  gap, timeout, fade entry-bar suppression, and determinism.
- The diagnostic ran on the nine `model_dev` onset bundles.
- No strategy or validation claim is made.
