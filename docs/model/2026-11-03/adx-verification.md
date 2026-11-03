# ADX/DI Verification Pilot

- Date: 2026-11-03 (UTC)
- Status: **Pilot; not a model baseline and not an edge claim**
- Epic: [#342](https://github.com/quant-rule/quant-rule/issues/342)
- Plan: [`docs/execution-plans/2026-11-03/342-model-verification-adx.md`](../../execution-plans/2026-11-03/342-model-verification-adx.md)
- Configs: `config/verification/shared/model.json`, `config/verification/shared/model-adx20.json`
- Local artifacts: `data/verification/`

## 1. Question

Does the ADX/DI family calculate correctly, and does the
`signal.trend_continuation` conjunction or the `signal.di_cross` event beat a
matched sign base rate on a development sample?

This is a harness and construct check, not a model validation. One instrument and
one month are too small for a gate decision.

## 2. Data and configurations

| Config | ADX threshold | ADX slope | EMA | MACD | Timeframes |
|---|---:|---:|---|---|---|
| `model.json` | 25 | 3 bars | 20/50/200 | 12/26/9 | 5m, 15m, 1h |
| `model-adx20.json` | 20 | 3 bars | 20/50/200 | 12/26/9 | 5m, 15m, 1h |

- Instrument: BTCUSD.
- Window: 2018-01-01 through 2018-01-31.
- Data version: `dukascopy-bid-btcusd-m1-2018-01`.
- Outcome: sign of the forward return at 5 bars, versus the unconditional base
  rate. This is the screening harness, not the path claim.

## 3. Results

ADX threshold 25:

| Event | TF | n | Hit | Base | Lift | 95% LCB |
|---|---|---:|---:|---:|---:|---:|
| `signal.di_cross` | 5m | 533 | 0.5610 | 0.4942 | +0.0667 | 0.4889 |
| `signal.di_cross` | 15m | 172 | 0.5058 | 0.5017 | +0.0041 | 0.3779 |
| `signal.di_cross` | 1h | 61 | 0.5410 | 0.4889 | +0.0521 | 0.3269 |
| `signal.trend_continuation` | 5m | 119 | 0.4286 | 0.4942 | -0.0657 | 0.2764 |
| `signal.trend_continuation` | 15m | 42 | 0.4286 | 0.5017 | -0.0731 | 0.1724 |
| `signal.trend_continuation` | 1h | 7 | 0.5714 | 0.4889 | +0.0825 | 0.0000 |

ADX threshold 20:

| Event | TF | n | Hit | Base | Lift | 95% LCB |
|---|---|---:|---:|---:|---:|---:|
| `signal.di_cross` | 5m | 533 | 0.5610 | 0.4942 | +0.0667 | 0.4889 |
| `signal.di_cross` | 15m | 172 | 0.5058 | 0.5017 | +0.0041 | 0.3779 |
| `signal.di_cross` | 1h | 61 | 0.5410 | 0.4889 | +0.0521 | 0.3269 |
| `signal.trend_continuation` | 5m | 150 | 0.4133 | 0.4942 | -0.0809 | 0.2784 |
| `signal.trend_continuation` | 15m | 45 | 0.5333 | 0.5017 | +0.0317 | 0.2838 |
| `signal.trend_continuation` | 1h | 8 | 0.5000 | 0.4889 | +0.0111 | 0.0000 |

`signal.di_cross` is unchanged by the ADX threshold because its definition does
not use ADX. The `trend_continuation` conjunction is sensitive to the threshold
and does not replicate its sign across timeframes.

## 4. Decision

**No edge in this pilot.** The gate is `no_edge`; no threshold is selected and no
configuration is promoted. The one-month sample is too small to distinguish a
real effect from noise. The result is recorded so the next run is a new,
pre-registered hypothesis rather than a silent threshold search.

## 5. What this proves

- ADX/DI is computed, surfaced in `readings.csv`, and enumerated by the
  registry-driven claim harness.
- The ADX states and DI state are published as numbers, not labels.
- The `trend_continuation` event fires only when ADX regime, ADX slope, DI
  direction, EMA stack, and MACD state agree.
- The harness now grades `signal.di_cross` and `signal.trend_continuation`
  without a hardcoded event list.

## 6. Limitations

- One instrument, one month, one timeframe ladder.
- Sign outcome at 5 bars; not the path claim (1 ATR target before 1 ATR stop
  within 20 bars).
- No costs, slippage, or execution.
- Same-session pilot; not a Quant Validator record.
- The development and validation partitions are not opened by this pilot.

## 7. Next step

Build the full fast-loop matrix and run the pre-registered `model_dev` matrix
across BTCUSD, ETHUSD, and XAUUSD. The pilot does not authorise a strategy run.
