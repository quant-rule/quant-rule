# Annotation Zone Analysis — 2018–2019

- Date: 2026-11-10 (UTC)
- Status: Current; descriptive evaluation only
- Inputs: 595 2018 marks and 1,025 2019 marks, each with 121 readings columns
- Model: `M1.2`
- Claim type: descriptive annotation analysis; no strategy or edge claim.

> **2026-11-10 correction:** the close-based features formerly labelled
> "Fibonacci test" and "Band test" are renamed `fib_close_proximity` and
> `band_close_inside`. They are not the model events `level.fib_test` or
> `level.band_test`; the reported numbers are unchanged.

## 1. Method

The mark is treated as a zone, not one candle:

- `radius = 3`: three bars before the mark, the mark, and three bars after.
- A sensitivity run uses `radius = 5`.
- Context: `data/improvement/2026-11-09/BINANCE-BTCUSDT/annotations/human-2018-v12/`
  and `human-2019-v12/`.
- Tool: `tools/trades/annotation_zones.py`.
- Output: `data/improvement/2026-11-09/BINANCE-BTCUSDT/annotations/zones-2026-11-10/`.

Feature definitions:

| Feature | Definition |
|---|---|
| EMA tap | A zone bar's high or low is within `0.2 × ATR` of the EMA |
| Fibonacci close proximity | A zone bar's close has `fib_nearest_atr <= 0.2` |
| Band close-inside | A zone close is inside the selected band |
| Pivot 2+ | The zone contains a count of at least two consecutive same-kind confirmed pivots |
| Forward return | Side-adjusted close change from the mark close at 4, 16, and 50 bars |

All data was generated from the recorded model bundle; no model or strategy was
changed.

## 2. Overall groups

| Group | n | EMA tap | Fib close | Band inside | 50-bar return | 50-bar hit |
|---|---:|---:|---:|---:|---:|---:|
| Long entry | 488 | 91.2% | 85.7% | 15.0% | +1.43% | 75.4% |
| Long exit | 488 | 71.9% | 59.8% | 12.9% | −0.48% | 61.9% |
| Short entry | 322 | 87.9% | 85.7% | 16.8% | +1.21% | 76.4% |
| Short exit | 322 | 68.0% | 56.5% | 15.2% | −0.71% | 69.3% |

Entry returns are favorable and exit returns are favorable at the 50-bar
horizon. The hit rate is the share of entries with a positive return or exits
with a negative return.

## 3. 2018 versus 2019

| Group | n | EMA tap | Fib close | 50-bar return | 50-bar hit |
|---|---:|---:|---:|---:|---:|
| 2018 long entry | 177 | 90.4% | 77.4% | +2.44% | 78.5% |
| 2018 long exit | 176 | 72.2% | 60.8% | −0.56% | 56.8% |
| 2018 short entry | 121 | 88.4% | 81.0% | +2.39% | 78.5% |
| 2018 short exit | 121 | 69.4% | 45.5% | −1.29% | 66.9% |
| 2019 long entry | 311 | 91.6% | 90.4% | +1.05% | 73.6% |
| 2019 long exit | 312 | 71.8% | 59.3% | −0.44% | 64.7% |
| 2019 short entry | 201 | 87.6% | 88.6% | +1.05% | 75.1% |
| 2019 short exit | 201 | 67.2% | 63.2% | −0.60% | 70.6% |

The 2018 edge is stronger, but the 2019 direction is the same. Entry EMA-tap and
Fib-test rates are stable across the two years.

## 4. State at the mark

| Group | RSI | VWAP distance | Fast-EMA distance | Band-center distance |
|---|---:|---:|---:|---:|
| Long entry | 45.0 | −0.56 ATR | −0.55 ATR | +0.92 ATR |
| Long exit | 63.9 | +1.72 ATR | +1.47 ATR | +4.26 ATR |
| Short entry | 51.8 | +0.28 ATR | +0.25 ATR | +1.03 ATR |
| Short exit | 32.6 | −1.98 ATR | −1.85 ATR | −2.62 ATR |

The mean-reversion shape is stable: long entries below VWAP and the fast EMA,
long exits above them, short entries near/above VWAP, and short exits far below.

## 5. Pivot sequences

Percent of marks whose zone reaches at least two consecutive confirmed pivots:

| Group | Lower-high 2+ | Lower-low 2+ | Higher-high 2+ | Higher-low 2+ |
|---|---:|---:|---:|---:|
| Long entry | 27.7% | 23.8% | 27.0% | 33.2% |
| Long exit | 21.9% | 13.9% | 29.3% | 40.8% |
| Short entry | 28.0% | 19.9% | 22.4% | 32.9% |
| Short exit | 34.2% | 22.7% | 17.4% | 25.5% |

The second/third lower-high or lower-low pattern is real but not the majority of
marks. It is more common at short exits (lower-high 34.2%) and long exits
(higher-low 40.8%). A radius of five raises the entry lower-high rate to about
30–34%, so the rate depends on how wide the zone is.

## 6. Findings

1. **EMA taps are the strongest zone feature.** 91.2% of long entries and 87.9%
   of short entries have an EMA tap in the zone. The fast EMA is the dominant
   tap (86.5% and 83.2%); the slow EMA is rarely the tap (21.1% and 20.2%).
   However, 68–72% of exits also have a tap, so a tap is a zone property, not a
   sufficient entry trigger.

2. **Fibonacci levels are entry-side features.** 85.7% of both entry groups have
   a Fibonacci close-proximity feature in the zone, versus 59.8% of long exits
   and 56.5% of short exits. The nearest level at the mark is about 0.22–0.25 ATR from price on
   entries, versus 1.38–1.61 ATR on exits.

3. **Horizontal bands are not the entry trigger in this set.** Only 12.9–16.8%
   of marks have a close inside a band. The selected band center is about one ATR from entry
   price, but 4.26 ATR above long exits and 2.62 ATR below short exits. The band
   looks more like an extension/target reference than an entry zone.

4. **Multi-pivot sequences are a minority, not the default.** The lower-high 2+
   rate is 27.7% at long entries and 28.0% at short entries; the lower-low 2+
   rate is 23.8% and 19.9%. It is a meaningful subset, but a rule that requires
   it would miss most of the marked entries.

5. **2019 is weaker but directionally consistent.** Entry 50-bar returns fall
   from about +2.4% in 2018 to +1.05% in 2019; the hit rate stays 73.6–75.1%.
   Long exits remain around 62–65% correct, short exits around 69–71%.

## 7. Caveats

- The annotator showed the full 1,000-candle window. The marks are hindsight
  labels, and the zone includes bars after the mark. A high hit rate is not a
  causal result.
- The chart windows overlap. The 2018 marks contain only 505 unique timestamps;
  the effective independent sample is smaller than the mark count.
- All marks are `good`. There is no bad-entry/bad-exit comparison.
- 2018–2019 are development years. No validation partition and no 2024 data
  were used.
- The zone rates depend on the radius: radius 5 raises the EMA and Fibonacci
  rates and the pivot 2+ rates. Do not tune the radius to a preferred result.

## 8. Next

The next step is a pre-registered Strategy Research hypothesis, not a strategy
code change:

- long entry: EMA fast/mid tap plus Fibonacci proximity in a mixed/bearish stack;
- short entry: EMA fast tap plus Fibonacci proximity in a mixed/bullish stack;
- exit: force-loss event or giveback threshold, with the existing time stop as
  the control;
- development: 2018–2022; validation remains one-shot and closed.

No edge is claimed.
