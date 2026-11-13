# SR5-ABSORPTION — 15m Absorption Rejection with 1h Stack Context

- Date: 2026-11-09 (UTC)
- Status: **Underpowered — count gate failed at 5 long / 8 short; no backtest; 2024 not opened**
- Charter: `docs/governance/2026-11-09/SR5-CHARTER.md`
- Plan: `docs/execution-plans/2026-11-09/sr5-absorption.md`
- Summary: `docs/improvement/2026-11-09/SR5-ABSORPTION.md`
- Claim type: strategy research; no edge or profitability claim
- Record status: research-programme record; no `quantrule.improvement-run/v2`
  manifest and the programme stopped at the count gate

## Working records

- `results.json` — count gate totals, by year, and the verdict.
- `review.md` — Result Analyst review.
- `decision.md` — stop decision and next-hypothesis rule.
- `count-gate/count.json` — the deterministic gate output.
- `count/<year>/model/` — five per-year 15m/1h model bundles.

## Count gate

| Category | Long | Short |
|---|---:|---:|
| A1 absorption rejection + 1h stack | 5 | 8 |
| A2 A1 + 1h strong ADX | 1 | 6 |
| A3 absorption rejection only | 15 | 17 |

Threshold: 30 per side. **Fail.**

The first gate filtered by type/subtype/source and also counted
`signal.absorption_break` (code 51). The corrected gate filters on event code
52; the verdict is unchanged.

A1 by year: 2018 0/0; 2019 3/3; 2020 1/1; 2021 0/0; 2022 1/4.

## Decision

No backtest, no 9 bps, no 2024. The event is too rare when filtered by the 1h
stack, and the event-only control is below the gate and clustered in 2019.
A new premise needs a new approved charter and its own count gate.
