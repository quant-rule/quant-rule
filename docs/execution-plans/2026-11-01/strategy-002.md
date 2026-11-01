# Execution Plan: Strategy 002 — v6 5m/15m Sample-Size Test

- Date: 2026-11-01 (UTC)
- Status: **Approved** — human approved the continuing strategy cycle.
- Work item: `strategy-002-low-timeframe`
- Prior strategy comparison: [`STRATEGY-001`](../../improvement/2026-11-01/STRATEGY-001-comparison.md)
- AI transparency log: [`docs/ai-transparency-log/2026-11-01/strategy-002.md`](../../ai-transparency-log/2026-11-01/strategy-002.md)

## 1. Intent

The 15m/1h v6 breakout produced only 5 BTCUSD trades and lost on ETHUSD and
XAUUSD. This run tests the sample-size hypothesis: a 5m/15m v6 breakout should
produce more trades while keeping the same structural stop/target rule. The v3
5m/15m rule is the control.

## 2. Scope

Included:

- v3 5m/15m control and v6 5m/15m treatment on BTCUSD, ETHUSD, and XAUUSD.
- 2024 strategy validation, same costs and portfolio baseline.
- A comparison record and decision. No model change.

Excluded:

- Portfolio runs.
- Threshold changes after the result.
- Calling a single instrument green.

## 3. Green criteria

Same as STRATEGY-001: 2024 OOS, at least two of three instruments positive,
mean return >= 10%, drawdown <= 20%, >= 30 trades per instrument, PF >= 1.2 on
positive instruments, pooled CI lower > 0, no instrument > 60% contribution.

## 4. Verification

| Check | Command | Expected |
|---|---|---|
| Config | `policy check` on each cell | `policy ok` |
| Matrix | six backtests | completed |
| Gates | fmt, clippy, test, doc, links, terms | pass |

## 5. Stop conditions

- No green candidate: record `no edge`; no threshold change.
- A program defect: route to Audit Workflow.
