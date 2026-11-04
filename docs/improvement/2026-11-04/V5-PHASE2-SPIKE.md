# V5 Phase 2 Spike — No Edge

- Date: 2026-11-04 (UTC)
- Status: **No edge — Phase 2 stopped; volume family blocked by data quality**
- Program: `PROGRAM-2026-11-04-05`
- Charter: [`docs/governance/2026-11-04/v5-phase2-charter.md`](../../governance/2026-11-04/v5-phase2-charter.md)
- Pre-registration: [`V5-PHASE2-SPIKE-PREREG.md`](V5-PHASE2-SPIKE-PREREG.md)
- Partition: `model_dev` 2018–2020 (exploration)
- Instruments: BTCUSD, ETHUSD, XAUUSD
- Claim type: exploratory model research; no model baseline, no edge claim

## 1. Question

Does any of three price-only information families — candle anatomy, rejection at
a confirmed major level, or cross-asset relative strength — reach a **4 ATR
target before a 2 ATR stop within 100 1h bars** more often than its
direction-matched base, after a three-candidate correction and controls?

## 2. Data audit — volume family blocked

The pack's `volume` is the sum of M1 volume, but the source units change within a
year. Monthly median volume, `max/min` per instrument-year:

| Instrument-year | Min | Max | Ratio |
|---|---:|---:|---:|
| BTCUSD 2018 | 0.0667 | 59.95 | 899× |
| BTCUSD 2019 | 0.0445 | 27.36 | 614× |
| BTCUSD 2020 | 0.0292 | 34.97 | 1,199× |
| ETHUSD 2018 | 0.2647 | 4,720 | 17,835× |
| ETHUSD 2019 | 0.5774 | 9.056 | 15.7× |
| ETHUSD 2020 | 0.1776 | 10.03 | 56.5× |
| XAUUSD 2018 | 1.119 | 4.364 | 3.9× |
| XAUUSD 2019 | 0.9946 | 2.475 | 2.5× |
| XAUUSD 2020 | 1.846 | 3.868 | 2.1× |

BTC 2020 also contains multi-bar volume prints of ~35,900 against a monthly
median of ~0.03. These are unit and contract changes, not market volume.

Consequence: VWAP, volume profile, and volume climax are **not tested**. XAU
volume is stable but is one instrument and cannot meet two-instrument
replication. This is a data-source defect; it is not evidence that volume has or
lacks an edge. The volume family needs a consistent source before it can be
tested.

## 3. Pre-flight and sample

Raw events on `model_dev` with causal level availability, and the evaluated `n`
after the 100-bar forward window and ATR filter:

| Candidate | Raw | n | Floor |
|---|---:|---:|---:|
| C1 momentum bar | 1,791 | 1,767 | 200 |
| C2 rejection wick | 1,917 | 1,881 | 200 |
| C3 relative strength | 2,556 | 2,504 | 200 |
| D1 gap (XAU only) | 131 | 127 | 200 |

C1–C3 clear the floor. D1 does not and is a diagnostic only. The first C2
pre-flight script reported 3,189 because it did not exclude levels confirmed
after the signal bar; the causal rule and the run use `available_bar <= t`. The
correction is recorded in the pre-registration.

## 4. Results

Barrier 4 ATR / 2 ATR / 100 bars; fixed 100-bar blocks; seed 0.

| ID | Candidate | n | J | Hit | Base | Lift | Lift LCB | Rand p | Back lag | Shuffle p | Rep | Verdict |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| C1 | Momentum bar | 1,767 | 465 | 0.3514 | 0.3428 | +0.87pp | −2.15pp | 0.2886 | +1.89pp | 0.0398 | 2 | no edge |
| C2 | Rejection wick at major level | 1,881 | 435 | 0.3360 | 0.3346 | +0.14pp | −2.71pp | 0.4428 | −1.40pp | 0.7761 | 1 | no edge |
| C3 | Relative-strength cross | 2,504 | 423 | 0.3215 | 0.3379 | −1.64pp | −4.32pp | 0.9900 | +0.15pp | 0.9701 | 1 | no edge |
| D1 | Gap continuation (XAU only) | 127 | 89 | 0.3228 | 0.3282 | −0.54pp | −9.74pp | 0.6169 | +2.38pp | 0.7761 | 0 | inconclusive — underpowered |

Pass rule: `n ≥ 200`, `J ≥ 20`, lift `≥ +5pp`, LCB `> 0`, random `p ≤ 0.05`,
back-lag pass, shuffle `p ≤ 0.05`, replication `≥ 2`. No candidate passed.

Per instrument:

| ID | Instrument | n | Hit | Base | Lift |
|---|---|---:|---:|---:|---:|
| C1 | BTCUSD | 730 | 0.3589 | 0.3493 | +0.96pp |
| C1 | ETHUSD | 430 | 0.3698 | 0.3492 | +2.05pp |
| C1 | XAUUSD | 607 | 0.3295 | 0.3304 | −0.09pp |
| C2 | BTCUSD | 811 | 0.3440 | 0.3370 | +0.70pp |
| C2 | ETHUSD | 365 | 0.3342 | 0.3354 | −0.12pp |
| C2 | XAUUSD | 705 | 0.3277 | 0.3313 | −0.37pp |
| C3 | BTCUSD | 807 | 0.3259 | 0.3424 | −1.65pp |
| C3 | ETHUSD | 810 | 0.3432 | 0.3432 | +0.01pp |
| C3 | XAUUSD | 887 | 0.2976 | 0.3290 | −3.14pp |

## 5. Primary analysis

- **Momentum bar** is the only positive pooled lift, +0.87pp, and it is far below
  the +5pp threshold. The shuffle control passes (p = 0.0398), but the random
  control fails (p = 0.2886), the lower bound is −2.15pp, and the 100-bar
  back-lag placebo (+1.89pp) is stronger than the event. The wide-range bar's
  move has already happened; the next 100 bars do not continue enough to pay.
- **Rejection wick at a confirmed major level** is flat (+0.14pp). It replicates
  on one instrument (BTCUSD +0.70pp), the random control fails (p = 0.4428), and
  the shuffle control fails (p = 0.7761). The wick does not mark absorption.
- **Relative-strength cross** is negative (−1.64pp). Only ETHUSD is flat; BTCUSD
  is −1.65pp and XAUUSD is −3.14pp. The cross-sectional momentum claim fails, and
  the direction is wrong on the pooled sample.
- **Gap continuation** is underpowered at 127 XAU events and is not promoted. It
  is recorded, not tested as a hypothesis.

The common signature is the same as the v4 gates and the EURUSD session spike: a
small positive point estimate that the prior path explains better than the
event, or no lift at all. New price-derived information does not change it.

## 6. What this means

The price-only information surface is exhausted for directional claims on this
data:

| Family | Result |
|---|---|
| v4 events at 20 bars and 100 bars | No edge |
| Divergence, cross-timeframe, progression | No edge or underpowered |
| Session spikes (XAU, EURUSD) | No edge |
| Candle anatomy (momentum, rejection) | No edge |
| Cross-asset relative strength | No edge (negative) |
| Volume (VWAP, profile, climax) | **Untested — data not admissible** |

The volume family is the one remaining new-information family, and it cannot be
tested on the current pack. A 600×–17,800× unit shift within a year is a
data-source problem, not a market result. The honest next step for volume is a
data repair with its own work item, not another event variant.

## 7. Decision

- **No edge** on `model_dev`. Phase 2 stops.
- No Phase 2 model charter, no Rust implementation, no `model_validation`, no
  strategy, no baseline.
- The three price-only candidates are retired. Do not tune the thresholds
  (1.5 ATR range, 0.7 body, 0.8 close position, 0.6 wick, 0.25 ATR level,
  24-bar relative return, ±1.0 relative threshold) after this result.
- The v5 program stops. A new attempt requires a new premise and a new approved
  charter. The only new-information family left is volume, and it requires a
  consistent volume source first.

## 8. Evidence and limitations

- Local artifacts: `data/improvement/2026-11-04/V5-PHASE2-SPIKE/artifacts/`
  (`results.json`, `results.md`).
- Tool: `tools/improvement/v5_phase2_spike.py`; tests:
  `tools/improvement/test_v5_phase2_spike.py`.
- Exploration is on `model_dev`; no confirmatory partition was opened, so no
  claim is made.
- The three instruments are correlated; two-instrument replication is weaker
  than independent markets.
- The volume family is untested, not rejected. No volume claim is made in either
  direction.
- Same-session analysis; no independent Quant Validator record for an
  exploratory no-edge result.

## 9. Review

- A byte-identical rerun with seed 0 matched both `results.json` and
  `results.md` by SHA-256.
- The volume audit is computed from the same pack and is reproducible.
- The candidate rules, thresholds, block estimator, correction, and controls
  are the pre-registered ones. No threshold was changed after the result.
