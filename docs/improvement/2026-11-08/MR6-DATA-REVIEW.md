# MR6 Data Steward Review — Participation Admissibility

- Date: 2026-11-08 (UTC)
- Status: Input to the approved MR6 charter; run gate cleared with the restriction below
- Owner: Data Steward
- Reviewer: Quant Validator (read-only challenge)
- Partition contract: [`data/partitions.json`](../../../data/partitions.json)
- Claim type: data-admissibility review; no model result

## 1. Question

Can the MR6 participation family use the volume columns of XAU, BTC, S&P 500,
EURUSD, and ETH on `model_dev` 2018–2020, or does the known volume-unit defect
block the programme?

## 2. Source coverage

| Instrument | Source file | Rows 2018–2020 | First bar | Last bar | Zero volume | Duplicate stamps |
|---|---|---:|---|---|---:|---:|
| XAU | `XAUUSD_M1_2018-01-01_2024-12-31.csv` | 1,062,760 | 2018-01-01 23:00 | 2020-12-31 21:59 | 0 | 0 |
| BTC | `BTCUSD_M1_2018-01-01_2024-12-31.csv` | 1,350,966 | 2018-01-01 22:01 | 2020-12-31 21:59 | 0 | 0 |
| SP500 | `ESANDP500_M1_2018-01-01_2024-12-31.csv` | 921,883 | 2018-01-02 07:05 | 2020-12-31 21:14 | 0 | 0 |
| EURUSD | `EURUSD_M1_2018-01-01_2024-12-31.csv` | 1,120,167 | 2018-01-01 22:00 | 2020-12-31 21:59 | 0 | 0 |
| ETH | `ETHUSD_M1_2018-01-01_2024-12-31.csv` | 1,306,991 | 2018-01-01 22:01 | 2020-12-31 21:59 | 0 | 0 |

Every file is strictly increasing, has no duplicate timestamps, and has no zero
volume. The runs declare `mr6-<instrument>-m1-<year>` data versions.

## 3. Volume-unit audit

The v5 Phase 2 audit found that BTC and ETH volume units change within a year.
The monthly median M1 volume, maximum/minimum per instrument-year, reproduces
that finding:

| Instrument | 2018 ratio | 2019 ratio | 2020 ratio | Reading |
|---|---:|---:|---:|---|
| XAU | 4.2× | 2.6× | 2.2× | stable |
| BTC | 1,881× | 682× | 1,135× | unit/contract changes |
| SP500 | 4.3× | 3.5× | 11.7× | stable to moderate |
| EURUSD | 1.3× | 3.8× | 9.1× | stable to moderate |
| ETH | 24,000× | 33.8× | 40.2× | unit/contract changes |

A 600×–24,000× shift is a unit or contract change, not market volume. Absolute
volume levels and cumulative volume levels are therefore not comparable across
a BTC or ETH year.

## 4. Admissibility decision

The MR6 participation family is restricted to events that are invariant to a
constant volume-unit scale and are distorted only in a bounded window after a
unit change:

| Event | Volume quantity | Scale behaviour | Admitted |
|---|---|---|---|
| `signal.rvol_expansion` | RV5 relative to its own mean | scale-invariant; 5-bar boundary | yes |
| `signal.rvol_ratio` | RV5 / RV50 | scale-invariant; 50-bar boundary | yes |
| `signal.cmf_cross` | 20-bar money-flow ratio | scale-invariant; 20-bar boundary | yes |
| `signal.mfi_threshold` | 14-bar money-flow ratio | scale-invariant; 14-bar boundary | yes |
| `signal.climax_reversal` | RV5 and range | scale-invariant | yes |
| `signal.climax_continuation` | RV5 and range | scale-invariant | yes |
| `signal.absorption_break` | RV5 and range | scale-invariant | yes |
| `signal.absorption_rejection` | RV5 and range | scale-invariant | yes |
| `signal.obv_cross` | cumulative signed volume | level jumps at a unit change | **no** |
| `signal.ad_cross` | cumulative money-flow volume | level jumps at a unit change | **no** |
| `signal.volume_oscillator_cross` | relative volume oscillator | scale-invariant, but `context` role | **no** |

The two cumulative events are excluded before any outcome is read. The volume
oscillator is excluded because a direction prediction may name only `signal`
events; it remains available to a future context study.

## 5. Consequence for the grid

- The participation family is eight `signal` events, not ten.
- XAU, BTC, S&P 500, EURUSD, and ETH all contribute. ETH is not price-only in
  MR6; its relative-volume columns are admitted under the rule above. This
  supersedes the narrower MR5 statement that ETH volume is inadmissible for
  absolute-volume work.
- The bounded boundary distortion is disclosed, not removed. A unit change
  distorts at most the 50 bars after the change; that is roughly 0.15% of the
  M1 bars in the worst case and is not expected to drive a result. The condition
  does not special-case the boundary; a future data version should repair it.
- The `model_validation` partition remains closed. This review does not open it.

## 6. Recommendation

**Admit the restricted relative-volume participation family and run the MR6
count gate.** If a primary cell cannot reach 200 condition-matched predictions
and 20 clusters, the programme stops as `underpowered` before any outcome is
read. If the count gate passes, the development grid proceeds.

## 7. Limitations

- The admissibility argument assumes a constant volume unit inside each rolling
  window. The monthly unit changes are the residual risk, bounded above.
- The source files are local workspace data; the run records the data version
  and the file hash at run time.
- The validation partition is reused from earlier programmes; a later
  confirmation is not a fresh holdout.
- This review is a Data Steward pass by the same agent that later runs the
  Quant Developer and Quant Validator passes; it is not an independent review.
