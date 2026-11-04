# V5 1h Swing Model Gate — No Edge

- Date: 2026-11-04 (UTC)
- Status: **No edge — Phase 1 stopped; validation not opened**
- Program: `PROGRAM-2026-11-04-04`
- Charter: [`docs/governance/2026-11-04/v5-program-charter.md`](../../governance/2026-11-04/v5-program-charter.md)
- Pre-registration: [`V5-SWING-1H-PREREG.md`](V5-SWING-1H-PREREG.md)
- Run: `MODEL-2026-11-04-18` (`model_dev`)
- Partition: `model_dev` 2018–2020
- Instruments: BTCUSD, ETHUSD, XAUUSD
- Claim type: model research evidence; no strategy, cost, or edge claim

## 1. Question

Does a 1h v4 event family reach a **4 ATR target before a 2 ATR stop within 100
1h bars** more often than its direction-matched 1h base, after a three-cell
Bonferroni correction, negative controls, and instrument replication?

The premise was that the v4 events failed because the 20-bar horizon was too
short for cost and trend persistence, not because the information was absent.

## 2. Pre-flight

| Cell | Event | Raw 1h events (`adx25`) | Floor |
|---|---|---:|---:|
| `CELL-01` | 1h trend continuation | 919 | 200 |
| `CELL-02` | 1h major pivot break | 3,126 | 200 |
| `CELL-03` | 1h EMA50 test | 6,722 | 200 |

All three cleared the floor, so the gate ran.

## 3. Results

Primary config `adx25`; barrier 4 ATR target / 2 ATR stop / 100 bars; stop first;
seed 0.

| Cell | n | J | Hit | Base | Lift | Lift LCB | State | Incr. | Incr. LCB | Rand p | Back lag | Shuffle p | Rep | Grade |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 1h trend continuation | 904 | 147 | 0.3827 | 0.3505 | +3.23pp | −1.72pp | 0.3991 | −1.63pp | −6.58pp | 0.0398 | +7.43pp | 0.0896 | 2 | C |
| 1h major pivot break | 3,024 | 10 | 0.3459 | 0.3382 | +0.76pp | −0.60pp | — | — | — | 0.2289 | −0.27pp | 0.2139 | 2 | C |
| 1h EMA50 test | 6,621 | 45 | 0.3297 | 0.3341 | −0.44pp | −3.61pp | 0.3407 | −1.10pp | −4.28pp | 0.7761 | +0.81pp | 0.8557 | 1 | C |

Pass rule: `n ≥ 200`, `J ≥ 20`, lift `≥ +5pp`, LCB `> 0`, random `p ≤ 0.05`,
back-lag pass, shuffle `p ≤ 0.05`, replication `≥ 2`, and the `adx20` sensitivity
check. No cell passed. The harness selection is null.

Per instrument (`adx25`):

| Cell | Instrument | n | Hit | Base | Lift |
|---|---|---:|---:|---:|---:|
| Trend continuation | BTCUSD | 392 | 0.3903 | 0.3501 | +4.02pp |
| Trend continuation | ETHUSD | 207 | 0.4541 | 0.3665 | +8.77pp |
| Trend continuation | XAUUSD | 305 | 0.3246 | 0.3402 | −1.56pp |
| Major pivot break | BTCUSD | 1,253 | 0.3543 | 0.3433 | +1.11pp |
| Major pivot break | ETHUSD | 730 | 0.3589 | 0.3444 | +1.46pp |
| Major pivot break | XAUUSD | 1,041 | 0.3266 | 0.3279 | −0.13pp |
| EMA50 test | BTCUSD | 2,484 | 0.3462 | 0.3362 | +1.01pp |
| EMA50 test | ETHUSD | 1,364 | 0.3174 | 0.3399 | −2.25pp |
| EMA50 test | XAUUSD | 2,773 | 0.3210 | 0.3293 | −0.84pp |

## 4. Primary analysis

- **Trend continuation is the only cell with a positive lift, and the trigger is
  worse than the state it claims.** The event hits 38.27% against a 35.05%
  unconditional base, but the trend-state bars hit 39.91%: the incremental lift
  is −1.63pp. The event adds nothing beyond being in the trend.
- **The back-lag control is the decisive failure.** Entering in the same
  direction 100 bars *before* the event hits 42.48% (+7.43pp over base), which is
  better than the event itself. The prior path carried the move; the event is a
  lagging marker, not a trigger.
- **Major pivot break is flat.** +0.76pp lift, −0.60pp LCB, random p 0.229,
  shuffle p 0.214. It also fails the cluster floor: the 100-bar cluster gap
  chained the events into one cluster per bundle (10 clusters over 9 bundles),
  so its cluster-robust bound is not informative. The decision does not depend on
  that bound: the point lift is below the +5pp threshold and both controls fail.
- **EMA50 test is flat to negative.** −0.44pp lift against the unconditional
  base and −1.10pp against the EMA-side state; random and shuffle p-values are
  near 0.8. Only BTCUSD is positive; ETHUSD and XAUUSD are negative.
- The `adx20` sensitivity rows agree with `adx25` for the pivot-break and EMA50
  cells (identical event sets). The trend cell changes as expected with the ADX
  threshold and remains non-incremental.

## 5. What this means

The horizon explanation is **falsified**. The 1h events do not gain a directional
edge when the payoff is 4 ATR over 100 bars: the only positive lift is smaller
than the +5pp threshold, is not incremental to the trend state, and is beaten by
its own back-lag placebo. The same "the move already happened" signature appears
in the EURUSD New York opening-range breakout, which also failed its back-lag
control.

This result is consistent with the rest of the evidence: the v4 event layer
describes the market state but does not predict the next move, at 20 bars or at
100. A longer horizon does not rescue it.

## 6. Decision

- **No edge** on `model_dev`. Phase 1 stops.
- `model_validation` is **not opened**; no strategy, portfolio, or baseline work
  starts.
- The three cells are retired. Do not tune the horizon, target, stop, families,
  state, or costs after this result.
- Phase 2 (a new-information v5 model) is a separate premise. It requires a new
  charter, a new pre-registration, and human approval. It does not start from
  this result.

## 7. Evidence and limitations

- Run record: `data/improvement/2026-11-04/MODEL-2026-11-04-18/` (local).
- Artifacts: `artifacts/path_validation.json` and `path_validation.md`.
- The entry is the event close; no costs, slippage, funding, or execution. The
  gate is pre-cost by construction.
- The cluster gap equals the 100-bar horizon. For sparse trend events that is
  conservative; for dense pivot-break events it chains a whole bundle into one
  cluster, which is why the pivot cell fails `J ≥ 20`. The gate decision does not
  depend on the pivot cell's bound.
- XAUUSD carries market-hours gaps; the path includes them.
- Same-session analysis; no independent Quant Validator record. The result is
  no-edge, so there is no promising claim to validate.
- A byte-identical rerun check is recorded in the run's `review.md`.
