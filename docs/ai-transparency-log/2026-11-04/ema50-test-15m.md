# AI Transparency Log: EMA50 Test 15m

- Date: 2026-11-04 (UTC)
- Tool: OpenCode, model `deepseek-v4.1-flash`
- Work item: `ema50-test-15m` (no issue number)
- Plan: [`docs/execution-plans/2026-11-04/ema50-test-15m.md`](../../execution-plans/2026-11-04/ema50-test-15m.md)
- Roles performed: Quant Developer and Test Engineer, sequential passes in one
  session. No independent review is claimed.

## Entries

### 2026-11-04 — Preflight and worktree

- Intent: verify Phase 0, create the task worktree, and link the frozen data.
- Boundary: read-only preflight except the worktree, the data symlinks, and the
  files the brief assigns.
- Action: ran the three preflight probes; pruned stale worktree metadata whose
  directories no longer existed; created
  `/home/zantgo/Proyectos/quant-rule--ema50-test-15m` on
  `feat/ema50-test-15m` from `main` at `fec0a93`; linked `data/model-v4` and the
  three M1 source CSVs from the main checkout.
- Verification: all three preflight probes exited 0; `git worktree list` shows
  the task worktree; the symlinked CSVs resolve and their first data rows read
  back.
- Learning: the branch and worktree had been registered before but the directory
  was gone, so the branch was checked out rather than created. Pruning removed
  metadata only; no branch, commit, or file was deleted.

### 2026-11-04 — Configuration and builder

- Intent: write the pre-registered strategy, execution, portfolio, and model-gate
  families exactly as the brief fixes them, plus the matrix builder.
- Boundary: only `config/ema50-test-15m/**` and
  `tools/improvement/build_ema50-test-15m_matrix.py`; no shared file is edited.
- Action: wrote the four fixed JSON files verbatim from the brief; copied the v9
  builder and changed only the paths, name, output root, and matrix rule.
- Verification: files read back and match the brief. The builder was run once to
  materialize the pre-registered `dev` and `validation` cell configs; no backtest
  was executed. `quantrule policy check` on `dev/btc-2018/run.json` prints
  `policy ok` with `strategy=event-composite execution=15m ladder=15m,1h
  stop_atr=1 reward_to_risk=2`, and `quantrule strategy explain` prints
  `entry-all event tf=900 type=1 subtype=1 source=2 direction=either`,
  `time_stop_bars 20`, `risk stop_atr=1 reward_to_risk=2`.
- Learning: the run-store `run.py` path-validation command does not forward
  `--target-atr` or `--stop-atr`, so it cannot express the pre-registered 2.0/1.0
  barrier. I invoked the harness directly and assembled the run folder in the
  run-contract layout by hand, recording the exact command in `results.json`.
  I rejected the alternative of running `run.py` with its 1.0/1.0 default, which
  would have silently changed the registered protocol.

### 2026-11-04 — Model development gate

- Intent: test the EMA50 15m test family on `model_dev` against the
  direction-matched barrier base and the EMA50-side state.
- Boundary: evaluation only; no model, data, strategy, execution, portfolio,
  policy, or analytics change.
- Action: ran
  `python3 tools/improvement/path_validation.py --matrix config/model-v4/dev/matrix.json --families config/ema50-test-15m/model/families-dev.json --out data/improvement/2026-11-04/MODEL-2026-11-04-14/artifacts --timeframes 15m --horizon 20 --target-atr 2.0 --stop-atr 1.0 --primary-cells 5 --seed 0`.
  Both ADX settings were evaluated (no `--config`).
- Verification: exit 0; `path_validation.json` has `selection: null`, primary
  config `adx25`, grade C, n = 33,318, clusters = 2,763, lift −0.0020,
  incremental lift −0.0066. A byte-identical rerun into a temp directory
  reproduced both artifact SHA-256 digests; `validate.py` prints
  `ok MODEL-2026-11-04-14`; `python3 -m unittest test_path_validation` runs 23
  tests `OK`.
- Learning: the result is a clean falsification: the family is well sampled but
  does not beat either base or the negative controls. I did not look for a
  passing subset, alternate barrier, or alternate timeframe; doing so would
  break the pre-registration.

### 2026-11-04 — Stop at the model gate

- Intent: record no edge and stop the brief.
- Boundary: no validation run, no strategy run, no portfolio run.
- Action: wrote `results.json`, `review.md`, `decision.md`, and the candidate
  document; left `MODEL-2026-11-04-19`, `STRATEGY-2026-11-04-05`, and
  `STRATEGY-2026-11-04-10` uncreated.
- Verification: run folder contains manifest, config copies, results, review,
  decision, and two artifacts; manifest validates.
- Learning: a failed model gate is the expected majority outcome; the honest
  record is more useful than a retry. The next step is a human decision on
  whether to open a new hypothesis, not a parameter change.
