# Baselines

- Date: 2026-11-05 (UTC)
- Status: Accepted model baselines: 1m 4 ATR compression expansion, 1m compression-range, and 5m fresh-onset range; the compression bracket program stopped after the barrier path diagnostic; the directional event-family program stopped at the model gate; the divergence-at-pivot program stopped at the model gate; the cross-timeframe trend-agreement program stopped at the model gate; the progression early-stage program was inconclusive (78 pre-flight events, below 200); the v4 directional strategy foundation is stopped; the session/liquidity spike stopped with no edge; the EURUSD DST-aware session spike stopped with no edge; the v5 1h swing gate stopped with no edge; the v5 Phase 2 price-only spike stopped with no edge and the volume family is blocked by data quality; the M2.x 1h direction programme stopped with no edge; the M3.x state-conditioned preflight stopped as underpowered; the M4.x location-conditioned programme stopped with no edge; the MR5.x mechanism/path grid stopped with no edge; the MR6.x ordered family-confirmation grid stopped with no edge; the MR7.x opening-range sweep/reclaim grid stopped with no edge; the SR1 compression-breakout fade stopped early as infeasible; the SR2 dual fade stopped with no edge; the SR2-A BTCUSD audit found a fragile lead; the SR3 feature-filtered fade produced no candidate; the SR4 multi-instrument stack fade found a BTC-only result; the SR5 absorption count gate stopped as underpowered; the SR6 VWAP rejection run produced no candidate; the SR7 BTC+ETH H3 cross-section produced no candidate; the SR8 2023 buffer was data-ineligible; no strategy or portfolio baseline
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
| Model | `PROGRAM-2026-11-05-04` | — | 2026-11-05 | M3 state-conditioned preflight; count gate underpowered; no development run | — | Not accepted — underpowered |
| Model | `PROGRAM-2026-11-07-01` | — | 2026-11-07 | M4.x location-conditioned directional events; five primary cells; no primary passed; validation not opened | — | Not accepted — no edge |
| Model | `PROGRAM-2026-11-08-01` | — | 2026-11-08 | MR5.x mechanism/path grid; 30 cells; no primary passed; validation not opened | — | Not accepted — no edge |
| Model | `PROGRAM-2026-11-08-02` | — | 2026-11-08 | MR6.x ordered family confirmation; 15 cells; no primary passed; reverse controls did not pass; validation not opened | — | Not accepted — no edge |
| Model | `PROGRAM-2026-11-08-03` | — | 2026-11-08 | MR7.x opening-range sweep/reclaim; 15 cells; no primary passed the hit-rate and path gates; control positive but not reversing; validation not opened | — | Not accepted — no edge |
| Strategy | `STRATEGY-2026-11-04-01` | — | 2026-11-04 | v9 directional 1m compression; all folds halted; no edge | — | Not accepted — no edge |
| Strategy | `STRATEGY-2026-11-04-02` | — | 2026-11-04 | v10 1m compression bracket; all folds halted; no edge | — | Not accepted — no edge |
| Strategy | `STRATEGY-2026-11-04-03` | — | 2026-11-04 | v10 5m fresh-onset bracket; all folds halted; negative gross edge | — | Not accepted — no edge |
| Strategy | `MODEL-2026-11-04-13..17` | — | 2026-11-04 | Directional event-family program at 15m (pivot break, EMA50 test, DI cross, MACD histogram, trend continuation); all grade C at the model gate; no strategy run | — | Not accepted — program stopped |
| Strategy | `STRATEGY-2026-11-08-01` | — | 2026-11-08 | 1m compression-breakout fade; stopped after 10/15 cells; the 4:1 payoff fails the feasibility screen; validation not opened | — | Not accepted — infeasible |
| Strategy | `STRATEGY-2026-11-08-02` | — | 2026-11-08 | Dual fade: 1h 1:3 and 15m 1:4; 30 cells; both designs failed the development criteria; 9 bps and validation not opened | — | Not accepted — no edge |
| Strategy | `SR3-FEATURES` | — | 2026-11-08 | Feature-filtered BTCUSD fade; 8 variants × 2018–2022; 1h sides positive at 6 and 9 bps but 9–21 trades per side; 15m entry negative; H4 long 5.04%; validation not opened | — | Not accepted — no candidate |
| Strategy | `SR4-STACK-FADE` | — | 2026-11-09 | Multi-instrument stack-aligned 1h fade; 90 cells; S1 pooled positive but BTC-only (1/3 instruments); BTC cells breach 5% drawdown; 9 bps and validation not opened | — | Not accepted — no candidate |
| Strategy | `SR5-ABSORPTION` | — | 2026-11-09 | 15m absorption rejection with 1h stack; count gate 5 long / 8 short, below 30 per side; no backtest; 9 bps and validation not opened | — | Not accepted — underpowered |
| Strategy | `SR6-VWAP` | — | 2026-11-09 | 15m VWAP rejection with 1h stack; 30 cells; V1 long/short negative, PF below 1.10, every cell halted; 9 bps and validation not opened | — | Not accepted — no candidate |
| Strategy | `SR7-H3-CRYPTO` | — | 2026-11-09 | BTC+ETH H3 1h fade; 20 cells; H3 long 27 trades/5.62% DD; H3 short PF 1.067, 1/5 years, ETH negative; 9 bps and validation not opened | — | Not accepted — no candidate |
| Strategy | `SR8-H3-BUFFER` | — | 2026-11-09 | 2023 BTC one-shot buffer; 1h coverage 61.3%, 0 long / 4 short trades; ineligible; 2024 not opened | — | Not accepted — buffer ineligible |
| Portfolio | — | — | — | — | — | None accepted |

The SR2-A BTCUSD audit (`SR2-BTC-AUDIT`) did not accept a baseline. The BTC lead
is concentrated in the top five trades, is not statistically significant, and
is about 0.1%/month at the 5% ceiling; see
[the audit record](2026-11-08/SR2-BTC-AUDIT.md).

The SR3 feature programme (`SR3-FEATURES`) did not accept a baseline either. The
1h stack-aligned fade is positive at 6 and 9 bps, but the sample is 9–21 trades
per side, the 15m entry loses, and H4 long breaches the 5% ceiling; see
[the programme record](2026-11-08/SR3-FEATURES.md).

The SR4 multi-instrument programme (`SR4-STACK-FADE`) did not accept a baseline.
The pooled S1 sides are positive, but EURUSD and XAUUSD are negative on every
variant and side; the result is BTC-only, and the BTC cells breach the 5%
drawdown ceiling; see [the programme record](2026-11-09/SR4-STACK-FADE.md).

The SR5 absorption programme (`SR5-ABSORPTION`) did not reach a baseline. The
count gate found 5 long and 8 short A1 events, below the 30-per-side
threshold, so no backtest was run; see
[the programme record](2026-11-09/SR5-ABSORPTION.md).

The SR6 VWAP programme (`SR6-VWAP`) did not accept a baseline. V1 long and
short are negative with PF below 1.10, and every cell breached the 5% drawdown
ceiling; see [the programme record](2026-11-09/SR6-VWAP.md).

The SR7 H3 cross-section (`SR7-H3-CRYPTO`) did not accept a baseline. H3 long
is positive in all five pooled years but has 27 trades and a 5.62% drawdown;
H3 short has PF 1.067, one positive year, and a negative ETH side; see
[the programme record](2026-11-09/SR7-H3-CRYPTO.md).

The SR8 2023 buffer (`SR8-H3-BUFFER`) was stopped as data-ineligible: 61.3% 1h
coverage, below the 95% gate. It did not accept a baseline and did not open
2024; see [the programme record](2026-11-09/SR8-H3-BUFFER.md).

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
