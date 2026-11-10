# Invalidations

- Date: 2026-11-01 (UTC)
- Status: Current; append-only tombstone log
- Policy: [`docs/workflows/run-contract.md`](../workflows/run-contract.md)

An invalidation records that a run, baseline, or artifact is no longer valid
evidence, and what replaced it. The row survives cleanup: when the artifacts are
deleted, this table is the tombstone that says what was removed and why.

## Rules

- Append only. Never delete, rewrite, or reorder a row.
- `Date` is the UTC date of the invalidation.
- `Commit` is the commit that records the invalidation or the removal.
- `Removed` is the run ID, baseline ID, or artifact path.
- `Reason` is specific: the defect, the mismatched input, or the superseding
  decision. "Cleanup" alone is not a reason.
- `Superseded by` is the replacement run ID, or `—` when the work stopped.
- An invalidated run cannot be a baseline. Any baseline that used it is retired
  in [`baselines.md`](baselines.md) before the row is added.
- Deleting artifacts without a row here is a defect.

## Log

| Date | Commit | Removed | Reason | Superseded by |
|---|---|---|---|---|
| 2026-11-01 | worktree | `docs/experiments/2026-10-30/EXP-2026-10-30-01`, `EXP-2026-10-30-02` | Removed; the workflow, run identity, and evidence contract changed | `MODEL-2026-11-01-01` |
| 2026-11-01 | worktree | `docs/experiments/2026-11-01/EXP-2026-11-01-03`, `EXP-2026-11-01-04`, `EXP-2026-11-01-05`, `EXP-2026-11-01-06` | Removed; the v4/v5 `level_lost` behavior changed and the run contract replaced the experiment contract | `MODEL-2026-11-01-01` |
| 2026-11-01 | worktree | `docs/audits/2026-10-30/*`, `docs/audits/2026-10-31/*` | Removed; their closure no longer describes the workflow-reset head | `docs/audits/2026-11-01/workflow-reset-audit.md` |
| 2026-11-01 | worktree | `data/labs`, `data/reports`, `data/research` | Removed; replaced by `data/improvement` | `data/improvement/2026-11-01/MODEL-2026-11-01-01` |
| 2026-11-03 | worktree | `data/improvement/2026-11-01/MODEL-2026-11-01-01..04` | Invalidated by the model v3 flat contract; the axis-based measurements are not comparable with the v3 states and events | `MODEL-2026-11-03-01..02` |
| 2026-11-03 | worktree | `data/improvement/2026-11-01/STRATEGY-2026-11-01-01..15` | Invalidated by the model v3 contract and the 10% policy; the strategies used the retired axes and a 20% drawdown envelope | `MODEL-2026-11-03-01..02` |
| 2026-11-10 | worktree | `docs/improvement/2026-11-10/SR10.md` | Invalidated by `M1.3`: the SR10 entries used the M1.2 `level.fib_test` endpoint event set; M1.3 corrects the event to a bar-range intersection with one event per tested level | — (development only; no rerun) |
| 2026-11-10 | worktree | `docs/improvement/2026-11-10/SR11.md` | Invalidated by `M1.3`: the SR11 entries used the M1.2 `level.fib_test` endpoint event set; M1.3 corrects the event to a bar-range intersection with one event per tested level | — (development only; no rerun) |

2026-11-10 note: the SR10/SR11 rows above were recorded in `d3a6b42` and merged as `68606dc`; the `worktree` value is retained as the pre-commit placeholder.
