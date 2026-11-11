# Workflow Lab — Choose a Workflow and Run a Tier

- Date: 2026-11-07 (UTC)
- Status: Current
- Scope: the workflow index, the current run adapter, and the stage commands
- Claim type: tutorial; no model, strategy, or edge claim.

## 1. Choose the workflow

Read [`docs/workflows/README.md`](../../workflows/README.md). The workflow is
chosen by the decision boundary, not by the file you expect to edit. The same
guide is available in the shell:

```bash
quantrule help workflows
```

| If you want to… | Use | First command or tool |
|---|---|---|
| Find or fix a defect | Audit | audit document, then remediation plan |
| Restructure without behavior change | Refactor | refactor assessment, then plan |
| Inspect candles or timeframes | Data Lifecycle | `candles`, `candles-tolerant`, `timeframes` |
| Inspect events or the model report | Model Research | `events`, `report model` |
| Experiment with model configs or events | Model Research + Research Swarm | `report model`; `tools/research/` |
| Add or change an indicator or event | Model Change | registry, validation, verification gates |
| Test a strategy rule | Strategy Research | `strategy explain`, `strategy run`, `backtest run` |
| Test execution or portfolio mechanics | Portfolio Research | `execution run`, `portfolio replay`, `backtest run` |
| Validate the run composition | Requirements and Design | `run check` |
| Change a data source, version, or partition | Data Lifecycle | `fetch`, `data/partitions.json` |
| Add a command, report, or capability | Requirements & Design | plan first |
| Release, tag, rollback, or incident | Release & Operations | release note, `CHANGELOG.md` |
| Retire a model, event, command, or source | Deprecation & Retirement | history, invalidation |
| Record a finding, run, baseline, or status | Recordkeeping | `docs/workflows/recordkeeping.md` |
| Run a pre-registered improvement | Improvement Workflow | Run Contract |
| Make a run valid evidence | Run Contract | manifest, results, review, decision |
| Run parallel pre-registered cells | Research Swarm | coordinator + cell agents |

### Model research and the swarm

There is **no `swarm` shell command**. The swarm is an operating procedure: a
coordinator creates pre-registered cells, and each cell is one hypothesis, one
config, one worktree or output directory, and one result.

```text
Program Charter -> cells on model_dev -> aggregate -> human promotes
-> model_validation once -> grade A/B/C/D -> baseline or no_edge
```

- The charter is [`program-charter-template.md`](../../governance/2026-11-04/program-charter-template.md).
- The coordinator checks the program with `tools/research/program.py --check`.
- Config-only cells can use `tools/research/batch.py --phase dev --mode config`;
  code-bearing cells get one worktree per cell.
- `tools/research/report.py` aggregates; `tools/research/coverage.py` checks
  indicator/event coverage.
- A cell runs `report model` (or the research cell runner); the promoted cell
  runs `model_validation` once.
- `tools/research/README.md` records the current M4.x research path and the
  retained M2.x/M3.x records. The M4.x campaign refreshed the run configs to the
  current S3 sibling schemas and added the inherited-coverage check.
- The Run Contract records each run; the human accepts a baseline or records
  `no_edge`.

This lab runs a strategy tier on the shell fixtures. It is a command and artifact
lab, not a research claim.

## 2. Run the stage commands directly

The stage commands are independently runnable. The fixture model bundle is
created first:

```bash
quantrule report model \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/workflow-lab
```

The strategy stage reads the model bundle and writes decisions:

```bash
quantrule strategy run \
  --model target/workflow-lab/model \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/workflow-lab
```

The coupled run computes the model once and interleaves strategy, execution, and
portfolio:

```bash
quantrule backtest run \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/workflow-lab
```

Qualification evaluates the evidence against the human-owned criteria:

```bash
quantrule qualify \
  --evidence target/workflow-lab/backtest \
  --criteria crates/quantrule-shell/tests/fixtures/qualification/pass.json \
  --out target/workflow-lab
```

Analytics publishes descriptive metrics and tables:

```bash
quantrule analytics evaluate \
  --evidence target/workflow-lab/backtest \
  --out target/workflow-lab
```

The qualification record writes `human_decision: null`. The human accepts or
rejects in a separate `decision.md`.

## 3. Run the current improvement adapter

The adapter creates the run folder, copies the provenance, invokes the current
commands, and writes `results.json`:

```bash
python3 tools/improvement/run_current.py \
  /tmp/opencode/workflow-run-test-manifest.json \
  --qr target/debug/quantrule
```

A minimal v2 manifest:

```json
{
  "schema": "quantrule.improvement-run/v2",
  "run_id": "STRATEGY-2026-11-07-01",
  "tier": "strategy",
  "hypothesis": "The fixture strategy produces a reproducible decision and portfolio bundle.",
  "created_utc": "2026-11-07T12:00:00Z",
  "changed": "strategy",
  "frozen": ["model", "portfolio"],
  "partition": "strategy_dev",
  "run_config": "crates/quantrule-shell/tests/fixtures/config-run/run.json",
  "strategy_config": "crates/quantrule-shell/tests/fixtures/strategy/event-composite.json",
  "qualification_criteria": "crates/quantrule-shell/tests/fixtures/qualification/pass.json",
  "command": "backtest run",
  "analytics": true,
  "code_commit": "209530b",
  "seed": 0,
  "green_criteria": {"minimum_trades": 1},
  "holdout_access": "none",
  "output_root": "/tmp/opencode/workflow-run-test"
}
```

The run folder contains:

```text
manifest.json
config/
artifacts/
  backtest/
  qualification/
  analytics/
  evidence.json
results.json
review.md
decision.md
```

`results.json` records every command, its exit code, the captured streams, and
the artifact inventory. `evidence.py` writes the summary:

```bash
python3 tools/improvement/evidence.py /tmp/opencode/workflow-run-test
```

## 4. Read the evidence

| Artifact | What it answers |
|---|---|
| `artifacts/backtest/manifest.json` | The stage identity and provenance hashes |
| `artifacts/qualification/qualification.json` | The verdict and the per-criterion evidence |
| `artifacts/analytics/metrics.json` | The descriptive metrics and tables |
| `artifacts/evidence.json` | The run summary used by the review |
| `results.json` | What commands ran and what they printed |
| `review.md` | The Result Analyst's comparison against the baseline |
| `decision.md` | The Triage Officer's route and the human decision |

A qualification `pass` is evidence status, not acceptance. A `fail` is a valid
research result. The human records acceptance in `decision.md`.

## 5. Record the change

A meaningful user-facing or developer-facing change adds or updates a release
entry in [`CHANGELOG.md`](../../../CHANGELOG.md), grouped by release and type.
A change with no such effect records why no entry is needed. The PR template asks
for the workflow, the record IDs, the verification table, the changelog
disposition, and the Quant validation status. The changelog is the release
history, not a copy of the GitHub Project.

A documentation or tooling change records that no Quant Validator record is
required and why. A model or strategy behavior change requires the Quant
Validator record.

## 6. Stop conditions

Stop and route to the owning workflow when:

- the change is a defect, a refactor, or a new capability;
- the data is invalid;
- the run composition changes;
- a model or strategy rule changes without an approved hypothesis;
- a gate fails for a reason outside the scope.

## 7. Related documents

- `quantrule help workflows` — the in-shell selection guide.
- [`docs/workflows/README.md`](../../workflows/README.md)
- [`docs/workflows/improvement-workflow.md`](../../workflows/improvement-workflow.md)
- [`docs/workflows/run-contract.md`](../../workflows/run-contract.md)
- [`docs/workflows/strategy-research.md`](../../workflows/strategy-research.md)
- [`docs/architecture/command-surface.md`](../../architecture/command-surface.md)
- [`docs/config/2026-11-06/engine-artifacts.md`](../../config/2026-11-06/engine-artifacts.md)
