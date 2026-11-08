# SR2 — Dual Payoff/Timeframe Fade

- Date: 2026-11-08 (UTC)
- Status: **Complete — `no_edge`**; 30/30 cells; 9 bps not run; validation not opened
- Charter: [`SR2-CHARTER.md`](../../governance/2026-11-08/SR2-CHARTER.md)
- Tier: strategy
- Model baseline: `MODEL-2026-11-04-04..05`
- Claim type: strategy research; no edge or profitability claim

## 1. Intent

Test whether the SR1 fade entry becomes viable when the payoff no longer needs
a win rate above 50%. Two designs are pre-registered and run in parallel:
1h with a 1 ATR stop / 3 ATR target, and 15m with a 1 ATR stop / 4 ATR target.
The entry, side, universe, costs, and 10% drawdown ceiling are held fixed.

## 2. Current state

- SR1 stopped as infeasible after 10/15 cells; the 4:1 fade was rejected by the
  feasibility gate.
- The gate is merged and passes both SR2 designs at the 6 bps base on all 30
  cell preflights.
- At 9 bps, EURUSD 2019 1h is the tightest cell: 51.3% breakeven in the 2018
  ATR sample. The 9 bps sensitivity is a development criterion, not the
  preflight gate; a failure there is a `no_edge` result, not a reason to tune.
- The model baseline is the accepted 1m compression event. The 15m/1h use is a
  strategy extension, disclosed in the charter.

## 3. Scope

Included:

- `config/sr2/` model, execution, portfolio, policy, strategy, matrix, and
  qualification configs.
- 30 base cells: 2 designs × BTCUSD/EURUSD/XAUUSD × 2018–2022.
- Per-cell qualification; a dual-design aggregate; 9 bps sensitivity for any
  design that passes the base.
- Run record, review, decision, AI log, and registry updates.

Excluded:

- 2024 `strategy_validation`.
- Model promotion at 15m/1h.
- Any post-hoc filter, side, stop, target, timeframe, or cost change.
- Live or prop-firm claims.

## 4. Assumptions and decisions

- The entry is the SR1 fade: opposite `signal.compression_breakout`, no
  additional conditions.
- The 1 ATR stop / 3–4 ATR target is a new hypothesis, not an edit to SR1.
- Fixed $10,000 notional; the 10% drawdown ceiling is a hard halt.
- The activity floor is an average of at least one trade per trading day across
  the portfolio, plus 60% active days. It is not a per-day quota.
- The 600-trade floor remains the statistical sample floor.
- Parallel execution: one process per design, each sequential internally. The
  scripts are resumable after a restart.

## 5. Acceptance criteria

- All 30 base cells run or are explicitly stopped.
- The aggregate reports each design's pooled return, PF, drawdown, trades,
  activity, instrument/fold signs, and 9 bps PF when run.
- A design is green only when every charter criterion passes.
- The 2024 partition is not opened.
- The run record and committed summary state the verdict and limitations.

## 6. Work breakdown

| Step | Work | Owner | Paths |
|---:|---|---|---|
| 1 | Configs, policies, matrix, qualification | Quant Developer | `config/sr2/`, `config/strategy/`, `config/qualification/` |
| 1a | Optional `model_timeframes` ladder and equivalence test | Quant Developer | `crates/quantrule-shell/src/config/run.rs`, `pipeline.rs`, `crates/quantrule-model/src/run.rs` |
| 2 | 30-cell preflight at 6 bps | Quant Validator | `data/improvement/2026-11-08/SR2-DUAL/preflight/` |
| 3 | Base matrices A and B, parallel | Quant Developer | `artifacts/A/`, `artifacts/B/` |
| 4 | Aggregate and review | Result Analyst | `config/aggregate.py`, `artifacts/` |
| 5 | 9 bps sensitivity for a passing design | Quant Developer | `artifacts-9bps/` |
| 6 | Record, decision, AI log, PR | Orchestrator | `docs/improvement/`, `docs/ai-transparency-log/` |

## 7. Verification

| Check | Command | Expected |
|---|---|---|
| Rust tests | `cargo test` | pass |
| Feasibility tests | `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` | pass |
| Preflight | `feasibility.py` per cell at 6 bps | pass all 30 |
| Base run | `backtest run` per cell | 30 bundles |
| Aggregate | `python3 config/aggregate.py <run-root>` | per-design verdict |
| 9 bps | `backtest run` or execution/portfolio replay | only for a base-passing design |
| Determinism | rerun one cell per design | byte-identical |

## 8. Risks and recovery

- Two concurrent model runs use about 1 GB each; the machine has 5.7 GB. If
  memory pressure appears, run the designs sequentially; the scripts resume.
- A machine restart kills both processes; rerun the same scripts; completed
  cells are skipped.
- The 10% ceiling may halt cells early; a halt is a fail, not a reason to
  loosen it.
- The 9 bps sensitivity may fail EURUSD 2019; that is a valid no-edge result.
- The 15m/1h model baseline scope is disclosed; it is not a model promotion.
- The context-event sole-entry exception remains a limitation.

## 9. Stop conditions

Stop a design on a preflight fail, a base-criteria fail, a 9 bps fail, a
material resource failure, or a human stop. Stop the programme when both
designs fail. Do not tune a failed design.

## 10. Result

- 30 / 30 base cells completed.
- Design A (1h, 1:3): +0.63% pooled, profit factor 1.063, drawdown 10.06%,
  541 trades, 0.31 trades/day, 1/3 instruments positive.
- Design B (15m, 1:4): -5.38% pooled, profit factor 0.637, drawdown 10.56%,
  1,452 trades, 0.82 trades/day, 0/3 instruments positive.
- Both designs fail the pre-registered criteria. The 9 bps sensitivity was not
  run because the base failed; the 2024 validation was not opened.
- The run used the new `model_timeframes` subset; the reduced-ladder BTCUSD 2018
  results match the full-ladder results exactly.
