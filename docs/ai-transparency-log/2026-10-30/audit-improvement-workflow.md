# AI Transparency Log — Audit and Improvement Workflow

- Date: 2026-10-30 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: `audit-improvement-workflow`
- Plan: `docs/execution-plans/2026-10-30/audit-improvement-workflow.md`

## Intent and boundary

The human asked for an audit/continuous-improvement workflow with explicit policy
and strategy blueprints, model/strategy separation, no statistics in the signal
path, a governing loop, the operating swarms, and a full implementation plan.
The plan was approved and the implementation was authorized for this worktree.

The work stayed inside the approved plan. No code was changed in the audit/plan
artifacts. No commits, pushes, issues, or PRs were created because those remain
separate Git/GitHub approval gates.

## Implemented

- Added `policy.json` parsing, validation, and `policy check`.
- Added strategy v2 with a required thesis and risk/reward; moved stop/target
  ownership to the strategy; `execution.json` now carries only mechanics/costs.
- Added `strategy explain`.
- Added run schema v2, data `instrument`, and policy cross-config validation.
- Added a deterministic config fingerprint to provenance.
- Added a source-level boundary guard for `StrategyInput`.
- Added a deterministic experiment matrix runner.
- Added a proposed Planner dry-run for `AUD-2026-10-30-01`; it is not approved
  or implemented.
- Added ADRs for abstraction levels, the governing swarm, and rule purity.
- Added the governance charter, role manuals, audit/experiment formats, human
  improvement guide, and a seed MVP audit.
- Updated `AGENTS.md`, README, docs index, config/report docs, PR template, and
  the remediation issue template.
- Migrated the SOLUSD example and test fixtures to the v2 config shape.

## Verification and evidence

Commands actually run in this worktree:

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | Passed, no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed, no warnings |
| `cargo test --workspace` | All test binaries passed; 0 failed |
| `cargo doc --no-deps` | Passed, no warning |
| `quantrule policy check --config config/examples/solusd-2025/run.json` | `policy ok ...` |
| `quantrule strategy explain --config config/examples/solusd-2025/run.json` | Printed thesis, entry, exit, and risk lines |
| `policy_check_prints_the_validated_envelope` | Passed |
| `strategy_explain_prints_the_trader_rule` | Passed |
| `policy_check_rejects_a_cross_config_contradiction` | Passed |
| `the_strategy_input_exposes_only_primary_axis_values_and_identity` | Passed |
| `backtest` on a 100k-row EURUSD fixture with a temporary v2 config | Exit 0; five bundles; `provenance.json` records a 16-hex `config_hash` |
| `tools/experiment/run-matrix.sh` with two manifest cells | `matrix 2 cells`; each cell has five bundles and `backtest.json` |
| `tools/experiment/run-matrix.sh` syntax and manifest validation | Passed |
| `policy_check_prints_the_validated_envelope` / `strategy_explain_prints_the_trader_rule` | Passed |
| `docs/experiments/2026-10-30/EXP-2026-10-30-01-config-smoke.md` | Recorded; no market claim |

Not verified: a full `backtest` on the SOLUSD example could not run because the
2025 SOLUSD CSV is not committed; a 100k-row EURUSD v2 backtest did pass, but it
is not a committed example. The data-dependent examples and the first market
experiment remain open. The audit findings are recorded as `Open`, not fixed.

## Learning and rejected suggestions

- Rejected automatic threshold optimization in the Governing Swarm. It would be
  statistical tuning of a rule-based signal path and would invite data snooping.
- Rejected putting policy inside `portfolio.json`; the policy is a human-owned
  envelope, not an accounting contract.
- Kept model percentile/readiness diagnostics outside the strategy boundary
  instead of deleting them in this change; a source-level guard now makes the
  boundary explicit. If a diagnostic ever enters the signal path, it requires a
  separate approved model-contract change.
- Kept the first audit findings open. The workflow is established; remediation
  belongs to the Planner and a separate execution plan, not to this work item.
