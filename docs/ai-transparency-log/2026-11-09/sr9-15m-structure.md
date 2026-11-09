# AI Transparency Log — SR9 15m Structure Campaign

- Date: 2026-11-09 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: `sr9-15m-structure`
- Plan: [`sr9-15m-structure.md`](../../execution-plans/2026-11-09/sr9-15m-structure.md)
- Charter: [`SR9-CHARTER.md`](../../governance/2026-11-09/SR9-CHARTER.md)
- Record: [`SR9-15M-STRUCTURE.md`](../../improvement/2026-11-09/SR9-15M-STRUCTURE.md)
- Status: stopped; primary cells underpowered; no candidate; 2024 not opened
- Claim type: strategy research; no edge claim

## Intent and boundary

Run the pre-registered 15m structure-transition campaign on BTCUSD, XAUUSD, and
EURUSD over 2018–2022: six primary cells plus a break-only control. No 2024
access, no post-hoc filter changes, no control promotion.

## Implementation

- Added `config/sr9/` data, run, strategy, execution, portfolio, and policy
  configs for three instruments and five years.
- Added `config/qualification/SR9-DEV.json` with a 30-trade floor, a 5% drawdown
  ceiling, and the declared six-cell correction.
- Ran the policy check and the feasibility preflight; both passed.
- Ran 30 primary cell runs (3 instruments × 5 years × 2 sides) and 30 break-only
  control runs at 6 bps.
- Aggregated trades, net, PF, win rate, and worst-year drawdown.
- Stopped on the 30-trade floor; did not run 9 bps because the primary cells
  were underpowered at 6 bps.

## Verification and evidence

| Check | Observed |
|---|---|
| Policy check | pass |
| Feasibility preflight | pass; breakeven 16.4% |
| Primary cells | 9–16 trades each |
| Primary PF | 0.000–1.128 |
| Primary drawdown | ≤2.68% |
| Break-only BTC long | 30 trades, +$872.64, PF 1.862, DD 2.84% |
| 9 bps | not run — sample-floor stop |
| 2024 | not opened |

The full per-year results are in the local run record and the campaign summary.

## Learning

- The full retest + structure + EMA + MACD + normal-volatility conjunction is
  too restrictive for a 30-trade floor on 15m data.
- The retest event reduces trades substantially; the break-only control has more
  trades, but only BTC long is positive and it is not a candidate.
- The multi-instrument premise is not supported by the primary cells.
- The result is underpowered, not a clean no-edge conclusion.

## Rejected or corrected

- Did not lower the 30-trade floor.
- Did not promote the break-only BTC long control.
- Did not run 9 bps to manufacture a result.
- Did not change the event, filters, stop, target, or time stop.
- Did not open 2024.
- Did not claim independent Quant Validator review; the passes were sequential
  same-agent passes.

## Follow-up — 2026-11-09

- Ran the break-only BTC long control at 9 bps for 2018–2022: 30 trades,
  +$795.69, PF 1.747, max drawdown 2.87%, 4 of 5 years positive (2020 negative).
- The 9 bps control remains a control, not a candidate. A later dense Binance
  BTCUSDT cross-venue check did not reproduce it (2018–2022 65 trades, −$389.64,
  PF 0.871; 2023 8 trades, −$202.50, PF 0.229), so it is not a lead for a new
  programme.
- Corrected the committed `config/sr9/data` source paths from four parent levels
  to three so the configs rerun from `config/sr9/data`.
