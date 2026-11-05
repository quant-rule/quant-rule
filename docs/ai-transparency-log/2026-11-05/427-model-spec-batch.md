# AI Transparency Log — Model-Spec Batch and Leaderboard (#427)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session; model DeepSeek V4.1 Flash
- Work item: #427 P4 model-spec batch and leaderboard
- Parent plan: [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](../../execution-plans/2026-11-05/410-m2-model-programme.md)
- Charter: [`docs/governance/2026-11-05/M2-CHARTER.md`](../../governance/2026-11-05/M2-CHARTER.md)
- Status: P4 implemented and dry-run tested; commit and pull request follow this
  record

## 1. Intent and boundary

Add the config-only model-cell runner the M2.x program pre-registered. One cell
runs four instrument specs, keeps one pooled prediction evaluation, and writes
the root result the research adapter reads. The work does not run the real
development swarm, does not open validation, and does not change a model or
strategy rule.

## 2. Implemented work

| Item | Detail |
|---|---|
| `tools/research/model_cell.py` | Runs four `report model` calls, one pooled `model_prediction`, root JSON/MD, spec ID, phase, prediction digest |
| `config/research/PROGRAM-2026-11-05-03/runs/` | Four instrument run configs with data, policy, model, strategy, execution, portfolio, analytics |
| `tools/research/cell.py` | `{phase}` substitution; `spec_id` and `prediction_digest` in `cell-result.json` |
| `tools/research/adapters.py` | Model result records spec ID, family, instrument, digest, and phase |
| `tools/research/report.py` | Model leaderboard columns: hit, base, lift, LCB, control p-values, replication, grade |
| `tools/research/test_model_cell.py` | Dry run with a deterministic fake shell and the real evaluator; dev and validation split tests |
| `docs/workflows/research-swarm.md`, `config/research/README.md` | Model-cell commands and config-only batch note |

The dry run uses `tools/research/tests/fixture_model_shell.py`, which writes a
synthetic known-edge prediction/readings bundle. The real evaluator grades it
A/pass on `model_dev`; the same fixture grades D on `model_validation` because
its timestamps are outside that split.

## 3. Verification and evidence

Observed on 2026-11-05 in the `feat/427-model-spec-batch` worktree:

| Command | Observed result |
|---|---|
| `python3 -m unittest discover -s tools/research -p 'test_*.py'` | 41 passed |
| `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` | 135 passed |
| `python3 tools/research/coverage.py --program config/research/PROGRAM-2026-11-05-03/program.json` | `ok ... cells=15 indicators=46 events=143` |
| `python3 tools/research/program.py --program ... --check` | `ok ... cells=15 primary=5 phase=dev` |
| `cargo test --workspace --quiet` | exit 0; every reported suite passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `python3 tools/check_links.py` | 539 files, no broken links |
| `python3 tools/check_terms.py` | 340 files, no banned terms |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |

## 4. Learning

- The cell command needs `{phase}` as well as `{config}` and `{out}`: the
  evaluator must record `model_dev` or `model_validation`, and the phase changes
  the split bounds.
- The dry-run fixture needed prediction episodes separated by more than the
  horizon. Dense predictions make one cluster per instrument and fail the
  `min_clusters` criterion even when the hit rate is perfect.
- The run configs are committed data, not hidden in the runner. They use the
  current v3 example strategy only because the shell validates a complete run
  config; the model report does not read the strategy.
- The evaluator is run once with four `--bundle` arguments, which gives pooled,
  per-instrument, and per-fold evidence in one deterministic artifact.

## 5. Rejected or deferred

| Suggestion | Decision | Reason |
|---|---|---|
| Run the real development swarm in P4 | Deferred | P5 runs after the runner and charter approval |
| Open validation in the runner | Rejected | `cell.py` enforces one-shot access; P4 does not promote |
| Let the cell agent read another cell result | Rejected | The swarm rule forbids it |
| Write a second evaluator | Rejected | The real `model_prediction.py` is reused |
| Use a code worktree for config-only cells | Rejected | `batch.py --mode config` is enough |

## 6. Boundary

This log does not claim:

- that any real M2.x cell has run;
- that the development swarm has started;
- that validation was opened;
- that an edge exists.

The next gate is the human's authorization to commit, push, and open the P4 pull
request. P5 starts only after that merge.
