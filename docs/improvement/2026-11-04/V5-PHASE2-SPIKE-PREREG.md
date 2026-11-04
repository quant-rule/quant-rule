# V5 Phase 2 Spike — Pre-registration

- Date: 2026-11-04 (UTC)
- Status: **Pre-registered; frozen before the spike**
- Program: `PROGRAM-2026-11-04-05`
- Charter: [`docs/governance/2026-11-04/v5-phase2-charter.md`](../../governance/2026-11-04/v5-phase2-charter.md)
- Partition: `model_dev` 2018–2020 (exploration)
- Instruments: BTCUSD, ETHUSD, XAUUSD
- Claim type: exploratory model research; no model baseline, no edge claim

## 1. Question

Does any of three price-only information families — candle anatomy, rejection at
a confirmed level, or cross-asset relative strength — reach a **4 ATR target
before a 2 ATR stop within 100 1h bars** more often than its direction-matched 1h
base, after a three-candidate correction and negative controls?

The v5 1h swing gate tested the v4 events and failed. This spike tests new
information that the v4 model does not publish.

## 2. Data audit — volume family blocked

The pack's `volume` column is the sum of M1 volume, but the source units change
within a year. Monthly median volume, `max/min` per instrument-year:

| Instrument-year | Monthly median min | Monthly median max | Ratio |
|---|---:|---:|---:|
| BTCUSD 2018 | 0.0667 | 59.95 | 899× |
| BTCUSD 2019 | 0.0447 | 27.36 | 611× |
| BTCUSD 2020 | 0.0292 | 34.97 | 1,199× |
| ETHUSD 2018 | 0.2647 | 4,712 | 17,805× |
| ETHUSD 2019 | 0.5774 | 9.056 | 15.7× |
| ETHUSD 2020 | 0.1776 | 10.03 | 56.5× |
| XAUUSD 2018 | 1.119 | 4.356 | 3.9× |
| XAUUSD 2019 | 0.9946 | 2.475 | 2.5× |
| XAUUSD 2020 | 1.846 | 3.868 | 2.1× |

A 600×–17,800× shift is a unit or contract change, not market volume. BTC 2020
also contains multi-bar prints of ~35,900 against a monthly median of ~0.03.

Consequence: VWAP, volume profile, and volume climax are **not tested**. XAU
volume is stable but is one instrument and cannot meet two-instrument
replication. No volume claim is made in either direction. The family needs a
consistent volume source before it can be tested.

## 3. Primary candidates

All candidates use completed 1h bars, ATR 14 at the signal bar, and the signal
bar's close as the entry. One event per instrument per bar.

### C1 — Momentum bar

For bar `t` with range `R = high - low > 0` and `atr > 0`:

- `R >= 1.5 * atr`;
- body `B = |close - open|` and `B / R >= 0.7`;
- bullish when `close > open` and `(close - low) / R >= 0.8` → direction `+1`;
- bearish when `close < open` and `(high - close) / R >= 0.8` → direction `-1`;
- otherwise no event.

Rationale: a wide-range bar that closes at its extreme is initiative flow; the
claim is continuation.

### C2 — Rejection wick at a confirmed major level

For bar `t` with `R >= 0.5 * atr`:

- confirmed major 1h level with `available_bar <= t` and `t - available_bar <= 500`;
- upper wick `U = high - max(open, close)`;
- bearish when `U / R >= 0.6`, `close < open`, and some level is within
  `0.25 * atr` of `high` → direction `-1`;
- bullish when `D = min(open, close) - low`, `D / R >= 0.6`, `close > open`, and
  some level is within `0.25 * atr` of `low` → direction `+1`;
- if both qualify on one bar, drop the bar as ambiguous.

Rationale: a long wick into a confirmed level is absorbed liquidity; the claim
is rejection away from the level.

### C3 — Relative-strength cross

Panel of BTCUSD, ETHUSD, and XAUUSD 1h bars aligned on common timestamps within
the year. For instrument `i` at common bar `t` with 24 prior common bars:

```text
ret_i = (close_i[t] - close_i[t-24]) / atr_i[t]
rel_i = ret_i - mean(ret_j, ret_k)
```

- `rel_i` crosses above `+1.0` (previous `<= +1.0`, current `> +1.0`) → `+1`;
- `rel_i` crosses below `-1.0` (previous `>= -1.0`, current `< -1.0`) → `-1`.

Rationale: cross-sectional relative strength persists; the claim is that the
absolute price follows the relative move. The 24-common-bar window is roughly one
to two calendar days because XAU trades fewer hours.

## 4. Diagnostic

### D1 — Gap continuation (XAUUSD only)

`open[t] - close[t-1] >= 0.5 * atr` → `+1`; `<= -0.5 * atr` → `-1`.

XAU weekend and session gaps are real, but the pre-flight finds 131 events, below
the 200 floor and on one instrument. It is reported as `inconclusive —
underpowered` and does not enter the correction or the decision.

## 5. Sample pre-flight

Raw event counts on `model_dev`:

| Candidate | BTCUSD | ETHUSD | XAUUSD | Total |
|---|---:|---:|---:|---:|
| C1 momentum bar | 735 | 436 | 620 | 1,791 |
| C2 rejection wick | 827 | 376 | 714 | 1,917 |
| C3 relative strength | 820 | 831 | 905 | 2,556 |
| D1 gap (XAU only) | — | — | 131 | 131 |

C1, C2, and C3 clear the 200 floor in total and per instrument. D1 does not.

Erratum (recorded after the run, on discovering the count discrepancy): the
first C2 count was 3,189. That script did not exclude levels confirmed *after*
the signal bar. The candidate rule in section 3 always required
`available_bar <= t`; the run used the causal rule. The corrected count is
1,917, which still clears the floor by a wide margin. No hypothesis, threshold,
or criterion changed, and the decision does not depend on the count.

## 6. Barrier and base

For an event at bar `b`, direction `d`, entry `close[b]`, `atr = atr[b]`:

```text
target = close[b] + d * 4.0 * atr
stop   = close[b] - d * 2.0 * atr
```

Walk bars `b + 1` through `b + 100`. A stop touch is a miss, a target touch is a
hit, a same-bar touch is a stop first, and a timeout is a miss. A missing or zero
ATR makes the bar invalid.

The base is the same barrier on every valid bar of the bundle, direction-matched,
event-weighted across bundles.

## 7. Estimator

Paths overlap for 100 bars, so events inside one path window are not independent
evidence. The estimator groups events into **fixed, non-overlapping 100-bar
blocks** inside each bundle (`block = bar // 100`). This avoids the chaining that
a gap-based cluster rule produces on dense events.

With `J` non-empty blocks, `n_j` events and `h_j` hits in block `j`, the pooled
cluster-robust variance is:

```text
var = sum_bundles (J_b / (J_b - 1)) * sum_j (h_j - p * n_j)^2 / n^2
se  = sqrt(var)
```

`lift_lcb = lift - z * se`, `z = 2.394` (Bonferroni over three candidates). A
bundle with `J_b <= 1` contributes no variance term; the block count is reported.

## 8. Controls

| Control | Definition | Pass |
|---|---|---|
| Random | 200 draws of the event count from valid bars, direction mix preserved | `p <= 0.05` |
| Back-lag | entry moved 100 bars before the event | lift `<= +0.01` or event lift exceeds it by `>= +0.01` |
| Shuffle | 200 permutations of event directions within the bundle | `p <= 0.05` |

Seed 0. All controls are deterministic.

## 9. Pass rule

A candidate passes only when all hold:

1. `n >= 200` and `J >= 20`.
2. `lift >= +0.05` over the direction-matched 1h base.
3. `lift_lcb > 0` at `z = 2.394`.
4. Random `p <= 0.05`.
5. Back-lag passes.
6. Shuffle `p <= 0.05`.
7. At least two instruments have `n >= 30` and `hit_rate > base`.

A pass is **exploratory evidence**. It promotes the candidate to a Phase 2 model
charter; it does not confirm an edge.

## 10. Decision

| Outcome | Action |
|---|---|
| No candidate passes | Record no edge for the price-only information surface; stop Phase 2 |
| A candidate passes | Write a Phase 2 model charter, implement the event in the model engine, freeze it, and run a one-shot confirmatory gate on `model_validation` |
| D1 under 200 | Record `inconclusive — underpowered`; do not promote |
| Volume family | Blocked by data quality; no claim; needs a consistent volume source |

No candidate is changed after its result is seen. A failed candidate is retired,
not retried with a different threshold.

## 11. Command

```bash
python3 tools/improvement/v5_phase2_spike.py \
  --matrix config/model-v4/dev/matrix.json \
  --config adx25 \
  --out data/improvement/2026-11-04/V5-PHASE2-SPIKE/artifacts \
  --seed 0
```

## 12. Boundaries and limitations

- Entry is the signal close; no costs, slippage, funding, or execution.
- Exploration is on `model_dev`; a survivor is confirmed on `model_validation`,
  not on a second `model_dev` pass.
- The three instruments are correlated; two-instrument replication is weaker
  than independent markets.
- The volume family is untested because the data is not admissible; this is not
  evidence that volume has or lacks an edge.
- Same-session analysis; no independent Quant Validator record for an
  exploratory no-edge result.
