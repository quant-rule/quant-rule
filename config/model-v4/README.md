# M1.0 Baseline Evidence

This directory holds the accepted `M1.0` model baseline configs. It is evidence,
not a current run set.

The `run.json` files predate the P4 cleanup and may reference a historical
`../../../model-gate/shared/` directory, `strategy.json`, or `analytics.json`.
Those files are not current engine configs, and the current loader rejects the
retired strategy/analytics fields.

The current model contract is
[`docs/model/2026-11-05/model-m1.0.md`](../../docs/model/2026-11-05/model-m1.0.md).
The current example runs are under `config/examples/`.
