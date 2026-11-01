# STRATEGY-003 — Level-Fade v7 Validation

- Date: 2026-11-01 (UTC)
- Tier: strategy
- Model baseline: `MODEL-2026-11-01-01..04` (correctness baseline)
- Portfolio: fixed baseline, frozen
- Partition: `strategy_validation` (2024)
- Verdict: `not green — no strategy baseline accepted`
- Claim type: strategy evidence; no edge claim

## Hypothesis

A failed test of a confirmed level is a rejection zone. Fading the failed pierce
should produce a cost-inclusive edge without a trend gate. This is a new rule
family, not another breakout variant.

## Results

| Run | Rule | Instrument | Return | Max DD | Trades | PF | Halted | Verdict |
|---|---|---|---:|---:|---:|---:|---|---|
| `STRATEGY-2026-11-01-13` | v7 level-fade | BTCUSD | −15.23% | 20.10% | 67 | 0.444 | yes | not green |
| `STRATEGY-2026-11-01-14` | v7 level-fade | ETHUSD | −18.97% | 20.21% | 40 | 0.133 | yes | not green |
| `STRATEGY-2026-11-01-15` | v7 level-fade | XAUUSD | −4.73% | 4.73% | 17 | 0.155 | no | not green |

## Review

The fade family fails on 2024 validation. BTCUSD and ETHUSD hit the drawdown
halt; XAUUSD loses with only 17 trades. Win rates are 5.9–11.9%, below every
prior candidate. The failed-level thesis is not supported on this sample.

The pre-registered stop condition is met: 2024 validation is not green, so the
2025 OOS is not run and no threshold is changed.

## Decision

No strategy baseline is accepted. The level-fade family is rejected on 2024
validation. Do not run 2025 or portfolio.

## Limitations

- One validation year; 2023 excluded.
- No 2025 OOS because validation failed.
- Same-session review, not an independent Quant Validator record.
