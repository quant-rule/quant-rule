# Engine Execution Workflow

- Date: 2026-11-16 (UTC)
- Status: Current; the stage commands shipped in v1.3.0 (#850–#854)
- Scope: diagnostic execution of the data, model, strategy, and portfolio engines
- Owner: Quant Researcher (iteration), Quant Validator (evidence boundary), Implementation (commands)
- Contract: [ADR-2026-11-16-01](../adr/2026-11-16/01-stage-execution-and-evidence-boundary.md), [stage execution plan](../execution-plans/2026-11-16/842-stage-execution-surface.md)
- Claim type: Behaviour Claim
- Related: [Backtest Workflow](backtest.md), [Run Contract](run-contract.md), [Improvement Workflow](improvement-workflow.md)

## 1. Purpose

This workflow answers one question: **how does the trader run one engine alone,
inspect what it produced, and promote a frozen result without pretending the
stage is evidence?**

It does not decide whether a rule has an edge. That is Strategy Research and the
Improvement Workflow. It does not produce a canonical digest. That is the
[Backtest Workflow](backtest.md).

The workflow covers four stage runbooks — Data, Model, Strategy, Portfolio — and
the evidence rule that binds them.

## 2. When to use

Use it when:

- the data must be inspected or admitted before a model run;
- a model configuration needs event, level, progression, or coverage inspection;
- a strategy rule needs a decision and explanation against a frozen model artifact;
- execution and portfolio mechanics need fills, costs, drawdown, or exposure
  inspection;
- a stage artifact is being considered as the input to a later stage.

Do not use it when:

- the result must become evidence or a baseline: use the Backtest Workflow;
- the change is a model or strategy hypothesis: use Model Research or Strategy
  Research;
- the change is a defect: use the Audit Workflow;
- the task is a clean deployment check: use End-to-End Verification.

## 3. Stage runbooks

| Stage | Question | Input | Command | Output | Evidence |
|---|---|---|---|---|---|
| Data | Is the input valid, ordered, and what ladder does it produce? | CSV or a recorded dataset | `dataset upload`, `dataset show`, `inspect candles`, `inspect timeframes` | `data_version`, quality, ladder | Input identity; never a result |
| Model | What does the model measure on this data? | `data_version`, `config_id` | `stage run model <data_version> <config_id>` | `stage_run_id`, `model_artifact_id` | Diagnostic only |
| Strategy | What decisions does this rule make against a frozen model? | `model_artifact_id`, `config_id` | `stage run strategy <model_artifact_id> <config_id>` | `stage_run_id`, `decisions_artifact_id` | Diagnostic only |
| Portfolio | What fills and account outcome result from fixed decisions? | `decisions_artifact_id`, `config_id` | `stage run portfolio <decisions_artifact_id> <config_id>` | `stage_run_id`, `portfolio_artifact_id` | Diagnostic only |

There is no `data run`. The data engine is already reached by upload and
inspection; a separate run would duplicate the catalog record. There is no
`execution run`; execution is the fill half of the portfolio stage because a
fill without accounting and accounting without fills are not useful.

### 3.1 Data

1. `dataset upload <csv>` returns the content-addressed `data_version`, candle
   count, coverage, and quality.
2. `dataset show <data_version>` and `inspect candles` / `inspect timeframes`
   read the stored series and ladder.
3. Admit the dataset when the quality and coverage gates pass. The data version
   is the input identity for every later stage.

Artifacts: the dataset record, the quality report, the timeframe ladder.

### 3.2 Model

1. `stage run model <data_version> <config_id>` computes the model bundle and
   returns a `stage_run_id` and a `model_artifact_id`.
2. `stage show <stage_run_id>` reads coverage, event counts, levels, legs, and
   provenance. `stage events <stage_run_id> --timeframe <tf>` reads the event
   batch.
3. `stage fetch <stage_run_id> <dir>` downloads the artifact set; every file is
   checked against its SHA-256.
4. Iterate by changing the model file in the run config and running the stage
   again. The data version does not change.

The model artifact is the content-addressed model bundle. It publishes numbers
and codes, never labels. It carries no strategy, execution, portfolio, or P&L.

### 3.3 Strategy

1. Freeze the model artifact that the rule will read. The artifact's model config
   hash must match the model file in the run config.
2. `stage run strategy <model_artifact_id> <config_id>` computes the decisions
   bundle and returns a `stage_run_id` and a `decisions_artifact_id`.
3. `stage show <stage_run_id>` reads the decisions table;
   `stage explain <stage_run_id> --bar <n>` reads one decision's conditions and
   as-of context.
4. The stage is for inspection. It simulates its own position. The coupled
   backtest supplies the position execution actually holds, so the two decision
   sets can differ. **Never use a stage decision bundle as evidence.**

### 3.4 Portfolio

1. Freeze the decisions artifact to inspect.
2. `stage run portfolio <decisions_artifact_id> <config_id>` runs execution and
   portfolio accounting together and returns fills, costs, equity, drawdown,
   exposure, and halt state.
3. `stage show <stage_run_id>` reads the fills and metrics.
4. The stage is for debugging mechanics. **Never use stage fills or metrics as
   evidence.** The backtest recomputes execution and portfolio from the frozen
   strategy.

## 4. The evidence rule

| Rule | Meaning |
|---|---|
| Stage = diagnostic | A stage artifact is never evidence, never a baseline, never qualification input, and never part of the canonical digest |
| Model artifact reuse allowed | `run submit --model-artifact` may reuse a frozen model artifact when the model config hash matches; the run records the artifact hash and the model bytes must equal a full run |
| Strategy and portfolio recompute | The backtest recomputes strategy, execution, and portfolio; a stage decision or fill artifact is never a backtest input |
| TTL | Stage artifacts expire under `QR_STAGE_RETENTION_DAYS`; evidence is permanent |
| No partial publish | A killed stage worker leaves no visible artifact |
| Both clients | The CLI and the browser expose the same stage use cases; parity is tested |

The only evidence command remains `run submit`. There is no `backtest run`
alias.

## 5. Roles and boundaries

| Role | Responsibility | Boundary |
|---|---|---|
| Quant Researcher | Choose the stage, inspect the artifact, iterate the configuration | No post-hoc threshold change; no edge claim |
| Quant Developer | Implement the stage surface and its tests | No model or strategy contract change |
| Quant Validator | Check the evidence rule and the model-reuse equivalence | Read-only; no repair |
| Implementation | Server routes, CLI, GUI, migrations, retention | No engine in the client |
| Human | Accept the stage result as diagnostic only; approve a baseline through the Improvement Workflow | Sole final authority |

## 6. Human gates

| Gate | Human action |
|---|---|
| Stage output | Treat it as diagnostic; do not cite it as evidence |
| Model artifact reuse | Approve the `run submit --model-artifact` path when it is used for a research run |
| Promotion | Approve the coupled backtest; the backtest remains the evidence |
| Baseline | Accept only through the Improvement Workflow |

## 7. Green criteria

A stage run is green when:

- the input reference and config are valid;
- the stage artifact is content-addressed and its manifest verifies;
- the stage artifact carries `evidence: false` and no canonical digest;
- a rerun with the same input, config, and image digest produces the same bytes;
- a killed worker leaves no visible artifact;
- the stage is reachable from the CLI and the browser;
- the evidence boundary test rejects the artifact where evidence is required.

## 8. Artifacts and records

| Artifact | Shape | Owner | Status |
|---|---|---|---|
| `stage_run_id` | Content-derived stage identity | Evidence `stage` schema | queued, running, succeeded, failed, cancelled |
| `model_artifact_id` | Model bundle manifest digest | Model stage | diagnostic |
| `decisions_artifact_id` | Decisions bundle digest | Strategy stage | diagnostic |
| `portfolio_artifact_id` | Fills + metrics bundle digest | Portfolio stage | diagnostic |
| Stage manifest | `quantrule.stage-manifest/v1` | Evidence / object store | immutable |
| Stage provenance | `quantrule.stage-provenance/v1` | Stage bundle | immutable |

Stage records are local to the workspace and expire. They are not improvement
runs and are not in the committed improvement index.

## 9. Stop conditions

Stop and request human direction when:

- a stage artifact would be used as evidence or a baseline;
- the model artifact's config hash does not match the run config;
- the strategy stage is expected to equal the coupled run without recomputation;
- a stage run needs a model, strategy, or portfolio contract change;
- the stage surface cannot be reached from both clients;
- a migration or retention change would affect evidence;
- a stage run fails with an error outside the approved scope.

## 10. Handoff

| To | Packet |
|---|---|
| Backtest Workflow | The frozen model artifact and the full run config |
| Strategy Research | The model artifact, coverage, and diagnostic findings |
| Portfolio Research | The decisions artifact, fills, and metrics |
| Improvement Workflow | Nothing as evidence; only a hypothesis for a pre-registered run |
| Recordkeeping | Stage IDs and the diagnostic label, never as a baseline |

## 11. Related documents

- [ADR-2026-11-16-01](../adr/2026-11-16/01-stage-execution-and-evidence-boundary.md)
- [Backtest Workflow](backtest.md)
- [Stage to backtest tutorial](../tutorials/2026-11-17/stage-to-backtest.md)
- [Run Contract](run-contract.md)
- [Improvement Workflow](improvement-workflow.md)
- [Command surface](../architecture/command-surface.md)
- [`docs/workflows/README.md`](README.md)
- [`AGENTS.md`](../../AGENTS.md)
