# AI Transparency Log — Simulation Runs

- Date: 2026-11-10 (UTC)
- Work item: `simulation-runs`
- Plan: [`docs/execution-plans/2026-11-10/simulation-runs.md`](../../execution-plans/2026-11-10/simulation-runs.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash), implementation session
- Claim type: research tooling and exploratory labeling; no edge claim

## Entry 1 — Plan revision and implementation start

### Intent and boundary

- Turn the human's "10 propositions" into one approved simulation-run plan.
- Correct the model identity from `M1.2` to `M1.3`; the human supplied the
  correction after the first draft.
- Build the Phase 0 governance records and the Phase 1 harness; no Rust, model,
  strategy, execution, or portfolio behavior change.

### Work performed

- Read the existing annotation tooling, the M1.2 zone report, SR9/SR11/SR12
  decisions, the M1.3 contract, and the current registry.
- Corrected the plan to M1.3 and the merged base (`main` at `56c2beb`).
- Created the worktree `quant-rule--simulation-runs` and branch
  `feat/simulation-runs`.
- Wrote the approved plan of record, the simulation-research workflow, the run
  template, the data-store README, the workflow/tool/plan indexes, and this log.

### Verification

- `git worktree list` shows the task worktree on `feat/simulation-runs`.
- The M1.3 contract and registry `2026-11-10.2` are present on the base.
- No model, strategy, or engine file is modified at the time of this entry.

### Learning and corrections

- The first plan named `M1.2`; the human corrected it to `M1.3`. The plan now
  uses M1.3 for both the label and check windows and treats the M1.2 zone
  percentages as historical.
- The M1.3 branch was merged before the worktree was created, so the planned
  stacked base was unnecessary; the plan records the merged base instead.

## Entry 2 — Phase 0/1/2 implementation and SIM-02

### Intent and boundary

- Implement the approved harness, the M1.3 report, and the first real run
  (SIM-02, level-tap rejection).
- No Rust, model, strategy, execution, or portfolio change. The only generated
  artifacts are local simulation data.

### Work performed

- Created `tools/simulation/` with the bundle loader, deterministic window
  selector, renderer, mark schema, oracle, review, check, run orchestrator, and
  tests.
- Generated the M1.3 2018–2022 report as five per-year 15m bundles under
  `data/simulation/_data/M1.3-2018-2022/`. The source is the Binance BTCUSDT
  1m file; 2023–2025 were not read.
- Ran SIM-02: 48 windows selected from 192 candidates, 202 marks in 48 charts.
- Reviewed the context table and two charts; kept the oracle proposal. The
  review found the only entry-time separation is the rejection wick: long lower
  wick 0.37 good vs. 0.26 bad; short upper wick 0.34 vs. 0.27. EMA stack, RSI,
  ADX, and ATR ratio overlap.
- Ran the 2020–2022 check: long filtered mean net −0.0165% (n=1,902) vs. control
  −0.0433% (n=4,583); short filtered −0.0747% (n=1,692) vs. control −0.0767%.
  Verdict `no_pattern`; no validation or holdout access.

### Verification

- `python3 -m unittest discover -s tools/simulation -p 'test_*.py'`: 36 tests
  pass.
- `python3 -m unittest discover -s tools/trades -p 'test_*.py'`: 8 pass.
- `python3 -m unittest discover -s tools/charts -p 'test_*.py'`: 23 pass.
- `cargo test`: all suites pass (no Rust change).
- `check_structure.py`, `check_workflows.py`, `check_changelog.py`,
  `check_command_surface.py`, `check_registry.py`, and `check_legacy.py` pass.
- SIM-02 outputs: `manifest.json`, `windows.json`, `charts/`, `marks-ai.csv`,
  `marks-detail.csv`, `review.md`, `check.json`, `check.md`, `decision.md`.

### Learning and corrections

- `select.py` shadowed the Python standard library `select` module and broke
  `subprocess`; renamed to `window_select.py`.
- The first review used full-horizon MFE/MAE, which made the bad marks look
  better than they were; the review now uses the path to the rule exit and marks
  the perfect exit separately.
- The 400-bar chart is good for context but too wide for the rejection wick;
  the next run needs a zoom panel.
- The oracle's good/bad labels are forward-path outcomes; the context table is
  the only pre-exit discriminator, and it did not produce a positive screen.

## Entry 3 — Batch 1 (SIM-01, SIM-02, SIM-03) and zoom

### Intent and boundary

- Complete Batch 1 after the human approved proceeding.
- Add the zoom chart and the SIM-01/SIM-03 tools; no model, strategy, or engine
  change.

### Work performed

- Added a zoom render around the candidate (±30 bars) and used it for the contact
  sheet; the 400-bar context chart remains the mark-mapping surface.
- Added `meta.py` for SIM-01 and `exit_lab.py` for SIM-03, with tests.
- SIM-01: 810 human entries mapped; long 368 good / 120 bad, short 246 good / 76
  bad; best cell short/bear/ATR>=1 (n=38); check `no_pattern` (filtered −0.12%
  vs. control −0.09%, 1 positive half-year).
- SIM-03: 101 label entries, 3,328 check entries; time exit least bad on the
  check (long −0.02%, short −0.10%); fixed 2R and swing trail worse; verdict
  `no_exit_rescue`.
- Wrote `docs/reports/2026-11-11/simulation-batch-1.md` and updated the plan
  checklist and tool README.

### Verification

- `python3 -m unittest discover -s tools/simulation -p 'test_*.py'`: 44 tests
  pass.
- `check_structure.py`, `check_workflows.py`, `check_changelog.py`, and link
  checks pass after the report and workflow updates.
- The SIM-01 and SIM-03 records include the commands and numbers in
  `meta.md`/`meta-check.json` and `exit-lab.md`/`exit-lab.json`.

### Learning and corrections

- The zoom panel is necessary: the 400-bar chart hides the rejection wick.
- The human marks are positive by construction; the context split is the real
  test, and it did not generalise.
- The exit lab shows the entry, not the exit, is the weak part. The next batch
  should not search another exit threshold on the same entries.

## Entry 4 — Batch 2 (SIM-04, SIM-05, SIM-06)

### Intent and boundary

- Complete Batch 2 with the failed-break, retest-hold, and trend-pullback
  premises. No model, strategy, or engine change.

### Work performed

- Extended the check filter to support direction mapping, confirmation bars,
  close-back, EMA stack, ATR bucket, and RVOL.
- SIM-04: 32 windows, 116 marks; check long +0.0225% (n=419, hit 50.4%) vs.
  control −0.0289% (hit 51.9%), one positive half-year; short −0.1468%;
  `no_pattern`.
- SIM-05: 32 windows, 122 marks; check long −0.1294% (n=137), short +0.0486%
  (n=138, hit 42.0%, three half-years); `no_pattern`.
- SIM-06: 32 windows, 100 marks; the EMA-stack filter equals the event at the
  entry bar, so filtered equals control; long −0.0092%, short −0.0163%;
  `no_pattern`.
- Wrote `docs/reports/2026-11-11/simulation-batch-2.md` and updated the plan
  checklist and docs index.

### Verification

- `python3 -m unittest discover -s tools/simulation -p 'test_*.py'`: 45 tests
  pass, including the new direction/close-back filter test.
- The three run records contain `manifest.json`, `review.md`, `check.md`,
  `check.json`, and `decision.md`.

### Learning and corrections

- The close-back filter needs a confirmation bar; the check now delays the entry
  by one bar and reads the confirmation close.
- SIM-06's filter is the event itself; the check is a direction-only event
  study, not a filter comparison. Recorded as a limitation, not a pattern.
- Six runs are complete; the entry families tested do not produce a positive
  screen. The remaining decision is whether Batch 3 is worth running or the
  campaign should stop and redesign.
