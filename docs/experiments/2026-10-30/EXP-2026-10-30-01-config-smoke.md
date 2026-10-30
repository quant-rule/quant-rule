# Experiment EXP-2026-10-30-01: Policy and Strategy Config Smoke

- Date: 2026-10-30 (UTC)
- Status: Complete
- Owner: Governing Swarm (sequential smoke pass)
- Policy: `crates/quantrule-shell/tests/fixtures/config-run/policy.json`
- Strategy: `crates/quantrule-shell/tests/fixtures/config-run/strategy.json`
- Data versions: none read; this is a configuration smoke, not a market run
- Tickets: EURUSD
- Timeframes: 15m
- In-sample: n/a
- Out-of-sample: n/a
- Costs: n/a
- Green criteria: `policy check` and `strategy explain` succeed on the valid
  fixture; a cross-config contradiction is rejected
- Iteration cap: 1
- Stop conditions: any config error on the valid fixture

## Hypothesis

The v2 policy/strategy split is loadable and explainable without reading candles.
The policy envelope and the strategy risk/reward agree; a contradictory portfolio
size is rejected before a run.

## Matrix

| Cell | Ticket | Timeframe | Data version | Config | Runs |
|---|---|---|---|---|---|
| smoke-1 | EURUSD | 15m | none | `config-run/run.json` | `policy check` |
| smoke-2 | EURUSD | 15m | none | `config-run/run.json` | `strategy explain` |
| smoke-3 | EURUSD | 15m | none | contradiction fixture | `policy check` expected failure |

## Results

| Cell | Result |
|---|---|
| smoke-1 | `policy ok run=fixture account=USD 10000 target=0.1 365d max_drawdown=0.2 risk=1 leverage=1 positions=1 tickets=EURUSD timeframes=15m strategy=axis-agreement stop_atr=2 reward_to_risk=1.5` |
| smoke-2 | Strategy thesis, long/short entry, `agreement_lost` exit, stop 2 ATR, target 3 ATR, reward-to-risk 1.5 |
| smoke-3 | Rejected at `portfolio.json $.sizing.notional` |

## Triage

No market evidence. This is a workflow/config smoke, not a trading result. The
program findings from `docs/audits/2026-10-30/mvp-pre-release-audit.md` remain
open and belong to the Planner.

## Decision

The policy/strategy/explain path is usable. Re-run market experiments only after
the audit findings that affect data/timeframe and execution are planned and
fixed.

## Limitations

No backtest was run because this worktree has no committed SOLUSD source. No
profitability or edge claim is made.
