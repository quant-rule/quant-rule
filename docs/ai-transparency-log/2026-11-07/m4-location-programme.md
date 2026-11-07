# AI Transparency Log — M4 Location Programme

- Date: 2026-11-07 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: no GitHub issue; task slug `m4-location-programme`
- Plan: [`m4-location-programme.md`](../../execution-plans/2026-11-07/m4-location-programme.md)
- Charter: [`M4-CHARTER.md`](../../governance/2026-11-07/M4-CHARTER.md)
- Branch / worktree: `feat/m4-location-programme` in
  `../quant-rule--m4-location-programme`
- Scope: read-only preflight, proposed M4 charter, proposed execution plan

## Intent and boundary

The human instruction was "run Model Research + Research Swarm". The workflows
require a new premise and a human-approved charter before the first cell, so the
first pass stopped at the charter gate. The human approved the charter and plan
on 2026-11-07 (UTC) with "aprove"; this log now records the authorized build,
count gate, and development run.

The implementation pass does not open validation, start the strategy tier, or
accept a baseline. It does not commit, push, or open a PR; those remain gated by
`AGENTS.md` section 9.

## Preflight evidence

| Check | Observed result |
|---|---|
| `git status --short --branch` in the project root | `## main...origin/main`; clean |
| `git worktree list` | one worktree, project root at `ce85fd0` |
| `git rev-parse --short HEAD` | `ce85fd0` |
| `gh issue list --state all --limit 10` | no open issue for an M4 programme; the listed items are closed |
| `docs/README.md` | M2.x `no edge`; M3.x `underpowered`; no active campaign; no strategy or portfolio baseline |
| `docs/improvement/baselines.md` | three accepted non-directional compression baselines; no directional baseline |
| `docs/improvement/2026-11-05/M2-CLOSEOUT.md` | M2 stopped at the development gate; validation not opened |
| `docs/improvement/2026-11-05/M3-PREFLIGHT.md` | M3 count gate underpowered; no development run |
| `docs/improvement/2026-11-04/DIRECTIONAL-EVENT-PROGRAM-CLOSEOUT.md` | five 15m families grade C; validation not opened |
| `docs/workflows/model-research.md`, `research-swarm.md`, `run-contract.md` | charter, promotion, and one-shot validation gates confirmed |
| `target/release/quantrule` | present, dated 2026-11-04; older than `main`; a release build would be required before a run |
| `data/sources/` | the four M1 CSVs are present; `model_dev` is available |
| `tools/improvement/model_prediction.py` | `STATE_COLUMNS` has seven state columns and `values` only; `level_touches` is not gateable |
| `crates/quantrule-shell/src/model_bundle/artifacts.rs` | `level_touches` is published in `readings.csv` |

The task worktree was created with
`git worktree add -b feat/m4-location-programme ../quant-rule--m4-location-programme main`.
The session was moved to the worktree. The project root remains on `main`.

## Changes

- Drafted `docs/governance/2026-11-07/M4-CHARTER.md` as a **Proposed** charter
  for a location-conditioned directional event programme.
- Drafted `docs/execution-plans/2026-11-07/m4-location-programme.md` as a
  **Proposed** execution plan.
- Defined five primary cells: `signal.rsi_threshold`,
  `signal.macd_histogram_slope_flip`, `signal.di_cross`,
  `signal.trend_continuation`, and `signal.ema_stack_change`, each gated by
  `level_touches >= 1` at 15m.
- Defined the count-only power gate, the condition-matched base and controls,
  the location increment, the Bonferroni correction over five cells, the
  one-shot validation boundary, and the stop rule.
- Proposed a small evaluator extension (`level_touches` and a numeric `min`) and
  the inherited-coverage path, both to be implemented only after approval.
- Created no programme manifest, cell spec, run config, code change, or run
  artifact.

## Verification and evidence

| Check | Observed result |
|---|---|
| `python3 tools/check_links.py` | checked 640 Markdown files; no broken relative links |
| `python3 tools/check_terms.py` | checked 390 file(s); no banned terms |
| `python3 tools/check_legacy.py` | legacy scan ok: 707 current files |
| `python3 tools/check_structure.py` | structure ok: module docs, engine purity, naming, layout, and data allowlist pass |
| `python3 tools/check_workflows.py` | workflow ok: 15 workflows, 6 templates, index rows 15 |
| `python3 tools/check_changelog.py` | changelog ok: 2 releases, 15 entries |

No `cargo` or research run was executed. This pass is documentation-only.

Changelog: no entry is needed. This pass adds a Proposed charter, a Proposed
execution plan, and this transparency log; it does not change user-facing or
developer-facing behavior.

## Learning

- The M2 exploratory surface already covers 46/46 indicators and 143/143 events.
  Re-running it for M4 would be a variant of a failed programme, so the M4
  coverage is inherited rather than repeated.
- The M3 programme stopped because two rare state/event pairs were underpowered,
  even though three cells had enough counts. M4 uses five families with known
  15m counts and a count gate over every cell, with no timeframe switch.
- The evaluator's state-gate schema is the smallest useful extension point: the
  model already publishes `level_touches`, so the location condition needs no
  Rust change and no new indicator.
- The 15m family programme is the direct baseline: the same mechanisms were
  tested unconditionally and graded C. The new condition is location, not a
  parameter change.
- The validation partition has three prior uses from the accepted compression
  baselines. Any M4 validation access must disclose that residual data-snooping
  risk.

## Rejected or corrected

- Rejected running any cell before charter approval. The workflow and the charter
  both require the human gate.
- Rejected re-running the 10 M2 exploratory cells. They are coverage history, not
  a new premise.
- Rejected a timeframe switch after a count. M4 declares 15m and stops if it is
  underpowered.
- Rejected `level_age <= N` as the primary gate because the age threshold would
  be arbitrary; `level_touches >= 1` uses the model's fixed level definition.
- Rejected level confluence `level_touches >= 2` as the primary gate because it
  is likely underpowered; it remains a possible future premise.
- Rejected `signal.ema_pair_cross` as the EMA cell because one event ID covers
  several EMA-pair sources; the direction at a bar with two opposite crosses is
  ambiguous. `signal.ema_stack_change` has one source and a directional claim.
- Rejected a new `quantrule.model-condition-gate/v1` schema in favour of
  extending the existing `model-state-gate/v1` with `min`, preserving the M3
  `values` contract.
- Rejected requesting validation now. It is a separate gate after a grade-A
  development cell.
- Did not claim an independent Quant Validator review; the first pass is a single
  agent and no run existed at that point.

## Approved implementation pass

The human approved the charter and plan on 2026-11-07 (UTC) with "aprove". The
implementation pass ran the pre-registered programme and stopped at `no_edge`.

### Changes

- Extended `tools/improvement/model_prediction.py` with `level_touches` and
  integer `min`/`max` gate bounds, keeping the M3 `values` contract and the
  `state_*` output fields.
- Added the gate tests: bounds, null, empty, all-bar, no-bar, invalid shapes,
  determinism, and the model-boundary test that the shell run never receives
  `--state-file`.
- Added `tools/research/build_m4_program.py` and the
  `PROGRAM-2026-11-07-01` manifest, five cells, 15m specs, gate files, and
  one-year run configs. The copied M3 sibling configs were migrated to the
  current `quantrule.portfolio/v2` and `quantrule.execution/v2` contracts.
- Added the M4 inherited-coverage path to `tools/research/coverage.py` and its
  test.
- Built `target/release/quantrule` from the worktree at `ce85fd0` plus the M4
  changes.
- Ran the count-only preflight, the five-cell development swarm, the
  aggregation, and a one-cell reproducibility rerun.
- Added `docs/improvement/2026-11-07/M4-DEV-SWARM.md`; updated the baseline,
  run index, model-family, README, research-config, evaluator-contract, and
  changelog records.

### Results

| Cell | n | Lift | LCB | Random p | Shuffle p | Increment | Grade |
|---|---:|---:|---:|---:|---:|---:|---|
| M4.1 | 36,957 | +0.0025 | −0.0040 | 0.1493 | 0.6468 | +0.0006 | C |
| M4.2 | 58,574 | +0.0035 | −0.0033 | 0.0448 | 0.5721 | −0.0002 | C |
| M4.3 | 19,109 | −0.0015 | −0.0099 | 0.6567 | 0.9104 | −0.0013 | D |
| M4.4 | 3,523 | +0.0157 | −0.0055 | 0.0498 | 0.0050 | +0.0069 | C |
| M4.5 | 2,947 | +0.0181 | −0.0044 | 0.0199 | 0.0448 | +0.0068 | C |

The count gate passed for every cell. No cell passed the development gate; the
programme decision is `no_edge`. `model_validation` was not opened.

### Verification

| Check | Observed result |
|---|---|
| `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` | 101 tests passed |
| `python3 -m unittest discover -s tools/research -p 'test_*.py'` | 45 tests passed |
| `python3 tools/research/program.py --program ... --check` | `ok PROGRAM-2026-11-07-01 cells=5 primary=5 phase=dev` |
| `python3 tools/research/coverage.py ... --family M4 --inherit ...` | `ok PROGRAM-2026-11-07-01 cells=5 indicators=46 events=143 inherited=PROGRAM-2026-11-05-03` |
| Count-only preflight | Every cell above 200 predictions and 20 clusters |
| `python3 tools/research/batch.py ... --phase dev --mode config --max-parallel 5` | `ok batch PROGRAM-2026-11-07-01 dev cells=5` |
| `python3 tools/research/report.py ...` | `no_edge`; every cell reported |
| M4.5 rerun | Same `prediction_digest`, metrics, criteria, unconditional metrics, and increment |
| `python3 tools/check_links.py` | 641 Markdown files; no broken relative links |
| `python3 tools/check_terms.py` | 391 files; no banned terms |
| `python3 tools/check_legacy.py` | 785 current files; clean |
| `python3 tools/check_structure.py` | structure ok |
| `python3 tools/check_workflows.py` | workflow ok |
| `python3 tools/check_changelog.py` | changelog ok; 16 entries |
| `python3 tools/check_partitions.py` | 5 instruments, 6 partitions, 4 aliases |

### Learning

- The location condition is real but small: the two closest cells add about
  0.68–0.69pp over the unconditional event lift, below the +1pp increment. The
  unconditional events themselves are weak at 15m after costs.
- The back-lag dominates the closest cells. `M4.4` has a +19.67pp back-lag and
  `M4.5` a +7.62pp back-lag, so the event's edge is not tied to the bar it fires
  on. Conditioning on location does not repair a timing defect.
- The count gate is cheap and decisive. It let all five cells run without
  opening an outcome, and it would have stopped the programme before the
  development run if the gate were too rare.
- The copied M3 sibling configs were on the retired `quantrule.portfolio/v1`
  and `allow_short` shape. The builder now migrates them to the current v2
  contract; the run would have failed without that migration.
- `M4.5` is the only cell positive in every development year, but it still fails
  the pooled lift and LCB with the corrected alpha. A year-stable weak effect is
  not a pass.

### Limitations

- This is a single-agent Quant Developer and Quant Validator pass; it is not an
  independent review.
- The run used the worktree's uncommitted changes on top of `ce85fd0`; the commit
  ID is recorded when the branch is committed. The worktree is preserved.
- The validation partition remains closed. No baseline, strategy, execution,
  portfolio, or live claim is made.
- The away-from-structure subset is not separately reported by the evaluator;
  the unconditional comparison and the location increment are the gate.
- The result is a negative for this premise, not a proof that location has no
  information in any event family or timeframe.

### Closeout

The human accepted `no_edge` on 2026-11-07 and authorized the commit, push, PR,
and merge with "proceed with everything". No M4 variant is re-registered; the
location premise is retired for this programme.
