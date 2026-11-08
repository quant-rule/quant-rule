# AI Transparency Log — SR2-A BTCUSD Robustness Audit

- Date: 2026-11-08 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: `sr2-btc-audit`
- Plan: [`sr2-btc-audit.md`](../../execution-plans/2026-11-08/sr2-btc-audit.md)
- Audit record: [`SR2-BTC-AUDIT.md`](../../improvement/2026-11-08/SR2-BTC-AUDIT.md)
- Status: complete; fragile BTC lead; no candidate
- Claim type: read-only robustness audit; no edge or profitability claim

## Intent and boundary

The human asked whether the first positive SR2-A BTCUSD result is robust enough
to justify further research. This audit covers trade concentration, bootstrap,
exact 9 bps, exact 5% drawdown, path conditions, and the gap to the 10%/month
goal. It does not create a new strategy rule, run 2024, or claim an edge.

## Implementation

- Added five exact 9 bps BTCUSD runs and five exact 5% drawdown BTCUSD runs.
- Added `tools/improvement/sr2_btc_audit.py` and six unit tests for
  concentration, bootstrap, t-stat, percentile, path stop/target, and the goal
  gap.
- Wrote the audit record and local analysis artifacts (`analysis.json`,
  `top_trades.csv`, `path_buckets.csv`).

## Verification and evidence

| Check | Observed |
|---|---|
| 9 bps cells | 5 / 5 complete; +$3,060.65, PF 1.376, 2 halted |
| 5% cells | 5 / 5 complete; +$3,032.40, PF 1.490, 4 halted, observed max 6.23% |
| Bootstrap, seed 0, 10,000 draws | mean trade 95% CI −$5.83 to +$60.78 |
| t-stat | +1.53 |
| Top 5 share | 89.7% of net P&L |
| Path, 171 events | +0.074R, t = +0.56 |
| Unit tests | 6 / 6 pass |
| 2024 validation | not opened |

## Learning

- The positive BTC result survives 9 bps and the 5% run in sign, but it is not
  statistically significant, 90% of the P&L comes from the top five trades, and
  2019 is negative. It is a lead, not an edge.
- The 5% drawdown halt is bar-close and overshoots: observed max 6.23%. A strict
  5% ceiling needs a lower soft halt or intra-bar control.
- The condition buckets suggest quiet/high-ATR/Asian-session breakouts may be
  better fade candidates, and high-RVOL/London-session breakouts may be
  continuation. These are exploratory and must not be treated as evidence.
- The 10%/month target is 303.5× over five years; the current lead is about
  0.1%/month. The gap is not a tuning problem.
- EURUSD/XAUUSD have no edge on either side; BTC-only is the only candidate.

## Rejected or corrected

- Did not run a new filtered strategy or 2024; the audit only generates
  hypotheses.
- Did not treat the 00–06 UTC bucket as a finding; n = 20 and uncorrected.
- Did not call the 5% result compliant; the observed drawdown is 6.23%.
- Did not open 2024 or claim an edge.
