# SR2-BTC-AUDIT — SR2-A BTCUSD Robustness Audit

- Date: 2026-11-08 (UTC)
- Parent run: `STRATEGY-2026-11-08-02`
- Plan: `docs/execution-plans/2026-11-08/sr2-btc-audit.md`
- Audit record: `docs/improvement/2026-11-08/SR2-BTC-AUDIT.md`
- Status: complete; fragile BTC lead; no candidate; 2024 not opened

## Scope

Read-only audit of the SR2-A BTCUSD cells (1h, 1 ATR stop / 3 ATR target,
2018–2022), plus exact 9 bps and exact 5% drawdown sensitivity runs. The audit
does not create a new strategy rule or open 2024.

## Results

| Metric | Base 6 bps | 9 bps | 5% drawdown |
|---|---:|---:|---:|
| Trades | 139 | 131 | 96 |
| Net P&L | +3,544.69 | +3,060.65 | +3,032.40 |
| Profit factor | 1.420 | 1.376 | 1.490 |
| Max cell drawdown | 10.06% | 10.37% | 6.23% |
| Halted cells | 1 | 2 | 4 |

- Bootstrap mean trade 95% CI: −$5.83 to +$60.78.
- t-stat: +1.53.
- Top five trades: 89.7% of net P&L; without top five +$364; without top ten
  −$1,511.
- 2019 is negative; 2020–2022 carry the result.
- The 5% halt is bar-close and overshoots to 6.23%; a strict 5% ceiling needs a
  lower soft halt or intra-bar control.
- The 10%/month target is ~284× the current 5-year total.

## Artifacts

- `artifacts-9bps/btc-<year>/backtest/` — exact 9 bps bundles.
- `artifacts-5pct/btc-<year>/backtest/` — exact 5% drawdown bundles.
- `analysis/analysis.json` — all metrics, bootstrap, top trades, path buckets,
  and the goal gap.
- `analysis/top_trades.csv` — the ten largest trades.
- `analysis/path_buckets.csv` — the path table by year, side, ATR, RVOL, EMA
  stack, and session.
- `config/` — the audit matrix, 5% portfolio, and run configs.
- `logs/` — command logs.

## Reproduction

```bash
python3 tools/improvement/sr2_btc_audit.py \
  --base data/improvement/2026-11-08/STRATEGY-2026-11-08-02/artifacts/A \
  --audit data/improvement/2026-11-08/SR2-BTC-AUDIT \
  --out data/improvement/2026-11-08/SR2-BTC-AUDIT/analysis
```

The 9 bps and 5% runs use the audit configs and the recorded data versions.
