# Execution Plan: Workflow Reset — Audit and Improvement Workflows

- Date: 2026-11-01 (UTC)
- Status: **Program green** — code, docs, tooling, and audit complete; first model run not green; cleanup pending.
- Work item: `workflow-reset`
- Baseline tag: `v0.5.0` at `42307b7`
- AI transparency log: [`docs/ai-transparency-log/2026-11-01/workflow-reset.md`](../../ai-transparency-log/2026-11-01/workflow-reset.md)

## 1. Intent

Replace the ambiguous single umbrella workflow with two explicit
workflows:

```
AUDIT WORKFLOW
  Audit -> Remediation Plan -> Execution -> Verification

IMPROVEMENT WORKFLOW
  Run -> Results -> Review -> New Hypothesis
    Model Run -> gate -> Strategy Run -> gate -> Portfolio Run -> gate -> baseline
```

The improvement workflow changes one tier at a time while the other engines stay
frozen, so every comparison is attributable. The audit workflow changes the
program and never mixes with a trading hypothesis.

## 2. Decisions

| Decision | Choice | Approval |
|---|---|---|
| Terminology | “Audit Workflow” and “Improvement Workflow”; the retired workflow name is not kept | Human approved 2026-11-01 |
| Autonomy | Autonomous execution and reporting; human accepts baselines and policy | Human approved 2026-11-01 |
| Holdout | 2025 M1 is the final portfolio holdout; 2023 crypto is excluded | Human approved 2026-11-01 |
| Model trust | A/B axes usable in strategies; C diagnostic; D blocks | Human approved 2026-11-01 |
| Cleanup | Delete invalidated runs/audits/experiments with tombstones | Human approved 2026-11-01 |
| First strategy candidates | v3 `ladder-context`, v4 `level-retest`, v5 `three-timeframe`, v6 `breakout` | Human approved 2026-11-01 |

## 3. Scope

Included:

- Rename and split the workflow documentation; rewrite the normative role,
  workflow, and record sections in `AGENTS.md`.
- Add the improvement run contract, manifest, data layout, index, baselines,
  invalidations, and holdout enforcement.
- Add model, strategy, and portfolio evidence protocols and run tooling.
- Add the v6 breakout strategy candidate.
- Add the audit cycle for the reset and run it to program green.
- Run the first model, strategy, and portfolio cycles after audit green.
- Delete invalidated pre-reset artifacts with tombstones.

Excluded:

- Live trading, deployment, release automation.
- Multi-asset portfolio.
- Trailing/partial exits in this cycle; v6 uses structural stops/targets.
- Fixing 2023 crypto data; 2023 is excluded by scope.
- New dependencies or new crates.
- Backwards compatibility with retired schemas, paths, and records.

## 4. Work breakdown

| Phase | Work item | Deliverable | Depends on |
|---|---|---|---|
| 0 | `workflow-reset` | Capture the pre-reset candidate code; tag `v0.5.0` | — |
| 1A | `workflow-reset-docs` | Two workflow docs, run contract, roles, AGENTS.md, term guard | 0 |
| 1B | `workflow-reset-run-store` | Manifest, store, index, baselines, invalidations, partitions | 1A |
| 1C | `workflow-reset-evidence` | Model, strategy, portfolio evidence harnesses | 1B |
| 1D | `workflow-reset-v6` | v6 breakout strategy candidate and tests | 1B |
| 2 | `workflow-reset-audit` | Audit, remediation, verification to program green | 1A–1D |
| 3 | `run-001` | Model run 001, strategy run 001, portfolio run 001 | 2 |
| 4 | `workflow-reset-cleanup` | Delete invalidated artifacts; tombstones; term/link gates | 3 |
| 5 | `model-improvement` | Next model hypothesis from run 001 evidence | 4 |

## 5. Data partitions

| Partition | Window | Use |
|---|---|---|
| `model_dev` | 2018-01-01..2020-12-31 | Model development |
| `model_validation` | 2021-01-01..2022-12-31 | Model confidence |
| `strategy_dev` | 2018-01-01..2022-12-31 | Strategy candidates |
| `strategy_validation` | 2024-01-01..2024-12-31 | One-shot strategy OOS |
| `portfolio_dev` | 2024-01-01..2024-12-31 | Portfolio variants |
| `portfolio_holdout` | 2025-01-01..2025-12-31 | Final portfolio holdout |

The runner refuses a development run that reads the holdout. Every holdout access
is recorded in the run manifest.

## 6. Acceptance criteria

- No active document uses the retired workflow name; the term guard passes.
- The run contract validates manifests and rejects holdout misuse.
- Model, strategy, and portfolio run folders and indexes are deterministic.
- The v6 breakout rule has causality, determinism, golden, and edge-case tests.
- The audit workflow reaches program green with no open Critical/Major finding.
- The first model, strategy, and portfolio runs are recorded with manifests,
  results, reviews, decisions, and baselines.
- Cleanup removes invalidated artifacts and records tombstones.
- `cargo fmt`, `cargo clippy --all-targets`, `cargo test`, `cargo doc`, link
  check, and term check pass.

## 7. Verification

| Check | Command | Expected |
|---|---|---|
| Format | `cargo fmt --all -- --check` | exit 0 |
| Lints | `cargo clippy --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test --workspace` | all pass |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| Links | `python3 tools/check_links.py` | 0 broken |
| Terms | `python3 tools/check_terms.py` | 0 banned terms |
| Manifest | `python3 tools/improvement/validate.py <manifest>` | valid accepted; invalid rejected |
| Holdout | validate a development manifest reading holdout | refused |
| Determinism | rerun a run and `diff -r` | byte-identical |

## 8. Outcome

- The Audit Workflow is green for the program scope; see
  [`docs/audits/2026-11-01/workflow-reset-audit.md`](../../audits/2026-11-01/workflow-reset-audit.md).
- The Improvement Workflow run store executed `MODEL-2026-11-01-01`.
- The model gate did not pass: Direction and Momentum hit rates are below 0.50
  in five of six 2021 cells; Structure and Progression are not yet graded. No
  model, strategy, or portfolio baseline is accepted.
- The next hypothesis is a model-improvement run, not a strategy run.
- Pre-reset records are invalidated with tombstones; physical deletion is the
  cleanup phase.

## 9. Risks and stop conditions

- Audit finding not fixable in scope: stop and replan.
- 2025 data unavailable: use 2024 H2 as fallback and record the limitation.
- Model axis D: stop before the strategy run and open a model hypothesis.
- No strategy green: record `no edge`; no portfolio optimization.
- Cleanup deletion: require the deletion manifest and human approval.
- Any threshold, cost, split, or criterion change after a result: stop; new
  pre-registration.
