# Analytics Benchmarks

- Date: 2026-11-09 (UTC)
- Status: Current
- Owner: Quant Researcher (definition), Quant Developer (implementation), Quant Validator (reproduction)
- Artifact: `analytics/benchmarks.csv` and the `benchmarks` object in `analytics/metrics.json`
- Contract: [`docs/architecture/command-surface.md`](../../architecture/command-surface.md)
- Claim type: evaluation; no strategy or edge claim.

## 1. Purpose

`analytics evaluate` publishes two descriptive baselines for a completed
backtest:

| Baseline | Question |
|---|---|
| Buy-and-hold | Did the strategy beat simply holding the instrument over the same period? |
| Matched random timing | Did the entries beat random entries with the same trade count, sides, holding periods, costs, and sizing? |

Neither baseline is a strategy input. Neither baseline is read by the model,
strategy, execution, or portfolio stage.

## 2. Buy-and-hold

- Entry at the first execution-timeframe candle's open.
- One commission charge at the recorded portfolio commission rate.
- Quantity is the remaining cash divided by the entry price.
- Equity is marked at each UTC date's last close.
- No stop, target, funding, or rebalancing.
- `alpha_vs_buy_and_hold = strategy_total_return - buy_and_hold_total_return`.

## 3. Matched random timing

The null keeps the strategy's observed trade templates:

- the number of trades;
- each trade's side;
- each trade's holding period in bars;
- the same initial cash, commission rate, and fixed-notional sizing;
- a random entry bar after the previous simulated exit, with no overlap.

The replay uses an explicit seed and 1,000 paths. Each path reports its final
return, maximum drawdown, strategy percentile, and the share of paths that beat
the strategy. The seed is the backtest manifest's seed when present, otherwise
`0`. The same manifest and seed reproduce the same paths.

The nullable output is:

| Field | Meaning |
|---|---|
| `return_distribution` | Mean, median, p05, p95, min, and max final return |
| `max_drawdown_distribution` | The same statistics for maximum drawdown |
| `strategy_percentile` | Share of random paths whose return is below the strategy's |
| `probability_random_beats_strategy` | Share of random paths whose return is at or above the strategy's |

## 4. Limitations

- The null replays commission, not execution fees or slippage. The
  `execution_fees_replayed` flag is `false`.
- The null does not replay a drawdown halt; `max_drawdown_limit` is `null`.
- The null does not replay funding.
- The null is a matched timing test, not an i.i.d. random-walk price generator.
  A random-walk price null is a different test and is not published here.
- A percentile is descriptive. It is not a p-value and does not correct for
  multiple testing. A significance claim needs a pre-registered null and
  correction before the run.
- The benchmark uses the recorded execution timeframe and the recorded candles.
  A missing candle series or an empty trade list produces `null` with a reason
  code, never a synthesized value.

## 5. Output

`analytics/benchmarks.csv` has one row per baseline:

```text
baseline,paths,seed,total_return,max_drawdown,p05_return,p50_return,p95_return,strategy_percentile,probability_random_beats_strategy,alpha_vs_buy_and_hold
```

`analytics/metrics.json` carries the same values under `metrics.benchmarks`,
including the `assumptions` object (`initial_cash`, `commission_rate`,
`max_drawdown_limit`, `execution_fees_replayed`, `seed`, `paths`) and
`null_reasons`.

## 6. Determinism

The random replay is seeded and iterates in a fixed order. The same backtest
manifest, candles, trades, commission rate, and seed produce identical
`benchmarks.csv` bytes. The command has a byte-determinism test.
