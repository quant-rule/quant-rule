# MODEL-2026-11-04-01 — v4 Path Gate (Development)

- Date: 2026-11-04 (UTC)
- Tier: model
- Changed: model
- Frozen: strategy, portfolio
- Partition: `model_dev` (2018-01-01..2020-12-31)
- Code commit: `28e5e99` plus the uncommitted v4 working tree
- Manifest: `data/improvement/2026-11-04/MODEL-2026-11-04-01/manifest.json`
- Protocol: [`docs/model/2026-11-04/path-validation-v4.md`](../../model/2026-11-04/path-validation-v4.md)
- Local artifacts: `data/improvement/2026-11-04/MODEL-2026-11-04-01/artifacts/`
  and the model bundles under `data/model-v4/dev/`
- Verdict: **no edge — no tested signal-role family passed on `model_dev`**
- Claim type: model evidence; no strategy, cost, or edge claim

## Hypothesis

At least one of the four pre-registered v4 event families has a 20-bar ATR path
hit rate above both its matched barrier base and its state-conditioned base on
`model_dev`, after a Bonferroni correction, negative controls, and instrument
replication.

## Matrix

Nine one-year model bundles — BTCUSD, ETHUSD, and XAUUSD for 2018, 2019, and
2020 — each run under `adx20` and `adx25`. The four families are:

| ID | Event family |
|---|---|
| H1 | Trend continuation (type 3, subtype 6, source 16) |
| H2 | MACD line/signal cross (type 3, subtype 2, source 13) |
| H3 | RSI 50 cross (type 3, subtype 1, source 4, reference 50) |
| H4 | EMA level break (type 1, subtype 3, sources 1/2/3) |

Timeframes: 5m, 15m, 1h. `adx25` is primary; `adx20` is a sensitivity check.
H2, H3, and H4 produce identical event sets under both settings; H1 changes as
expected.

## Results

All 12 primary family-timeframe cells are grade C. No cell passes the lift,
state-conditioned incremental, random, back-lag, shuffle, replication, and
sensitivity conditions together.

| Family | TF | n | J | Hit | Base | Lift | Lift LCB | State | Incr. | Incr. LCB | Rand p | Back lag | Shuffle p | Rep | Grade |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| h1_trend_continuation | 5m | 13,269 | 9,314 | 0.4808 | 0.4941 | −0.0133 | −0.0259 | 0.4932 | −0.0124 | −0.0250 | 1.0000 | +0.1189 | 0.9950 | 0 | C |
| h2_macd_cross | 5m | 57,690 | 12,261 | 0.4802 | 0.4918 | −0.0116 | −0.0188 | 0.4791 | +0.0011 | −0.0062 | 1.0000 | −0.0574 | 1.0000 | 1 | C |
| h3_rsi50 | 5m | 90,467 | 10,204 | 0.4725 | 0.4914 | −0.0189 | −0.0273 | 0.4762 | −0.0038 | −0.0121 | 1.0000 | −0.0266 | 1.0000 | 1 | C |
| h4_level_break_ema | 5m | 112,899 | 7,531 | 0.4742 | 0.4918 | −0.0176 | −0.0268 | 0.4790 | −0.0048 | −0.0140 | 1.0000 | −0.0185 | 1.0000 | 1 | C |
| h1_trend_continuation | 15m | 4,118 | 2,895 | 0.4990 | 0.4926 | +0.0064 | −0.0161 | 0.4975 | +0.0016 | −0.0210 | 0.2090 | +0.1383 | 0.0498 | 2 | C |
| h2_macd_cross | 15m | 17,081 | 3,947 | 0.4858 | 0.4908 | −0.0050 | −0.0156 | 0.4865 | −0.0007 | −0.0113 | 0.8806 | −0.0588 | 0.8557 | 0 | C |
| h3_rsi50 | 15m | 26,611 | 3,279 | 0.4847 | 0.4908 | −0.0061 | −0.0138 | 0.4893 | −0.0046 | −0.0123 | 0.9751 | −0.0267 | 0.9950 | 2 | C |
| h4_level_break_ema | 15m | 32,595 | 2,550 | 0.4851 | 0.4909 | −0.0058 | −0.0141 | 0.4896 | −0.0044 | −0.0128 | 0.9900 | −0.0147 | 1.0000 | 1 | C |
| h1_trend_continuation | 1h | 916 | 629 | 0.5273 | 0.4993 | +0.0280 | −0.0214 | 0.5409 | −0.0136 | −0.0631 | 0.0597 | +0.1448 | 0.1393 | 3 | C |
| h2_macd_cross | 1h | 3,609 | 921 | 0.4968 | 0.4914 | +0.0054 | −0.0165 | 0.4975 | −0.0006 | −0.0225 | 0.3184 | −0.0647 | 0.3333 | 2 | C |
| h3_rsi50 | 1h | 5,123 | 751 | 0.4983 | 0.4913 | +0.0070 | −0.0101 | 0.4962 | +0.0022 | −0.0149 | 0.1393 | −0.0275 | 0.1045 | 3 | C |
| h4_level_break_ema | 1h | 6,334 | 645 | 0.4916 | 0.4917 | −0.0000 | −0.0174 | 0.4957 | −0.0041 | −0.0215 | 0.5025 | −0.0175 | 0.6070 | 2 | C |

The full artifact also carries the `adx20` sensitivity table, the
per-instrument breakdown, and the control distributions.

## Review

The 5m families are negative over the pooled three years. The 2018-only subset
had positive point lifts for H2, H3, and H4 at 5m; 2019–2020 reversed them,
which is why the pre-registered full matrix is the decision unit. The only
positive 1h lift with a positive cluster count is H1 (+2.80pp), but its
corrected lift LCB is negative (−0.0214), its state-conditioned lift is negative
(−0.0136), and its back-lag placebo is higher (+0.1448). It is not incremental
information. No family replicates across two instruments.

## Decision

No model baseline is accepted. The development partition records **no edge**.
The validation partition is not opened, and no strategy or portfolio run is
performed. The result is a property of the tested event families on the tested
data and timeframes, not a claim that every possible model thesis is exhausted.

## Limitations

- One-year folds are pooled per instrument.
- The path outcome uses the event close as entry and excludes costs, slippage,
  funding, and execution.
- The baseline is a matched barrier baseline, not a matched price level.
- XAUUSD has market-hours gaps; the 20-bar path includes them.
- The harness, review, and decision are same-session passes, not an independent
  Quant Validator record.
