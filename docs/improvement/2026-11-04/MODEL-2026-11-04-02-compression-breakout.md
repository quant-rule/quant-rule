# MODEL-2026-11-04-02 — Compression-Breakout Range-Expansion Gate

- Date: 2026-11-04 (UTC)
- Tier: model
- Changed: model
- Frozen: strategy, portfolio
- Partition: `model_dev` (2018-01-01..2020-12-31)
- Code commit: `3df10a9` plus the uncommitted compression working tree
- Manifest: `data/improvement/2026-11-04/MODEL-2026-11-04-02/manifest.json`
- Protocol: [`docs/model/2026-11-04/compression-breakout.md`](../../model/2026-11-04/compression-breakout.md)
- Local artifacts: `data/improvement/2026-11-04/MODEL-2026-11-04-02/artifacts/`
  and the model bundles under `data/model-v4/compression/dev/`
- Verdict: **inconclusive — the 2 ATR / 20-bar outcome saturates**
- Claim type: model evidence; no strategy, cost, or edge claim

## Hypothesis

A coiling volatility regime followed by a confirmed compression breakout
produces a larger forward range than coiling alone on `model_dev`, after matched
baselines, negative controls, and a multiple-testing correction.

## Matrix

Nine one-year model bundles — BTCUSD, ETHUSD, and XAUUSD for 2018, 2019, and
2020 — with the compression-breakout event enabled. Timeframes: 1h and 15m. The
primary timeframe is chosen by event count only: 1h has 302 events, so 1h is
primary; 15m has 1,355 and is secondary.

Event definition: five prior bars with context code 2 (ATR ratio < 1 and
RVOL < 1), a close beyond the five-bar range by 0.2 ATR, and RVOL ≥ 1.2 on the
trigger bar.

## Results

Primary (1h):

| n | J | Hit | All base | Coiling base | Placebo base | Lift | Lift LCB | Rand p | Back lag | Rep |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 302 | 290 | 0.9967 | 0.9827 | 0.9888 | 0.9923 | +0.0079 | +0.0004 | 0.1443 | −0.0119 | 3 |

Secondary (15m, descriptive):

| n | J | Hit | Coiling base | Lift | Lift LCB | Rand p | Back lag | Rep |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1,355 | 1,274 | 0.9786 | 0.9583 | +0.0203 | +0.0112 | 0.0050 | −0.0129 | 3 |

Per instrument at 1h: BTC 108 events, 0.9907 vs 0.9837; ETH 70, 1.0000 vs
0.9902; XAU 124, 1.0000 vs 0.9924.

## Review

The outcome is near its ceiling. The coiling base is 98.88% at 1h and 95.83% at
15m, so the maximum possible lift is 1.12pp and 4.17pp; the pre-registered 5pp
lift cannot be observed. The 1h event lift is +0.79pp with a corrected lower
bound of +0.04pp, but the random control p-value is 0.144 and the placebo
transition is higher. The 15m secondary cell has a +2.03pp lift, a corrected
lower bound of +1.12pp, a random p-value of 0.005, and three-instrument
replication, but it is not a primary cell and cannot be promoted after the
result.

The result is **inconclusive**, not evidence against the thesis. The event and
harness are implemented; the outcome threshold is the binding constraint.

## Decision

No validation partition is opened. No model baseline is accepted. No strategy or
portfolio work starts. The event, thresholds, and outcome are not changed.

The next hypothesis must pre-register a **harder expansion outcome** that leaves
room for a 5pp lift, for example a 3 ATR target over 20 bars or a 2 ATR target
over 10 bars, and must use a **fresh sample**; the current `model_dev` paths have
been seen, including the 15m secondary. It is a new hypothesis and needs a new
issue and human approval.

## Limitations

- The same instruments were used for prior directional tests; the validation
  partition is unopened, but residual data-snooping risk remains.
- The 15m secondary observation is descriptive only.
- The harness, review, and decision are same-session passes, not an independent
  Quant Validator record.
