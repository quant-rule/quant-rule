# Unified Ladder — Quant Validation Record

- Date: 2026-10-30 (UTC)
- Status: Sequential validation pass complete; independent verifier unavailable
- Issue: [#237](https://github.com/quant-rule/quant-rule/issues/237)
- Plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- Experiment: [`docs/experiments/2026-10-30/EXP-2026-10-30-02-unified-ladder.md`](../../improvement/INVALIDATIONS.md)
- Claim type: implementation validation; no edge claim

## 1. Scope

Validate the unified ladder implementation and reproduce the pre-registered
matrix: control, A, B, and C. This record checks determinism, causality, split
integrity, costs, execution realism, and residual risk. It does not approve the
work and does not claim a profitable rule.

## 2. Data and configs

| Item | Value |
|---|---|
| Data version | `dukascopy-bid-solusd-m1-2025-01-01_2025-12-31` |
| Source | `data/SOLUSD_M1_2025-01-01_2025-12-31.csv`, fetched from Dukascopy BID |
| Rows | 522,633 accepted |
| Period | 2025-01-01..2025-12-31 UTC |
| Instrument | SOLUSD |
| Policy | `docs/experiments/2026-10-30/config/base/policy.json` |
| Matrix | `docs/experiments/2026-10-30/matrix.txt` |
| Configs | `docs/experiments/2026-10-30/config/{control,a,b,c}/` |

The source was fetched once with:

```bash
cargo run -q -p quantrule-shell -- fetch sol/usd 2025-01-01 2025-12-31 \
  --csv data/SOLUSD_M1_2025-01-01_2025-12-31.csv
```

## 3. Reproduction

```bash
cargo build --release -p quantrule-shell
QR=target/release/quantrule tools/experiment/run-matrix.sh \
  docs/experiments/2026-10-30/matrix.txt \
  target/experiment-EXP-2026-10-30-02

python3 docs/experiments/2026-10-30/analyze_unified_ladder.py \
  target/experiment-EXP-2026-10-30-02/control
```

The control and variation A were rerun into separate roots and compared bundle
by bundle:

| Cell | Bundles identical |
|---|---|
| Control | model, strategy, execution, portfolio, analytical |
| A | model, strategy, execution, portfolio, analytical |

## 4. Results

Full-year run:

| Cell | Return | Max DD | Trades | Profit factor | Win rate | Average trade | Exposure | Cost drag |
|---|---|---|---|---|---|---|---|---|
| Control | +0.594% | 1.966% | 12 | 1.233 | 41.67% | +4.95 | 0.000134 | 243.07 |
| A | -9.098% | 9.219% | 40 | 0.162 | 22.50% | -22.74 | 0.000088 | 755.74 |
| B | +1.160% | 1.525% | 12 | 1.473 | 50.00% | +9.67 | 0.000087 | 243.20 |
| C | -1.372% | 2.730% | 7 | 0.523 | 42.86% | -19.59 | 0.000078 | 140.34 |

Out-of-sample filter (entries on or after 2025-09-01):

| Cell | OOS return | OOS max DD | OOS trades | OOS profit factor | OOS win rate | OOS average trade | OOS cost drag |
|---|---|---|---|---|---|---|---|
| Control | -1.549% | 1.745% | 5 | 0.114 | 20.00% | -31.65 | 101.90 |
| A | -1.429% | 1.560% | 13 | 0.353 | 38.46% | -10.13 | 237.44 |
| B | -0.716% | 1.364% | 4 | 0.478 | 50.00% | -18.24 | 81.85 |
| C | -1.557% | 1.760% | 3 | 0.117 | 33.33% | -51.99 | 59.98 |

Every OOS cell is negative. The full-year control and B are positive before the
OOS filter, but the OOS read does not support a stability claim. No cell is
called promising.

## 5. Checks

| Check | Evidence |
|---|---|
| Look-ahead in context | `future_bars_do_not_change_an_earlier_context` passes; the builder selects only bars closed at or before the execution close. |
| Look-ahead in strategy | `later_bars_do_not_change_earlier_decisions` and the runner causality tests pass. |
| Look-ahead in execution | `later_bars_do_not_change_earlier_fills` passes. |
| Event availability | Event tests assert no event appears before its availability bar. |
| Split integrity | OOS metrics use trades entered on or after 2025-09-01 from one continuous causal run. No threshold was tuned after the run. The limitation is recorded below. |
| Costs | Commission 0.1% of equity per operation, slippage 2 bps, funding 0.0001 per 8h, execution fee 0.0. Cost drag is in the results. |
| Execution realism | Next-open fills, ATR brackets, stop-first ties, zero-volume rejection, no partial fills, no order-book model. |
| Determinism | Control and A reruns are bundle-identical. |
| Capacity | Fixed 10,000 USD notional on one instrument; exposure is tiny; no market-impact model, so capacity is not established. |
| Edge cases | Empty, single candle, short history, gaps, zero volume, equal highs/lows, null axes, missing context, exact close, 4h partial/boundary, duplicate/out-of-order, non-positive price, missing ATR, invalid bracket, rejected entry/exit, and end-of-data open position are covered by the data/model/strategy/execution tests. |
| Benchmark | Context builder: 600,000 execution bars in 0.270 s (release). |

## 6. Verdict

The implementation evidence passes: the matrix completes, the control and A are
reproducible, the context is causal, and the execution path is deterministic.
The OOS evidence does not support an edge: every cell is negative after the
split. Triage is `no edge` for this sample. Keep the control and stop the
iteration loop at the pre-registered cap; no threshold change is justified.

## 7. Residual risk

- One instrument and one year; the sample is small for a regime claim.
- The OOS read filters a continuous run; it does not restart the model or
  strategy at the split.
- No independent verifier or independent model judgment was available. This
  record is a disclosed sequential pass.
- No R-multiple field exists in this delivery.
- The execution loader is still v2; the v3 schema remains a target.
- Capacity, market impact, and live execution are not tested.
