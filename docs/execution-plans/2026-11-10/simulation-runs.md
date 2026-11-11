# Execution Plan — Simulation Runs (Hindsight Labeling Program)

- Date: 2026-11-10 (UTC)
- Status: **Approved** — execution started 2026-11-10
- Work item: `simulation-runs` (no issue number; descriptive slug agreed with the human)
- Model identity: **`M1.3`**, registry **`2026-11-10.2`**
- Workflow: `docs/workflows/simulation-research.md` (Current; indexed)
- Owner: Quant Researcher (hypotheses), AI labeler (marks), Quant Validator (check), Human (acceptance)
- Claim type: research tooling and exploratory labeling; no edge claim
- Plan of record: `docs/execution-plans/2026-11-10/simulation-runs.md`
- AI transparency log: `docs/ai-transparency-log/2026-11-10/simulation-runs.md`
- Branch/worktree: `feat/simulation-runs` in `quant-rule--simulation-runs`
- Base: `main` at `56c2beb`, after PR #579 merged M1.3 and PR #580–#581 closed the audit and cleanup rules
- Data store: `data/simulation/2026-11-10/SIM-01..SIM-10/` (local; marks and PNGs are not committed)
- Approval: human instruction “proceed” on 2026-11-10, following the M1.3 plan revision

## 0. Model identity and base branch

`M1.3` is the current model identity. Its contract is
`docs/model/2026-11-10/model-m1.3.md`, registry `2026-11-10.2`. It corrects
`M1.2` in four ways that matter to this program:

1. sequence events record the pivot occurrence and the confirmation availability
   separately (`occurred_bar_index` != `available_bar_index`);
2. `level.fib_test` is a bar-range intersection test and emits one event per
   tested level, so several Fib events can share one bar;
3. the Fibonacci and band tolerances are fixed model constants (`fib_test_atr`
   0.2, `band_atr` 0.25), not config fields;
4. `band_age` is measured from the youngest level's confirmation bar.

PR #579 merged M1.3 into `main` at `68606dc`; PR #580–#581 followed. The
simulation worktree `quant-rule--simulation-runs` is based on `main` at
`56c2beb`, so M1.3 is the base and there is no stacked branch.

Base rules for the simulation work:

- The branch is `feat/simulation-runs` in `quant-rule--simulation-runs`, based on
  `main` at `56c2beb`.
- The simulation PR contains only the harness, docs, and local-data README
  changes. It does not modify Rust code or the M1.3 contract.
- If the M1.3 code cannot generate the 2018–2022 report without a code change,
  stop and route to Model Change. Do not patch M1.3 to unblock this program.

The existing `artifacts-v12` bundle is M1.2 and remains historical reference
only. The human marks are timestamps and are model-independent, but their
context must be recomputed under M1.3. The M1.2 zone report is historical and is
not used for a claim in this program.

## 1. Intent

Build one thin, deterministic simulation-run harness that lets the AI label
"perfect" entries and exits on historical charts with hindsight, using only one
or two model events or indicators per run. The marks use the same CSV schema as
the human annotator, the harness renders PNGs with the marks drawn on the
candles for human review, and the review extracts one simple, falsifiable rule
per run.

Run the ten pre-registered propositions sequentially on the development
partition, check every extracted rule on 2020–2022 under M1.3, and gate any
survivor into the normal Strategy Research workflow. This is exploration: it
generates hypotheses, it does not prove them. No model, strategy, execution, or
portfolio behavior changes; no validation or holdout data is opened; no edge is
claimed.

## 2. Current state (read-only findings)

- `tools/charts/trade_charts.py` renders charts around recorded backtest trades.
- `tools/charts/annotate.py` is the human click annotator; it writes
  `annotations.csv` with `chart,bar,marked_time_utc,action,quality,side,note`
  and validates the bar against the chart manifest.
- `tools/trades/annotations.py` extracts the 250-bar context around marks;
  `annotation_zones.py` and `exit_quality.py` describe zones and exits.
- Human marks: 1,620 rows in
  `data/improvement/2026-11-09/BINANCE-BTCUSDT/annotations-binance.csv`
  (488 long entries/exits, 322 short entries/exits), all `good`, 2018–2019.
- The M1.2 zone report
  (`docs/reports/2026-11-10/annotation-zone-analysis.md`) finds 91.2% of long
  entries and 87.9% of short entries have an EMA tap, 85.7% of both have a
  Fibonacci test, and the shape is mean-reversion. It also flags that all marks
  are `good`, so there is no bad-entry/bad-exit contrast. Those numbers are M1.2
  context and will be recomputed under M1.3 for SIM-01.
- SR9 (compression retest plus four filters) was underpowered; SR11
  (EMA/Fibonacci entry) found no edge; SR12 (VWAP/band fade) found no candidate.
  2024 remains closed. No strategy baseline exists.
- M1.3 is merged into `main` at `56c2beb`; the M1.2 bundles remain historical.
- Session events are not emitted in the current bundles because no session
  block is configured. The London session block is required for SIM-10.
- Partitions: `strategy_dev` 2018–2022; 2023 excluded; `strategy_validation`
  2024; `portfolio_holdout` 2025.
- No simulation-run workflow, tool, or store exists.

## 3. Scope

### Included

- New `tools/simulation/` harness: M1.3 bundle loading, deterministic window
  selection, clean chart rendering, marks validation, hindsight oracle, contact
  sheet, review tables, and tests.
- New `data/simulation/` local store with `README.md` and a row in
  `data/README.md`.
- New `docs/workflows/simulation-research.md`, indexed in
  `docs/workflows/README.md`, plus a record template.
- One M1.3 `report model` pass for 2018–2022, used for both the label and check
  windows. This is a model report, not a backtest.
- Ten simulation runs, executed in the order in section 8.
- Synthesis report, changelog entry, tool README, and AI transparency log.

### Non-goals

- No Rust changes; no model, strategy, execution, or portfolio code changes.
  M1.3 is already on `main`; this work consumes it.
- No new dependencies; the harness uses the Python standard library and the
  Pillow version already used by `tools/charts/`.
- No validation (2024) or holdout (2025) access; 2023 stays excluded.
- No strategy config, backtest, or baseline in this plan. A surviving pattern
  gets its own Strategy Research pre-registration.
- No changes to `tools/charts/annotate.py`, `trade_charts.py`, or the trade
  tools. They are reused, not rewritten.
- No claim of edge, no profitability claim, no promotion of a control.

## 4. Assumptions and uncertainty

- The M1.3 code on `main` can generate a 2018–2022 model report without a code
  change. If it cannot, stop and ask; do not patch M1.3 to unblock the check.
- The M1.3 registry (`2026-11-10.2`) and the report's event codes agree. The
  loader validates this before a run starts.
- The human marks are timestamps from M1.2-era charts. They remain usable, but
  every context value is recomputed from the M1.3 report. The M1.2 zone numbers
  are not carried forward as evidence.
- 30–60 windows per run is enough for a visual pattern. Some events are rare;
  a run with fewer than 30 non-overlapping candidates is recorded as
  underpowered and does not make a rule claim.
- The AI's visual judgment is fallible. The controls are the pre-registered
  criteria, the human corrections, the bad-example marks, and the 2020–2022
  check.
- The check is an event study, not a backtest. A positive screen is a reason to
  write a Strategy Research hypothesis, not evidence of an edge.

## 5. Decisions

| ID | Decision | Recommendation | Status |
|---|---|---|---|
| D1 | Where simulation runs live | `data/simulation/<date>/<run-id>/`, a new local store; not the improvement run store | Proposed |
| D2 | Workflow home | New `docs/workflows/simulation-research.md` (Proposed), not a section in Strategy Research | Proposed |
| D3 | How marks are produced | Hybrid: deterministic hindsight oracle plus AI visual review; pure visual only for a 12-chart calibration set | Proposed |
| D4 | Check window | 2020–2022 under M1.3; label window 2018–2019; no 2023–2025 | Proposed |
| D5 | Sample and window | 30–60 windows per run, 400 bars per chart, candidate event in bars 80–120, 60-bar exit horizon, seed 0 | Proposed |
| D6 | Dependencies | No new dependencies; reuse Pillow | Proposed |
| D7 | Base branch | `main` at `56c2beb`, with M1.3 merged by PR #579 | Approved |
| D8 | Human marks under M1.3 | Treat the marks as timestamps; recompute all context under M1.3; do not carry M1.2 zone numbers forward | Proposed |

D1–D5, D7, and D8 need explicit approval because they create a new record type,
a new workflow, and a stacked base. D6 is a constraint, not a choice.

## 6. Acceptance criteria

- The harness selects windows deterministically from a seed, renders the same
  chart twice to the same PNG hash, and round-trips marks without loss.
- One dry run on a fixture bundle produces `windows.json`, a chart PNG with
  marks, and a review table; tests pass.
- The M1.3 2018–2022 report is generated, validated (121 columns, registry
  `2026-11-10.2`, Fib multiplicity, sequence availability, no 2023–2025 rows),
  and its provenance records the M1.3 code commit and data version.
- The ten runs exist in order, each with a manifest, marks, charts, review, and
  decision. Each run states its premise, inputs, sample, mark criterion, exit,
  check result, and next action.
- No validation or holdout data is read; no model/strategy code is changed; no
  new dependency is added.
- The synthesis report lists every run, the pattern found or not found, the
  check result, and the next action. Any survivor has a Strategy Research
  hypothesis note with costs and a pre-registration.
- `CHANGELOG.md`, the workflow index, the data README, the tool README, and the
  AI transparency log are updated. Repo checks pass.

## 7. Work breakdown

### Phase 0 — Governance and setup (0.5 day)

1. Write `docs/workflows/simulation-research.md` with `Status: Proposed` and
   index it in `docs/workflows/README.md`. The workflow defines: premise ->
   select -> mark -> review -> check -> decide, the roles, the exploration
   boundary, and the "no edge claim" rule.
2. Write `docs/workflows/templates/simulation-run.md` (manifest and review
   skeleton).
3. Add `data/simulation/README.md` and a row in `data/README.md`.
4. Commit the execution plan to `docs/execution-plans/2026-11-10/` after
   approval; create the worktree and branch on the M1.3 base.
5. Start the AI transparency log.

Gate: human approves the workflow, the record store, and this plan.

### Phase 1 — Harness (1–2 days)

Create `tools/simulation/`:

| File | Responsibility |
|---|---|
| `bundle.py` | Load one M1.3 bundle's readings, events, levels, and provenance; validate columns, event codes, timeframe, date range, and registry version |
| `window_select.py` | Deterministic, seeded, stratified window selection with overlap exclusion; writes `windows.json`. Named to avoid shadowing the Python `select` module. |
| `render.py` | Clean chart: candles, volume, the run's one indicator panel, event markers, and marks; writes PNG + manifest geometry |
| `marks.py` | Same seven-column schema as the annotator; validation, atomic write, sorted output |
| `oracle.py` | Deterministic hindsight labeler: good/bad entry and perfect/rule exit under the run's declared criterion |
| `review.py` | MFE/MAE, forward returns, captured %, context at the mark, aggregate tables |
| `check.py` | Event study on the unmarked window: filtered vs. control, 6 bps proxy, half-years |
| `contact_sheet.py` | 3x4 thumbnail sheet for fast human review |
| `run.py` | Orchestrates select -> render -> oracle -> review for one run; no judgment |
| `tests/` | Unit tests and one golden fixture |

Harness rules:

- No network, no wall clock inside chart output, no filesystem access outside
  the repository, no new dependencies.
- Window selection sorts candidates by `(timestamp, event_index)` before
  sampling; the seeded sample is independent of dict/set iteration order.
- The renderer records `plot_left`, `plot_width`, `price_top`, `price_bottom`,
  `price_low`, `price_high`, `candles`, and `bars` in the same shape as the
  trade-chart manifest, so the annotator can map a click to a bar.
- The chart shows no existing backtest trade markers and no future shading.
  Only the run's one or two studies are drawn.
- Marks are validated against the chart manifest; a mark outside the window or
  on a missing bar is rejected. `note` is limited to 500 characters and carries
  the run id plus the visual/indicator reason.
- The loader rejects a bundle whose registry version is not `2026-11-10.2` or
  whose M1.3 event semantics are absent.

Gate: `python3 -m unittest discover -s tools/simulation -p 'test_*.py'` passes;
the dry run produces deterministic output.

### Phase 2 — M1.3 report (0.5 day, can run in parallel with Phase 1)

1. Reconstruct the M1.3 model config from the contract and the M1.3 branch.
2. Run `report model` for 2018, 2019, 2020, 2021, and 2022 (one range if the
   CLI supports it; one report per year otherwise). Record the exact command in
   `reproduce.md`.
3. Store the reports under `data/simulation/_data/M1.3-2018-2022/` with
   provenance.
4. Validate:
   - 121 reading columns;
   - registry `2026-11-10.2`;
   - `level.fib_test` and `level.band_test` present;
   - Fib events can be multiple per bar, one per tested level;
   - sequence events carry `occurred_bar_index` != `available_bar_index`;
   - fixed tolerances (`fib_test_atr` 0.2, `band_atr` 0.25);
   - timeframe 15m, coverage, no 2023–2025 rows.

Gate: the report is reproducible and documented. If it requires a model or code
change, stop and route to Model Change.

### Phase 3 — Batch 1 (recommended first)

- **SIM-01 — Meta-label the human marks.** Map the existing 2018–2019 human
  entries to the M1.3 readings; label each `good`/`bad` by the 50-bar forward
  path. Compare one context state (EMA stack) and one volatility state (ATR
  ratio or ADX). Recompute the zone profile under M1.3; do not reuse the M1.2
  percentages. Output a two-condition split and check it on 2020–2022.
- **SIM-02 — Level-tap rejection.** Select `level.test`/`level.fib_test`
  windows in 2018–2019; deduplicate Fib events by bar, ratio code, and level;
  mark rejection entries and the tempting bad entries; compare `good` vs. `bad`
  context; extract one rejection rule; check 2020–2022.
- **SIM-03 — Exit lab.** Take a fixed entry set (human marks or SIM-02 winners);
  mark the perfect exit with hindsight; compare fixed 2R, EMA20 trail, swing
  trail, time stop, and partial at 1R. Output the best simple exit and the
  captured-MFE table.

Gate: human reviews the batch's contact sheet and `review.md`; the batch
decision is recorded before Batch 2 starts. The plan authorizes the sequence,
so execution continues unless the human stops it or a stop condition triggers.

### Phase 4 — Batch 2

- **SIM-04 — Failed breakout / sweep reclaim.**
- **SIM-05 — Breakout retest, shallow only.**
- **SIM-06 — Trend pullback to the fast EMA.**

Gate: human review of the batch.

### Phase 5 — Batch 3

- **SIM-07 — Compression first thrust.**
- **SIM-08 — Divergence at a confirmed pivot.**
- **SIM-09 — Climax / absorption reversal.**
- **SIM-10 — Opening-range sweep/reclaim.** This run needs the London session
  block in the M1.3 report. If enabling it changes the model rather than adding
  a report configuration, stop and route to Model Change.

Gate: human review of the batch.

### Phase 6 — Synthesis and gate

1. Write `docs/reports/2026-11-11/simulation-synthesis.md`: each run's premise,
   sample, pattern, check result, and decision.
2. Rank survivors by simplicity (at most two conditions), sample size,
   stability across 2020–2022, and check evidence.
3. For each survivor, write a Strategy Research hypothesis note with the exact
   rule, costs, partitions, and a pre-registration. Do not write strategy code.
4. Update the improvement registry only if the human accepts a hypothesis for
   research; this is not a baseline.
5. Update the workflow status (`Proposed` -> `Current` only if approved), the
   changelog, and the AI transparency log.

## 8. Run definitions

All runs use BTCUSDT 15m, `strategy_dev` 2018–2022, and the M1.3 report. The
label window is 2018–2019; the check window is 2020–2022. The chart window is
400 bars, the candidate event sits in bars 80–120, and the exit horizon is 60
bars. Each run marks both sides and both qualities.

| Run | Proposition | Inputs (1–2) | Entry mark | Exit mark | Check |
|---|---|---|---|---|---|
| SIM-01 | Meta-label human marks | EMA stack; ATR ratio or ADX | Human entry | Human exit + 50-bar path | 2-condition split on 2020–2022 |
| SIM-02 | Level-tap rejection | `level.test` or `level.fib_test`; ATR | Rejection bar at the level | Swing / 2 ATR / 24 bars | Rejection rule vs. proximity control |
| SIM-03 | Exit lab | Human/SIM-02 entries; ATR | Fixed entries | Perfect exit + five rules | Captured MFE and net return on 2020–2022 |
| SIM-04 | Failed breakout / sweep | `level.break` or `signal.opening_range_sweep`; range | Reclaim bar | Opposite boundary / VWAP / 1.5 ATR | Sweep vs. genuine break control |
| SIM-05 | Breakout retest | `signal.compression_breakout` or `signal.breakout_retest_hold`; RVOL | Shallow retest hold | Measured move / EMA20 trail | Retest quality vs. breakout-only control |
| SIM-06 | Trend pullback | EMA20/50/200; ADX (`signal.trend_continuation`) | First pullback rejection | EMA cross / 2 ATR / 24 bars | Pullback vs. reversal control |
| SIM-07 | Compression first thrust | `signal.compression_breakout` or `signal.atr_compression`; RVOL | First expansion bar | Measured move / 2 ATR / 16 bars | First thrust vs. retest |
| SIM-08 | Divergence at pivot | `divergence.rsi_regular_*`; confirmed pivot | Rejection after confirmation | Prior swing / EMA20 / 1.5 ATR | Divergence vs. level-only control |
| SIM-09 | Climax / absorption | `signal.climax_reversal` or `signal.absorption_rejection`; VWAP or POC | Reversal bar | VWAP/POC / 1 ATR / 10 bars | Exhaustion vs. no-event control |
| SIM-10 | Opening-range sweep | `signal.opening_range_sweep`; OR high/low | Reclaim bar | Opposite OR / session close / 1.5 ATR | Sweep-reclaim vs. sweep-control |

### Mark criterion (pre-registered)

For a long candidate, use the event's available bar (never an earlier bar):

- **Good:** within 60 bars, MFE >= 2 ATR before MAE <= 1 ATR.
- **Bad:** within 60 bars, MAE >= 1.5 ATR before MFE >= 1 ATR.
- **Perfect exit:** the bar of MFE for a good mark; the best exit before the
  adverse move for a bad mark.
- **Rule exit:** the run's declared simple exit; the gap between the perfect
  exit and the rule exit is the exit efficiency.

Shorts mirror the criterion. The oracle proposes; the AI visual review accepts,
rejects, or moves the mark; the accepted marks are `marks-ai.csv`. Human
corrections are `marks-human.csv`. Rejected marks and reasons are recorded.

### Check criterion (pre-registered)

Extract at most two conditions (one event/state plus one filter) and one exit.
Run an event study on 2020–2022:

- at least 30 non-overlapping events;
- mean net 20-bar return > 0 after a 6 bps round-trip cost proxy;
- hit rate > 50%;
- the filtered set beats the unfiltered event control on mean return and hit
  rate;
- the direction holds in at least 3 of 5 half-years.

If the rule cannot be expressed in the current strategy config, route it to
Requirements & Design. A passing screen is a hypothesis for Strategy Research,
not an edge.

## 9. Verification

| Check | Command | Expected evidence |
|---|---|---|
| Harness unit tests | `python3 -m unittest discover -s tools/simulation -p 'test_*.py'` | All pass |
| Trade-tool regression | `python3 -m unittest discover -s tools/trades -p 'test_*.py'` | All pass |
| Chart-tool regression | `python3 -m unittest discover -s tools/charts -p 'test_*.py'` | All pass |
| Repo structure | `python3 tools/check_structure.py` | Pass |
| Workflow index | `python3 tools/check_workflows.py` | Pass |
| Changelog | `python3 tools/check_changelog.py` | Pass |
| Command surface | `python3 tools/check_command_surface.py` | Pass (no shell command change) |
| Rust regression | `cargo test` | Pass; no Rust change |
| Determinism | Run select/render twice; compare `windows.json` and PNG sha256 | Identical |
| Marks round-trip | Write, read, rewrite `marks.csv` | Identical rows |
| M1.3 identity | `python3 tools/check_registry.py`; report provenance | Registry `2026-11-10.2`; model `M1.3` |
| M1.3 report | Column count, event codes, Fib multiplicity, sequence availability | 121 columns; 149/150 present; occurred != available |
| Check data range | Read the report date range | No 2023–2025 rows |

Limitations to record in the synthesis: hindsight labels are not causal
evidence; the oracle is a heuristic; the AI visual review is subjective; the
check is a screen, not a fill simulation; rare events may be underpowered; the
human marks are all positive and the new negative labels are AI-generated.

## 10. Risks and recovery

| Risk | Mitigation | Recovery |
|---|---|---|
| The AI rationalizes patterns in hindsight | Pre-register the criteria; check on 2020–2022; human review; no validation access | Record no pattern and stop the run |
| The M1.3 report is blocked | Test the config on a small range first | Stop and route to Model Change; do not fall back to M1.2 silently |
| The M1.3 report is blocked | Test the config on a small range first | Stop and route to Model Change; do not fall back to M1.2 silently |
| The harness changes an existing tool | New `tools/simulation/` only; regression tests for charts/trades | Revert the harness PR |
| The new workflow is not approved | Keep the runs in `data/scratch/simulation/`; do not index or change the workflow | Stop and ask |
| Scope creep into strategy code | The plan produces hypotheses only; a survivor goes to Strategy Research | Stop and route to the owning workflow |
| The sample is underpowered | Record the count before marking; do not lower the floor after seeing results | Stop the run as underpowered |

## 11. Stop conditions

Stop and report when:

- a model, strategy, execution, or portfolio code/config change is required;
- the M1.3 base cannot be used;
- 2023, 2024, or 2025 data is needed;
- the M1.3 report cannot be reproduced or its registry does not match;
- a rule cannot be expressed in the current strategy config;
- a data-quality defect or a missing event code appears;
- the human stops the campaign or rejects the workflow/record type;
- a run's result is positive only after a post-hoc change to the mark or check
  criterion.

## 12. Approval

This plan is `Proposed`. On approval:

1. Mark the plan `Approved` and commit it to
   `docs/execution-plans/2026-11-10/simulation-runs.md`.
2. Create the worktree `quant-rule--simulation-runs` and branch
   `feat/simulation-runs` on the M1.3 base.
3. Execute Phase 0 and Phase 1; then the ten runs in the order above.

In Plan mode no files outside `/home/zantgo/.opencode/plan` are changed.
Execution requires switching to an implementation agent.

## 13. Implementation checklist

Use this table as the implementation checklist. `☐` means not started; mark
`☑` only when the evidence exists, and record the evidence path or command
result in the run record.

| # | Phase | Checklist item | Evidence / command | Done |
|---|---|---|---|---|
| 0.1 | 0 | Plan approved and committed to the repo | `docs/execution-plans/2026-11-10/simulation-runs.md` exists; status Approved | ☐ |
| 0.2 | 0 | Simulation workflow written and indexed | `docs/workflows/simulation-research.md`; `check_workflows.py` | ☑ |
| 0.3 | 0 | Run record template | `docs/workflows/templates/simulation-run.md` | ☑ |
| 0.4 | 0 | Data store and README row | `data/simulation/README.md`; `data/README.md` | ☑ |
| 0.5 | 0 | Worktree/branch on the M1.3 base | `git worktree list`; `feat/simulation-runs` based on `main` @ `56c2beb` | ☑ |
| 0.6 | 0 | AI transparency log started | `docs/ai-transparency-log/2026-11-10/simulation-runs.md` | ☑ |
| 1.1 | 1 | Harness modules exist | `tools/simulation/{bundle,window_select,render,marks,oracle,review,contact_sheet,run,check,meta,exit_lab}.py` | ☑ |
| 1.2 | 1 | Harness tests pass | `python3 -m unittest discover -s tools/simulation -p 'test_*.py'` | ☑ |
| 1.3 | 1 | Selection is deterministic | Same seed -> identical `windows.json` | ☑ |
| 1.4 | 1 | Render is deterministic | Same window -> identical PNG sha256 | ☑ |
| 1.5 | 1 | Marks round-trip | Write, read, rewrite `marks.csv` identical | ☑ |
| 1.6 | 1 | Dry run completes | Fixture produces windows, PNG, marks, review | ☑ |
| 1.7 | 1 | No new dependency | Import list/dependency files unchanged | ☑ |
| 2.1 | 2 | M1.3 2018–2022 report generated | `report model` commands in `reproduce.md` | ☑ |
| 2.2 | 2 | M1.3 provenance recorded | Code commit, registry `2026-11-10.2`, data version, model `M1.3` | ☑ |
| 2.3 | 2 | Columns and events validated | 121 columns; `level.fib_test` and `level.band_test` present | ☑ |
| 2.4 | 2 | Fib multiplicity validated | Multiple code-149 events on one bar where levels test | ☑ |
| 2.5 | 2 | Sequence availability validated | Sequence events have `occurred_bar_index` != `available_bar_index` | ☑ |
| 2.6 | 2 | Date range validated | No 2023–2025 rows | ☑ |
| 2.7 | 2 | Report stored | `data/simulation/_data/M1.3-2018-2022/` | ☑ |
| 3.1 | 3 | SIM-01 complete | Manifest, windows, marks, charts, review, decision | ☑ |
| 3.2 | 3 | SIM-01 check | M1.3 two-condition split on 2020–2022 | ☑ |
| 3.3 | 3 | SIM-02 complete | Manifest, windows, marks, charts, review, decision | ☑ |
| 3.4 | 3 | SIM-02 check | Rejection rule vs. proximity control on 2020–2022 | ☑ |
| 3.5 | 3 | SIM-03 complete | Manifest, marks, charts, review, decision | ☑ |
| 3.6 | 3 | SIM-03 check | Captured MFE and five-exit comparison on 2020–2022 | ☑ |
| 3.7 | 3 | Batch 1 review recorded | Human review in the batch record | ☐ |
| 4.1 | 4 | SIM-04 complete and checked | Manifest, marks, charts, review, decision, check | ☑ |
| 4.2 | 4 | SIM-05 complete and checked | Manifest, marks, charts, review, decision, check | ☑ |
| 4.3 | 4 | SIM-06 complete and checked | Manifest, marks, charts, review, decision, check | ☑ |
| 4.4 | 4 | Batch 2 review recorded | Human review in the batch record | ☐ |
| 5.1 | 5 | SIM-07 complete and checked | Manifest, marks, charts, review, decision, check | ☑ |
| 5.2 | 5 | SIM-08 complete and checked | Manifest, marks, charts, review, decision, check | ☑ |
| 5.3 | 5 | SIM-09 complete and checked | Manifest, marks, charts, review, decision, check | ☑ |
| 5.4 | 5 | SIM-10 complete and checked | Manifest, marks, charts, review, decision, check; session block recorded | N/A |
| 5.5 | 5 | Batch 3 review recorded | Human review in the batch record | ☐ |
| 6.1 | 6 | Synthesis report | `docs/reports/2026-11-11/simulation-synthesis.md` | ☑ |
| 6.2 | 6 | Survivor Strategy Research notes | One note per survivor with costs and pre-registration | ☑ |
| 6.3 | 6 | Registry update only if accepted | `docs/improvement/` entry; no baseline unless accepted | N/A |
| 6.4 | 6 | Changelog entry | `CHANGELOG.md` under `## [Unreleased]` | ☑ |
| 6.5 | 6 | AI transparency log complete | Decisions, corrections, evidence, rejected suggestions | ☑ |
| 7.1 | 7 | Repo checks pass | `check_structure.py`, `check_workflows.py`, `check_changelog.py`, `check_command_surface.py` | ☑ |
| 7.2 | 7 | Existing tool tests pass | `tools/trades`, `tools/charts` unittest suites | ☑ |
| 7.3 | 7 | Rust regression passes | `cargo test` | ☑ |
| 7.4 | 7 | No validation/holdout access | Report date range and read audit; 2023–2025 absent | ☑ |
| 7.5 | 7 | No model/strategy changes | Diff contains only tools, docs, data README | ☑ |
| 7.6 | 7 | PR opened and merged | Human gate; PR links the plan and AI log | ☐ |
| 7.7 | 7 | Worktree cleanup | After merge, per the section 9 cleanup exception | ☐ |


## 14. Closeout — 2026-11-11

- Campaign: ten simulation runs complete.
- Result: six `no_pattern`, one `no_exit_rescue`, one `underpowered`, one
  `blocked`, and one `candidate_screen` (SIM-08 bullish divergence long).
- Disposition: the candidate is retired as a variant of the failed
  divergence-at-pivot programme (`PROGRAM-2026-11-04-01`). No model or strategy
  baseline is opened; no 2024 access.
- Evidence: `docs/reports/2026-11-11/simulation-synthesis.md` and the three batch
  reports.
- Remaining gates: human review of the closeout and a PR decision. The worktree
  and branch are retained because the task is not merged.
