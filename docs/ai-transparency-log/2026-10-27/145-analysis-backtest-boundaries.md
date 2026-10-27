# AI Transparency Log — Analysis and Backtest Boundaries

- Date: 2026-10-27 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017
- Work item: [#145](https://github.com/quant-rule/quant-rule/issues/145)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md`](../../execution-plans/2026-10-27/141-market-analysis-and-backtest.md)
- ADR: [`docs/adr/2026-10-27/analysis-and-backtest-boundaries.md`](../../adr/2026-10-27/analysis-and-backtest-boundaries.md)
- Branch/worktree: `docs/145-analysis-backtest-boundaries` in `quant-rule--analysis-backtest-boundaries`
- Status: Implementation complete; pull request open.

## Intent and boundary

The human approved the program plan and asked for the ADR that fixes the vocabulary and the module boundary before code changes. This work item is documentation and architecture only:

- Define market analysis as the descriptive model-output path and backtest as the full-execution path.
- Fix the command names `market-report`, `strategy-run`, and `backtest`.
- Define the new pure `quantrule-backtest` crate and its module boundaries.
- Keep the existing `market-*` commands and the 100k store/research caps unchanged.
- Record the alternatives and consequences.

No Rust code, command implementation, dependency, schema, or user-facing behavior is changed.

## Work performed

1. Read the approved program plan, the model contract, the history/retention ADR, the dashboard ADR pattern, and the shell command surface.
2. Wrote `docs/adr/2026-10-27/analysis-and-backtest-boundaries.md`:
   - The two-path table and the no-order guarantee for market analysis.
   - The name table, including the model Signal-event versus strategy entry/exit collision.
   - The command signatures and artifact sets.
   - The `quantrule-backtest` crate with `strategy`, `execution`, `portfolio`, `runner`, and `report` modules.
   - The dependency direction and the model-report view boundary.
   - The shared full-year window decision and determinism/causality rules.
   - Rejected alternatives: one combined command, model-owned execution, shell-owned engine, three new crates, analytics/research naming, strategy `signals`, extending existing commands, and raising the global cap.
3. Added the ADR to the `Current architecture decisions` table in `docs/README.md`.
4. Updated the program plan header, D0 status, D1 status, D2 queue, and first-action line.

## Verification and evidence

- `git diff --check` — no whitespace errors.
- `git status --short` — only the intended documentation paths.
- Link paths in the ADR were checked against the repository layout.
- The ADR states no command is implemented by this work item.
- `cargo fmt --all -- --check` — passed.
- `cargo test` — passed: all executed workspace unit, integration, and doc tests; 0 failed. Two 100k benchmarks remained ignored by design.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo doc --no-deps` — passed.

## Learning

- The model already publishes Signal events, so the strategy-only command is `strategy-run`; calling it `signals` would erase the distinction between a descriptive event and a decision.
- A separate `quantrule-backtest` crate is the smallest boundary that keeps the model descriptive and the shell free of domain logic. Three independent engine crates are deferred because no independent consumer exists yet.
- The full-year window is shared by both paths, so it stays a separate work item (D2) rather than a backtest-only flag.
