# AI Transparency Log — SR1 Strategy Implementation and Stopped Run

- Date: 2026-11-08 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: task slug `sr1-programme`
- Plan: [`sr1-programme.md`](../../execution-plans/2026-11-08/sr1-programme.md)
- Charter: [`SR1-CHARTER.md`](../../governance/2026-11-08/SR1-CHARTER.md)
- Run record: `data/improvement/2026-11-08/STRATEGY-2026-11-08-01/`
- Status: stopped early — infeasible; 10 of 15 cells completed; validation not
  opened
- Claim type: strategy development evidence; no edge or profitability claim

## Intent and boundary

The human approved the SR1 fade premise, the `contrarian` engine addition, the
development criteria, and the 15% drawdown ceiling. This log covers the
implementation, the partial development run, and the stop decision. It does not
cover the 2024 validation, which was not opened.

## Implementation

- Added an optional `contrarian` boolean to the event-composite event condition.
  The event identity and evidence are unchanged; a `true` value takes the
  opposite side of the matched event. The default is `false`, so existing
  strategies are unchanged.
- The parser accepts `contrarian` only on an event-composite event condition and
  rejects it on a state condition and on the compression-bracket event object.
- Added engine tests for inverted long and inverted short entries.
- Added parser tests for the accepted flag and both rejection paths.
- Documented the field in `docs/config/2026-11-06/strategy-config.md`.
- Fixed the event-snapshot window selection: it now binary-searches the
  availability-sorted event batch instead of rescanning the whole batch on every
  execution bar. Behavior is unchanged; the linear scan was quadratic on the
  1-minute series.
- Built the SR1 model, execution, portfolio, policy, and 15 run configs; the
  9 bps sensitivity configs; the per-cell qualification criteria; and the
  development matrix descriptor.

## Cost-model correction

The first implementation used 4 bps commission **per operation** and 2 bps
slippage **per fill**, which is 12 bps round trip. The charter declares 6 bps
with a 9 bps sensitivity, and the model research convention (`barrier_path.py`)
applies the declared cost once per round trip. The config was corrected **before
the recorded run** to 2 bps commission per operation plus 1 bps slippage per
fill (6 bps round trip). The earlier smoke runs at 12 bps are discarded and are
not evidence.

## Feasibility screen and stop

The run started before the feasibility screen existed. After reviewing the
structural arithmetic, the screen was added:

- The 4 ATR stop / 1 ATR target needs an 80% win rate before costs.
- At 6 bps on BTCUSD 1m, the median ATR is 6.60 bps, so the cost is 0.909 ATR;
  the net target is 0.091 ATR and the breakeven win rate is 98.18%.
- On XAUUSD 1m, the median ATR is 1.79 bps; the 6 bps cost is 3.347 ATR and the
  net target is negative. Even a 100% win rate cannot pay.

The background matrix was stopped after 10 of 15 cells. Every completed cell had
already halted at the 15% drawdown ceiling. The XAUUSD cells, the 9 bps
sensitivity, and the pooled aggregate were not run. The 2024 validation was not
opened.

The new gate is `tools/improvement/feasibility.py`. It reads only the strategy
risk block, the data config, and the raw candles; it does not read an outcome.
A hypothesis passes only when the net target is positive and the breakeven win
rate is at most 50% after the declared costs, and every timeframe is at or below
1 hour.

## Verification and evidence

| Check | Observed |
|---|---|
| Engine tests | `cargo test -p quantrule-backtest -p quantrule-shell`; unit tests and 39 doctests pass |
| Strategy explain | bar 418 of the BTCUSD 2018 model: event direction `-1`, decision side `long`, stop 4 ATR, target 0.25 reward/risk |
| Development matrix | 10 of 15 cells completed (BTCUSD and ETHUSD 2018–2022); all halted |
| Per-cell qualification | 10 qualification records; per-cell drawdown ceiling and trade presence only |
| Feasibility preflight | BTCUSD 1m and XAUUSD 1m both fail; reports in `artifacts/preflight/` |
| 9 bps sensitivity | not run |
| Pooled aggregate | not run |
| Reproducibility | not evaluated for a stopped run |

The full numbers are in the run record and
`docs/improvement/2026-11-08/STRATEGY-2026-11-08-01-sr1-fade.md`.

## Learning

- The 1-minute cost floor and the 4:1 stop/target asymmetry dominate the fade.
  A 1 ATR target against a 4 ATR stop needs a target-hit rate above roughly 80%
  before costs; the completed cells are far below that.
- A hypothesis can be rejected by a five-minute arithmetic screen. Running the
  remaining cells would not have changed the decision.
- The `signal.compression_breakout` event is registered `context` and its claim
  is direction-neutral. The fade uses its direction code as a side selector,
  which is the hypothesis under test, not a directional claim inherited from the
  model. The Strategy Research workflow says a context event may filter but not
  be the sole entry; the approved plan is an explicit exception.
- The event snapshot's linear window scan was a real performance defect on a
  full 1-minute year. The fix preserves the selected events exactly and has an
  equivalence test.
- The qualification command evaluates one bundle, so the pooled multi-cell
  criteria would have been evaluated by the recorded aggregate script. The
  aggregate was not run after the stop.

## Rejected or corrected

- Corrected the cost config from 12 bps round trip to the declared 6 bps round
  trip before the recorded run.
- Rejected using the 12 bps smoke runs as evidence.
- Stopped the remaining cells after the feasibility screen failed; did not run
  XAUUSD, 9 bps, or the aggregate for a structurally infeasible rule.
- Rejected a post-hoc filter, side change, stop, target, timeframe, or cost
  change after seeing the result.
- Did not open `strategy_validation` 2024.
- Did not claim an edge or a profitability result.
