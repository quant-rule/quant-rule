# SR9-15M-STRUCTURE — 15m Structure-Transition Campaign

- Date: 2026-11-09 (UTC)
- Status: **No candidate — underpowered; 2024 not opened**
- Charter: [`SR9-CHARTER.md`](../../governance/2026-11-09/SR9-CHARTER.md)
- Plan: [`sr9-15m-structure.md`](../../execution-plans/2026-11-09/sr9-15m-structure.md)
- Claim type: strategy research; no edge claim

## 1. Intent

Test a 15m structure-transition rule on BTCUSD, XAUUSD, and EURUSD: enter on a
compression breakout confirmed by a retest that holds, filter by confirmed
structure, EMA alignment, MACD alignment, and normal volatility, and exit when
the structure flips.

## 2. Data

Session-aware 15m coverage per instrument-year:

| Instrument | 2018 | 2019 | 2020 | 2021 | 2022 |
|---|---:|---:|---:|---:|---:|
| BTCUSD | 64.6% | 86.0% | 92.6% | 99.7% | 89.6% |
| XAUUSD | 94.5% | 92.8% | 95.2% | 95.3% | 95.3% |
| EURUSD | 100.0% | 99.9% | 97.9% | 92.7% | 97.9% |

BTC is below 95% in three of five years; XAU 2019 and EUR 2021 are slightly
below 95%. The campaign is therefore descriptive for those years. The primary
underpowering is present across all three instruments, so the data caveat does
not change the stop decision.

## 3. Results

Primary cells, 6 bps, 2018–2022:

| Cell | Trades | Net | PF | Max DD | Win rate |
|---|---:|---:|---:|---:|---:|
| BTCUSD long | 9 | −$104.86 | 0.659 | 2.68% | 22.2% |
| BTCUSD short | 12 | +$60.87 | 1.128 | 2.55% | 25.0% |
| XAUUSD long | 16 | −$58.34 | 0.708 | 0.75% | 12.5% |
| XAUUSD short | 10 | −$31.71 | 0.737 | 0.72% | 20.0% |
| EURUSD long | 11 | −$118.02 | 0.000 | 0.78% | 0.0% |
| EURUSD short | 11 | −$107.96 | 0.000 | 0.42% | 0.0% |

All six cells are below the 30-trade floor. Five have PF below 1.0; the BTC
short PF 1.128 has 12 trades.

Break-only control, 6 bps:

| Cell | Trades | Net | PF | Max DD | Win rate |
|---|---:|---:|---:|---:|---:|
| BTCUSD long | 30 | +$872.64 | 1.862 | 2.84% | 30.0% |
| BTCUSD short | 29 | −$538.05 | 0.554 | 4.09% | 6.9% |
| XAUUSD long | 30 | −$196.74 | 0.508 | 1.62% | 10.0% |
| XAUUSD short | 23 | −$230.97 | 0.321 | 1.81% | 13.0% |
| EURUSD long | 15 | −$72.67 | 0.430 | 0.63% | 20.0% |
| EURUSD short | 21 | −$98.86 | 0.476 | 0.80% | 14.3% |

The break-only BTC long is the only positive control with 30 trades; it is not a
candidate and is not promoted. The other controls are negative or underpowered.

Control cost sensitivity: the break-only BTC long at 9 bps has 30 trades,
+$795.69, PF 1.747, max drawdown 2.87%, and 4 of 5 years positive (2020 is
negative). It remains a control; the result is a lead for a future pre-registered
programme, not a candidate accepted here.

## 4. Criteria

| Criterion | Result |
|---|---|
| Each primary cell ≥30 trades | fail (9–16) |
| PF > 1.1 at 6 bps | fail (one 1.128, five below) |
| PF > 1.0 at 9 bps (primary) | not run — sample floor failed; break-only BTC long control 9 bps PF 1.747 |
| Max DD ≤5% | pass (all ≤2.68%) |
| No halt | pass |
| 2/3 instruments positive per side | fail |

The stop rule fires on the sample floor. The primary 9 bps sensitivity was not
run because a higher cost cannot make an underpowered cell testable.

## 5. What the result says

- **The full conjunction is too restrictive.** The retest event plus structure,
  EMA, MACD, and normal-volatility filters reduces BTC long from 30 break-only
  trades to 9 retest trades, and does not improve the aggregate.
- **The retest filter did not add enough.** The break-only control has more
  trades; only BTC long is positive, and it is a control, not a candidate.
- **The multi-instrument premise is not supported.** XAU and EUR are negative on
  both sides; BTC short is positive but underpowered.
- **This is underpowered, not no-edge.** The sample floor is not met; a
  post-hoc filter cut or control promotion would be data snooping.
- **2024 remains closed.**

## 6. Goal gap

The campaign produced no candidate. The 10%/month target is not addressed by
these cells. The next hypothesis must be a new pre-registered programme, not a
variant of this one.

## 7. Recommendation

Stop the 15m structure-transition campaign. Do not promote the break-only BTC
long control. The 9 bps control is a lead for a future pre-registered programme,
not a candidate accepted here. A future programme may test a less restrictive
entry or a different structure-transition definition, but it must declare its
cells, criteria, and out-of-sample plan before running.

## 8. Limitations

- The runs are per instrument-year; aggregate PF is computed from trades and
  drawdown is the worst year, not a single continuous equity curve.
- BTC 15m coverage is below 95% in three years; XAU/EUR have two marginal years.
- Primary 9 bps sensitivity was not run after the sample-floor stop; the break-only BTC long control was run at 9 bps.
- Quant Researcher, Quant Developer, and Quant Validator were sequential
  same-agent passes, not independent agents.

## 9. Links

- [Charter](../../governance/2026-11-09/SR9-CHARTER.md)
- [Plan](../../execution-plans/2026-11-09/sr9-15m-structure.md)
- [AI transparency log](../../ai-transparency-log/2026-11-09/sr9-15m-structure.md)
- Local evidence: `data/improvement/2026-11-09/SR9-15M-STRUCTURE/`
