# SR7-H3-CRYPTO — BTC+ETH H3 Cross-Section

- Date: 2026-11-09 (UTC)
- Status: **No candidate — H3 long fails trade-count/drawdown; H3 short fails PF/years/ETH/drawdown; 2024 not opened**
- Charter: `docs/governance/2026-11-09/SR7-CHARTER.md`
- Plan: `docs/execution-plans/2026-11-09/sr7-h3-crypto.md`
- Summary: `docs/improvement/2026-11-09/SR7-H3-CRYPTO.md`
- Claim type: strategy research; no edge or profitability claim
- Record status: research-programme record; no `quantrule.improvement-run/v2`
  manifest and the cells ran outside `run.py`

## Working records

- `results.json` — 20 cells, pooled and per-instrument aggregates, artifact
  digests.
- `review.md` — Result Analyst review.
- `decision.md` — stop decision and next-hypothesis rule.
- `count-gate/count.json` — pooled H3 39 long / 42 short; pass.
- `preflight/` — 20 six-bps feasibility reports.
- `artifacts/` — 20 base bundles and qualification records.

## Results

| Variant | Trades | Net P&L | PF | Max cell DD | Halts | Positive years | Positive instruments |
|---|---:|---:|---:|---:|---:|---:|---:|
| H3 long pooled | 27 | +2,289.36 | 2.098 | 5.62% | 1 | 5/5 | 2/2 |
| H3 short pooled | 36 | +147.02 | 1.067 | 5.64% | 1 | 1/5 | 1/2 |

BTC control: long 18 +$1,619.50 PF 2.232; short 21 +$723.44 PF 1.731.
ETH: long 9 +$669.86 PF 1.870; short 15 −$576.42 PF 0.527.

No side passes. The 9 bps sensitivity and 2024 were not opened.

## Decision

No candidate, no 9 bps, no 2024. The stack-fade family is closed as a candidate;
the next programme needs a new premise and its own count and feasibility gates.
