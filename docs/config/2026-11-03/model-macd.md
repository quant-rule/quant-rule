# MACD Configuration and Events

- Date: 2026-11-03 (UTC)
- Issue: [#347](https://github.com/quant-rule/quant-rule/issues/347)
- Status: Implemented; opt-in on the legacy model path, active in `config/defaults/model.json`
- Contract: [`docs/model/2026-11-03/model-v3.md`](../../registry/history.md)

## 1. Formula

```text
line      = EMA(close, fast) - EMA(close, slow)
signal    = EMA(line, signal_period)
histogram = line - signal
```

The EMA seed is the arithmetic mean of the first `period` values, and each later
value follows `alpha * value + (1 - alpha) * previous` with `alpha = 2/(period+1)`.
The line is available once the slow EMA is available. The signal and histogram
are available after the signal EMA has received its seed of line values. A null
line value is skipped, never treated as zero.

Defaults: `fast = 12`, `slow = 26`, `signal = 9`.

## 2. Configuration

```json
{
  "defaults": {
    "macd": {
      "fast": 12,
      "slow": 26,
      "signal": 9,
      "active": true
    }
  }
}
```

`active` is optional and defaults to `true` when the block is present. The block
is optional; a model config without it keeps MACD disabled for the legacy v2
path. `config/defaults/model.json` enables it for new runs. Per-timeframe
overrides are supported:

```json
{
  "timeframes": {
    "1h": { "macd": { "fast": 12, "slow": 26, "signal": 9, "active": true } }
  }
}
```

The parser rejects a zero period, `fast >= slow`, or an unknown field.

## 3. States

| State | Code | Condition |
|---|---|---|
| `macd_state` | +1 | line > signal |
| `macd_state` | -1 | line < signal |
| `macd_state` | 0 | line == signal |

The state is null when the line or signal is null. The flat report also publishes
the histogram sign as `macd_histogram_state`.

## 4. Signal events

| Subtype | Code | Condition | Source |
|---|---|---|---|
| Series crossover | 2 | line crosses the signal | MACD line (13), reference signal (14) |
| Histogram slope flip | 4 | the histogram's slope changes sign | MACD histogram (15) |
| Zero cross | 5 | the line crosses zero | MACD line (13) |

The line/signal crossover and the histogram zero cross are the same underlying
histogram sign change, so only the series crossover is emitted. The histogram
slope flip is a distinct momentum-acceleration event. State changes are not
duplicated on the same bar.

## 5. Report and artifacts

The flat model report adds:

- `readings.macd_line`, `readings.macd_signal`, `readings.macd_histogram`.
- `states.macd_state`, `states.macd_histogram_state`.

The model bundle writes `readings.csv` with the three series per bar and
`events.csv` with the full event batch. `levels.csv` and `legs.csv` carry the
remaining model records.

## 6. Tests

- Golden values for a five-bar sequence.
- Constant-series zero line, signal, and histogram.
- Signal seed delay.
- Causality: appending bars does not change earlier values.
- Determinism: repeated runs are identical.
- Short, empty, and invalid period inputs are all null.
- Signal detector: line/signal cross, zero cross, and histogram slope flip are
  distinct records.

## 7. Limitations

- MACD is opt-in on the legacy v2 model path; enabling it changes event counts
  and the model report. The v3 path enables it by default.
- The independent axis/dashboard goldens intentionally use MACD disabled so they
  keep testing the legacy path without a mixed change.
- Funding and execution are unaffected; MACD is a model reading only.
