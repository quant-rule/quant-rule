# SR8-H3-BUFFER — 2023 BTC Buffer Test

- Date: 2026-11-09 (UTC)
- Status: **Buffer ineligible — 2023 1h coverage 61.3%, below the 95% gate; 2024 not opened**
- Charter: `docs/governance/2026-11-09/SR8-CHARTER.md`
- Plan: `docs/execution-plans/2026-11-09/sr8-h3-buffer.md`
- Summary: `docs/improvement/2026-11-09/SR8-H3-BUFFER.md`
- Claim type: strategy robustness buffer; no candidate or edge claim
- Record status: research-programme record; no `quantrule.improvement-run/v2`
  manifest and the buffer is outside the formal partitions

## Working records

- `results.json` — data quality, cells, and the buffer verdict.
- `data-quality.json` — source gaps and 1h coverage.
- `review.md` — Result Analyst review.
- `decision.md` — stop decision and data-source next step.
- `preflight/` — 2 six-bps feasibility reports.
- `artifacts/`, `artifacts-9bps/` — 2 base and 2 sensitivity bundles.

## Data quality

| Check | Value | Gate | Result |
|---|---:|---:|---|
| 1m source rows | 487,479 | — | — |
| Gaps over 60s | 19,699 | — | — |
| 1h model bars | 5,354 | — | — |
| 1h missing bars | 3,378 | — | — |
| 1h coverage | 61.3% | ≥95% | fail |

## Results

| Cell | Trades | Net P&L | PF | Max DD |
|---|---:|---:|---:|---:|
| H3 long 6 bps | 0 | 0.00 | — | 0.00% |
| H3 short 6 bps | 4 | −98.78 | 0.572 | 1.84% |
| H3 long 9 bps | 0 | 0.00 | — | 0.00% |
| H3 short 9 bps | 4 | −108.71 | 0.544 | 1.90% |

The buffer is ineligible: the data coverage fails, long has no trades, and short
is below the 5-trade floor and negative. This is not a no-edge conclusion.

## Decision

No 2024 access. The H3 BTC family remains underpowered. The next step is a
complete BTC 2023 or 2014–2017 1m data source, not a new strategy variant.
