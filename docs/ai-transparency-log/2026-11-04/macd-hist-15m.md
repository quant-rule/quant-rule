# AI Transparency Log — MACD Histogram Flip 15m

- Date: 2026-11-04 (UTC)
- Tool: OpenCode agent, model `opencode-go/deepseek-v4.1-flash`
- Work item: `docs/execution-plans/2026-11-04/macd-hist-15m.md`
- Branch: `feat/macd-hist-15m`
- Worktree: `/home/zantgo/Proyectos/quant-rule--macd-hist-15m`
- Roles performed: Quant Developer and Test Engineer, plus orchestrator and
  result review. These were sequential passes by one agent; no independent
  agent or independent model judgment is claimed.

## Entries

### 1. Preflight and worktree setup

- Intent: verify the Phase 0 prerequisites and create an isolated task
  worktree without touching `main` or the other session's worktree.
- Commands and observed results:
  - `git status --short --branch` → `## main...origin/main`.
  - `git worktree list` → main at `fec0a93` plus a separate
    `feat/ema50-test-15m` worktree.
  - `rg -n 'time_stop_bars' crates/quantrule-backtest/src/strategy/event.rs` →
    matches at lines 172–369.
  - `rg -n 'state_applicable' tools/improvement/path_validation.py` → matches
    at lines 372–846.
  - `test -f data/model-v4/dev/btc-2018-adx25/model/readings.csv` → exit 0.
- Worktree: `git worktree add -b feat/macd-hist-15m ...` first failed because
  the branch already existed at `fec0a93` with no worktree. I attached the
  existing branch with `git worktree add ../quant-rule--macd-hist-15m
  feat/macd-hist-15m` rather than recreating or deleting it.
- Data: symlinked `data/model-v4` and the three M1 source files from the main
  checkout into the worktree. `data/**` is ignored by Git.
- Learning: the branch name existed before the worktree; attaching it was
  safer than forcing a new branch.

### 2. Reading the approved inputs

- Intent: work from the project instructions, the v8 strategy contract, the run
  contract, and the existing run records rather than from memory.
- Read: `AGENTS.md`; `docs/config/2026-11-03/strategy-v8.md`;
  `docs/workflows/run-contract.md`; `docs/workflows/improvement-workflow.md`;
  the `MODEL-2026-11-04-04` and `STRATEGY-2026-11-04-01` run records;
  `tools/improvement/path_validation.py`;
  `tools/improvement/build_strategy_v9_matrix.py`;
  `tools/improvement/strategy_v9_evaluate.py`;
  `crates/quantrule-backtest/src/strategy/event.rs`.
- Verified the event code: `MACD_HISTOGRAM_SOURCE_CODE: u16 = 15` and
  `SignalSubtype::HistogramSlopeFlip = 4` in
  `crates/quantrule-model/src/events/signal.rs`.
- Counted the 15m type 3 / subtype 4 / source 15 events in the `adx25` packs:
  5,044 to 9,439 per instrument-year, 60,328 selected in the dev gate.

### 3. Pre-registration artifacts

- Intent: write the plan and every input before seeing a result.
- Wrote the execution plan as `Approved`, the strategy/execution/portfolio
  configs under `config/macd-hist-15m/strategy/shared/`, and the family lists
  under `config/macd-hist-15m/model/`.
- Wrote the `MODEL-2026-11-04-16` manifest before the run and validated it:
  `python3 tools/improvement/validate.py
  data/improvement/2026-11-04/MODEL-2026-11-04-16/manifest.json` → `ok
  MODEL-2026-11-04-16`.

### 4. Model development gate

- Intent: run the pre-registered 15m barrier test exactly once on `model_dev`.
- Command:
  `python3 tools/improvement/path_validation.py --matrix
  config/model-v4/dev/matrix.json --families
  config/macd-hist-15m/model/families-dev.json --out
  data/improvement/2026-11-04/MODEL-2026-11-04-16/artifacts --timeframes 15m
  --horizon 20 --target-atr 2.0 --stop-atr 1.0 --primary-cells 5 --seed 0`
- Observed: exit 0, `ok path validation -> ...`. The artifact reports
  `selection: null`; `adx25` grade C with 60,328 events, 74 clusters, hit
  0.3151, base 0.3073, lift +0.0078, lift LCB −0.0126, random p 0.0050,
  back-lag lift −0.0019, shuffle p 0.3532, replication 3. Failed checks:
  `lift`, `lift_lcb`, `shuffle`.
- Determinism: reran the same command into
  `data/macd-hist-15m/determinism/model-dev` and compared SHA-256 digests. Both
  files matched the recorded artifacts exactly (`c82f1298…` JSON and
  `465722c1…` Markdown). This was a reproducibility check, not a second draw.
- Decision: the brief's model gate failed. I did not open `model_validation`,
  did not create a strategy matrix, and did not run a backtest.
- Verification: recorded the exact command, exit code, streams, and SHA-256
  digests in `results.json`; wrote `review.md` and `decision.md`.

### 5. Stop and documentation

- Intent: make the stop auditable without turning it into a tuning loop.
- Wrote `docs/improvement/2026-11-04/MACD-HIST-15M-CANDIDATE.md` with the
  hypothesis, gate table, model detail, verdict, and limitations.
- Updated the execution plan's Outcome section with the observed result.
- Did not edit `docs/improvement/index.md`, `baselines.md`, or
  `INVALIDATIONS.md`; the brief did not authorize those shared files and no
  baseline or invalidation was created.

## Rejected or corrected suggestions

- Rejected rerunning the dev gate with a different seed or draw count after
  seeing the C grade. That would be post-hoc tuning and is outside the brief.
- Rejected opening `model_validation` or a strategy run to "see" whether the
  event behaves differently. The brief requires a model dev pass first.
- Rejected changing the event, target, stop, horizon, timeframe, filters, or
  costs after the result.
- Decided not to create or run
  `tools/improvement/build_macd-hist-15m_matrix.py`. The builder is part of the
  strategy stage, and the model gate failed before that stage. Creating an
  unexercised builder would add dead code and imply a stage that was not
  opened. The pre-registered strategy configs remain in the tree as inputs, but
  no strategy matrix or backtest was produced.
- Did not make any Rust change. The event, model, execution, portfolio, policy,
  and analytics code are unchanged.

## Verification boundary

- The numbers in this log and in the candidate document were read from
  `data/improvement/2026-11-04/MODEL-2026-11-04-16/artifacts/path_validation.json`
  and its markdown rendering, not reproduced by a second independent run.
- No independent Quant Validator record exists. The human remains responsible
  for accepting or rejecting the `no edge` stop.
- The run folder is local and ignored by Git; the committed evidence is the
  plan, the candidate document, the configs, and this log.
