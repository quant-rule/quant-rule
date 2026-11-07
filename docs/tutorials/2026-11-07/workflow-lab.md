# Workflow Lab — Choose a Workflow and Run a Tier

- Date: 2026-11-07 (UTC)
- Status: Current
- Scope: the workflow index, the current run adapter, and the stage commands
- Claim type: tutorial; no model, strategy, or edge claim.

## 1. Choose the workflow

Read [`docs/workflows/README.md`](../../workflows/README.md). The workflow is
chosen by the decision boundary, not by the file you expect to edit:

| If the change is… | Use |
|---|---|
| A defect | Audit |
| A structural problem | Refactor |
| A model hypothesis | Model Research |
| An indicator or event implementation | Model Change |
| A strategy hypothesis | Strategy Research |
| An execution or portfolio hypothesis | Portfolio Research |
| A policy envelope change | Policy Change |
| A data source or partition change | Data Lifecycle |
| A new command or capability | Requirements & Design |
| A release or incident | Release & Operations |
| A retirement | Deprecation & Retirement |
| A record or changelog issue | Recordkeeping |

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
- the policy envelope changes;
- a model or strategy rule changes without an approved hypothesis;
- a gate fails for a reason outside the scope.

## 7. Related documents

- [`docs/workflows/README.md`](../../workflows/README.md)
- [`docs/workflows/improvement-workflow.md`](../../workflows/improvement-workflow.md)
- [`docs/workflows/run-contract.md`](../../workflows/run-contract.md)
- [`docs/workflows/strategy-research.md`](../../workflows/strategy-research.md)
- [`docs/architecture/command-surface.md`](../../architecture/command-surface.md)
- [`docs/config/2026-11-06/engine-artifacts.md`](../../config/2026-11-06/engine-artifacts.md)
