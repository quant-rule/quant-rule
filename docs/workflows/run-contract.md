# Improvement Run Contract

- Date: 2026-11-01 (UTC)
- Updated: 2026-11-17 for the v3 research ledger and gateway-reference manifest
- Status: Current; `quantrule.improvement-run/v3` is current and `v2` is
  historical; the v3 store and schema are implemented, and the research commands
  that reach them are a #857 deliverable
- Scope: model, strategy, and portfolio runs
- Tooling: Rust evidence service for the current record; `tools/improvement/`
  (Python) is historical v2 tooling

## Purpose

A run is the unit of evidence for the [Improvement Workflow](improvement-workflow.md).
This contract fixes its identity, its pre-registration, its pipeline link, its
decision, its baselines, its holdout rule, and its invalidation rule, so two runs
can be compared without reconstructing what happened from memory.

The v3 record is a **pre-registration**, not a result. It names gateway
references and the human-owned protocol; the pipeline run and the canonical
digest are linked after submission, and the decision is recorded after review.
A v3 record never carries a model reading, a metric, a stage artifact, or a
canonical digest at creation.

## Current record (v3)

The record lives in the evidence service, in these profile-scoped tables:

| Table | Holds | Update rule |
|---|---|---|
| `research.improvement_runs` | One v3 manifest, pipeline run id, canonical digest, state, decision, note, and timestamps | Manifest immutable; pipeline link and decision recorded once |
| `research.baselines` | Accepted baseline rows from the committed registry | Append-only |
| `research.invalidations` | Invalidation tombstones from the committed log | Append-only |

The state machine is `created -> submitted -> decided`:

| State | Meaning | Write |
|---|---|---|
| `created` | The v3 pre-registration exists; no pipeline run is linked | Create once; an identical create is idempotent by profile and improvement id |
| `submitted` | One pipeline run is linked; the canonical digest may still be pending | Link the run once; record the digest once when the run produces it |
| `decided` | The human decision and note are recorded | Decide once; an `accepted` decision requires the digest |

A stage artifact is not a pipeline run and cannot be linked as one. The evidence
service accepts a pipeline `RunId`; the digest is linked separately because a
queued run does not have one at submission. A failed run that never produces a
digest can still be `rejected`, `no-edge`, `new-hypothesis`, or `invalidated`,
but it cannot be `accepted`. The baseline-link validation is a #857 deliverable.
The committed `docs/improvement/` registry remains the human-owned snapshot. The
evidence service appends a baseline or invalidation row only on an explicit human
acceptance or invalidation; a duplicate id is refused and an existing row is
never updated or deleted. The seeded baseline and invalidation rows are
historical records; they do not transfer acceptance authority to an agent.

## Run identity

```text
MODEL-YYYY-MM-DD-NN
STRATEGY-YYYY-MM-DD-NN
PORTFOLIO-YYYY-MM-DD-NN
```

| Part | Rule |
|---|---|
| Tier | `MODEL`, `STRATEGY`, or `PORTFOLIO`; one tier per run |
| Date | UTC date on which the run was pre-registered |
| `NN` | Two-digit sequence within the tier and date, starting at `01` |
| Uniqueness | An improvement id is assigned once and never reused, even after invalidation |
| Immutability | The v3 manifest is never rewritten; a rerun is a new improvement id |

`MODEL-2026-11-02-01` is the first model run recorded on 2026-11-02 (UTC).
`STRATEGY-2026-11-02-01` is a different run on the same date.

A run's partition determines what it may read:

| Tier | Partition | Use |
|---|---|---|
| `MODEL` | `model_dev` | Model development |
| `MODEL` | `model_validation` | Model confidence |
| `STRATEGY` | `strategy_dev` | Strategy candidates |
| `STRATEGY` | `strategy_validation` | One-shot strategy out-of-sample |
| `PORTFOLIO` | `portfolio_dev` | Portfolio variants |
| `PORTFOLIO` | `portfolio_holdout` | Final portfolio holdout |

The partition names and windows are declared in `data/partitions.json`; the
manifest may not cite an undeclared partition. A validation or holdout run is a
separate improvement id from the development run it tests. The comparison run is
named in `baseline` (the accepted baseline, or the frozen development candidate
for a validation run).

## Coupled run identity and model reuse

A full evidence run keeps its identity:

```text
run_id = sha256(data_version ‖ config_hash ‖ model_version ‖ image_digest)
```

A promoted run appends one tagged identity component:

```text
run_id = sha256(.. full components .. ‖ "model-artifact" ‖ model_artifact_id)
```

The full-run id is unchanged and every existing body remains valid. A promoted
run is therefore a distinct run from the same full run, while its canonical
digest can still be equal.

`run submit --model-artifact <id>` names a succeeded model stage in the active
profile. Evidence checks the stage's `data_version`, `config_hash`, and
`model_version`, records the artifact id, and the worker loads the stage manifest
and bytes instead of calling the model service. The model bytes must equal the
full-run model bundle; strategy, execution, and portfolio are recomputed. The
model artifact id is displayed by `run status` and the browser run detail; it is
an input reference, not evidence in itself.

The improvement record links the resulting pipeline `run_id` and canonical
digest after submission. It does not compute either value.

## Manifest v3

The manifest is the run's pre-registration. It is written once, before
submission, and is never edited. The schema is
`quantrule.improvement-run/v3`; v2 is historical and is refused for a new run.

| Field | Type | Required | Meaning |
|---|---|---|---|
| `schema` | string | yes | `quantrule.improvement-run/v3` |
| `improvement_id` | string | yes | `TIER-YYYY-MM-DD-NN`; the tier prefix must match `tier` |
| `tier` | string | yes | `model`, `strategy`, or `portfolio` |
| `hypothesis` | string | yes | One falsifiable sentence |
| `created_utc` | string | yes | RFC 3339 UTC, e.g. `2026-11-17T09:00:00Z` |
| `changed` | string | yes | The tier that changes; must equal `tier` |
| `frozen` | array | yes | The tiers held fixed; no `changed` and no duplicate |
| `partition` | string | yes | A declared partition from the table above |
| `data_version` | digest | yes | Gateway dataset reference, `sha256:<64 hex>` |
| `config_id` | string | yes | Gateway configuration reference, not a path |
| `criteria_id` | string | strategy/portfolio | Gateway qualification-criteria reference; optional for model |
| `green_criteria` | object | yes | Non-empty pass/fail rule, recorded before the run |
| `holdout_access` | string | yes | `none` except `final` for `portfolio_holdout` |
| `baseline` | string | no | Accepted baseline improvement id or historical run id |
| `code_commit` | string | yes | Repository commit at pre-registration |
| `seed` | integer | yes | Non-negative deterministic seed |
| `command` | string | no | Only `run submit` when present |
| `analytics` | boolean | no | Whether analytics output is part of the run |

`changed` and `frozen` are the changed/frozen contract:

| Run tier | `changed` | `frozen` |
|---|---|---|
| Model | `model` | `strategy`, `portfolio` |
| Strategy | `strategy` | `model`, `portfolio` |
| Portfolio | `portfolio` | `model`, `strategy` |

`frozen` lists the trading tiers held fixed. Data and analytics are always
frozen by the contract and do not appear in `frozen`; the policy layer is
retired and is not a run input.

The v3 validator refuses an unknown field; a v2 schema; an id whose tier prefix
or date is wrong; a `created_utc` whose calendar date differs from the date in
`improvement_id`; a `changed` value that differs from `tier`; a `frozen` list
that is not exactly the two tiers held fixed for the run (missing, extra, or
duplicate); an undeclared partition or one that belongs to another tier; a
holdout declaration that does not match the partition; a non-UTC `created_utc`;
a negative `seed`; an empty required string; an empty `green_criteria` object; a
strategy or portfolio run without `criteria_id`; a `command` other than
`run submit`; and a `config_id`, `criteria_id`, or `baseline` that looks like a
filesystem path. `data_version` is a digest, so a path cannot deserialize.

### Example manifest

```json
{
  "schema": "quantrule.improvement-run/v3",
  "improvement_id": "STRATEGY-2026-11-17-01",
  "tier": "strategy",
  "hypothesis": "Level-retest entries improve cost-adjusted return over the accepted ladder-context baseline.",
  "created_utc": "2026-11-17T09:00:00Z",
  "changed": "strategy",
  "frozen": ["model", "portfolio"],
  "partition": "strategy_dev",
  "data_version": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "config_id": "cfg_btcusd_2024_strategy",
  "criteria_id": "SR1-DEV",
  "green_criteria": {
    "return_target": 0.10,
    "max_drawdown": 0.20,
    "minimum_trades": 30,
    "rule": "validation return positive after costs and drawdown within the portfolio limit"
  },
  "holdout_access": "none",
  "baseline": "MODEL-2026-11-04-04..05",
  "code_commit": "42307b7",
  "seed": 0,
  "command": "run submit",
  "analytics": true
}
```

## Current execution flow

The gateway is the execution path: `run submit`, `run status`, and
`run fetch`. The improvement record is created before submission and linked
after the worker has produced a canonical digest.

1. Create the v3 record in the active profile. The evidence service validates the
   manifest and writes `state = created`; an identical create returns the same
   record, and a different manifest under the same id is refused.
2. Submit the pipeline run through the gateway:

   ```text
   quantrule run submit <data_version> <config_id>
   quantrule run status <run_id>
   quantrule run fetch <run_id> <root>
   ```

   A strategy or portfolio run is submitted only after its model baseline is
   accepted. A model run uses a model-only config and produces no strategy
   artifact.
3. Link the pipeline `run_id` once and move the record to `state = submitted`;
   a second link is refused. The canonical digest is still unknown for a queued
   run.
4. Record the canonical digest once when the run produces it. A digest can be
   linked while the record is `submitted` or later; it can never be changed once
   present.
5. The human records exactly one decision — `accepted`, `rejected`, `no-edge`,
   `invalidated`, or `new-hypothesis` — with a non-empty note. An `accepted`
   decision requires the digest; a non-accepted decision may be recorded without
   one. A second decision is refused.

A non-zero pipeline exit is a failure. The pipeline run remains available for
diagnosis, but the improvement record is not decided as accepted evidence.

## Baselines

A baseline is the accepted run that the next tier is compared against. It is not
a live-trading approval, and it is not a permanent rank.

| Rule | Meaning |
|---|---|
| One current baseline per tier | `model`, `strategy`, and `portfolio` each have at most one current accepted baseline; superseded rows remain as history |
| Human acceptance | The run may pass its criteria; only the human accepts it as a baseline |
| Green and valid | A failed run, an invalidated run, or a run with an open Critical finding cannot be a baseline |
| Immutable | A baseline is identified by its improvement id or historical run id; changing the comparison requires a new accepted run |
| Supersession | A new baseline marks the old one `superseded`; the old record remains |
| Dependency | No strategy run without a model baseline; no portfolio run without a strategy baseline |

The committed registry is [`docs/improvement/baselines.md`](../improvement/baselines.md).
The evidence service seeds the three accepted historical model baselines
(`MODEL-2026-11-04-04..05`, `MODEL-2026-11-04-06..07`, and
`MODEL-2026-11-04-10..11`) for the `default` profile with their committed scopes,
accepted by Human on 2026-11-04. Strategy and portfolio have no accepted
baseline. The committed registry remains the human-owned source of truth and the
append-only evidence rows mirror it; when they disagree, the committed registry
wins and the row is corrected by a new decision, never by a rewrite. The evidence
service appends a baseline row only on an explicit human acceptance; a duplicate
id is refused, and an existing row is never updated or deleted.

The manifest's optional `baseline` field names the comparison run.

## Holdout access rule

`portfolio_holdout` is the final out-of-sample window. It is opened once for the
reset.

- Only a `PORTFOLIO` run with `partition: portfolio_holdout` may set
  `holdout_access: final`.
- Every other partition must set `holdout_access: none`. The validator refuses a
  development or strategy validation run that declares `final`.
- The run config must match the declared partition. A config that reads the
  holdout window under a development partition is a contract breach and
  invalidates the run.
- The candidate is frozen before the access. No threshold, cost, split, or
  criterion may change after the holdout result is seen.
- A byte-identical rerun for determinism verification is allowed and recorded. A
  rerun with a changed config is a new candidate, not another holdout access.
- If the holdout fails, no portfolio baseline is accepted. The 2025 window is
  not reused to tune a later candidate; a new hypothesis needs a new holdout
  window and a new human decision.

Every holdout access is recorded in the manifest and in the decision note.

## Invalidation policy

A run is invalid when:

- its manifest, config, or artifact digest does not match;
- it is not reproducible byte-for-byte from the recorded inputs;
- it reads a partition outside its declaration, or declares `final` without a
  `portfolio_holdout` partition;
- a program defect is found on its path;
- a threshold, cost, split, or criterion changed after the result was seen;
- the data version is wrong, incomplete, or outside its declared window;
- the decision is missing, contradictory, or made by a role without authority;
- the human decides to retire it.

An invalidated run:

- is excluded as evidence and cannot be a baseline;
- retires any baseline that used it, and blocks the downstream tier until a new
  baseline is accepted;
- is recorded in [`docs/improvement/INVALIDATIONS.md`](../improvement/INVALIDATIONS.md)
  with `Date`, `Commit`, `Removed`, `Reason`, and `Superseded by`, and in the
  append-only `research.invalidations` table under the `default` profile.

The invalidation log is append-only. A row is never deleted or rewritten. When
artifacts are removed in cleanup, the row is the tombstone that preserves what
was removed and why. Deleting artifacts without a tombstone is a defect. The
evidence service appends a tombstone only on an explicit human invalidation; a
duplicate id is refused. The table seeds the committed tombstones; the committed
log remains the human-owned source of truth.

## Historical v2 layout and tooling

The v2 contract wrote a manifest and copied config into a local folder under
`data/improvement/<YYYY-MM-DD>/<RUN-ID>/`, with `review.md`, `decision.md`, and a
generated index. That layout, the `tools/improvement/` Python validator and
run-store scripts, and the v2 manifest with `run_config`, `strategy_config`,
`qualification_criteria`, and `output_root` paths are **historical**. They are
retained only to reproduce pre-v3 records. A v3 record does not create a local
run folder, does not copy a config, and does not use `output_root`.

The committed `docs/improvement/` files remain durable history. Do not import a
pre-reset experiment record, a historical audit, or a v2 folder as a v3 run.

## Validation before a run is accepted

1. The v3 manifest validates and the pre-registration exists once.
2. The improvement id, tier, changed/frozen split, partition, and holdout
   declaration agree.
3. `data_version`, `config_id`, and `criteria_id` are gateway references, not
   paths; a strategy or portfolio run has a criteria reference.
4. The pipeline run is linked once; the canonical digest is recorded once before
   an accepted decision.
5. The partition matches the tier; a holdout run declares `final`.
6. A rerun with the same inputs produces a byte-identical canonical digest.
7. The review compares against the declared baseline and names the costs,
   sample, stability, and limitations.
8. The human decision is recorded once with a non-empty note; an accepted
   decision carries the digest, and a non-accepted decision may omit it.
9. The committed baseline and invalidation registries agree with the evidence
   rows; a disagreement is resolved by a new human decision, not a rewrite.

## Related documents

- [Improvement Workflow](improvement-workflow.md)
- [Audit Workflow](audit-workflow.md)
- [Improvement registry](../improvement/README.md)
- [Baseline registry](../improvement/baselines.md)
- [Invalidation log](../improvement/INVALIDATIONS.md)
- [Workflow reset execution plan](../execution-plans/2026-11-01/workflow-reset.md)

## Distributed execution — v0.9.0 (prepared as v1.0.0)

An **improvement run** is a research record. A **run** is one execution of the
pipeline. The compound term is always written in full; a bare "run" always means
the pipeline execution.

From the distributed release (v0.9.0, prepared as v1.0.0) an improvement run is
produced by submitting a pipeline run:

| Field | Source |
|---|---|
| `run_id` | `sha256(data_version ‖ config_hash ‖ model_version ‖ image_digest)` |
| `data_version` | returned by the dataset upload |
| `config_hash` | computed by the gateway at submission |
| `image_digest` | reported by `/health` and recorded in provenance |
| `canonical_digest` | returned by `GET /runs/{id}/digest` |

A desktop package has no container image. Its supervisor exports the SHA-256 of
the six installed sidecars as `QR_IMAGE_DIGEST`, because a different package is a
different execution even when its canonical artifacts match. An explicit
`QR_IMAGE_DIGEST` wins, so the desktop/server parity harness can hold run identity
constant.

The v3 manifest keeps the hypothesis, tier, partition, changed/frozen split,
green criteria, references, seed, and stop rule. It carries no local artifact
path and no model measurement.

Reproducing a recorded improvement run requires the same `data_version`, the same
`config_hash`, and the same `image_digest`. A different image digest is a
different execution, even when the canonical digest matches; record it.
