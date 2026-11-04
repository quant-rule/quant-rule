# DI Cross 15m — Candidate Report

- Date: 2026-11-04 (UTC)
- Brief: one pre-registered 15m event family on the accepted v4 model
- Claim type: development gate report; **no edge claim, no validation claim, no
  profitability claim**
- Run record: `MODEL-2026-11-04-15` (local `data/improvement/`)

## Hypothesis

A +DI/−DI crossover marks a directional regime change. The next 20 15m bars
reach +2 ATR in the new DI direction before −1 ATR more often than the
unconditional base rate.

## Event

Type 3 (signal), subtype 2 (series crossover), source 17 (+DI), reference 18
(−DI), 15m, direction ±1. Direction is the new dominant DI side; the event is
available on the bar it occurs. The target is 2 ATR, the stop is 1 ATR, the
horizon is 20 bars, and the development gate uses `--primary-cells 5` with seed
0 and both `adx20` and `adx25`.

## Gate table

| Gate | Run | Partition | Result |
|---|---|---|---|
| Model dev | `MODEL-2026-11-04-15` | `model_dev` 2018–2020 | **Fail** — selection null; `adx25` grade C (`lift`, `lift_lcb`, `random`, `shuffle`) |
| Model validation | `MODEL-2026-11-04-20` | `model_validation` 2021–2022 | Not opened |
| Strategy dev | `STRATEGY-2026-11-04-06` | `strategy_dev` 2018–2022 | Not opened |
| Strategy validation | `STRATEGY-2026-11-04-11` | `strategy_validation` 2024 | Not opened |

## Model dev result

| Config | n | Clusters | Hit | Base | Lift | Lift LCB | Rand p | Back lag | Shuffle p | Rep | Grade |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| adx25 | 17,851 | 3,563 | 0.3121 | 0.3073 | +0.0048 | −0.0038 | 0.0796 | −0.0716 | 0.7612 | 3 | C |
| adx20 | 17,851 | 3,563 | 0.3121 | 0.3073 | +0.0048 | −0.0038 | 0.0796 | −0.0716 | 0.7612 | 3 | sensitivity |

The pass floor was a +1.00pp lift, a positive lift lower bound, random p ≤ 0.05,
and shuffle p ≤ 0.05. The event clears the count, cluster, back-lag, and
replication checks but not the lift or control checks. The `adx20` event set is
identical, so the failure is not an ADX threshold artifact.

Per instrument (`adx25`):

| Instrument | n | Hit | Base | Lift |
|---|---:|---:|---:|---:|
| BTCUSD | 6,645 | 0.3234 | 0.3166 | +0.0068 |
| ETHUSD | 5,205 | 0.3124 | 0.3105 | +0.0019 |
| XAUUSD | 6,001 | 0.2993 | 0.2943 | +0.0050 |

## Strategy table

No strategy backtest was run. The pre-registered strategy (single zero-direction
event entry, 2 ATR target, 1 ATR stop, 20-bar time stop, 4 bps commission per
operation, 2 bps slippage, no funding) remains untested because the model
development gate did not open validation.

| Partition | Instruments | Folds | Result |
|---|---|---|---|
| `strategy_dev` 2018–2022 | BTCUSD, ETHUSD, XAUUSD | 5 one-year folds each | Not run |
| `strategy_validation` 2024 | BTCUSD, ETHUSD, XAUUSD | 1 fold each | Not run |

## Verdict

**No edge.** The brief stops at the model development gate. The validation
partition is not opened, no strategy run is made, no strategy baseline exists,
and nothing here is a validated or accepted result.

## Limitations

- 15m events overlap in time; 3,563 clusters are the relevant sample, not 17,851
  independent observations.
- The gate measures a barrier outcome on the model pack, not execution fills,
  fees, or slippage. A model pass would still have required the separate
  cost-aware strategy gate.
- One data pack, three instruments, and a single pre-registered hypothesis.
- Same-session harness, review, and decision; there is no independent Quant
  Validator record.

## Next human decision

Accept the no-edge result and retire this brief, or authorize a new
pre-registered hypothesis. The event, timeframe, target, stop, horizon, filters,
and costs are not retuned under this run ID.
