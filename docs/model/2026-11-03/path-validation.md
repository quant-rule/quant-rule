# Path Validation Protocol

- Date: 2026-11-03 (UTC)
- Status: **Approved and executed**; the first pilot (`COPPERCMD 1h`) stopped with a negative primary lift. Issue [#365](https://github.com/quant-rule/quant-rule/issues/365)
- Contract: [`docs/model/2026-11-03/model-v3.md`](model-v3.md)
- Plan: [`docs/execution-plans/2026-11-03/365-path-validation.md`](../../execution-plans/2026-11-03/365-path-validation.md)
- Scope: model validation evidence; not a strategy or edge claim

## 1. Purpose

The model's event claims are path claims. A Level test claims a reaction, a
Level break claims continuation, a divergence claims reversal, and a signal
cross claims a directional move. A fixed-horizon sign is a screening proxy; it
is not the outcome those claims name.

This protocol defines the claim-aligned outcome: did price reach a
pre-registered ATR target before a pre-registered ATR stop, in the event
direction, within a fixed number of bars?

The model output and signal path do not change. The protocol is evaluation only.

## 2. Outcome

For an event at bar `b` with direction `d` in `{-1, +1}`, entry `close[b]`, and
`atr = atr[b] > 0`:

```text
target = close[b] + d * target_atr * atr
stop   = close[b] - d * stop_atr * atr
```

Walk bars `b + 1` through `b + horizon`:

- Long (`d = +1`): if `low <= stop` and `high >= target` on the same bar, the
  stop is first. Otherwise the first `stop` touch is a miss and the first
  `target` touch is a hit.
- Short (`d = -1`): if `high >= stop` and `low <= target` on the same bar, the
  stop is first. Otherwise the first `stop` touch is a miss and the first
  `target` touch is a hit.
- Neither touched within the horizon: a timeout, counted as a miss.

The event bar's own high and low are not used. The path starts at the next bar.
The tie-break is stop first, which is conservative. An ATR of zero or null is
not a valid barrier and the event is dropped with a reason.

The first pilot fixes `target_atr = 1.0`, `stop_atr = 1.0`, and `horizon = 20`
bars. A later hypothesis may pre-register different values; it may not change
them after seeing a result.

## 3. Baseline

For every bar with a valid ATR and at least `horizon` future bars, compute the
same symmetric barrier for a long and a short entry at that bar's close. The
long and short base rates are computed per one-year bundle, so each year's drift
and volatility are respected.

For an event set, the directional base is the event-weighted average:

```text
base = sum_years (n_long_year * base_long_year
                + n_short_year * base_short_year) / n
```

The baseline is a matched barrier baseline, not a matched price level. It
answers the question the claim asks: does the event add path information beyond
a random entry with the same ATR-scaled barrier and direction mix?

## 4. Event preparation

The event filter and de-duplication are fixed in the pilot plan. The common
rules are:

- One role and one claim per event. A correlated event is not a second vote.
- A repeated test is fresh only when the reference had at least one bar without
  a test. Consecutive-bar repeats are one test.
- One vote per bar. A bar with conflicting directions is ambiguous and is
  dropped.
- Availability is respected: the path starts after `available_bar_index`.
- The event must have enough future bars for the full horizon.

The raw, fresh, prepared, and valid-path counts are reported separately, so the
reader can see how much the rules removed.

## 5. Estimator

The path outcomes of events whose 20-bar windows overlap are not independent.
The estimator groups them into clusters:

- Sort valid events by bar.
- Start a new cluster when the event bar is at least `horizon` bars after the
  previous event bar, within the same one-year bundle. Events whose paths can
  overlap stay together. Clusters never span bundles because bar indices restart
  at zero each year.
- Let `J` be the number of clusters, `n_j` the events and `h_j` the hits in
  cluster `j`, `n` the total events, and `p = hits / n`.
- The cluster-robust variance is

```text
var = (J / (J - 1)) * sum_j (h_j - p * n_j)^2 / n^2
se  = sqrt(var)
```

The 95% lower bounds are `p - 1.96 * se` and `(p - base) - 1.96 * se`. The naive
`sqrt(p*(1-p)/n)` standard error is reported for comparison, not used for the
decision. If `J <= 1`, the cluster-robust bound is undefined and the partition is
inconclusive.

The base rate is a fixed benchmark estimated from all valid bars. The event
sample is a subset of that sample, so the primary bound uses the event variance;
adding the full base variance would double-count the shared bars. Per-year base
rates are reported as a sensitivity.

## 6. Decision

The pilot plan pre-registers the minimum `n`, minimum clusters `J`, minimum
point lift, and the lower-bound rule. The decision is one of:

| Condition | Decision |
|---|---|
| Too few events or clusters in development | `inconclusive` |
| Development fails the pass rule | `stop` |
| Validation too small after a development pass | `validation_inconclusive` |
| Validation fails the pass rule | `development_only` |
| Both partitions pass | `proceed_to_replication` |

`proceed_to_replication` is not a strategy go. A single asset or timeframe is a
pilot; the protocol must replicate on a second fresh asset or timeframe before a
strategy baseline is considered. The validation partition is one-shot for the
hypothesis.

## 7. Boundaries and limitations

- The entry reference is the event close. A strategy would use the next open and
  costs; this protocol is not a backtest.
- Gaps are part of the path. The horizon is a bar count, not a clock duration.
- The baseline is a barrier baseline, not a matched price level. A matched price
  level is a separate hypothesis.
- A path pass is model evidence, not a profitability claim.
- Statistics is used here to evaluate a rule-based event. It never enters the
  model or the strategy.
