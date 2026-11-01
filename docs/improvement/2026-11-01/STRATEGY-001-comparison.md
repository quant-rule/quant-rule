# STRATEGY-001 — First Controlled Strategy Comparison

- Date: 2026-11-01 (UTC)
- Tier: strategy
- Model baseline: `MODEL-2026-11-01-01..04` (correctness baseline, C-axis limitation)
- Portfolio: fixed baseline, frozen
- Partition: `strategy_validation` (2024)
- Verdict: `not green — no strategy baseline accepted`
- Claim type: strategy evidence; no edge claim

## Candidates

v3 `ladder-context`, v4 `level-retest`, v5 `three-timeframe`, and v6
`breakout`, on BTCUSD; v6 also on ETHUSD and XAUUSD to test stability. Costs and
portfolio are identical across runs.

## Results

| Run | Rule | Instrument | Return | Max DD | Trades | PF | Verdict |
|---|---|---|---:|---:|---:|---:|---|
| `STRATEGY-2026-11-01-01` | v3 ladder-context | BTCUSD | -9.0649% | 10.8655% | 46 | 0.478 | not green |
| `STRATEGY-2026-11-01-02` | v4 level-retest | BTCUSD | -11.1115% | 11.1115% | 43 | 0.300 | not green |
| `STRATEGY-2026-11-01-03` | v5 three-timeframe | BTCUSD | -4.5023% | 4.5023% | 21 | 0.009 | not green |
| `STRATEGY-2026-11-01-04` | v6 breakout | BTCUSD | 2.6216% | 1.8200% | 5 | 2.175 | not green |
| `STRATEGY-2026-11-01-05` | v6 breakout | ETHUSD | -2.5256% | 2.5256% | 3 | 0.000 | not green |
| `STRATEGY-2026-11-01-06` | v6 breakout | XAUUSD | -1.1897% | 1.1897% | 1 | 0.000 | not green |

## Review

- v3 and v4 lose materially with enough trades.
- v5 loses less but has only 21 trades.
- v6 is positive only on BTCUSD with 5 trades; it loses on ETHUSD and XAUUSD
  with 3 and 1 trades.
- No candidate reaches 30 trades on enough instruments or meets the return,
  drawdown, PF, and stability criteria. The positive BTCUSD v6 cell is not
  evidence of an edge.

## Decision

No strategy baseline is accepted. The next hypothesis must change the rule or
the data, not select the BTCUSD v6 cell. A v6 rule with a lower-timeframe or
higher-frequency trigger could increase the sample, but it is a new
pre-registration.

## Limitations

- Only BTCUSD has the full v3–v6 comparison; v6 is the only candidate tested on
  all three instruments.
- One OOS year; 2023 excluded.
- The model baseline has C axes; the strategy uses them as explicit hypotheses.
- No independent Quant Validator record yet.
