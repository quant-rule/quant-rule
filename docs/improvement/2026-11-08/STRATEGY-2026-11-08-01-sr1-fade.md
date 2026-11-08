# STRATEGY-2026-11-08-01 — SR1 Compression-Breakout Fade, Stopped Early

- Date: 2026-11-08 (UTC)
- Status: **Stopped early — infeasible; 10 of 15 cells completed; validation not opened; strategy tier closed**
- Tier: strategy
- Charter: [`SR1-CHARTER.md`](../../governance/2026-11-08/SR1-CHARTER.md)
- Plan: [`sr1-programme.md`](../../execution-plans/2026-11-08/sr1-programme.md)
- Run record: `data/improvement/2026-11-08/STRATEGY-2026-11-08-01/`
- Baseline: `MODEL-2026-11-04-04..05` (1m 4 ATR compression expansion)
- Claim type: strategy development evidence; no edge or profitability claim

## 1. Outcome

The SR1 fade was stopped after the BTCUSD and ETHUSD cells (10 of 15). Every
completed cell halted at the 15% drawdown ceiling. The structural feasibility
screen shows that the rule cannot work at 1 minute: a 4 ATR stop against a 1 ATR
target needs an 80% win rate before costs, and about 88% at 6 bps on BTCUSD. On
XAUUSD the 1 ATR target is smaller than the round-trip cost, so even a 100% win
rate cannot pay.

**The programme decision is `stopped — infeasible`; no candidate was frozen,
`strategy_validation` 2024 was not opened, and the strategy tier is closed.**

| Item | Result |
|---|---|
| Cells run | 10 / 15 (BTCUSD 2018–2022, ETHUSD 2018–2022) |
| Base cost | 6 bps round trip |
| 9 bps sensitivity | not run |
| Aggregate verdict | not run; the completed cells all halted |
| Candidate | none |
| Validation | not opened |
| Strategy tier | closed |

The completed cell artifacts, the feasibility reports, and the review are in
`data/improvement/2026-11-08/STRATEGY-2026-11-08-01/`.

## 2. Protocol

| Item | Value |
|---|---|
| Strategy | `SR1-FADE-1M`, `event_composite` |
| Event | `signal.compression_breakout` `(type 3, subtype 7, source 5)` |
| Direction match | `0` (either side) |
| Entry side | Opposite the matched event (`contrarian: true`) |
| Stop | 4 ATR |
| Target | 1 ATR (`reward_to_risk: 0.25`) |
| Time stop | 20 bars |
| Additional conditions | none |
| Instruments | BTCUSD, ETHUSD, XAUUSD |
| Partition | `strategy_dev`, 2018-01-01..2022-12-31 |
| Folds | One calendar year per run, 2018–2022 |
| Account | Accepted model-gate policy envelope; $10,000 per cell |
| Base cost | 6 bps round trip: 2 bps commission per operation + 1 bps slippage per fill |
| Sensitivity | 9 bps round trip: 3 bps commission per operation + 1.5 bps slippage per fill |
| Execution | Next execution bar, existing path |

The 6 bps base is the model-research convention and the charter's declared
round-trip cost. The legacy v9 strategy config used 4 bps per operation plus
2 bps slippage (12 bps round trip); that is not the basis here. The conversion
is recorded in the run record's `README.md` and in the matrix descriptor.

## 3. Completed cells

| Cell | Trades | Net P&L | Return | Max DD | PF | Halted |
|---|---:|---:|---:|---:|---:|---|
| BTC 2018 | 104 | -1534.58 | -0.1535 | 0.1547 | 0.3568 | yes |
| BTC 2019 | 247 | -1494.82 | -0.1495 | 0.1503 | 0.0840 | yes |
| BTC 2020 | 181 | -1501.13 | -0.1501 | 0.1505 | 0.0684 | yes |
| BTC 2021 | 125 | -1472.22 | -0.1472 | 0.1502 | 0.4352 | yes |
| BTC 2022 | 256 | -1465.90 | -0.1466 | 0.1506 | 0.2964 | yes |
| ETH 2018 | 62 | -1503.93 | -0.1504 | 0.1504 | 0.1653 | yes |
| ETH 2019 | 196 | -1490.11 | -0.1490 | 0.1500 | 0.4693 | yes |
| ETH 2020 | 237 | -1486.50 | -0.1487 | 0.1512 | 0.3914 | yes |
| ETH 2021 | 232 | -1485.50 | -0.1486 | 0.1518 | 0.6059 | yes |
| ETH 2022 | 201 | -1484.57 | -0.1485 | 0.1504 | 0.3318 | yes |

The 9 bps sensitivity and the pooled aggregate were not run. A partial aggregate
would not change the decision and would present an incomplete matrix as if it
were complete.

## 4. Feasibility preflight

The structural screen was added after the run started and applied to the
declared rule. It reads only the raw candles, the strategy risk block, and the
data config; it does not read an outcome.

| Instrument | Timeframe | Median ATR | 6 bps cost in ATR | Net target | Breakeven win rate | Verdict |
|---|---|---:|---:|---:|---:|---|
| BTCUSD | 1m | 6.60 bps | 0.909 ATR | 0.091 ATR | 98.18% | fail |
| XAUUSD | 1m | 1.79 bps | 3.347 ATR | -2.347 ATR | n/a | fail |

The screen rule is: the net target must be above zero and the breakeven win rate
must be at most 50% after the declared round-trip cost. A hypothesis that needs
a win rate above 50% is rejected before the first backtest cell. The reports are
in `data/improvement/2026-11-08/STRATEGY-2026-11-08-01/artifacts/preflight/`.

## 5. What the result says

- The failure is structural, not a missing filter or a parameter choice. The
  4:1 payoff needs a high win rate; the observed cells are far below it.
- The 1-minute cost floor makes XAUUSD and EURUSD-style instruments worse: a
  1 ATR target can be smaller than the round-trip cost.
- Moving to 5m, 15m, or 1h changes the cost/ATR ratio, but the payoff asymmetry
  remains. A new hypothesis must pass the feasibility screen before it runs.
- `signal.compression_breakout` is registered `context` and its claim is
  direction-neutral. The fade uses its direction code as a side selector; the
  approved plan is an explicit exception to the Strategy Research guidance that
  a context event may filter but not be the sole entry. The result does not
  support promoting the event to a directional `signal`.

## 6. Freeze and validation

No candidate is frozen. The 2024 `strategy_validation` partition was not opened.
A stopped or failed development result is not tuned into a candidate; a new
hypothesis requires a new plan and a new human approval.

## 7. Quant validation

The validator pass covers:

- the `contrarian` inversion and its parser/engine tests;
- the 6 bps cost identity from the execution and portfolio provenance
  (2 bps commission per operation + 1 bps slippage per fill);
- the event-snapshot causality test and the binary-search window equivalence
  test;
- the feasibility screen and its regression tests;
- the completed-cell arithmetic in the run review.

The Quant Developer and Quant Validator passes are sequential same-agent passes,
not independent agents. No byte-identical rerun was performed because the run
stopped before a candidate existed; the reproducibility criterion is not
evaluated for a stopped run.

## 8. Limitations

- The matrix is incomplete: 10 of 15 cells. The review does not claim a pooled
  result.
- The feasibility preflight was added after the run started. The run is a
  retrospective lesson, not a pre-registered screen.
- The fade is a post-hoc mirror of the failed directional strategy; the
  development evidence is not a pre-registered prior.
- The 1-minute data has gaps; `require_contiguous` skips entries after a missing
  bar. The coverage is recorded in each model bundle.
- No portfolio-tier run is opened from a stopped strategy.

## 9. Links

- [Charter](../../governance/2026-11-08/SR1-CHARTER.md)
- [Plan](../../execution-plans/2026-11-08/sr1-programme.md)
- [AI transparency log](../../ai-transparency-log/2026-11-08/sr1-programme.md)
- [Feasibility preflight](../../../tools/improvement/feasibility.py)
- [Strategy Research workflow](../../workflows/strategy-research.md)
- [Baselines](../baselines.md)
- [Run index](../index.md)
