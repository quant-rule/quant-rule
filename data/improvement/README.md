# Improvement runs

This directory is the local store for improvement runs. Git tracks this README
only; manifests, configs, artifacts, and reviews stay on the machine.

```text
data/improvement/
  index.json
  index.md
  <YYYY-MM-DD>/
    <RUN-ID>/
      manifest.json       the run contract, copied unchanged by run.py
      config/             copied run config and sibling JSON files
        sources.json      source paths and copy status
      artifacts/          shell bundles, then evidence.json
        model/
        strategy/
        execution/
        portfolio/
        analytical/
        evidence.json
      results.json        command, exit code, streams, artifact digests
      review.md           review template, verdict starts pending
      decision.md         decision template, verdict starts pending
```

## Run id

A run id is `MODEL-YYYY-MM-DD-NN`, `STRATEGY-YYYY-MM-DD-NN`, or
`PORTFOLIO-YYYY-MM-DD-NN`. The prefix names the tier the run changes. `NN` is a
two-digit sequence within that date and tier. The partition names come from
`data/partitions.json`.

A manifest follows `quantrule.improvement-run/v1` and is validated by
`tools/improvement/validate.py`. The changed tier must match the run tier and
must not appear in the frozen list. A development partition must record
`holdout_access: none`; `portfolio_holdout` must record `final`.

## Lifecycle

1. `validate.py` accepts the manifest.
2. `run.py` refuses a non-empty run folder, copies provenance, runs the shell,
   and writes `results.json`.
3. `evidence.py` extracts the published numbers into `artifacts/evidence.json`.
4. The reviewer completes `review.md` and `decision.md`; the verdict starts as
   `pending` and is replaced deliberately.
5. `index.py` rebuilds `index.json` and `index.md` from the manifests.

## Index order

Rows sort by date directory, then tier (`model`, `strategy`, `portfolio`), then
run id. Both index files are generated from the same rows, so a rerun over the
same tree is byte-identical. The index never edits a manifest.

`docs/improvement/index.md` is the committed snapshot. `index.py` writes the local
`data/improvement/index.md`; copying the snapshot into `docs/` is a separate
committed step.
