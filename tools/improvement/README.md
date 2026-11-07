# Improvement tooling

> Status: current. `run_current.py` invokes the S3 command surface; `run.py` is
> the historical adapter for the retired `backtest` command and is retained for
> reproducibility only. The manifest schema is `quantrule.improvement-run/v2`;
> v1 is historical and is rejected for a new run.

Python 3 standard-library tooling for the improvement run store. The scripts are
offline and deterministic: they never read the wall clock, never use randomness,
and sort every collection before writing. They record and validate evidence; the
shell runs the model, strategy, execution, portfolio, qualification, and analytics
stages.

| Script | Purpose |
|---|---|
| `validate.py <manifest>` | Check a `quantrule.improvement-run/v2` manifest |
| `index.py --root data/improvement` | Rebuild `index.json` and `index.md` from run manifests |
| `run_current.py <manifest> [--qr PATH]` | Validate, create the run folder, invoke the current S3 commands, write `results.json`, refresh the index |
| `run.py <manifest> [--qr PATH]` | Historical adapter for the retired `backtest` command; retained for reproducibility only |
| `evidence.py <run-dir>` | Write `artifacts/evidence.json` from the current model, portfolio, qualification, and analytics bundles |
| `model_confidence.py --source CSV --timeframe TF` | Axis hit rates and ordered buckets from `model-series` |
| `model_prediction.py --bundle NAME=DIR ...` | Forward/barrier model-prediction evaluation with base rate, controls, and grade |
| `cleanup.py --check` | Report retired terms and legacy run paths |
| `common.py` | Shared schema, JSON, UTC, path, and process helpers |

## Manifest

Required fields:

| Field | Rule |
|---|---|
| `schema` | `quantrule.improvement-run/v2` |
| `run_id` | `MODEL-YYYY-MM-DD-NN`, `STRATEGY-...`, or `PORTFOLIO-...`; `NN` starts at `01` |
| `tier` | `model`, `strategy`, or `portfolio`; must match the run-id prefix and `changed` |
| `hypothesis` | Non-empty string |
| `created_utc` | ISO 8601 timestamp with `Z` or `+00:00` |
| `changed` | Tier that changed; must not appear in `frozen` |
| `frozen` | List of trading tiers held fixed: `model`, `strategy`, `portfolio`; data, policy, and analytics are always frozen |
| `partition` | A key in `data/partitions.json` for the run's tier |
| `run_config` | Path to the shell run configuration |
| `code_commit` | Non-empty commit identifier |
| `seed` | Non-negative integer |
| `green_criteria` | Non-empty object recorded before the run |
| `holdout_access` | `none` or `final` |
| `strategy_config` | Required for a strategy or portfolio run; path to the strategy file |
| `qualification_criteria` | Required for a strategy or portfolio run; path to the criteria file |
| `analytics` | Optional boolean, default `true`; run `analytics evaluate` |
| `command` | Optional; `backtest run` or `report model` |

`output_root` is optional. A development partition requires
`holdout_access: none`; `portfolio_holdout` requires `final`.

`run_config` resolves relative to the manifest directory first, then the
repository root. `output_root` resolves relative to the repository root.

The index `artifacts` column is the number of files under the run's
`artifacts/` directory. A run whose `results.json` records `ok: false` shows the
verdict `failed`; otherwise the verdict comes from `results.json`, then the
`Verdict:` line in `decision.md`, then `pending`.

## Run folder

The default root is `data/improvement/<date>/<run_id>/`. `run.py` refuses a root
that already has any entry, copies `manifest.json` unchanged, copies the run
config and every sibling `*.json` into `config/`, records outside config paths in
`config/sources.json`, creates `artifacts/`, and starts `review.md` and
`decision.md` with `Verdict: pending`.

The current shell invocations are:

```text
# Model run
<qr> report model --config <run_config> --out <root>/artifacts --json

# Strategy or portfolio run
<qr> backtest run --config <run_config> --strategy <strategy_config> --out <root>/artifacts --json
<qr> qualify --evidence <root>/artifacts/backtest --criteria <qualification_criteria> --out <root>/artifacts --json
<qr> analytics evaluate --evidence <root>/artifacts/backtest --out <root>/artifacts --json
```

`results.json` records every command, return code, raw stdout and stderr, parsed
stdout JSON, and a sorted inventory of `artifacts/` with SHA-256 digests. A
non-zero exit is a failure and the index is not refreshed.

## Evidence

Strategy and portfolio tiers read `artifacts/analytics/metrics.json` when it
exists, then `artifacts/portfolio/metrics.json`, then the historical
`artifacts/portfolio/report.json`. They extract return, max drawdown, trades,
profit factor, win rate, halt state, final equity, exposure, drawdown duration,
and the available benchmark fields.

The model tier reads `artifacts/model/report.json` and reports `model_version`,
`view_contract`, `data_version`, the requested period, the ladder, and per
timeframe bars, missing bars, and coverage. It reports no confidence number the
report did not publish.

If `artifacts/qualification/qualification.json` exists, the evidence document
also records the verdict, the human decision, and the reasons.

A missing report leaves the summary fields null and adds a note.

## Model confidence

`model_confidence.py` runs:

```text
quantrule model-series <source> --timeframe <tf> --window full
```

It joins each bar with the bar exactly five series steps later for direction and
structure, and three steps later for momentum and progression. A gap is skipped,
never joined across. Direction and momentum report coverage, evaluated bars,
hits, hit rate, and `p - 1.96 * sqrt(p * (1 - p) / n)`. Structure codes and
progression stages report ordered buckets with counts, mean axis value, and mean
forward return. `--format csv` emits one tidy row per statistic.

## Term check

`cleanup.py --check` scans `docs/`, `README.md`, and `data/` for
`Continuous Improvement`, `continuous improvement`,
`Governing / Experiment Control Swarm`, `data/labs`, `data/reports`, and
`data/research`. It prints each match and exits 1. It never deletes or rewrites.
Files larger than 5 MiB and binary files are skipped.

`tools/check_terms.py` is the gate used by the repository checks. It scans
`docs/`, `README.md`, and `AGENTS.md` and excludes
`docs/ai-transparency-log/` and `docs/improvement/INVALIDATIONS.md`, which record
retired terms on purpose.

## Limitations

- `run.py` refreshes only the local `data/improvement/index.json` and `index.md`.
  Copying the committed snapshot to `docs/improvement/index.md` remains a
  separate committed step; this workstream does not modify `docs/`.
- A custom `output_root` outside `data/improvement` is recorded in its run folder
  but, by contract, is not part of the canonical index.
- The store is local data. `data/partitions.json` is ignored by the current
  `.gitignore` rule that admits only `data/**/README.md`; it is created but not
  tracked until that rule changes.
