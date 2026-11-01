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
