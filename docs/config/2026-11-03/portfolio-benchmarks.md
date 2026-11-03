# Portfolio Benchmarks and Time Metrics

- Date: 2026-11-03 (UTC)
- Issue: [#345](https://github.com/quant-rule/quant-rule/issues/345)
- Status: Implemented
- Contract: [`docs/model/2026-11-03/model-v3.md`](../../model/2026-11-03/model-v3.md)
- Defaults: [`config/defaults/README.md`](../../../config/defaults/README.md)

## 1. Purpose

A raw return is not evidence of an edge. Every portfolio run now reports the same
capital, cost, and drawdown metrics against two nulls:

1. **Buy-and-hold** — the market return of the traded instrument.
2. **Random-timing replay** — the same trade count, sides, holding periods, and
   exit brackets, with entry times drawn from a seeded distribution.

The strategy's percentile against the random paths is the visual and numerical
statement of skill. A strategy inside the random envelope has no demonstrated
edge.

## 2. Buy-and-hold

- Entry at the first execution candle's open.
- One commission charge at the configured rate.
- The position is held to the end of the run and marked at the last close of each
  UTC date.
- Funding is not charged to the benchmark; the benchmark is a market reference,
  not a second portfolio.

## 3. Random-timing replay

The random benchmark is an evaluation null. It never enters a strategy decision.

| Property | Definition |
|---|---|
| Trade count | Equal to the strategy's closed trades |
| Side | The strategy trade's side |
| Holding period | The strategy trade's `hold_bars` as the maximum |
| Exit bracket | The strategy trade's exit reason and distance: a stop exits at the same adverse fraction, a target at the same favourable fraction |
| Entry timing | Uniform over the available non-overlapping bars |
| Position size | Fixed notional, capped at current equity |
| Costs | The configured commission on both fills |
| Marking | Open positions are marked at each UTC date's last close |
| Halt | The configured maximum-drawdown halt stops new trades |
| Seed | `DEFAULT_SEED`, recorded in the report |
| Paths | `DEFAULT_PATHS`, recorded in the report |
| Funding | Not replayed; recorded as a limitation |

The replay is deterministic: the same trades, candles, config, and seed produce
the same paths. The generator is a local SplitMix64 implementation, so the crate
adds no dependency and reads no clock.

A random replay is not a second strategy and is not a substitute for a
pre-registered hypothesis. It is a null used to evaluate one.

## 4. Time metrics

The policy target is annualised over `evaluation_days`. For a run spanning
`run_days`, the total target is:

```text
target_total = (1 + performance_target)^(run_days / evaluation_days) - 1
```

A one-year run therefore targets 10% total; a longer run compounds the same
annual rate. The report records:

| Field | Meaning |
|---|---|
| `time_to_target_days` | Days from the first point to the first target crossing, or null |
| `target_reached` | Whether the final equity reached the target |
| `max_drawdown_peak_to_trough_days` | Peak-to-trough span of the worst drawdown |
| `max_drawdown_recovery_days` | Trough-to-recovery span, or null |
| `longest_underwater_days` | Longest run below the running peak |
| `max_drawdown_duration_bars` | The existing longest underwater run in strategy bars |

A drawdown is measured from the running peak. Recovery means the equity reaches
the peak that preceded the worst drawdown.

## 5. Results JSON

`portfolio/report.json` adds:

```json
{
  "summary": {
    "time_to_target_days": 92.03,
    "target_reached": true,
    "max_drawdown_peak_to_trough_days": 15.92,
    "max_drawdown_recovery_days": null,
    "longest_underwater_days": 61.91,
    "benchmark_buy_and_hold_return": 0.2711,
    "benchmark_random_median_return": -0.0156,
    "strategy_percentile_vs_random": 0.0
  },
  "benchmarks": {
    "buy_and_hold": {
      "entry_price": 2062.598,
      "quantity": 4.8434,
      "metrics": {
        "total_return": 0.2711,
        "max_drawdown": 0.0799,
        "time_to_target_days": 92.03,
        "target_reached": true
      }
    },
    "random": {
      "method": "random_timing_daily_replay",
      "seed": 0,
      "paths": 1000,
      "return": {"mean": -0.0169, "median": -0.0156, "p05": -0.0328, "p95": -0.0053},
      "max_drawdown": {"mean": 0.0176, "median": 0.0163, "p05": 0.0067, "p95": 0.0330},
      "time_to_target_days": null,
      "time_to_target_reached_share": 0.0,
      "strategy_percentile": 0.0,
      "probability_random_beats_strategy": 1.0
    }
  }
}
```

The numbers above are illustrative from the XAUUSD v7 validation run; they are
not an edge claim. `equity.png` draws the strategy, buy-and-hold, and the random
5th/95th percentile band with the median line.

`artifacts/evidence.json` extracts the summary fields into the improvement
record. The raw paths are kept in memory for the chart and are not written to
the report.

## 6. Defaults

`config/defaults/policy.json` sets a 10% annualised target and a 10% maximum
drawdown. `config/defaults/portfolio.json` sets the matching 10% halt. The
historical example and improvement configs keep the 20% envelope they were run
with; they are evidence, not defaults.

## 7. Verification

- Unit tests cover equity metrics, buy-and-hold, daily marking, deterministic
  random paths, and a different seed producing different paths.
- A portfolio report over a fixed run shows the same JSON and PNG bytes for the
  same seed.
- The strategy boundary test proves the random benchmark is not readable by a
  strategy.
