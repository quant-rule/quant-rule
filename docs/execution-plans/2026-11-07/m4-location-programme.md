# M4 Location Programme — Execution Plan

- Date: 2026-11-07 (UTC)
- Status: **Verified** (2026-11-07) — authorized development scope complete;
  independent Quant Validator review and the commit remain pending
- Work item: no GitHub issue; task slug `m4-location-programme`
- Charter: [`M4-CHARTER.md`](../../governance/2026-11-07/M4-CHARTER.md)
- AI transparency log: [`m4-location-programme.md`](../../ai-transparency-log/2026-11-07/m4-location-programme.md)
- Workflows: [Model Research](../../workflows/model-research.md) and
  [Research Swarm](../../workflows/research-swarm.md)
- Claim type: execution plan for a pre-registered model programme; no edge claim

## 1. Intent

Run the first M4.x model research programme under the Model Research workflow
and the Research Swarm execution mechanism. The programme tests one falsifiable
premise on `model_dev`: a directional event has edge only when it occurs at a
confirmed major level, and the same event away from structure does not.

The intended outcome is evidence, not a baseline: five pre-registered cells, a
count-only power gate, a development result, an aggregation with the declared
correction, and a `no_edge` / `development_only` / `candidate` verdict for the
human. The human accepts or rejects any baseline.

## 2. Current state

| Item | Observed state |
|---|---|
| Branch / worktree | `feat/m4-location-programme` in `../quant-rule--m4-location-programme`, based on `main` at `ce85fd0` |
| M1.0 model | Current; 46 indicators, 143 active events; `quantrule.model/v4` |
| Accepted model baselines | Three non-directional compression baselines; no directional baseline |
| M2 programme | `PROGRAM-2026-11-05-03` closed at `no_edge`; all 15 cells grade C/D |
| M3 programme | `PROGRAM-2026-11-05-04` stopped at the count gate as `underpowered`; no development run |
| 15m family programme | Five directional families grade C at 15m; no validation |
| Strategy / portfolio | No accepted baseline; both tiers closed |
| Research tooling | `tools/research/` is retained/historical; it must be verified against the current S3 surface before a new campaign |
| Evaluator | `tools/improvement/model_prediction.py` supports `model-state-gate/v1` with `values`; it does not accept `level_touches` or a numeric `min` |
| Data | M1 CSVs are present under `data/sources/`; `model_dev` 2018–2020 is available |
| Binary | `target/release/quantrule` exists but is older than the current `main`; a release build is required before the run |

The read-only preflight found no active campaign and no open M4 charter. The
Model Research workflow requires a new premise and a human-approved charter
before any cell runs, so this plan stops at the approval gate.

## 3. Scope

Included after approval:

- Generalize the condition gate in `tools/improvement/model_prediction.py` to
  accept `level_touches` and a numeric `min`/`max` bound, with tests.
- Build `PROGRAM-2026-11-07-01`: manifest, five cell directories, four
  instrument specs per cell at 15m, `state.json`, `coverage.json`, and run
  configs.
- Add the inherited-coverage path to `tools/research/coverage.py` and a
  programme check for the M4 manifest.
- Run the count-only preflight; stop if any cell is underpowered.
- Run the five-cell development swarm on `model_dev`.
- Aggregate with the declared Bonferroni correction; write `cell-result.json`
  per cell and the programme report.
- Write the development record, review, and decision; update the committed
  evidence registries; record the AI transparency log; prepare the PR.
- Independently reproduce the result and record a Quant Validator verdict.

Excluded:

- Any `model_validation` access. That is a separate human gate after a grade-A
  development cell is promoted.
- Any strategy, execution, portfolio, policy, or data change.
- Any change to the Rust model, event definitions, indicator formulas, or spec
  schema.
- Any post-hoc change to an event, timeframe, horizon, target, stop, cost,
  instrument set, gate column, or gate threshold.
- Any re-run of the 10 M2 exploratory cells; their coverage is inherited, not
  repeated.

## 4. Assumptions and uncertainty

| Assumption | If it is wrong |
|---|---|
| `level_touches` is a valid, causal context reading at the prediction bar | The gate is rejected and the premise is not testable; the programme stops before an outcome |
| The 15m event counts from 2026-11-04 are representative | The count gate may be underpowered; the programme stops as `underpowered` and the premise is retired |
| The five events are emitted by the current M1.0 model | A missing emitter fails the coverage or count gate; no outcome is read |
| The M3 run configs are reusable with a data-version rename | A config mismatch stops the build; it is corrected before the run, not after a result |
| The condition gate does not leak the future | The causality test fails; the gate is not used and the programme stops |
| The validation partition's prior use is acceptable | The human may reject validation access; the development result remains `development_only` |

## 5. Decisions

| Decision | Options | Consequence | Status |
|---|---|---|---|
| Premise | Unconditional (M2, no edge); state-conditioned (M3, underpowered); location-conditioned (proposed) | Location is orthogonal to the failed conditions and is cheap to test with a published context reading | Proposed; human gate |
| Timeframe | 15m vs 1h | 15m has the higher event counts and matches the failed family baseline; 1h would be a new, underpowered test | Proposed: 15m, no switch |
| Cells | 5 primary vs 10 exploratory + 5 primary | M4 adds no event or indicator; the M2 surface already covers 46/46 and 143/143; re-running it would be a variant of a failed programme | Proposed: 5 primary, inherited coverage |
| Gate | `level_touches >= 1` vs `level_age <= N` vs level confluence `>= 2` | `>= 1` uses the model's fixed level definition and avoids an arbitrary age threshold; confluence is likely underpowered | Proposed: `>= 1` |
| Evaluator | Extend `model-state-gate/v1` with `min` vs new schema | Extending the existing schema keeps the M3 `values` contract and the `state_*` output stable; a new schema duplicates the evaluator | Proposed: extend |
| Validation | Request now vs after grade A | The workflow forbids validation before the development gate and the human promotion | Proposed: not now |

The human approves or rejects each proposed decision at the gate in section 14
of the charter. No implementation begins before that approval.

## 6. Acceptance criteria

The programme is complete when:

- the evaluator gate tests pass, including causality, determinism, null values,
  empty gates, and the model-boundary test;
- the programme manifest validates and the coverage check reports `cells=5`,
  `indicators=46`, `events=143`, and the inherited M2 surface;
- the count-only preflight either passes every primary cell at `n >= 200` and
  `clusters >= 20`, or stops the programme as `underpowered` without reading an
  outcome;
- all five cells run on `model_dev` and each writes a `cell-result.json`;
- the programme report applies the Bonferroni correction over five cells and
  records every cell, including failures;
- a byte-identical rerun of one cell produces the same prediction digest and
  metrics;
- `review.md` and `decision.md` record the result and the next route without
  accepting a baseline;
- the committed records, the AI log, and the changelog disposition are updated;
- a Quant Validator record reproduces the result from the recorded inputs.

## 7. Work breakdown

| Step | Work | Role | Paths / command | Depends on |
|---|---|---|---|---|
| P1 | Approve the charter and this plan | Human | `docs/governance/2026-11-07/M4-CHARTER.md` | — |
| P2 | Extend the condition gate and tests | Quant Developer, Test Engineer | `tools/improvement/model_prediction.py`, `tools/improvement/test_model_prediction.py`, `docs/analytics/2026-11-05/model-prediction.md` | P1 |
| P3 | Build the programme, cells, specs, gate, coverage, run configs | Quant Developer | `config/research/PROGRAM-2026-11-07-01/`, `tools/research/coverage.py`, `tools/research/test_coverage.py` | P1, P2 |
| P4 | Build the release shell and run the count-only preflight | Loop Governor | `cargo build --release -p quantrule-shell`; `tools/research/model_cell.py --count-only` | P3 |
| P5 | Run the five-cell development swarm | Loop Governor | `tools/research/batch.py --phase dev --mode config --max-parallel 5` | P4 |
| P6 | Aggregate every cell with the correction | Result Analyst | `tools/research/report.py` | P5 |
| P7 | Reproduce and review the result | Quant Validator, Code Reviewer | same inputs; `review.md`, `decision.md` | P6 |
| P8 | Record the evidence and close out | Coordinator, Triage Officer | `docs/improvement/2026-11-07/M4-DEV-SWARM.md`, baselines, model families, `docs/README.md`, `CHANGELOG.md`, AI log, PR | P7 |
| P9 | Validation only after human promotion | Quant Validator | one-shot `model_validation` | P8 + separate human approval |

P2–P7 are one branch and one PR where practical. P9 is a separate one-shot run
and is not authorized by this plan approval.

## 8. Verification

| Check | Command | Expected evidence |
|---|---|---|
| Evaluator tests | `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` | All tests pass; the new gate cases are included |
| Research tests | `python3 -m unittest discover -s tools/research -p 'test_*.py'` | All tests pass; coverage and count tests included |
| Manifest | `python3 tools/research/program.py --program config/research/PROGRAM-2026-11-07-01/program.json --check` | `ok PROGRAM-2026-11-07-01` |
| Coverage | `python3 tools/research/coverage.py --program ... --inherit ...` | `ok ... cells=5 indicators=46 events=143` |
| Count gate | `python3 tools/research/model_cell.py --count-only ...` for five cells | `count.json` per cell; pass or `underpowered` |
| Development | `python3 tools/research/batch.py --program ... --phase dev --mode config --max-parallel 5` | Five `cell-result.json` files |
| Aggregation | `python3 tools/research/report.py --program ... --results ... --out ...` | `program-results.json/md` with the correction |
| Reproducibility | Re-run one cell with the same inputs | Same prediction digest and metrics |
| Docs | `python3 tools/check_links.py`, `check_terms.py`, `check_legacy.py`, `check_structure.py`, `check_workflows.py` | No failures |
| Changelog | `python3 tools/check_changelog.py` | No failure; entry or no-entry reason recorded |

The Rust test suite is run if any Rust file changes. P2–P3 are expected to be
Python and configuration only; the release build in P4 is the shell gate.

## 9. Risks and recovery

| Risk | Mitigation / recovery |
|---|---|
| The gate is underpowered | The count-only gate stops before an outcome; record `underpowered`; retire the premise; no variant |
| The gate leaks the future | The causality test fails; do not run; fix the gate or stop |
| A cell changes after a result | The run is invalid; record the reason and stop; no post-hoc retune |
| The evaluator change alters M3 behavior | The `values` path is unchanged and its tests remain; the M3 records are historical and are not re-run |
| The run configs use the wrong data version | Fix the config before the run; a manifest/config mismatch invalidates the run |
| Validation is requested before grade A | The charter and run contract forbid it; stop and return to the development gate |
| The worktree or branch is lost | The plan, charter, specs, and records are committed; the run artifacts are local and can be regenerated from the recorded inputs |

## 10. Stop conditions

Stop and report when:

- the human declines the premise, the charter, or this plan;
- the evaluator gate cannot be made causal and deterministic;
- the coverage check fails on an event, indicator, or gate column;
- the count gate is underpowered;
- a shared input or run config is missing;
- a result would require a post-hoc change to an event, timeframe, horizon,
  target, stop, cost, instrument set, gate column, or threshold;
- validation access is requested without a grade-A development cell and human
  promotion;
- the iteration cap is reached.

## 11. Verification record

- Completed: 2026-11-07 (UTC)
- Scope: P2–P8 of the authorized development plan
- Outcome: count gate powered; five cells ran; no cell passed; `no_edge`

| Step | Observed evidence |
|---|---|
| P2 evaluator gate | `tools/improvement` tests: 101 passed, including the `level_touches` bounds, null, empty, all-bar, no-bar, invalid-shape, determinism, and model-boundary cases |
| P3 programme build | Manifest check `ok PROGRAM-2026-11-07-01 cells=5 primary=5 phase=dev`; inherited coverage `cells=5 indicators=46 events=143`; research tests 45 passed |
| P4 count gate | M4.1 36,970/9,311; M4.2 58,594/11,014; M4.3 19,112/6,777; M4.4 3,523/2,784; M4.5 2,949/2,604 |
| P5 development swarm | `ok batch PROGRAM-2026-11-07-01 dev cells=5`; every cell wrote `cell-result.json` |
| P6 aggregation | `no_edge`; every cell reported with the Bonferroni correction |
| P7 reproducibility | M4.5 rerun matched the prediction digest, metrics, criteria, unconditional metrics, and increment |
| P8 records | Development record, baseline, index, model families, README, changelog, and AI log updated |
| Docs | `check_links`, `check_terms`, `check_legacy`, `check_structure`, `check_workflows`, `check_changelog`, and `check_partitions` pass |

Unresolved:

- The Quant Validator pass is same-agent and is disclosed as not independent.
- The worktree changes are uncommitted on top of `ce85fd0`; the commit, push, and
  PR require the human's separate authorization.
- `model_validation` remains closed; no baseline was accepted.

