# SR2-A BTCUSD Robustness Audit

- Date: 2026-11-08 (UTC)
- Status: **Approved** (human, 2026-11-08: "approved proceed with the full implementation")
- Parent run: `STRATEGY-2026-11-08-02`
- Parent record: [`STRATEGY-2026-11-08-02-sr2-dual.md`](../../improvement/2026-11-08/STRATEGY-2026-11-08-02-sr2-dual.md)
- Audit record: [`SR2-BTC-AUDIT.md`](../../improvement/2026-11-08/SR2-BTC-AUDIT.md)
- Claim type: read-only robustness audit; no edge or profitability claim

## 1. Intent

SR2-A (1h, 1 ATR stop / 3 ATR target, fade the compression breakout) is the
first design with a positive pooled BTCUSD result: +$3,545 over 139 trades,
+35.4% over five years, profit factor 1.29-2.22 by year. The result is not
statistically significant (t = 1.53) and the top five trades dominate it. This
audit tests whether the result is robust enough to justify a new hypothesis.

## 2. Questions

1. Is the result concentrated in a few trades? What are their dates, sides, and
   regimes?
2. What is the bootstrap confidence interval for the mean trade and total P&L?
3. Does the result survive the 9 bps round-trip cost?
4. Does the result survive a 5% per-cell drawdown ceiling?
5. Which years, sides, and model conditions drive the path edge?
6. How far is the result from the 10%/month at 5% drawdown goal?

## 3. Scope

Included:

- Read-only trade and path analysis of the SR2-A BTCUSD cells 2018–2022.
- Exact 9 bps backtests for the five BTCUSD cells.
- Exact 5% drawdown backtests for the five BTCUSD cells.
- Bootstrap, concentration, top-trade, year/side, and condition tables.
- A written audit record and local analysis artifacts.

Excluded:

- A new strategy rule, filter, threshold, or parameter.
- EURUSD/XAUUSD re-runs or new instruments.
- 2024 `strategy_validation`.
- Any edge, profitability, or live-trading claim.

## 4. Protocol

| Item | Value |
|---|---|
| Subject | SR2-A BTCUSD, 1h, 1 ATR stop / 3 ATR target |
| Cells | 2018, 2019, 2020, 2021, 2022 |
| Base cost | 6 bps round trip |
| Cost sensitivity | 9 bps round trip |
| Drawdown sensitivity | 5% per cell, hard halt |
| Account | $10,000 per cell, fixed notional, 1x leverage |
| Bootstrap | 10,000 resamples, seed 0 |
| Path horizon | 20 bars |
| Data | The recorded SR2 data versions |

The 9 bps and 5% drawdown runs are **sensitivity analyses** of the SR2-A
strategy. They change only the cost or drawdown envelope and are not new
strategy hypotheses.

## 5. Acceptance criteria

- The analysis is reproducible from the recorded SR2-A bundle and the audit
  configs.
- The top-trade concentration, bootstrap interval, 9 bps result, and 5% result
  are recorded.
- The audit states whether the BTCUSD lead survives each check.
- The 10%/month gap is stated in numbers, not prose.
- The 2024 partition is not opened.
- The findings list the next research questions without running them.

## 6. Work breakdown

| Step | Work | Owner | Paths |
|---:|---|---|---|
| 1 | 5% drawdown audit configs and 9 bps run configs | Quant Developer | `config/sr2/audit/` |
| 2 | 9 bps BTCUSD cells | Quant Developer | audit `artifacts-9bps/` |
| 3 | 5% BTCUSD cells | Quant Developer | audit `artifacts-5pct/` |
| 4 | Trade, bootstrap, path, and concentration analysis | Result Analyst | `tools/improvement/sr2_btc_audit.py` |
| 5 | Audit record and AI log | Orchestrator | `docs/improvement/`, `docs/ai-transparency-log/` |

## 7. Verification

| Check | Command | Expected |
|---|---|---|
| Rust tests | `cargo test` | pass |
| Audit script test | `python3 -m unittest tools/improvement/test_sr2_btc_audit.py` | pass |
| 9 bps cells | `backtest run` | 5 bundles |
| 5% cells | `backtest run` | 5 bundles |
| Analysis | `python3 tools/improvement/sr2_btc_audit.py ...` | deterministic JSON/CSV |
| Docs | link, term, structure, changelog, workflow checks | pass |

## 8. Risks and recovery

- The bucket analysis is exploratory and multiple-comparison-prone; it generates
  hypotheses, not evidence.
- The 5% run changes the position state after a halt, so it is a full run, not a
  post-hoc equity truncation.
- The 9 bps run changes fills and therefore decisions; it is a full run.
- The sample is one instrument and five years; a positive audit does not remove
  regime risk.
- A machine restart is recoverable by re-running the small audit matrix.

## 9. Stop conditions

Stop and record the finding when:

- the 9 bps or 5% run fails the positive result;
- the bootstrap interval includes zero and the top-trade concentration is high;
- the path buckets do not identify a stable condition;
- a new hypothesis would be needed to continue.

No new strategy run begins from this audit without a new charter and human
approval.
