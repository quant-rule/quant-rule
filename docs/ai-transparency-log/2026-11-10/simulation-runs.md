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
