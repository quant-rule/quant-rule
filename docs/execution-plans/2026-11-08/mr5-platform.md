# MR5 Platform — Phase 1 Execution Plan

- Date: 2026-11-08 (UTC)
- Status: **Approved** (human, 2026-11-08) — platform delivery authorized; model
  research starts only after the MR5 charter approval
- Work item: task slug `mr5-platform`
- Charter: [`MR5-CHARTER.md`](../../governance/2026-11-08/MR5-CHARTER.md) (Proposed)
- Phase 2 plan: [`mr5-programme.md`](mr5-programme.md) (Proposed)
- AI transparency log: [`mr5-platform.md`](../../ai-transparency-log/2026-11-08/mr5-platform.md)
- Claim type: platform and evaluation delivery; no model result

## 1. Intent

Deliver the platform required to run a 10-hypothesis × 3-config MR5 grid on the
unchanged `M1.0` model, without starting the model research. The delivery ends
at a merged pull request and a proposed research charter for human approval.

## 2. Current state

- `M1.0` is the model; `M2.x`–`M4.x` are historical research families.
- The M4 evaluator supports a condition gate with `level_touches` and `min`.
- The research runner uses one shared model config per programme.
- There is no sequence, confluence, freshness, or path evaluator.
- No MR5 grid or charter exists.

## 3. Scope

Included:

- `MR` research prefix and `-Cnn` configuration grammar; history aliases.
- Per-cell model config in the model-cell runner; model config hash in
  provenance.
- Condition preparer for sequence, confluence, and column/freshness conditions.
- Descriptive path/expectancy evaluator.
- MR5 coverage checker and grid builder.
- Reference MR5 grid; tests.
- Power/MDE worksheet and Data Steward data review.
- Proposed MR5 charter and Phase 2 plan.
- Documentation, changelog, and AI log.

Excluded:

- Any 30-cell build or run.
- Any `model_validation` access.
- Any strategy, execution, or portfolio work.
- Any model engine, indicator, or event change.
- Any data source or partition change.

## 4. Decisions

| Decision | Consequence | Status |
|---|---|---|
| `MR` prefix instead of `M5.x` | Research families cannot be read as model versions | Approved |
| `-Cnn` suffix | Configuration variant is explicit and stable | Approved |
| Per-cell `model.json` | Model config is a research dimension; data/policy/execution/portfolio stay frozen | Approved |
| Condition preparer instead of a Rust event | No model change; existing events/levels compose the mechanism | Approved |
| Path report separate from the primary gate | Path evidence does not replace the hit-rate gate | Approved |
| Five primary base cells | Power/MDE supports five; ten needs fresh data | Proposed in the charter |

## 5. Acceptance criteria

- `MR` and `-Cnn` are documented and the historical aliases are recorded.
- The runner accepts a cell model config and records its hash.
- The condition preparer handles sequence, confluence, and column conditions
  with causality, determinism, edge, and gate tests.
- The path evaluator reports MFE/MAE, time-to-target/stop, and expected R.
- The builder produces a valid 2-cell reference grid; the MR5 coverage check
  passes with 46 inherited indicators and 143 inherited events.
- The power/MDE worksheet recommends five primary cells.
- The Data Steward review records validation reuse and fresh-data options.
- The MR5 charter and Phase 2 plan are Proposed.
- All tests and docs checks pass.
- The PR is merged; no model research has started.

## 6. Work breakdown

| Step | Work | Role | Paths |
|---|---|---|---|
| P1.1 | Naming and aliases | Architect | `docs/model/2026-11-05/model-families.md`, `docs/registry/history.md` |
| P1.2 | Per-cell model config and runner | Quant Developer | `tools/research/model_cell.py`, tests |
| P1.3 | Condition and path evaluators | Quant Developer, Test Engineer | `tools/improvement/model_condition.py`, `model_path.py`, tests |
| P1.4 | MR5 coverage and builder | Quant Developer | `tools/research/coverage_mr5.py`, `build_mr5_program.py`, tests |
| P1.5 | Power and data review | Statistical Methodologist, Data Steward | `docs/improvement/2026-11-08/MR5-POWER.md`, `MR5-DATA-REVIEW.md` |
| P1.6 | Charter and Phase 2 plan | Quant Researcher, Coordinator | `docs/governance/2026-11-08/MR5-CHARTER.md`, `docs/execution-plans/2026-11-08/mr5-programme.md` |
| P1.7 | Verification | Test Engineer, Code Reviewer | full gate |
| P1.8 | Commit, PR, merge | Release / Operations | Git delivery |

## 7. Verification

| Check | Command |
|---|---|
| Improvement tests | `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` |
| Research tests | `python3 -m unittest discover -s tools/research -p 'test_*.py'` |
| Reference manifest | `python3 tools/research/program.py --program config/research/reference-mr5/program.json --check` |
| Reference coverage | `python3 tools/research/coverage.py --program config/research/reference-mr5/program.json --family MR5 --inherit config/research/PROGRAM-2026-11-05-03/program.json` |
| Docs | `check_links`, `check_terms`, `check_legacy`, `check_structure`, `check_workflows`, `check_changelog`, `check_partitions` |
| Rust | `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` |

## 8. Risks and recovery

| Risk | Mitigation |
|---|---|
| The condition evaluator leaks the future | It reads only event/level availability and the prediction bar; causality tests |
| The per-cell model config breaks the shared run configs | The runner copies the config into an isolated prepared run directory; dry-run test |
| The MR5 grid becomes a search | The charter freezes the primary base cells and limits the exploratory round to hypothesis generation |
| The power worksheet is over-read | It is an approximation; the MDE table and the observed cluster counts are recorded |
| The data review is mistaken for a data change | It recommends; a fresh instrument requires Data Lifecycle |

## 9. Stop conditions

- A test or coverage check fails.
- The condition evaluator cannot be made causal and deterministic.
- The per-cell model config changes shared inputs or the model contract.
- The human declines the MR5 charter.
- A new dependency, data source, or model change is required.
