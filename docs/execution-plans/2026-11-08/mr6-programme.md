# MR6 Programme — Ordered Family Confirmation

- Date: 2026-11-08 (UTC)
- Status: **Verified** (2026-11-08) — 15 cells run sequentially; `no_edge`; validation not opened
- Charter: [`MR6-CHARTER.md`](../../governance/2026-11-08/MR6-CHARTER.md)
- Programme: `PROGRAM-2026-11-08-02`
- Claim type: pre-registered model research; no strategy or edge claim

## 1. Intent

Run the MR6 ordered family-confirmation grid on `model_dev` and decide whether a
trend/momentum event followed by an independent participation event carries
directional information, while the reverse order does not. The result is
`no_edge`, `development_only`, or a candidate for the human's validation
decision. The strategy, execution, and portfolio tiers remain closed.

## 2. Current state

- The MR5 platform provides the condition, path, per-cell model, coverage, and
  grid tooling.
- MR5 stopped at `no_edge`; `MR5.7` is the seed hypothesis, not a candidate.
- The sequence condition accepts one first and one second event; MR6 needs event
  lists and a confirmed-level filter on the sequence bar.
- The base model `M1.0` already publishes the trend, momentum, relative-volume,
  flow, climax, and absorption events used by MR6. No model-engine change is
  required.
- `data/partitions.json` already declares ETHUSD; the runs are synthesized from
  the XAU template with the ETH source file.

## 3. Scope

Included:

- Add `first_events` / `second_events`, `min_gap_bars`, and optional
  `level_touches_min` / `level_age_max` to the sequence condition; keep the
  single-event fields backward compatible.
- Add tests for family lists, strict ordering, and the level filter.
- Generalize the grid builder for `family`, per-hypothesis timeframe,
  `run_prefix`, and `extra_instruments`.
- Build the 15-cell MR6 grid at `PROGRAM-2026-11-08-02`.
- Run the count gate on the five primary base cells.
- Run all 15 cells sequentially on `model_dev`.
- Report every cell, rank only the five primary base cells, and record the
  decision.

Excluded:

- `model_validation`; the strategy, execution, and portfolio tiers.
- Any model-engine, indicator, event, or data-contract change.
- Any post-hoc event, window, timeframe, or threshold change.
- The cumulative OBV and AD events, excluded by the data review before the run.

## 4. Assumptions and uncertainty

- The family-level event base is materially larger than the MR5.7 pair; the
  count gate will test that assumption.
- The 5m cells add sample but also cost drag; the path report is descriptive.
- The level arm may be underpowered. If the count gate fails, the programme
  stops rather than relaxing the level rule.
- BTC and ETH volume units change within a year. The relative-volume restriction
  is the admissibility decision; the residual boundary distortion is disclosed.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Primary cells | Five base cells | Bonferroni alpha 0.01; controls are not candidates |
| Order | `min_gap_bars = 1` | The second event is strictly later; same-bar pairs do not count |
| Families | 7 trend × 8 participation | One family-level mechanism, not 56 hypotheses |
| Level arm | `level_touches_min = 1` | Location-confound check; may be underpowered |
| ETH | Included | Relative-volume columns are admissible; not a fresh holdout |
| Execution | Sequential | The platform restarted during long parallel runs; one cell at a time |

## 6. Acceptance criteria

- Manifest check passes: 15 cells, 5 primary.
- Coverage check passes for `MR6` with 75 specs and the inherited M2 matrix.
- The count gate passes all five primary base cells (≥200 predictions, ≥20
  clusters) before any outcome is read.
- All 15 cells produce a `cell-result.json`; the report aggregates all 15.
- The development record names the decision, the primary-cell results, the
  controls, the path evidence, the limitations, and the reproduction digest.
- The registries, model-family table, README, changelog, and AI log are updated.
- The single-cell rerun matches the recorded digest and metrics.

## 7. Work breakdown

| Step | Work | Owner | Paths |
|---:|---|---|---|
| 1 | Condition extension and tests | Quant Developer; Test Engineer | `tools/improvement/model_condition.py`, `test_model_condition.py` |
| 2 | Builder/coverage generalization | Quant Developer; Test Engineer | `tools/research/build_mr5_program.py`, `coverage_mr5.py`, `coverage.py`, `test_mr6.py` |
| 3 | Grid, runs, coverage | Quant Developer | `config/research/PROGRAM-2026-11-08-02/` |
| 4 | Data review and power worksheet | Data Steward; Statistical Methodologist | `docs/improvement/2026-11-08/MR6-*.md` |
| 5 | Count gate | Quant Developer | `data/research/PROGRAM-2026-11-08-02/preflight/` |
| 6 | Sequential development run | Quant Developer | `data/research/PROGRAM-2026-11-08-02/dev/` |
| 7 | Aggregation and decision | Quant Validator; Result Analyst | `program-results/`, `MR6-DEV-SWARM.md` |
| 8 | Records and delivery | Orchestrator | registries, changelog, AI log, PR |

## 8. Verification

| Check | Command | Expected |
|---|---|---|
| Manifest | `python3 tools/research/program.py --program ... --check` | 15 cells, 5 primary |
| Coverage | `python3 tools/research/coverage.py --program ... --family MR6 --inherit ...` | 75 specs, inherited M2 |
| Count gate | `model_cell.py --count-only` on five primary cells | ≥200 n and ≥20 clusters each |
| Development | `batch.py --max-parallel 1` | 15/15 results |
| Aggregation | `report.py` | every cell, decision |
| Reproducibility | rerun one cell | same digest and metrics |
| Python tests | improvement and research unittest discovery | green |
| Rust | `cargo test`, clippy, fmt | green |
| Docs | link, term, legacy, structure, workflow, changelog, partition checks | green |

## 9. Risks and recovery

- A platform restart can kill a long run. The batch is sequential and resumable;
  completed cells are not re-run.
- The level arm may be underpowered. The stop rule is `underpowered`, not a
  relaxed threshold.
- The volume-unit defect may contaminate a relative-volume cell near a monthly
  change. The data review bounds it; a future data version repairs it.
- The validation partition is reused; no validation is opened here.
- The same agent performs the Quant Developer and Quant Validator passes; the AI
  log discloses that they are sequential, not independent.

## 10. Stop conditions

Stop and report if the count gate fails, a reverse-order control passes, a
shared prerequisite (data, binary, registry, coverage) is missing, or a
material deviation from the approved protocol would be required. Do not change
an event, window, timeframe, threshold, or instrument to rescue a result.

## 11. Verification result

| Check | Observed |
|---|---|
| Manifest | `ok PROGRAM-2026-11-08-02 cells=15 primary=5 phase=dev` |
| Coverage | `ok ... cells=15 indicators=46 events=143 inherited=PROGRAM-2026-11-05-03`; 75 specs |
| Count gate | 77,385/14,207; 81,001/14,138; 252,220/45,968; 267,313/45,878; 61,100/12,913 |
| Development | 15/15 cells; `no_edge`; 0/5 primary passed |
| Reproducibility | `MR6.1-C01` digest `01a3a3c928a5...`; same n, clusters, lift, LCB, controls |
| Python tests | 113 improvement, 51 research |
| Rust | `cargo test`, clippy, fmt pass |
| Docs | all checkers pass |

The result record is [`MR6-DEV-SWARM.md`](../../improvement/2026-11-08/MR6-DEV-SWARM.md).
