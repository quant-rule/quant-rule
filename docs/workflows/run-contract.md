# Improvement Run Contract

- Date: 2026-11-01 (UTC)
- Status: Current; schema `quantrule.improvement-run/v1` is implemented by the
  run-store phase
- Scope: model, strategy, and portfolio runs
- Tooling: `tools/improvement/` (Python)

## Purpose

A run is the unit of evidence for the [Improvement Workflow](improvement-workflow.md).
This contract fixes its identity, its folder layout, its manifest, its index, its
baselines, its holdout rule, and its invalidation policy, so two runs can be
compared without reconstructing what happened from memory.

The contract is deliberately narrow. A run records one tier, one partition, one
hypothesis, and one frozen set of inputs. It does not contain a program fix, a
policy change, or a second hypothesis.

## Run identity

```text
MODEL-YYYY-MM-DD-NN
STRATEGY-YYYY-MM-DD-NN
PORTFOLIO-YYYY-MM-DD-NN
```

| Part | Rule |
|---|---|
| Tier | `MODEL`, `STRATEGY`, or `PORTFOLIO`; one tier per run |
| Date | UTC date on which the run was recorded; also the parent folder name |
| `NN` | Two-digit sequence within the tier and date, starting at `01` |
| Uniqueness | A run ID is assigned once and never reused, even after invalidation |
| Immutability | A run folder is never overwritten; a rerun is a new run ID |

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
separate run ID from the development run it tests. The comparison run is named in
`baseline` (the accepted baseline, or the frozen development candidate for a
validation run), and its config is copied into `config/`.

## Layout

Run folders live in the local workspace, not in Git:

```text
data/improvement/
  README.md                     local index of the store
  index.json                    generated machine-readable index
  index.md                      generated human-readable index
  baselines.json                optional working copy of the baseline registry
  invalidations.md              optional working copy of the invalidation log
  <YYYY-MM-DD>/
    <RUN-ID>/
      manifest.json             immutable run identity and provenance
      config/                   exact configuration copied by run.py
        sources.json            source paths and copy status
        <run config and sibling JSON files>
      artifacts/                shell bundles and evidence
        model/
        strategy/
        execution/
        portfolio/
        analytical/
        evidence.json           extracted published numbers
      results.json              command, exit code, streams, artifact digests
      review.md                 Result Analyst review; verdict starts `pending`
      decision.md               decision; verdict starts `pending`
```

The committed registry is:

```text
docs/improvement/
  README.md                     registry guide and run record template
  index.md                      committed snapshot of the generated run index
  baselines.md                  committed baseline registry
  INVALIDATIONS.md              committed invalidation tombstones
```

Run folders and generated index files under `data/improvement/` are local. The
committed `docs/improvement/` files are the durable record. When the working copy
and the committed copy disagree, the committed copy wins and the working copy is
regenerated.

`data/improvement/baselines.json` and `data/improvement/invalidations.md` are
optional working copies. The committed registries in `docs/improvement/` are the
source of truth; the working copies exist only to give the reviewer a local
machine-readable view.

A custom `output_root` is allowed for an isolated run, but it is outside the
canonical store and is not part of `data/improvement/index.json`. The evidence is
still recorded in the run folder.

## Manifest

The manifest is the run's identity. It is written once, before the first
artifact, and is not edited afterwards. The source file is staged outside the
canonical run folder; `run.py` creates the run folder and copies the manifest
into it unchanged. The schema is `quantrule.improvement-run/v1`.

| Field | Type | Required | Meaning |
|---|---|---|---|
| `schema` | string | yes | `quantrule.improvement-run/v1` |
| `run_id` | string | yes | Tier-Date-Sequence; must match the folder name |
| `tier` | string | yes | `model`, `strategy`, or `portfolio` |
| `hypothesis` | string | yes | One falsifiable sentence |
| `created_utc` | string | yes | ISO 8601 timestamp with a UTC offset, e.g. `2026-11-02T09:00:00Z` |
| `changed` | string | yes | The tier that changes; must equal `tier` |
| `frozen` | array | yes | The tiers held fixed; must not contain `changed` |
| `partition` | string | yes | A partition declared in `data/partitions.json` |
| `run_config` | string | yes | Path to the shell run configuration |
| `code_commit` | string | yes | Repository commit at run start |
| `seed` | integer | yes | Non-negative deterministic seed |
| `green_criteria` | object | yes | The pass/fail rule, recorded before the run |
| `holdout_access` | string | yes | `none` for every partition except `portfolio_holdout`; `final` for `portfolio_holdout` |
| `baseline` | string or object | no | Accepted baseline run used for comparison |
| `output_root` | string | no | Isolated output root; defaults to the canonical store |

`changed` and `frozen` are the changed/frozen contract:

| Run tier | `changed` | `frozen` |
|---|---|---|
| Model | `model` | `strategy`, `portfolio` |
| Strategy | `strategy` | `model`, `portfolio` |
| Portfolio | `portfolio` | `model`, `strategy` |

`frozen` lists the trading tiers held fixed. Data, policy, and analytics are
always frozen by the contract and do not appear in `frozen`.

The validator refuses a `changed` value that differs from `tier`, a `frozen` list
that contains `changed`, a duplicate entry, an unknown tier, an undeclared
partition, a non-UTC `created_utc`, a negative `seed`, an empty
`green_criteria`, or a holdout declaration that does not match the partition.

A manifest that marks a tier frozen must name the accepted baseline that froze it
in `baseline` or in `decision.md`, or record a fixed control in `config/`. The run
record is not valid evidence if the freeze cannot be traced.

### Example manifest

```json
{
  "schema": "quantrule.improvement-run/v1",
  "run_id": "STRATEGY-2026-11-02-01",
  "tier": "strategy",
  "hypothesis": "Level-retest entries improve cost-adjusted return over the accepted ladder-context baseline.",
  "created_utc": "2026-11-02T09:00:00Z",
  "changed": "strategy",
  "frozen": ["model", "portfolio"],
  "partition": "strategy_dev",
  "run_config": "config/examples/btcusd-2024/run.json",
  "code_commit": "42307b7",
  "seed": 0,
  "green_criteria": {
    "return_target": 0.10,
    "max_drawdown": 0.20,
    "minimum_trades": 30,
    "costs": "0.1% commission, 2 bps slippage, funding as configured",
    "rule": "validation return positive after costs and drawdown within policy"
  },
  "holdout_access": "none",
  "baseline": "STRATEGY-2026-10-31-01"
}
```

## Run execution

`run.py` is the only command that creates a run folder. It:

1. validates the manifest;
2. refuses a non-empty output root;
3. copies `manifest.json` unchanged;
4. copies the `run_config` and every sibling `*.json` into `config/`, and writes
   `config/sources.json` with the source paths and copy status;
5. creates `artifacts/` and the starting `review.md` and `decision.md` templates;
6. invokes the shell once:

   ```text
   <qr> backtest --config <run_config> --out <root>/artifacts --json
   ```

7. writes `results.json` with the command, exit code, captured stdout and
   stderr, parsed stdout JSON, and a sorted artifact inventory with SHA-256
   digests;
8. refreshes `data/improvement/index.json` and `index.md` on success.

A non-zero shell exit is a failure. The run stays in the store and the index is
not refreshed. A failure is not a partially successful run.

`evidence.py <run-dir>` then writes `artifacts/evidence.json` from the published
report. Strategy and portfolio runs extract `return` (`summary.total_return`),
`max_drawdown`, `trades`, `profit_factor`, `win_rate`, `halted`, and
`final_equity`. A model run records the model version, view contract, data
version, requested period, ladder, and per-timeframe coverage. A missing report
leaves the field null and adds a note; it is never replaced by zero.

## Index

The index is generated from the run folders. It never reads the wall clock and
never depends on filesystem iteration order.

### `index.json`

```json
{
  "schema": "quantrule.improvement-index/v1",
  "count": 1,
  "runs": [
    {
      "run_id": "STRATEGY-2026-11-02-01",
      "date": "2026-11-02",
      "tier": "strategy",
      "hypothesis": "Level-retest entries improve cost-adjusted return over the accepted ladder-context baseline.",
      "changed": "strategy",
      "frozen": "model, portfolio",
      "baseline": "STRATEGY-2026-10-31-01",
      "partition": "strategy_dev",
      "verdict": "pending",
      "artifacts": 0
    }
  ]
}
```

`verdict` is derived in this order:

1. `results.json` has a non-empty `verdict`;
2. `results.json` has `ok: false` → `failed`;
3. `decision.md` has a `Verdict:` line → that value;
4. otherwise `pending`.

`artifacts` is the number of regular files under `artifacts/`.

### `index.md`

The human-readable index uses these columns, in this order:

| run id | date | tier | hypothesis | changed | frozen | baseline | partition | verdict | artifacts |
|---|---|---|---|---|---|---|---|---|---|

Rows sort by date, then tier (`model`, `strategy`, `portfolio`), then run ID.
`index.json` and `data/improvement/index.md` are written from the same rows, so a
rerun over the same tree is byte-identical. `docs/improvement/index.md` is the
committed snapshot, refreshed from the generated index when a run is recorded.

## Baselines

A baseline is the run that the next tier is compared against. It is not a
live-trading approval, and it is not a permanent rank.

| Rule | Meaning |
|---|---|
| One baseline per tier | `model`, `strategy`, and `portfolio` each have at most one accepted baseline at a time |
| Human acceptance | The run may pass its criteria; only the human accepts it as a baseline |
| Green and valid | A failed run, an invalidated run, or a run with an open Critical finding cannot be a baseline |
| Immutable | A baseline is identified by its run ID; changing the comparison requires a new accepted run |
| Supersession | A new baseline marks the old one `superseded`; the old record remains |
| Dependency | No strategy run without a model baseline; no portfolio run without a strategy baseline |

The committed registry is [`docs/improvement/baselines.md`](../improvement/baselines.md).
The run manifest's optional `baseline` field names the comparison run, and the
index displays it. Baseline acceptance itself is recorded in the registry and in
the accepted run's `decision.md`; the current run-store index does not infer it.

The optional working copy `data/improvement/baselines.json` uses one top-level
`baselines` array. Each entry carries `tier`, `run_id`, `accepted_by`,
`accepted_on`, `scope`, `supersedes`, and `status`. A baseline acceptance writes
the same values to this file and to the committed table.

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

Every holdout access is recorded in the manifest and `decision.md`; the index
shows the `portfolio_holdout` partition for the run.

## Invalidation policy

A run is invalid when:

- its manifest, config copy, or artifact digest does not match;
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
- remains visible in the index with its verdict, and is recorded in
  [`docs/improvement/INVALIDATIONS.md`](../improvement/INVALIDATIONS.md) with
  `Date`, `Commit`, `Removed`, `Reason`, and `Superseded by`.

The invalidation log is append-only. A row is never deleted or rewritten. When
artifacts are removed in cleanup, the row is the tombstone that preserves what
was removed and why. Deleting artifacts without a tombstone is a defect.

The run store does not yet write the baseline or invalidation registries
automatically. A baseline acceptance and an invalidation are recorded in the
committed registry by the reviewer, and the run's `decision.md` carries the
matching verdict and reason.

## Tooling

The run-store tooling lives in `tools/improvement/` and is written in Python. See
[`tools/improvement/README.md`](../../tools/improvement/README.md) for the
current scripts. It does not replace the shell: the shell still executes the
backtest, and the tooling records, validates, and indexes the evidence.

| Tool | Responsibility |
|---|---|
| `validate.py <manifest>` | Validate the schema, run ID, tier, changed/frozen split, partition, UTC timestamp, seed, green criteria, and holdout declaration; reject an invalid manifest |
| `run.py <manifest> [--qr PATH]` | Validate, create the run folder, copy provenance, invoke the shell, write `results.json`, and refresh the index |
| `evidence.py <run-dir>` | Write `artifacts/evidence.json` from the published report |
| `index.py --root data/improvement` | Rebuild `index.json` and `index.md` in deterministic order |
| `model_confidence.py` | Axis hit rates and ordered buckets from `model-series` |
| `strategy_compare.py` | Compare the portfolio reports of the v3–v6 cells that exist |
| `cleanup.py --check` | Report retired terms and legacy run paths; never delete or rewrite |
| `common.py` | Shared schema, JSON, UTC, path, and process helpers |

The tooling is deterministic: it takes `created_utc` from the manifest, never the
wall clock; it sorts every collection; it is offline; it never computes a signal
or a metric that belongs to the engines.

A manifest without a recognized schema is rejected. Pre-reset experiment
records, run folders, and schemas are not imported, migrated, or read. The reset
has no backwards-compatibility layer; see the
[workflow reset ADR](../adr/2026-11-01/workflow-reset.md).

## Validation before a run is accepted

1. `python3 tools/improvement/validate.py <run-folder>/manifest.json` prints
   `ok <run_id>`.
2. The folder name, run ID, and manifest date agree.
3. `config/` contains the run config and its siblings; `sources.json` records
   any outside path.
4. `results.json` records a zero exit code and a complete artifact inventory;
   `artifacts/evidence.json` is present.
5. The partition matches the tier; a holdout run declares `final`.
6. `changed`/`frozen` matches the tier and traces each frozen tier to an accepted
   baseline.
7. A rerun with the same inputs produces byte-identical artifacts.
8. `review.md` compares against the declared baseline and names the costs,
   sample, stability, and limitations.
9. `decision.md` records the verdict and, when applicable, the human baseline
   acceptance.
10. `index.py --root data/improvement` regenerates the index without changing a
    row.

## Related documents

- [Improvement Workflow](improvement-workflow.md)
- [Audit Workflow](audit-workflow.md)
- [Improvement registry](../improvement/README.md)
- [Baseline registry](../improvement/baselines.md)
- [Invalidation log](../improvement/INVALIDATIONS.md)
- [Workflow reset execution plan](../execution-plans/2026-11-01/workflow-reset.md)
