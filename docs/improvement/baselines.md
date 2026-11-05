# Baselines

- Date: 2026-11-05 (UTC)
- Status: Accepted model baselines: 1m 4 ATR compression expansion, 1m compression-range, and 5m fresh-onset range; the compression bracket program stopped after the barrier path diagnostic; the directional event-family program stopped at the model gate; the divergence-at-pivot program stopped at the model gate; the cross-timeframe trend-agreement program stopped at the model gate; the progression early-stage program was inconclusive (78 pre-flight events, below 200); the v4 directional strategy foundation is stopped; the session/liquidity spike stopped with no edge; the EURUSD DST-aware session spike stopped with no edge; the v5 1h swing gate stopped with no edge; the v5 Phase 2 price-only spike stopped with no edge and the volume family is blocked by data quality; the M2.x 1h direction programme stopped with no edge; no strategy or portfolio baseline
- Contract: [`docs/workflows/run-contract.md`](../workflows/run-contract.md)

A baseline is the accepted run that the next tier is compared against. There is
at most one accepted baseline per tier. A baseline is accepted by the human, not
by the agent that ran or reviewed the result.

## Registry

| Tier | Baseline run | Accepted by | Accepted on | Scope | Supersedes | Status |
|---|---|---|---|---|---|---|
| Model | `MODEL-2026-11-01-01..04` | Human | 2026-11-01 | Model correctness baseline; C axes are explicit strategy hypotheses only | — | Superseded by the model v3 gate; retained as history |
| Model | `MODEL-2026-11-03-01..02` | — | 2026-11-03 | v3 claim gate; no signal-role family passed both partitions | `MODEL-2026-11-01-01..04` | Not accepted — no edge |
| Model | `MODEL-2026-11-04-01` | — | 2026-11-04 | v4 path gate; no tested signal-role family passed `model_dev`; validation not opened | `MODEL-2026-11-03-01..02` | Not accepted — no edge |
| Model | `MODEL-2026-11-04-02` | — | 2026-11-04 | Compression-breakout range gate; outcome saturated; validation not opened | — | Not accepted — inconclusive |
| Model | `MODEL-2026-11-04-03` | — | 2026-11-04 | Harder 3 ATR compression cells; 1m positive lead, 5m placebo fail; validation not opened | — | Not accepted — no edge |
| Model | `MODEL-2026-11-04-04..05` | Human | 2026-11-04 | 1m, 4 ATR over 20 bars, BTCUSD/ETHUSD/XAUUSD; dev and validation pass | — | Accepted |
| Model | `MODEL-2026-11-04-06..07` | Human | 2026-11-04 | 1m, 5-bar compression range, next-bar boundary touch; dev and validation pass; second use of `model_validation` disclosed | — | Accepted |
| Model | `MODEL-2026-11-04-08` | — | 2026-11-04 | 5m compression range; strong lift but back-lag control fails; validation not opened | — | Not accepted — no edge |
| Model | `MODEL-2026-11-04-10..11` | Human | 2026-11-04 | 5m fresh-onset range; dev and validation pass; third use of `model_validation` disclosed | — | Accepted |
| Model | `PROGRAM-2026-11-04-01` | — | 2026-11-04 | Divergence at a confirmed pivot (regular/hidden bullish/bearish) at 15m; four cells grade C; no validation | — | Not accepted — no edge |
| Model | `PROGRAM-2026-11-04-02` | — | 2026-11-04 | Cross-timeframe 15m event with 1h trend agreement; three cells grade C; no validation | — | Not accepted — no edge |
| Model | `PROGRAM-2026-11-04-03` | — | 2026-11-04 | Early progression target (stage 0/1); pre-flight 78 events, below the 200 minimum; gate not opened | — | Not accepted — inconclusive (underpowered) |
| Model | `V4-DIRECTIONAL-STRATEGY` | — | 2026-11-04 | Directional strategy foundation; all tested cells failed or were underpowered; stopped | — | Stopped — no directional baseline |
| Model | `SESSION-LIQUIDITY-SPIKE` | — | 2026-11-04 | Fixed-UTC session spike (ORB/ORF/NCF); no primary pass; no validation | — | Not accepted — no edge |
| Model | `EURUSD-SESSION-SPIKE` | — | 2026-11-04 | DST-aware EURUSD London/NY session spike; no primary pass; no validation | — | Not accepted — no edge |
| Model | `MODEL-2026-11-04-18` | — | 2026-11-04 | V5 1h swing gate, 4 ATR / 2 ATR / 100 bars; three cells grade C; horizon explanation falsified; no validation | — | Not accepted — no edge |
| Model | `PROGRAM-2026-11-04-05` | — | 2026-11-04 | V5 Phase 2 price-only spike; three candidates no edge; volume family blocked by data quality; no validation | — | Not accepted — no edge |
| Model | `PROGRAM-2026-11-05-03` | — | 2026-11-05 | M2.x 1h panel; 10 exploratory + 5 primary cells; no primary passed; validation not opened | — | Not accepted — no edge |
| Strategy | `STRATEGY-2026-11-04-01` | — | 2026-11-04 | v9 directional 1m compression; all folds halted; no edge | — | Not accepted — no edge |
| Strategy | `STRATEGY-2026-11-04-02` | — | 2026-11-04 | v10 1m compression bracket; all folds halted; no edge | — | Not accepted — no edge |
| Strategy | `STRATEGY-2026-11-04-03` | — | 2026-11-04 | v10 5m fresh-onset bracket; all folds halted; negative gross edge | — | Not accepted — no edge |
| Strategy | `MODEL-2026-11-04-13..17` | — | 2026-11-04 | Directional event-family program at 15m (pivot break, EMA50 test, DI cross, MACD histogram, trend continuation); all grade C at the model gate; no strategy run | — | Not accepted — program stopped |
| Portfolio | — | — | — | — | — | None accepted |

## Rules

- A model correctness baseline is required before a strategy run. The v3 gate records the signal-usability grades; C measurements are diagnostic only, and a no-edge gate stops the strategy tier. The v4 path gate repeats the conclusion on the v4 event contract; it is not a baseline either. The retired axis baseline is history.
- A strategy baseline is required before a portfolio run.
- A baseline run must be green for its tier and must not appear in
  [`INVALIDATIONS.md`](INVALIDATIONS.md).
- A new accepted run supersedes the old baseline; the old row stays in the
  registry with status `superseded`.
- An invalidated baseline is marked `retired`, and the downstream tier stops
  until a replacement is accepted.
- A baseline is not a live-trading approval. It is the comparison point for the
  next run.
- The optional machine-readable working copy is `data/improvement/baselines.json`;
  this table is the committed registry. When they disagree, this table wins.

## Acceptance record

When a baseline is accepted, the registry row must name the run ID, the human
who accepted it, the UTC acceptance date, the scope (tickets, timeframes,
partitions, and any model-grade limitation), and the baseline it supersedes. The
acceptance is also written to that run's `decision.md`.
