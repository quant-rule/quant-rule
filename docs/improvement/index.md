# Run Index

- Date: 2026-11-08 (UTC)
- Status: Committed snapshot; v3 history retained from the pre-reset store, with the v4, compression, 4 ATR, range, and strategy gates, the M2.x development swarm, the M3.x underpowered preflight, the M4.x no-edge development swarm, the MR5.x no-edge grid, the MR6.x no-edge ordered-confirmation grid, the MR7.x no-edge opening-range sweep grid, and the SR1.x stopped compression-breakout fade the SR2.x dual-design no-edge run, and the SR2-A BTCUSD robustness audit appended
- Contract: [`docs/workflows/run-contract.md`](../workflows/run-contract.md)

The local `data/improvement/` store was reset after the 2026-11-03 records, so
the committed table retains the durable v3 history rather than being replaced by
a one-run generated snapshot. New rows are appended from the run record when a
run is recorded; the generated `data/improvement/index.md` remains the working
view for the local store.

| run id | date | tier | hypothesis | changed | frozen | baseline | partition | verdict | artifacts |
|---|---|---|---|---|---|---|---|---|---|
| `MODEL-2026-11-03-01` | 2026-11-03 | model | Model gate on the development partition | model | strategy, portfolio | — | `model_dev` | not green | [record](2026-11-03/MODEL-2026-11-03-01-model-gate-dev.md) |
| `MODEL-2026-11-03-02` | 2026-11-03 | model | Model gate on the validation partition | model | strategy, portfolio | — | `model_validation` | not green | [record](2026-11-03/MODEL-2026-11-03-02-model-gate-validation.md) |
| `MODEL-2026-11-04-01` | 2026-11-04 | model | v4 path gate on the development partition | model | strategy, portfolio | — | `model_dev` | no edge | [record](2026-11-04/MODEL-2026-11-04-01-model-v4-dev.md) |
| `MODEL-2026-11-04-02` | 2026-11-04 | model | Compression-breakout range-expansion gate | model | strategy, portfolio | — | `model_dev` | inconclusive | [record](2026-11-04/MODEL-2026-11-04-02-compression-breakout.md) |
| `MODEL-2026-11-04-03` | 2026-11-04 | model | Harder 3 ATR compression cells on 5m and 1m | model | strategy, portfolio | — | `model_dev` | no edge | [record](2026-11-04/MODEL-2026-11-04-03-compression-harder.md) |
| `MODEL-2026-11-04-04` | 2026-11-04 | model | 4 ATR compression cells; 1m development pass | model | strategy, portfolio | — | `model_dev` | pass — freeze 1m | [record](2026-11-04/MODEL-2026-11-04-04-compression-4atr-dev.md) |
| `MODEL-2026-11-04-05` | 2026-11-04 | model | 4 ATR compression; 1m validation pass | model | strategy, portfolio | MODEL-2026-11-04-04 | `model_validation` | accepted | [record](2026-11-04/MODEL-2026-11-04-05-compression-4atr-validation.md) |
| `MODEL-2026-11-04-06` | 2026-11-04 | model | Compression-range next-bar gate; development pass | model | strategy, portfolio | MODEL-2026-11-04-04..05 | `model_dev` | pass — freeze event | [record](2026-11-04/MODEL-2026-11-04-06-compression-range-dev.md) |
| `MODEL-2026-11-04-07` | 2026-11-04 | model | Compression-range next-bar gate; validation pass | model | strategy, portfolio | MODEL-2026-11-04-06 | `model_validation` | accepted | [record](2026-11-04/MODEL-2026-11-04-07-compression-range-validation.md) |
| `MODEL-2026-11-04-08` | 2026-11-04 | model | Compression-range 5m next-bar gate; back-lag fail | model | strategy, portfolio | MODEL-2026-11-04-06..07 | `model_dev` | no edge | [record](2026-11-04/MODEL-2026-11-04-08-compression-range-5m-dev.md) |
| `MODEL-2026-11-04-10` | 2026-11-04 | model | Fresh-onset 5m compression gate; development pass | model | strategy, portfolio | MODEL-2026-11-04-06..07 | `model_dev` | pass — freeze onset event | [record](2026-11-04/MODEL-2026-11-04-10-compression-onset-5m-dev.md) |
| `MODEL-2026-11-04-11` | 2026-11-04 | model | Fresh-onset 5m compression gate; validation pass | model | strategy, portfolio | MODEL-2026-11-04-10 | `model_validation` | accepted | [record](2026-11-04/MODEL-2026-11-04-11-compression-onset-5m-validation.md) |
| `STRATEGY-2026-11-04-03` | 2026-11-04 | strategy | 5m fresh-onset compression bracket; all folds halted | strategy | model, portfolio | MODEL-2026-11-04-10..11 | `strategy_dev` | no edge | [record](2026-11-04/STRATEGY-2026-11-04-03-compression-onset-bracket.md) |
| `MODEL-2026-11-04-12` | 2026-11-04 | model | 5m onset barrier path; momentum and fade | model | strategy, portfolio | MODEL-2026-11-04-10..11 | `model_dev` | no directional edge | [record](2026-11-04/MODEL-2026-11-04-12-barrier-path.md) |
| `MODEL-2026-11-04-13` | 2026-11-04 | model | Major pivot break 15m; grade C | model | strategy, portfolio | — | `model_dev` | no edge | [record](2026-11-04/PIVOT-BREAK-15M-CANDIDATE.md) |
| `MODEL-2026-11-04-14` | 2026-11-04 | model | EMA50 test 15m; grade C | model | strategy, portfolio | — | `model_dev` | no edge | [record](2026-11-04/EMA50-TEST-15M-CANDIDATE.md) |
| `MODEL-2026-11-04-15` | 2026-11-04 | model | DI cross 15m; grade C | model | strategy, portfolio | — | `model_dev` | no edge | [record](2026-11-04/DI-CROSS-15M-CANDIDATE.md) |
| `MODEL-2026-11-04-16` | 2026-11-04 | model | MACD histogram flip 15m; grade C | model | strategy, portfolio | — | `model_dev` | no edge | [record](2026-11-04/MACD-HIST-15M-CANDIDATE.md) |
| `MODEL-2026-11-04-17` | 2026-11-04 | model | Trend continuation 15m; grade C | model | strategy, portfolio | — | `model_dev` | no edge | [record](2026-11-04/TREND-CONT-15M-CANDIDATE.md) |
| `PROGRAM-2026-11-04-01` | 2026-11-04 | model | Divergence at a confirmed pivot; four cells grade C | model | strategy, portfolio | — | `model_dev` | no edge | [record](2026-11-04/PROGRAM-2026-11-04-01.md) |
| `PROGRAM-2026-11-04-02` | 2026-11-04 | model | 15m event with 1h trend agreement; three cells grade C | model | strategy, portfolio | — | `model_dev` | no edge | [record](2026-11-04/PROGRAM-2026-11-04-02.md) |
| `PROGRAM-2026-11-04-03` | 2026-11-04 | model | Early progression target; pre-flight 78 events, below 200 | model | strategy, portfolio | — | `model_dev` | inconclusive | [record](2026-11-04/PROGRAM-2026-11-04-03.md) |
| `MODEL-2026-11-04-18` | 2026-11-04 | model | 1h swing gate, 100 bars, 4 ATR / 2 ATR; three cells grade C | model | strategy, portfolio | — | `model_dev` | no edge | [record](2026-11-04/V5-SWING-1H-GATE.md) |
| `PROGRAM-2026-11-05-03` | 2026-11-05 | model | M2.x 1h panel; 10 exploratory + 5 primary cells; no primary passed | model | strategy, portfolio | — | `model_dev` | no edge | [record](2026-11-05/M2-DEV-SWARM.md) |
| `PROGRAM-2026-11-05-04` | 2026-11-05 | model | M3 state-conditioned preflight; count gate underpowered; no development run | model | strategy, portfolio | — | `model_dev` | underpowered | [record](2026-11-05/M3-PREFLIGHT.md) |
| `PROGRAM-2026-11-07-01` | 2026-11-07 | model | M4.x location-conditioned directional events; 5 primary cells; no primary passed | model | strategy, portfolio | — | `model_dev` | no edge | [record](2026-11-07/M4-DEV-SWARM.md) |
| `PROGRAM-2026-11-08-01` | 2026-11-08 | model | MR5.x mechanism/path grid; 10 hypotheses × 3 configs; no primary passed | model | strategy, portfolio | — | `model_dev` | no edge | [record](2026-11-08/MR5-DEV-SWARM.md) |
| `PROGRAM-2026-11-08-02` | 2026-11-08 | model | MR6.x ordered family confirmation; 5 hypotheses × 3 configs; no primary passed | model | strategy, portfolio | — | `model_dev` | no edge | [record](2026-11-08/MR6-DEV-SWARM.md) |
| `PROGRAM-2026-11-08-03` | 2026-11-08 | model | MR7.x opening-range sweep/reclaim; 5 hypotheses × 3 configs; no primary passed | model | strategy, portfolio | — | `model_dev` | no edge | [record](2026-11-08/MR7-DEV-SWARM.md) |
| `STRATEGY-2026-11-08-01` | 2026-11-08 | strategy | 1m compression-breakout fade; 10/15 cells; stopped by the feasibility screen | strategy | model, portfolio | MODEL-2026-11-04-04..05 | `strategy_dev` | stopped — infeasible | [record](2026-11-08/STRATEGY-2026-11-08-01-sr1-fade.md) |
| `STRATEGY-2026-11-08-02` | 2026-11-08 | strategy | Dual fade: 1h 1:3 and 15m 1:4; 30 cells; no edge | strategy | model, portfolio | MODEL-2026-11-04-04..05 | `strategy_dev` | no edge | [record](2026-11-08/STRATEGY-2026-11-08-02-sr2-dual.md) |

| `PROGRAM-2026-11-04-05` | 2026-11-04 | model | V5 Phase 2 price-only spike; three candidates no edge; volume blocked | model | strategy, portfolio | — | `model_dev` | no edge | [record](2026-11-04/V5-PHASE2-SPIKE.md) |
| `STRATEGY-2026-11-04-02` | 2026-11-04 | strategy | 1m compression bracket; all folds halted | strategy | model, portfolio | MODEL-2026-11-04-06..07 | `strategy_dev` | no edge | [record](2026-11-04/STRATEGY-2026-11-04-02-compression-bracket.md) |
| `STRATEGY-2026-11-04-01` | 2026-11-04 | strategy | Directional 1m compression strategy | strategy | model, portfolio | MODEL-2026-11-04-04..05 | `strategy_dev` | no edge | [record](2026-11-04/STRATEGY-2026-11-04-01-compression-strategy.md) |
| `MODEL-2026-11-01-01` | 2026-11-01 | model | Model baseline harness pilot | model | strategy, portfolio | — | `model_validation` | not green | [record](2026-11-01/MODEL-2026-11-01-01-model-baseline.md) |
| `MODEL-2026-11-01-02` | 2026-11-01 | model | Multi-horizon axis study | model | strategy, portfolio | — | `model_validation` | not green | [record](2026-11-01/MODEL-2026-11-01-02-multi-horizon.md) |
| `MODEL-2026-11-01-03` | 2026-11-01 | model | Context-conditioned axis study | model | strategy, portfolio | — | `model_validation` | not green | [record](2026-11-01/MODEL-2026-11-01-03-context.md) |
| `MODEL-2026-11-01-04` | 2026-11-01 | model | Event-confidence study | model | strategy, portfolio | — | `model_validation` | not green | [record](2026-11-01/MODEL-2026-11-01-04-events.md) |
| `STRATEGY-2026-11-01-01` | 2026-11-01 | strategy | v3 control on BTCUSD | strategy | model, portfolio | MODEL-2026-11-01-01..04 | `strategy_validation` | not green | [comparison](2026-11-01/STRATEGY-001-comparison.md) |
| `STRATEGY-2026-11-01-02` | 2026-11-01 | strategy | v4 level-retest on BTCUSD | strategy | model, portfolio | MODEL-2026-11-01-01..04 | `strategy_validation` | not green | [comparison](2026-11-01/STRATEGY-001-comparison.md) |
| `STRATEGY-2026-11-01-03` | 2026-11-01 | strategy | v5 three-timeframe on BTCUSD | strategy | model, portfolio | MODEL-2026-11-01-01..04 | `strategy_validation` | not green | [comparison](2026-11-01/STRATEGY-001-comparison.md) |
| `STRATEGY-2026-11-01-04` | 2026-11-01 | strategy | v6 breakout on BTCUSD | strategy | model, portfolio | MODEL-2026-11-01-01..04 | `strategy_validation` | not green | [comparison](2026-11-01/STRATEGY-001-comparison.md) |
| `STRATEGY-2026-11-01-05` | 2026-11-01 | strategy | v6 breakout on ETHUSD | strategy | model, portfolio | MODEL-2026-11-01-01..04 | `strategy_validation` | not green | [comparison](2026-11-01/STRATEGY-001-comparison.md) |
| `STRATEGY-2026-11-01-06` | 2026-11-01 | strategy | v6 breakout on XAUUSD | strategy | model, portfolio | MODEL-2026-11-01-01..04 | `strategy_validation` | not green | [comparison](2026-11-01/STRATEGY-001-comparison.md) |
| `STRATEGY-2026-11-01-07` | 2026-11-01 | strategy | v3 5m/15m control BTCUSD | strategy | model, portfolio | MODEL-2026-11-01-01..04 | `strategy_validation` | not green | [comparison](2026-11-01/STRATEGY-002-comparison.md) |
| `STRATEGY-2026-11-01-08` | 2026-11-01 | strategy | v6 5m/15m BTCUSD | strategy | model, portfolio | MODEL-2026-11-01-01..04 | `strategy_validation` | not green | [comparison](2026-11-01/STRATEGY-002-comparison.md) |
| `STRATEGY-2026-11-01-09` | 2026-11-01 | strategy | v3 5m/15m control ETHUSD | strategy | model, portfolio | MODEL-2026-11-01-01..04 | `strategy_validation` | not green | [comparison](2026-11-01/STRATEGY-002-comparison.md) |
| `STRATEGY-2026-11-01-10` | 2026-11-01 | strategy | v6 5m/15m ETHUSD | strategy | model, portfolio | MODEL-2026-11-01-01..04 | `strategy_validation` | not green | [comparison](2026-11-01/STRATEGY-002-comparison.md) |
| `STRATEGY-2026-11-01-11` | 2026-11-01 | strategy | v3 5m/15m control XAUUSD | strategy | model, portfolio | MODEL-2026-11-01-01..04 | `strategy_validation` | not green | [comparison](2026-11-01/STRATEGY-002-comparison.md) |
| `STRATEGY-2026-11-01-12` | 2026-11-01 | strategy | v6 5m/15m XAUUSD | strategy | model, portfolio | MODEL-2026-11-01-01..04 | `strategy_validation` | not green | [comparison](2026-11-01/STRATEGY-002-comparison.md) |
| `STRATEGY-2026-11-01-13` | 2026-11-01 | strategy | v7 level-fade BTCUSD | strategy | model, portfolio | MODEL-2026-11-01-01..04 | `strategy_validation` | not green | [comparison](2026-11-01/STRATEGY-003-comparison.md) |
| `STRATEGY-2026-11-01-14` | 2026-11-01 | strategy | v7 level-fade ETHUSD | strategy | model, portfolio | MODEL-2026-11-01-01..04 | `strategy_validation` | not green | [comparison](2026-11-01/STRATEGY-003-comparison.md) |
| `STRATEGY-2026-11-01-15` | 2026-11-01 | strategy | v7 level-fade XAUUSD | strategy | model, portfolio | MODEL-2026-11-01-01..04 | `strategy_validation` | not green | [comparison](2026-11-01/STRATEGY-003-comparison.md) |

### Research analyses

| ID | date | subject | verdict | record |
|---|---|---|---|---|
| `SR2-BTC-AUDIT` | 2026-11-08 | SR2-A BTCUSD concentration, 9 bps, 5% drawdown, and path conditions | fragile lead; no candidate | [record](2026-11-08/SR2-BTC-AUDIT.md) |




The model runs are not green as standalone predictors; the model correctness baseline is accepted with the C-axis limitation in `baselines.md`.

Cycles: [`CYCLE-001`](2026-11-01/CYCLE-001.md), [`CYCLE-002`](2026-11-01/CYCLE-002.md), [`CYCLE-003`](2026-11-01/CYCLE-003.md), [`CYCLE-004`](2026-11-01/CYCLE-004.md), [`CYCLE-005`](2026-11-01/CYCLE-005.md), [`CYCLE-006`](2026-11-01/CYCLE-006.md), [`CYCLE-007`](2026-11-01/CYCLE-007.md), [`CYCLE-008`](2026-11-03/CYCLE-008.md), [`CYCLE-009`](2026-11-03/CYCLE-009.md), [`CYCLE-010`](2026-11-03/CYCLE-010.md), [`CYCLE-011`](2026-11-03/CYCLE-011.md), and [`CYCLE-012`](2026-11-03/CYCLE-012.md) — no model, strategy, or portfolio baseline.

## Program status

| program | date | status | record |
|---|---|---|---|
| `V4-DIRECTIONAL-STRATEGY` | 2026-11-04 | stopped — no directional baseline | [stop](2026-11-04/V4-DIRECTIONAL-STRATEGY-STOP.md) |
| `SESSION-LIQUIDITY-SPIKE` | 2026-11-04 | no edge — stopped | [record](2026-11-04/SESSION-LIQUIDITY-SPIKE.md) |
| `EURUSD-SESSION-SPIKE` | 2026-11-04 | no edge — stopped | [record](2026-11-04/EURUSD-SESSION-SPIKE.md) |
| `PROGRAM-2026-11-05-03` | 2026-11-05 | no edge — stopped | [record](2026-11-05/M2-DEV-SWARM.md) |
| `PROGRAM-2026-11-05-04` | 2026-11-05 | underpowered — stopped before the development run | [record](2026-11-05/M3-PREFLIGHT.md) |
| `PROGRAM-2026-11-07-01` | 2026-11-07 | no edge — stopped after the development swarm | [record](2026-11-07/M4-DEV-SWARM.md) |
| `PROGRAM-2026-11-08-01` | 2026-11-08 | no edge — stopped after the 30-cell grid | [record](2026-11-08/MR5-DEV-SWARM.md) |
| `PROGRAM-2026-11-08-02` | 2026-11-08 | no edge — stopped after the 15-cell ordered-confirmation grid | [record](2026-11-08/MR6-DEV-SWARM.md) |
| `PROGRAM-2026-11-08-03` | 2026-11-08 | no edge — stopped after the 15-cell opening-range sweep grid | [record](2026-11-08/MR7-DEV-SWARM.md) |

Rows sort by date, then tier (`model`, `strategy`, `portfolio`), then run ID. See
[`baselines.md`](baselines.md) for accepted baselines and
[`INVALIDATIONS.md`](INVALIDATIONS.md) for retired evidence.
