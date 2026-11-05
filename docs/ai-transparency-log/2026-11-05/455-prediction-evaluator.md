# AI Transparency Log — Model Prediction Evaluator (#455)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session; model DeepSeek V4.1 Flash
- Work item: #455 P2 prediction evaluator
- Parent plan: [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](../../execution-plans/2026-11-05/410-m2-model-programme.md)
- Contract: [`docs/analytics/2026-11-05/model-prediction.md`](../../analytics/2026-11-05/model-prediction.md)
- Status: P2 implemented and tested in the `feat/455-prediction-evaluator`
  worktree; commit and pull request follow this record

## 1. Intent and boundary

Evaluate a v2 model prediction against the forward path after costs, with a
direction-matched base, cluster-robust lift, controls, per-instrument and
per-year views, and a grade. The evaluator is an offline Python tool; it does
not change the model, the strategy, the shell, or any rule. It never opens the
validation partition; the caller declares the partition and the bounds.

## 2. Implemented work

| Item | Detail |
|---|---|
| `tools/improvement/model_prediction.py` | Forward/barrier outcomes, costs, split bounds and purge, base rate, cluster-robust lift, Wilson fallback, random/shuffle/back-lag controls, per-instrument/per-year, grade A–D |
| `tools/research/adapters.py` | `model_prediction` parser |
| `tools/research/program.py` | Registers the new parser |
| `tools/improvement/test_model_prediction.py` | 9 tests: outcomes, cost, stop tie, known edge, lagged failure, empty, determinism, splits |
| `tools/research/test_adapters.py` | Passing and failing adapter records |
| `docs/analytics/2026-11-05/model-prediction.md` | Contract |
| `tools/improvement/README.md`, `docs/workflows/research-swarm.md` | Tool and command references |

The evaluator reuses `path_validation.cluster_stats`, `control_p`, `percentile`,
and `lower` rather than creating a second statistics stack. The Wilson fallback
is used only when every prediction is its own cluster; `lcb_method` makes the
choice visible.

## 3. Verification and evidence

Observed on 2026-11-05 in the `feat/455-prediction-evaluator` worktree:

| Command | Observed result |
|---|---|
| `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` | 135 passed |
| `python3 -m unittest discover -s tools/research -p 'test_*.py'` | 33 passed |
| `cargo test --workspace --quiet` | exit 0; every reported suite passed |
| `python3 tools/check_links.py` | 535 files, no broken links |
| `python3 tools/check_terms.py` | 339 files, no banned terms |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |

The known-edge test produces a 0.50 lift and passes the random and shuffle
controls; the lagged prediction fails with a negative lift; the empty
prediction fails safely; the split-end test purges crossing outcomes. The
evaluator is run twice in the known-edge test and the JSON is identical.

## 4. Learning

- The path-validation harness already had cluster-robust statistics, the
  add-one control p-value, the percentile, and the corrected z. Reusing them
  kept the evaluator consistent with the accepted model gate.
- All-singleton clusters leave the cluster variance undefined. The evaluator
  falls back to the Wilson lower bound and records `lcb_method: "wilson"` rather
  than silently treating the lift as significant.
- The first known-edge test graded C because the default correction was five
  cells. The test passes `--primary-cells 1`; the production default stays five.
- Purge is explicit: a prediction whose outcome exit is after `split_end` is
  removed. The evaluator cannot open a partition itself, so the caller still owns
  the partition bounds.

## 5. Rejected or deferred

| Suggestion | Decision | Reason |
|---|---|---|
| Build a second statistics module | Rejected | The path-validation helpers already encode the accepted conventions |
| Treat a single instrument as replication | Rejected | A single instrument can grade B, not A |
| Bootstrap the lift lower bound | Deferred | The cluster/Wilson path is simpler and already recorded; a bootstrap can be added when the charter requires it |
| State-conditioned base | Deferred | The M2 programme uses the unconditional matched base first; a state base is a later refinement |
| Model fills, position size, or P&L | Rejected | Those belong to strategy, execution, and portfolio |
| Open validation automatically | Rejected | Validation is one-shot and human-approved |

## 6. Boundary

This log does not claim:

- that any model passes the M2 gate;
- that validation was opened;
- that an edge exists;
- that the M2 charter or swarm has run.

The next gate is the human's authorization to commit, push, and open the P2 pull
request.
