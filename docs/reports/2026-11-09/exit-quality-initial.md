# Exit Quality — Initial Read

- Date: 2026-11-09 (UTC)
- Status: Exploratory; no rule and no edge claim
- Tools: [`tools/trades/exit_quality.py`](../../../tools/trades/exit_quality.py)
- Contract: [`docs/config/2026-11-09/trade-context.md`](../../config/2026-11-09/trade-context.md)
- Data: `data/improvement/2026-11-09/BINANCE-BTCUSDT/exit_quality.csv` and
  `data/improvement/2026-11-09/SR9-15M-STRUCTURE/exit_quality.csv`

## 1. Summary

| Feed | Trades | Stop | Target | Strategy |
|---|---:|---:|---:|---:|
| Binance BTCUSDT 2018–2023 | 73 | 60 | 10 | 3 |
| Dukascopy BTCUSD 2018–2022 | 30 | 21 | 7 | 2 |

Most exits are stops, not state exits.

| Feed | Exit | Median exit efficiency | Median post-50 MFE |
|---|---|---:|---:|
| Binance | target | 0.809 | +1.58% |
| Binance | stop | −1.359 | +1.40% |
| Binance | strategy | 0.378 | +1.71% |
| Dukascopy | target | 0.782 | +1.88% |
| Dukascopy | stop | −1.281 | +1.46% |
| Dukascopy | strategy | 0.628 | +1.91% |

`exit_efficiency = captured_return / trade MFE`. A negative value on a stop is
expected: the trade gave back its favorable excursion. The post-50 MFE is the
best favorable move in the 50 bars after the exit, measured from the exit price.

## 2. Reading

- **Stops dominate both feeds.** 60 of 73 Binance exits and 21 of 30 Dukascopy
  exits are stops.
- **A stop is often followed by a favorable move.** Median post-stop MFE over 50
  bars is +1.40% (Binance) and +1.46% (Dukascopy). This is consistent with the
  human observation that the exits, not only the entries, are the problem.
- **Target exits are efficient in the narrow sense.** Median efficiency is about
  0.81/0.78, so the target captures most of the trade's favorable excursion.
- **Strategy exits are rare.** Only 3 Binance and 2 Dukascopy exits are strategy
  exits; the rest are mechanical stops/targets.
- **2018 trade 3 is the clearest early time-stop example.** It exited with the
  EMA stack, structure, and MACD still positive and RSI 67; the post-exit window
  is in its context bundle.

## 3. Limitations

- Post-exit MFE is not a trade result. It ignores the path, costs, stop/target
  order, and the possibility of being stopped before the favorable move.
- The 50-bar window is descriptive; it is not a holding-period proposal.
- The selected trades are a small, human-selected set; the aggregate table covers
  all recorded trades but still mixes regimes and costs.
- 2023 is outside the formal partitions and remains exploratory.
- This report does not define a new exit rule. Any rule change needs a new
  pre-registered Strategy Research hypothesis and out-of-sample evidence.

## 4. Next step

Use the context bundles to classify stops into:

1. **trend intact at stop** — stack/structure still positive, price above the
   model EMAs, stop hit by volatility;
2. **state deterioration before stop** — structure/MACD/EMA stack already
   negative;
3. **time stop with trend intact** — the 2018 trade 3 pattern.

Then test exit variants only in a new pre-registered programme. No parameter
change is made here.
