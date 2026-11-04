# AI Transparency Log — DI Cross 15m Strategy Brief

- Date: 2026-11-04 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Plan: [`docs/execution-plans/2026-11-04/di-cross-15m.md`](../../execution-plans/2026-11-04/di-cross-15m.md)
- Issue: none; the human's task brief is the authority

## Intent and boundary

Execute one pre-registered 15m event-family test — the +DI/−DI crossover — on
top of the v4 model, with the event, timeframe, target, stop, horizon, filters,
and costs fixed in advance. The session acts as Quant Developer and Test
Engineer. It does not tune a failed gate, change shared model/execution code,
push, open a pull request, merge, or claim validation or an edge.

## Preflight (actual)

Commands run in the new worktree, observed results:

- `rg -n 'time_stop_bars' crates/quantrule-backtest/src/strategy/event.rs` →
  matches at lines 172, 182, 193, 245, 261–262, 368–372.
- `rg -n 'state_applicable' tools/improvement/path_validation.py` → matches at
  lines 372, 434, 437, 448, 551, 706, 752, 808, 846.
- `test -f data/model-v4/dev/btc-2018-adx25/model/readings.csv` → present.
- `data/model-v4` and the three `M1_2018-01-01_2024-12-31` source CSVs are
  symlinked into the worktree, not copied.
- `config/model-v4/dev/matrix.json` and `config/model-v4/validation/matrix.json`
  both list `adx20` and `adx25`.

Phase 0 is ready.

## Design decisions

- The strategy config, execution config, and portfolio config are transcribed
  verbatim from the brief; no value was chosen in this session.
- The model gate family declares `"state": "none"`, so the state-conditioned
  incremental test is not applicable and the lift/base comparison is the test.
- The strategy builder is a copy of `build_strategy_v9_matrix.py` with only the
  base, shared-config, name, output-root, and matrix-rule constants changed.
- No Rust or shared tooling change was needed.

## Worktree and pre-registration reuse (actual)

- `git worktree list` showed `feat/di-cross-15m` at `0c04618` ("chore:
  pre-register DI cross 15m protocol") with a prunable worktree entry whose
  directory no longer existed. The branch already contained the plan, configs,
  builder, and this log from the pre-registration step.
- Rather than discard the branch, I attached a fresh worktree to it:
  `git worktree add ../quant-rule--di-cross-15m feat/di-cross-15m` → exit 0,
  HEAD `0c04618`.
- `data/model-v4` and the three `M1_2018-01-01_2024-12-31` CSVs were symlinked
  into the worktree. `git status --short --branch` showed a clean tree before
  the run records.
- A direct `git worktree remove --force ../quant-rule--di-cross-15m` returned
  128 ("not a working tree") because the directory was already gone. The
  subsequent `git worktree add` succeeded without a destructive operation.

## Model dev gate (actual)

- A read-only event count over the nine `adx25` dev cells found 17,878
  type-3/subtype-2/source-17 events on 15m; this was a pre-run sanity check, not
  a gate result.
- `config/di-cross-15m/model/families-dev.json` and the dev matrix were copied
  into `data/improvement/2026-11-04/MODEL-2026-11-04-15/config/` before the run.
  The manifest carries `created_utc` `2026-11-04T18:20:12Z`.
- Command run (exit 0):

  ```text
  python3 tools/improvement/path_validation.py \
    --matrix config/model-v4/dev/matrix.json \
    --families config/di-cross-15m/model/families-dev.json \
    --out data/improvement/2026-11-04/MODEL-2026-11-04-15/artifacts \
    --timeframes 15m --horizon 20 --target-atr 2.0 --stop-atr 1.0 \
    --primary-cells 5 --seed 0
  ```

- `python3 tools/improvement/validate.py .../manifest.json` → `ok
  MODEL-2026-11-04-15`.
- Observed `path_validation.json`: selection `null`; `adx25` grade C; n 17,851;
  clusters 3,563; hit 0.3121; base 0.3073; lift +0.0048; lift LCB −0.0038;
  random p 0.0796; shuffle p 0.7612; back-lag −0.0716; replication 3. Failed
  checks: `lift`, `lift_lcb`, `random`, `shuffle`. `adx20` was identical and is
  labelled `sensitivity`.
- Artifact SHA-256: `path_validation.json`
  `54b55b621332b8a12b48274055a6e932b7be78fa2e76b1a3a3027c8d5341a0ef`;
  `path_validation.md`
  `ac5bc854a2219ddf4be781fd0c55eca24280e72418571b72342e9449338818bc`.
- Determinism rerun: the same command with `--out /tmp/di-model-dev-rerun`
  exited 0 and produced byte-identical files with the same SHA-256 values above.
- The stop condition fired. `MODEL-2026-11-04-20`,
  `STRATEGY-2026-11-04-06`, and `STRATEGY-2026-11-04-11` were not created or
  run. No Rust build, backtest loop, or strategy evaluator was run.

## Learning

- The development gate is a genuine no-edge result, not a harness failure: the
  command exited 0, both configs were evaluated, and the sensitivity event set
  was identical. There was no reason to rerun or tune.
- The pre-registration branch from an earlier session was reused, not recreated.
  This preserved the committed protocol and avoided a second branch with the
  same name.
- The brief's stop conditions were followed literally: a grade-C/null-selection
  dev gate ends the work item even though the event count, cluster count, and
  replication were ample. The tempting but rejected action was to inspect
  per-instrument or alternate barrier settings; the protocol forbids it and no
  such run was made.
- No independent Quant Validator was involved. This log makes no validation or
  edge claim.
