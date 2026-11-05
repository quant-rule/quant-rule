# Defaults

- Date: 2026-11-03 (UTC)
- Issue: [#345](https://github.com/quant-rule/quant-rule/issues/345)
- Status: Current after P4

These are the defaults for a new run:

| File | Default |
|---|---|
| `policy.json` | 10% annualised target, 10% maximum drawdown, 1x leverage, one position |
| `portfolio.json` | 10% drawdown halt, 0.1% commission, 0.01% funding per 8h |
| `model.json` | M1.0 (`quantrule.model/v4`): EMA 20/50/200, RSI 14, ATR 14, RVOL 20, MACD 12/26/9, ADX 14 threshold 25 slope 3 active |

A current `run.json` also names `data.json` and `execution.json`; there are no
default files for them because the source path and fill timeframe are
run-specific. `strategy.json` and `analytics.json` are retired and are not part
of a current run.

`config/examples/` keeps the settings each example was run with. `config/compression*`,
`config/compression-range*`, `config/model-v4/`, and `config/research/` are
evidence and historical research, not defaults. See `config/README.md`.

The time-to-target metric uses the annualised target over the actual run span:
a one-year run targets 10% total; a multi-year run compounds the same annual
rate. The maximum-drawdown time is the peak-to-trough span of the worst
drawdown, with recovery and longest-underwater durations reported separately.
