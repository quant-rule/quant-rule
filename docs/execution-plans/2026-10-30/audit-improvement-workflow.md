# Audit and Improvement Workflow (Pre-Reset)

- Status: Superseded by the 2026-11-01 workflow reset; retained as history
- Date: 2026-10-30 (UTC)
- Work item: `audit-improvement-workflow`
- Branch: `feat/audit-improvement-workflow`
- Base: `main` at `2739a67`
- Human approval: the human approved the architecture and the full plan in the working session on 2026-10-30.

> This plan predates the 2026-11-01 reset. Terminology is updated for navigation;
> the current workflows are [`docs/workflows/audit-workflow.md`](../../workflows/audit-workflow.md)
> and [`docs/workflows/improvement-workflow.md`](../../workflows/improvement-workflow.md).

## Intent

Make QuantRule improve through an explicit workflow: the human sets a policy and an explainable strategy, the system measures, the Improvement Workflow runs experiments and triages results, the Audit Workflow audits and plans program fixes, the Engineering Swarm implements them, and the Quant Swarm validates trading/model changes. The program must remain deterministic, rule-based, and free of statistics in the signal path.

The work also makes the abstraction levels explicit:

1. Human policy blueprint.
2. Human strategy blueprint.
3. Engine contracts: model, execution, portfolio, analytics.
4. Implementation and runs.
5. Audits, experiments, plans, and verified closure.

## Current state

- The run config has seven files: `run.json`, `data.json`, `model.json`, `strategy.json`, `execution.json`, `portfolio.json`, `analytics.json`.
- `execution.json` owns stop/target ATR multiples; `strategy.json` owns only the timeframe and thresholds.
- No policy blueprint exists.
- No strategy explanation command exists.
- No audit, experiment, governance, or human improvement guide exists.
- Model/strategy separation is enforced by code shape but not by a named boundary test.
- The model publishes percentile diagnostics; the strategy currently reads only primary axis values. Statistical fields must not enter the signal path.
- The existing audit findings from the 2026-10-30 session are the seed for the first audit document after the workflow is established.

## Scope

Included:

- Policy blueprint schema, loader, validation, and `policy check`.
- Strategy blueprint schema, risk/reward move, `strategy explain`, and strategy-run config use.
- Model/strategy boundary tests and a no-statistics-in-signal-path guard.
- Improvement Workflow, Audit Workflow, Engineering, and Quant role definitions.
- Audit, experiment, plan, traceability, and closure documents/templates.
- Human improvement guide.
- ADRs for abstraction levels, the improvement workflow, and rule purity.
- AGENTS.md, README, docs index, PR/issue templates, and examples.
- Dry-run acceptance.

Excluded:

- Live trading, broker connectivity, or deployment.
- Statistical, machine-learning, or neural-network signal generation.
- Automatic tuning of model/strategy thresholds from backtest results.
- A combined multi-instrument portfolio. Each run is one ticket; the Improvement Workflow compares independent runs.
- News, weekend, or session filters.
- Any change to the four-axis model formulas beyond isolation/validation work.

## Assumptions and uncertainty

- The human owns `policy.json` and the strategy thesis; engine configs are technical and must be validated against the policy.
- One run is one instrument; multiple tickets are a run matrix, not a combined portfolio.
- The strategy remains `axis-agreement` for this work item. The change is configuration and explainability, not a new strategy hypothesis.
- Moving risk/reward to `strategy.json` changes configuration shapes and golden fixtures, not the numeric strategy logic if the same 2.0/3.0 values are used.
- The percentile diagnostics may remain if they are proven outside the signal path. If the audit finds a path into entry/exit, the implementation stops for a decision.

## Decisions

1. **Policy is the highest configuration layer.** `run.json` gains a `policy` reference and `data.json` gains an instrument ticket. The policy is validated against portfolio/strategy/execution/analytics.
2. **Strategy owns entry/exit and risk/reward.** `execution.json` keeps fill mechanics and costs. The execution engine still receives bracket multiples, but they are derived from the strategy risk block.
3. **Strategy is explainable.** `strategy.json` carries a human thesis and structured entry/exit/risk; `strategy explain` renders the human-readable contract.
4. **Statistics are banned from the signal path.** The model and strategy use deterministic arithmetic; analytics and validation own statistics.
5. **The Improvement Workflow is a thin control layer.** It runs experiments and recommends; it cannot merge, accept a trading change, or release.
6. **The Audit Workflow is cross-cutting oversight.** It audits/plans/verifies and cannot implement or approve model/strategy behavior.
7. **The human remains the final authority.** Material policy/strategy/config changes and releases require approval.
8. **Backward compatibility:** the run schema becomes `quantrule.run/v2`. Existing examples are migrated; old configs fail with a clear schema error rather than silently defaulting.

## Acceptance criteria

- A run validates a policy blueprint against the strategy, model, execution, portfolio, and analytics.
- `policy check --config <run.json>` validates and prints a deterministic result.
- `strategy explain --config <run.json>` prints the thesis, entry, exit, and risk/reward in human trader words.
- The strategy input boundary exposes only primary axis values and bar identity; no percentile, readiness, event, context, portfolio, or execution field is reachable.
- A regression test proves a statistical field cannot enter the strategy decision.
- The execution stop/target values come from `strategy.risk`; execution config contains mechanics/costs only.
- The existing axis-agreement strategy produces the same trades with the migrated 2.0/3.0 risk values.
- Audit, experiment, plan, and closure documents exist and are discoverable.
- The human guide explains policy, strategy, the matrix, triage, edge criteria, and stop conditions.
- The dry run executes a policy/strategy validation, a two-cell experiment, and an audit→plan→closure traceability check without changing code in the audit/plan phases.
- No open Critical finding is released without explicit human residual-risk acceptance.

## Work breakdown

| Step | Work | Roles | Paths |
|---|---|---|---|
| 1 | Write this plan and the AI transparency log | Orchestrator | `docs/execution-plans/2026-10-30/`, `docs/ai-transparency-log/2026-10-30/` |
| 2 | Add abstraction/loop/rule-purity ADRs and governance charter | Architect, Quant Researcher | `docs/adr/2026-10-30/`, `docs/governance/2026-10-30/` |
| 3 | Add policy schema, loader, cross-validation, `policy check` | Implementation, Test | `crates/quantrule-shell/src/config/`, `crates/quantrule-shell/src/command/` |
| 4 | Add strategy v2, risk move, `strategy explain`, strategy-run config | Implementation, Test, Quant Developer | `crates/quantrule-shell/src/config/`, `crates/quantrule-backtest/`, `crates/quantrule-shell/src/command/` |
| 5 | Enforce model/strategy boundary and rule purity tests | Test, Code Reviewer | `crates/quantrule-backtest/src/strategy/`, `crates/quantrule-model/` |
| 6 | Add audit/experiment/plan/role documents and human guide | Auditor, Planner, Architect | `docs/audits/`, `docs/experiments/`, `docs/roles/`, `docs/tutorials/` |
| 7 | Update AGENTS.md, README, docs index, PR/issue templates, examples | Orchestrator, Architect | `AGENTS.md`, `README.md`, `docs/README.md`, `.github/`, `config/examples/` |
| 8 | Dry run and verification | Test Engineer, Improvement Verifier | fixtures, shell tests, docs |
| 9 | Seed the first audit from the 2026-10-30 findings | Audit Agent | `docs/audits/2026-10-30/mvp-pre-release-audit.md` |

Dependencies: 1 → 2 → 3 → 4 → 5 → 8; 6 and 7 may proceed in parallel with 3–5 once the decisions are fixed; 9 follows 8.

## Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `cargo doc --no-deps`
- `policy check` and `strategy explain` shell tests.
- Policy cross-validation tests for each contradiction.
- Strategy/model boundary test and no-statistics-in-signal-path test.
- Golden backtest comparison for the migrated strategy risk values.
- Determinism checks for the new commands.
- Dry-run audit/experiment/plan traceability.

## Risks and recovery

- **Config migration:** v1 configs fail. Recovery: migrate all examples and fixtures in the same change; document the error.
- **Risk/reward move:** golden fixtures change if values differ. Recovery: keep 2.0/3.0; verify trade parity and review the diff.
- **Statistics boundary:** if a statistical field feeds a decision, stop and ask before changing model behavior. Recovery: isolate diagnostics in analytics or remove them.
- **Role proliferation:** keep the Improvement Workflow and Audit Workflow thin and read-only by default.
- **Overfitting:** experiments pre-register criteria; the improvement workflow may conclude “no edge”.
- **Multi-ticket scope:** independent runs only; a combined portfolio needs a new approved plan.

## Stop conditions

- The policy/strategy abstraction conflicts with an existing contract or makes a human-owned value impossible to express.
- A required model/strategy behavior change appears that is not covered by this plan.
- A statistical field is found in the signal path and cannot be isolated without changing model outputs.
- Tests cannot be kept passing without weakening them.
- A new dependency or destructive migration is required.
- Human approval is missing for a material config-schema or behavior decision.

## Implementation record

Implemented in this worktree:

- Policy schema/loader/cross-validation and `policy check`.
- Strategy v2 thesis and risk/reward; `strategy explain`; `strategy-run --config`.
- Run v2, data instrument, execution v2, config fingerprint, provenance hash.
- Model/strategy boundary guard and policy/strategy shell tests.
- ADRs, governance charter, role manuals, audit/experiment formats, human guide,
  seed audit, config smoke experiment.
- AGENTS.md, README, docs index, config/report docs, PR/issue templates, examples.

Verification run:

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | Passed; 0 failed |
| `cargo doc --no-deps` | Passed |
| `policy check` + `strategy explain` | Printed the expected fixture output |
| `policy_check_rejects_a_cross_config_contradiction` | Passed |
| Strategy boundary guard | Passed |
| v2 `backtest` on a temporary 100k-row EURUSD fixture | Exit 0; five bundles; `config_hash` recorded |
| `tools/experiment/run-matrix.sh` with two cells | `matrix 2 cells`; five bundles per cell |

Remaining, deliberately not implemented here:

- Remediation of the seed audit findings; the Planner must produce the fix plan
  and the Engineering/Quant swarms must execute it.
- A real market experiment/edge search. The SOLUSD source is not committed, so
  the config smoke is the only executed experiment.
- Issue/sub-issue/PR creation, pushes, and merges; the workflow commit exists on
  this branch, and the remaining operations are Git and GitHub approval gates.
- Independent Improvement Verifier closure; the findings are `Open`, not
  `Verified`.

The plan is **Approved and partially implemented**, not `Verified`.
