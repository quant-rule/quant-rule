# MR5 Programme — Phase 2 Execution Plan

- Date: 2026-11-08 (UTC)
- Status: **Verified** (2026-11-08) — 30 cells run; `no_edge`; validation not opened
- Work item: task slug `mr5-platform`; Phase 2 slug `mr5-programme`
- Charter: [`MR5-CHARTER.md`](../../governance/2026-11-08/MR5-CHARTER.md)
- Power worksheet: [`MR5-POWER.md`](../../improvement/2026-11-08/MR5-POWER.md)
- Data review: [`MR5-DATA-REVIEW.md`](../../improvement/2026-11-08/MR5-DATA-REVIEW.md)
- Claim type: model research programme; no edge claim

## 1. Intent

Run the first MR5 exploratory grid on `model_dev`, review every cell, and
promote at most the five pre-declared primary base cells to a one-shot
`model_validation`. The result is a model baseline or `no_edge`; the strategy
tier follows only after a baseline is accepted.

## 2. Current state

The Phase 1 platform is delivered in this branch:

- `MR` research prefix and `-Cnn` configuration grammar;
- per-cell model config in the research runner;
- sequence, confluence, column/freshness, and path evaluators;
- MR5 coverage and grid builder;
- power/MDE worksheet and Data Steward review;
- reference MR5 grid and tests.

The 30-cell grid has not been built or run. No validation has been opened.

## 3. Scope

Included:

- Build `PROGRAM-2026-11-08-01` from the approved MR5 grid: 10 hypotheses × 3
  configs = 30 cells; five primary base cells.
- Run the count-only gate on the five primary base cells.
- Run the 30-cell exploratory swarm on `model_dev` with `max_parallel` 20.
- Aggregate every cell; the primary base cells are ranked; the exploratory
  cells are descriptive.
- Write the development record and the recommendation.
- If a primary base cell is grade A, request validation access; otherwise stop
  as `no_edge`.
- If validation is approved, run the one-shot validation and record the Quant
  Validator verdict.
- Update the baseline, model-family, run-index, README, changelog, and AI
  records.

Excluded:

- A 31st cell, a new event, or a new indicator.
- Any post-hoc change to a condition, model config, timeframe, instrument,
  horizon, target, stop, cost, or criterion.
- Any strategy, execution, or portfolio run.
- Any model engine change.

## 4. Assumptions and uncertainty

| Assumption | If it is wrong |
|---|---|
| The sequence/confluence/path gates are causal and deterministic | The tests fail; the gate is not used and the programme stops |
| The five primary base cells reach 200 condition-matched predictions and 20 clusters | The programme stops as `underpowered` before an outcome |
| The 30-cell exploratory round is hypothesis generation | If it is treated as evidence, the programme is invalid; the report labels it exploratory |
| The validation partition reuse is acceptable | The human may reject validation access; the development result remains `development_only` |
| A model config variant is a legitimate pre-registered variant | If it is used as a post-hoc knob, the programme is invalid |

## 5. Decisions

| Decision | Proposed | Status |
|---|---|---|
| Naming | `M1.0 / MR5.x / -Cnn` | Proposed; human gate |
| Primary cells | Five base cells `MR5.1-C01`–`MR5.5-C01` | Proposed; power worksheet |
| Grid | 10 × 3 = 30 cells | Proposed |
| Correction | Bonferroni over five; alpha 0.01 | Proposed |
| Data | Four core instruments; ETH optional for price-only | Proposed; Data Steward review |
| Validation | One-shot after human promotion | Not approved now |
| Strategy | After an accepted baseline | Closed |

## 6. Acceptance criteria

- The grid builds 30 cells; the programme check passes; the MR5 coverage check
  passes with 46 indicators and 143 inherited events.
- The five primary base cells pass the count gate; otherwise the programme stops
  as `underpowered`.
- All 30 cells run on `model_dev`; every cell has a `cell-result.json`.
- The report includes every cell; only the five primary base cells are ranked.
- A reproducibility rerun of one cell matches the prediction digest and metrics.
- The development record states `no_edge`, `development_only`, or a candidate.
- If validation is approved, the one-shot validation is recorded with a
  Quant Validator verdict and the human baseline decision.
- The AI log and changelog are updated.

## 7. Work breakdown

| Step | Work | Role | Paths / command |
|---|---|---|---|
| P2.1 | Write the approved `grid.json`; build the 30 cells | Quant Developer | `config/research/PROGRAM-2026-11-08-01/`; `tools/research/build_mr5_program.py` |
| P2.2 | Validate the manifest and inherited coverage | Test Engineer | `program.py --check`; `coverage.py --family MR5 --inherit ...` |
| P2.3 | Count-only gate on the five primary base cells | Loop Governor | `model_cell.py --count-only` |
| P2.4 | Run the 30-cell exploratory swarm | Loop Governor | `batch.py --phase dev --mode config --max-parallel 20` |
| P2.5 | Aggregate and review every cell | Result Analyst, Quant Researcher | `report.py`; `program-results/` |
| P2.6 | Human promotes at most the five primary base cells | Human | Promotion record |
| P2.7 | One-shot validation, if promoted | Quant Validator | `model_validation`; Quant Validator record |
| P2.8 | Closeout and records | Triage Officer, Coordinator | `docs/improvement/2026-11-08/MR5-DEV-SWARM.md`; baselines; README; AI log |

P2.1–P2.5 are one branch. P2.7 is a separate one-shot run and is not authorized
by this plan's approval; it needs the human's validation-access decision.

## 8. Verification

| Check | Command | Expected |
|---|---|---|
| Manifest | `python3 tools/research/program.py --program ... --check` | `ok PROGRAM-2026-11-08-01 cells=30 primary=5` |
| Coverage | `python3 tools/research/coverage.py ... --family MR5 --inherit ...` | `ok ... cells=30 indicators=46 events=143` |
| Count gate | five `model_cell.py --count-only` runs | every primary base cell `n >= 200`, `clusters >= 20` |
| Development | `python3 tools/research/batch.py ... --max-parallel 20` | `ok batch ... cells=30` |
| Aggregation | `python3 tools/research/report.py ...` | every cell reported; decision |
| Reproducibility | rerun one cell | same digest and metrics |
| Docs | `check_links`, `check_terms`, `check_legacy`, `check_structure`, `check_workflows`, `check_changelog`, `check_partitions` | pass |

## 9. Risks and recovery

| Risk | Mitigation / recovery |
|---|---|
| The gate is underpowered | Count gate stops before an outcome; record `underpowered`; retire the premise |
| A gate leaks the future | Causality test fails; do not run; fix or stop |
| The 30-cell grid is read as a search | Primary base cells are frozen; exploratory cells are not ranked; no post-hoc promotion |
| The validation partition is reused | Disclose; Quant Validator records the residual risk; a fresh instrument is a separate Data Lifecycle item |
| A cell changes after a result | The run is invalid; record the reason; stop |
| The builder or runner fails | Fix within Phase 1; rerun the affected cell; no outcome is read before the fix |

## 10. Stop conditions

- The human declines the charter or the plan.
- The grid or coverage check fails.
- A primary base cell is underpowered at the count gate.
- A condition or model config changes after a result.
- Validation is requested without a grade-A primary base cell and human
  promotion.
- The iteration cap is reached.
