# AI Transparency — Strategy Boundary ADR

- Date: 2026-11-26 (UTC)
- Work item: `REM-STR-003` ([#1229](https://github.com/quant-rule/quant-rule/issues/1229))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: ADR (Architecture Decision); no model, strategy, signal, or edge
  claim.

## 1. Intent and boundary

Record the human-approved D4 decision: keep signal computation in
`quantrule-app::signal` and do not create `crates/quantrule-strategy` in
v1.6.1. The work records the decision, the dependency graph, and the
no-behavior-change boundary. It does not move code, change a hash, or amend the
architecture document; `REM-STR-004` owns the plan and architecture amendment.

## 2. Implemented

- `docs/adr/2026-11-26/1229-strategy-boundary.md` records the context, decision,
  alternatives, consequences, invariants, and verification, with D4 as the human
  approval.
- The dependency graph shows the signal module using the app pack/config/stage
  types and the backtest decision types, with the strategy service as a
  stateless adapter over it.
- The plan records V-STR-03 as `Pass` and leaves the plan/architecture
  deviation to STR-004.
- No crate, source file, command, schema, or artifact was changed.

## 3. Verification

Commands actually run in this worktree:

- `cargo test -p quantrule-app --lib signal::` → 24 passed.
- `cargo test -p quantrule-app --test signal_artifact_public` → 6 passed.
- `cargo test -p quantrule-strategy-service` → 2 unit, 3 boundary, 10 signal
  tests passed.
- `cargo metadata --no-deps` → no package named `quantrule-strategy`.
- `cargo run --quiet -p quantrule-governance -- check --all` →
  `governance check ok`.
- `wc -l` ADR → 107 lines, under the 110-line cap.
- `tools/gate.sh --changed main` → recorded in the pull request.

No live run, composed tier, lab, or release run was performed. The ADR claims no
behavior change and no edge.

## 4. Learning

- The signal module is not a clean leaf: it uses pack parsing, strategy config,
  strategy bundle types, and backtest decision types. Extracting it would either
  move those with it or depend back on `quantrule-app`.
- There is one consumer, the strategy service, so the extraction would add
  manifests and edges without removing a real coupling.
- Purity is enforced by the module's tests and docs rather than by a crate
  boundary; that is the trade-off this ADR accepts.
- The original 65-ID change map is not reconstructed. The approved D4 records
  the deviation, and STR-004 is the place that states it in the plan and
  architecture.

## 5. Next action

Push, pull request, merge and cleanup remain gated. STR-004 amends the plan and
architecture to record the keep decision; no signal change is required.
