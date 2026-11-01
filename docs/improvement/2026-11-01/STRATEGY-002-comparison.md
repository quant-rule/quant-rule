# STRATEGY-002 — 5m/15m Sample-Size Test

- Date: 2026-11-01 (UTC)
- Tier: strategy
- Model baseline: `MODEL-2026-11-01-01..04` (correctness baseline)
- Portfolio: fixed baseline, frozen
- Partition: `strategy_validation` (2024)
- Verdict: `not green — no strategy baseline accepted`
- Claim type: strategy evidence; no edge claim

## Hypothesis

The 15m/1h v6 breakout was too selective. A 5m/15m v6 breakout should produce
more trades at the same structural rule; the v3 5m/15m rule is the control.

## Results

| Run | Rule | Instrument | Return | Max DD | Trades | PF | Halted | Verdict |
|---|---|---|---:|---:|---:|---:|---|---|
| `STRATEGY-2026-11-01-07` | v3 control | BTCUSD | −19.64% | 20.22% | 117 | 0.272 | yes | not green |
| `STRATEGY-2026-11-01-08` | v6 5m/15m | BTCUSD | +1.76% | 1.28% | 5 | 2.362 | no | not green |
| `STRATEGY-2026-11-01-09` | v3 control | ETHUSD | −20.05% | 20.06% | 106 | 0.235 | yes | not green |
| `STRATEGY-2026-11-01-10` | v6 5m/15m | ETHUSD | −2.02% | 2.44% | 3 | 0.000 | no | not green |
| `STRATEGY-2026-11-01-11` | v3 control | XAUUSD | −20.07% | 20.07% | 96 | 0.013 | yes | not green |
| `STRATEGY-2026-11-01-12` | v6 5m/15m | XAUUSD | 0.00% | 0.00% | 0 | — | no | not green |

## Review

- The faster execution does not increase the v6 sample: BTCUSD still produces 5
  trades, ETHUSD 3, XAUUSD 0. The 15m level context is the binding filter, not
  the execution timeframe.
- The v3 control loses about 20% and halts on every instrument; the 5m/15m
  ladder is cost-dominated.
- The v6 BTCUSD cell is positive again but has 5 trades and does not repeat.
- No candidate meets the 30-trade floor or stability criteria.

## Decision

No strategy baseline is accepted. The sample-size hypothesis is rejected. A
lower-timeframe level context or a less strict breakout is a new hypothesis; no
threshold is changed here.

## Limitations

- One OOS year; 2023 excluded.
- Same-session review, not an independent Quant Validator record.
- The v6 XAUUSD cell produced no trades.
