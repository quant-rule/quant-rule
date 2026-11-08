# AI Transparency Log — SR2 Dual Design

- Date: 2026-11-08 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: `sr2-dual-design`
- Plan: [`sr2-dual-design.md`](../../execution-plans/2026-11-08/sr2-dual-design.md)
- Charter: [`SR2-CHARTER.md`](../../governance/2026-11-08/SR2-CHARTER.md)
- Status: development complete — `no_edge`; 9 bps and validation not opened
- Claim type: strategy research; no edge or profitability claim

## Intent and boundary

The human approved testing both feasible designs in parallel after SR1 was
stopped as structurally infeasible. This log covers the configs, the 30-cell
feasibility preflight, the parallel base run, and the aggregate. The 2024
validation remains closed.

## Implementation

- Created the SR2 model, execution, portfolio, policy, strategy, matrix, and
  qualification configs.
- Design A: 1h, 1 ATR stop, 3 ATR target. Design B: 15m, 1 ATR stop, 4 ATR
  target. Same SR1 fade entry in both.
- Set the 10% drawdown halt and the 6/9 bps cost pair.
- Added an optional `model_timeframes` ladder to `run.json`; the model computes
  only the selected timeframes while the data engine still builds the full data
  ladder. The A/B BTCUSD 2018 reduced-ladder metrics match the full-ladder
  metrics exactly, and a model unit test proves the selected timeframe is
  independent of the ladder.
- Ran the feasibility preflight for all 30 cells at the 6 bps base cost; all
  pass.
- The 9 bps preflight was also run as a diagnostic. EURUSD 2019 1h fails at
  51.3% breakeven; this is a known sensitivity risk, not a base-gate failure.
- Started both base matrices in parallel, one process per design; each design
  runs its 15 cells sequentially and is resumable. The first two launches were
  killed by OpenCode server restarts; the final launch uses `setsid nohup`,
  persistent logs, and resume-by-manifest so a server restart does not lose a
  completed cell.

## Verification and evidence

| Check | Observed |
|---|---|
| Feasibility tests | unit suite passes, including the 4:1 fail and 1:2 pass cases |
| Preflight cells | 30 / 30 pass at 6 bps |
| Base matrices | 30 / 30 cells complete |
| Design A | +0.63% pooled, PF 1.063, DD 10.06%, 541 trades, 0.31 trades/day, 1/3 instruments positive |
| Design B | -5.38% pooled, PF 0.637, DD 10.56%, 1,452 trades, 0.82 trades/day, 0/3 instruments positive |
| 9 bps sensitivity | not run; both designs failed the base criteria |
| 2024 validation | not opened |

## Learning

- The preflight gate at the **base** cost is the admission rule; the 9 bps
  sensitivity is a green criterion. Mixing the two would have falsely stopped
  the programme on a stress case.
- The two designs have the same entry and differ only in timeframe/payoff; this
  isolates the design choice and keeps the multiple-testing count at two.
- The first BTCUSD 2018 cells looked promising, but the result did not
  replicate across instruments or folds. A single cell is not evidence.
- Design A's positive pooled return is entirely BTCUSD; Design B is negative
  outside BTCUSD 2018. The entry does not have the directional edge the payoff
  needs.
- The daily activity floor also fails for both designs. A hard quota would not
  create a positive edge.
- The OpenCode server restarted twice during the first parallel attempts. A
  plain background shell does not survive that; a detached `setsid nohup`
  process with persistent logs does. This is an environment lesson, not a
  strategy result.
- The 15m/1h model baseline scope is a disclosed extension, not a promotion.
- The reduced model ladder cut each cell from minutes to seconds and made the
  run survive the environment restarts; the selected timeframe output is
  unchanged.

## Rejected or corrected

- Corrected the preflight from 9 bps to the declared 6 bps base before the
  base run; the 9 bps reports are diagnostics only.
- Did not change the entry, stop, target, timeframe, costs, or universe after
  seeing the preflight.
- Did not run the 9 bps sensitivity after both base designs failed; it would
  not change the decision.
- Did not open 2024 or claim an edge.
