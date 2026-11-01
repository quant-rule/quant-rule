# Baselines

- Date: 2026-11-01 (UTC)
- Status: Initial registry; no baseline accepted yet
- Contract: [`docs/workflows/run-contract.md`](../workflows/run-contract.md)

A baseline is the accepted run that the next tier is compared against. There is
at most one accepted baseline per tier. A baseline is accepted by the human, not
by the agent that ran or reviewed the result.

## Registry

| Tier | Baseline run | Accepted by | Accepted on | Scope | Supersedes | Status |
|---|---|---|---|---|---|---|
| Model | — | — | — | — | — | None accepted |
| Strategy | — | — | — | — | — | None accepted |
| Portfolio | — | — | — | — | — | None accepted |

## Rules

- A model baseline is required before a strategy run.
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
