# Baselines

- Date: 2026-11-04 (UTC)
- Status: Accepted model baselines: 1m 4 ATR compression expansion and 1m compression-range next-bar resolution; v3/v4 direction, prior compression gates, v9 directional, and v10 1m bracket no edge; no strategy or portfolio baseline
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
| Strategy | `STRATEGY-2026-11-04-01` | — | 2026-11-04 | v9 directional 1m compression; all folds halted; no edge | — | Not accepted — no edge |
| Strategy | `STRATEGY-2026-11-04-02` | — | 2026-11-04 | v10 1m compression bracket; all folds halted; no edge | — | Not accepted — no edge |
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
