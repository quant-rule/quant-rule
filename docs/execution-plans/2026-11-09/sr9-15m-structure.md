# SR9 15m Structure-Transition Campaign

- Date: 2026-11-09 (UTC)
- Status: **Stopped — primary cells underpowered; no candidate**
- Charter: [`SR9-CHARTER.md`](../../governance/2026-11-09/SR9-CHARTER.md)
- Predecessor: `SR9-15M-STRUCTURE` prerequisites (model states, session coverage,
  retest event)
- Claim type: strategy research; no edge claim

## 1. Intent

Test the 15m structure-transition premise on BTCUSD, XAUUSD, and EURUSD:
enter on a compression breakout confirmed by a retest that holds, filter by
confirmed structure, EMA alignment, MACD alignment, and normal volatility, and
exit when the structure flips.

## 2. Current state

- `vol_regime` and `structure_state` are strategy-readable context states.
- `signal.breakout_retest_hold` is a registered signal event.
- Session-aware coverage is available for XAUUSD/EURUSD.
- 2023 is excluded; 2024 is closed.
- No strategy or portfolio baseline is accepted.

## 3. Scope

Included:

- Six primary cells: three instruments × two sides.
- A break-only control per instrument and side.
- Per-instrument-year data and run configs for 2018–2022.
- A feasibility preflight, backtests, qualification, and a campaign record.

Excluded:

- 2024 validation.
- 9 bps sensitivity after the primary cells fail the sample floor.
- Post-hoc filter cuts, control promotion, or parameter tuning.
- 1h context.

## 4. Design

| Field | Value |
|---|---|
| Tier | strategy |
| Changed | strategy evaluation |
| Frozen | data, policy, model, execution, portfolio |
| Instruments | BTCUSD, XAUUSD, EURUSD |
| Timeframe | 15m |
| Partition | `strategy_dev` 2018–2022, per instrument-year |
| Costs | 6 bps base; 9 bps sensitivity |
| Account | $10,000; fixed notional; 1x leverage; one position |
| Drawdown | 5% hard halt |

Primary long: `signal.breakout_retest_hold` +1, `structure_state=+1`,
`ema_stack=+1`, `macd_state=+1`, `vol_regime=1`; short mirrored. Exit:
`structure_state` against the position, 1 ATR stop, 50-bar time stop, 6:1 target
backstop.

## 5. Acceptance criteria

- Each primary cell has at least 30 trades.
- PF > 1.1 at 6 bps.
- PF > 1.0 at 9 bps.
- Max drawdown ≤ 5%; no halt.
- At least 2 of 3 instruments positive per side.
- Data coverage is recorded per instrument-year.
- 2024 is not opened.

## 6. Work breakdown

| Step | Work | Owner | Paths |
|---|---|---|---|
| 1 | Charter and plan | Quant Researcher | `docs/governance/2026-11-09/`, this file |
| 2 | Configs and qualification criteria | Quant Developer | `config/sr9/`, `config/qualification/SR9-DEV.json` |
| 3 | Policy and feasibility preflight | Quant Developer | run configs, preflight |
| 4 | Base cells and break-only control | Quant Developer | `data/improvement/2026-11-09/SR9-15M-STRUCTURE/` |
| 5 | Aggregate results and stop decision | Result Analyst / Quant Validator | run record |
| 6 | Records, registry, changelog | Orchestrator | `docs/`, `CHANGELOG.md` |

## 7. Verification

- `quantrule policy check` on a run config.
- `tools/improvement/feasibility.py` on the primary strategy/data.
- `quantrule backtest run` for each instrument-year and side.
- Per-year model coverage from the backtest model report.
- Aggregate trades, net, PF, win rate, and max drawdown from the trade records.
- Repository checks and `git diff --check`.

## 8. Risks and recovery

- The retest and four filters may reduce the sample below the floor.
- BTC 15m coverage is weak in some years; XAU/EUR session coverage is mixed.
- The control may look better than the primary; it is not promoted.
- The per-year runs do not form one continuous equity curve; PF is aggregated
  from trades and drawdown is the worst year.

## 9. Stop conditions

Stop when a primary cell is underpowered or fails, data coverage is inadequate
for a cell, or the human stops the programme. Do not lower the floor, promote the
control, or open 2024.

## 10. Outcome

| Cell | Trades | Net | PF | Max DD | Win rate |
|---|---:|---:|---:|---:|---:|
| BTCUSD long | 9 | −$104.86 | 0.659 | 2.68% | 22.2% |
| BTCUSD short | 12 | +$60.87 | 1.128 | 2.55% | 25.0% |
| XAUUSD long | 16 | −$58.34 | 0.708 | 0.75% | 12.5% |
| XAUUSD short | 10 | −$31.71 | 0.737 | 0.72% | 20.0% |
| EURUSD long | 11 | −$118.02 | 0.000 | 0.78% | 0.0% |
| EURUSD short | 11 | −$107.96 | 0.000 | 0.42% | 0.0% |

All primary cells are below the 30-trade floor. Five of six have PF below 1.0;
the BTC short PF 1.128 has only 12 trades. The primary 9 bps sensitivity was not
run because the primary cells failed the sample floor at 6 bps; a higher cost
cannot make an underpowered cell testable.

Break-only control:

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
At 9 bps it has 30 trades, +$795.69, PF 1.747, max drawdown 2.87%, and 4 of 5
years positive (2020 negative); it remains a control and a lead for a future
programme, not a candidate.

Verdict: `no_candidate_underpowered`. The premise is not supported by enough
trades; this is not a clean no-edge conclusion. 2024 remains closed.
