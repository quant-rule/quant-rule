# SR2-BTC-FEATURES — Top-Trade Attribution

- Date: 2026-11-08 (UTC)
- Status: **Complete — hypothesis generated; no strategy run**
- Parent audit: [`SR2-BTC-AUDIT.md`](SR2-BTC-AUDIT.md)
- Plan: [`sr2-feature-programme.md`](../../execution-plans/2026-11-08/sr2-feature-programme.md)
- Tool: [`sr2_feature_audit.py`](../../../tools/improvement/sr2_feature_audit.py)
- Claim type: exploratory feature attribution; no edge or profitability claim

## 1. What was done

- Extracted the decision-bar features for all 139 SR2-A BTCUSD trades.
- Compared the top five trades with the full sample and with condition subsets.
- Generated 1,000-candle 1h charts for the top five, with volume, EMA 10/50/200,
  entry/exit, and a feature strip.
- The charts are in
  `data/improvement/2026-11-08/SR2-BTC-FEATURES/charts/`.
- The feature table is `features.csv`; the summary is `features.json`.

## 2. Top five trades

| Trade | Side | Event | EMA stack | ADX | RVOL | ATR ratio | Session |
|---|---|---|---:|---:|---:|---:|---|
| 2020-03-15 | long | −1 | −1 | 14.6 | 1.34 | 0.69 | 18–24 |
| 2018-04-06 | long | −1 | −1 | 17.6 | 1.29 | 0.97 | 06–12 |
| 2021-02-25 | long | −1 | −1 | 22.1 | 1.21 | 0.83 | 06–12 |
| 2020-03-25 | short | +1 | +1 | 21.3 | 1.31 | 0.95 | 06–12 |
| 2021-01-31 | short | +1 | 0 | 17.9 | 1.21 | 0.88 | 12–18 |

The common pattern:

- **Four of five have event direction equal to the EMA stack**, so the fade is
  against the prevailing stack. The 2021-01-31 short is stack-neutral.
- **ADX is below 25** in all five: weak trend.
- **RVOL is low**, 1.21–1.34, versus a median near 1.38.
- ATR ratio is mixed; high volatility alone does not explain the top five.

## 3. Full-sample attribution

| Subset | Trades | Net P&L | Mean | PF |
|---|---:|---:|---:|---:|
| All | 139 | +3,544.69 | +25.50 | 1.42 |
| Aligned with stack | 55 | +2,447.88 | +44.51 | 1.70 |
| Against stack (non-neutral) | 47 | +1,003.74 | +21.36 | 1.38 |
| Stack neutral | 36 | +198.48 | +5.51 | 1.09 |
| Stack unavailable | 1 | −105.42 | −105.42 | — |
| ADX < 25 | 91 | +3,455.04 | +37.97 | 1.65 |
| ADX ≥ 25 | 48 | +89.64 | +1.87 | 1.03 |
| RVOL < 1.4 | 77 | +3,895.76 | +50.59 | 1.84 |
| RVOL ≥ 1.4 | 62 | −351.07 | −5.66 | 0.91 |
| Aligned + ADX < 25 | 38 | +2,416.59 | +63.59 | 2.09 |
| Aligned + RVOL < 1.4 | 33 | +2,729.92 | +82.72 | 2.28 |
| Aligned + ADX < 25 + RVOL < 1.4 | 22 | +2,253.36 | +102.43 | 2.67 |

The RVOL split is the strongest single feature by mean and PF. The aligned and
ADX splits add when combined with it: the three-condition intersection has the
best mean and PF (2.67), on 22 trades. Four of the top five sit in that
intersection; the fifth is stack-neutral.

## 4. The 2019 problem remains

| Filter | 2018 | 2019 | 2020 | 2021 | 2022 |
|---|---:|---:|---:|---:|---:|
| RVOL < 1.4 | +847 | −473 | +2,008 | +918 | +595 |
| Aligned + ADX < 25 | +166 | −390 | +1,697 | +412 | +531 |
| RVOL < 1.4 + aligned | +532 | −309 | +1,738 | +778 | −9 |
| RVOL < 1.4 + ADX < 25 | +201 | −300 | +2,080 | +959 | +345 |

The filters improve per-trade quality (mean and PF) but do not remove the 2019
regime loss. Every filter subset is negative in 2019; the RVOL+aligned subset is
also slightly negative in 2022. The feature set is not a regime switch; it is a
quality filter.

## 5. What this means

- The pattern is coherent in the aligned cases: fade **low-participation,
  weak-trend breakouts that are aligned with the prevailing EMA stack**; avoid
  high-RVOL, strong-trend breakouts, which continue.
- The feature is in-sample. The RVOL threshold 1.4 is a round boundary above
  the 1.21–1.34 top-five range, fixed before the Phase C run; it is not tuned
  after the result. The subset still needs a pre-registered out-of-sample test.
- At Phase A the strategy could read `ema_stack` and `adx_regime` but not
  `rvol_ratio`. Phase B promoted the surviving feature into the `rvol_regime`
  state; Phase C tested it.
- The top five remain a tail. The full-sample subsets are the evidence to carry
  forward, not the five trades.

## 6. Charts

The top-five charts show:

- the 2020-03-15 long at the COVID crash low, after a bearish breakout aligned
  with a bearish stack;
- the 2018-04-06 and 2021-02-25 longs at reversal points;
- the 2020-03-25 short after a bullish breakout aligned with a bullish stack;
- the 2021-01-31 stack-neutral short after a bullish breakout.

Open `data/improvement/2026-11-08/SR2-BTC-FEATURES/charts/*.png`. Each image is
1,000 1h candles, with volume, EMA 10/50/200 drawn for display only, entry/exit
markers, and the event/stack/ADX/RVOL/ATR/session strip.

## 7. What followed

`rvol_regime` was promoted as a discrete context state:

- low: `RVOL20 < 1.4`;
- normal: `1.4 ≤ RVOL20 < 2.0`;
- high: `RVOL20 ≥ 2.0`.

The pre-registered cells in
[`sr2-feature-programme.md`](../../execution-plans/2026-11-08/sr2-feature-programme.md)
then ran on `strategy_dev`:

1. 1h aligned + ADX weak + RVOL low, long and short.
2. 15m entry with 1h context, long and short.
3. Controls without ADX and without RVOL.

The result is in [`SR3-FEATURES.md`](SR3-FEATURES.md): H2 (15m entry) is
negative; H1/H3/H4 are positive at 6 and 9 bps but hold 9–21 trades per side;
no candidate; 2024 was not opened.

## 8. Limitations

- Exploratory in-sample attribution; no out-of-sample evidence.
- The 2019 loss persists in every filter; regime risk remains.
- The charts use 1h candles, not a 1m zoom.
- The top five are not independent observations.
- The sample is one instrument and five years.
