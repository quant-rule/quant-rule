# AI Transparency Log — Workflow Reset

- Date: 2026-11-01 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: `workflow-reset`
- Plan: [`docs/execution-plans/2026-11-01/workflow-reset.md`](../../execution-plans/2026-11-01/workflow-reset.md)
- Claim type: workflow and program change; no strategy edge claim

## Intent and boundary

The human approved replacing the single umbrella workflow with two explicit
workflows: an **Audit Workflow** (audit, remediation plan, execution,
verification) and an **Improvement Workflow** (run, results, review, new
hypothesis). The improvement workflow is tiered into model, strategy, and
portfolio runs; each tier changes one engine while the others stay frozen.

The approved boundary covers documentation, run tooling, data partition
enforcement, evidence protocols, the v6 breakout candidate, the reset audit, the
first improvement cycle, and the cleanup of invalidated artifacts. It excludes
live trading, multi-asset portfolios, trailing/partial exits, 2023 crypto data
recovery, new dependencies, and backwards compatibility.

## What the AI did

| Step | Action | Evidence |
|---|---|---|
| Tag | Created `v0.5.0` at `42307b7` before any workflow change | `git tag --list v0.5.0` |
| Worktree | Created `quant-rule--workflow-reset` on `feat/workflow-reset` | `git worktree list` |
| Plan | Wrote the approved execution plan | `docs/execution-plans/2026-11-01/workflow-reset.md` |
| Implementation | Added the workflow docs, run contract, Python run store, data partitions, v6 breakout candidate, and audit record | See the plan and audit |
| Audit | Published `docs/audits/2026-11-01/workflow-reset-audit.md`; one Major fixed, Minor/Info routed | Program green |
| First run | Executed `MODEL-2026-11-01-01`; model gate not green | `data/improvement/2026-11-01/MODEL-2026-11-01-01/` |

## Verification and evidence

Observed in this worktree:

- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --workspace`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
  pass.
- `python3 tools/check_links.py` and `python3 tools/check_terms.py` pass.
- `python3 tools/improvement/validate.py` accepts the six staged manifests and
  rejects an invalid frozen list.
- `MODEL-2026-11-01-01` ran through the run store; the confidence table shows
  Direction and Momentum below 0.50 in five of six 2021 cells. No model baseline
  was accepted.
- The v6 breakout smoke produced entry and exit records through `strategy-run`
  and a five-bundle backtest through the release shell.

## Learning and rejected suggestions

- **Learned:** the first model gate failed before the strategy tier. The
  Direction/Momentum hit rates were below 0.50 in five of six 2021 cells, so the
  prior strategy failures are consistent with a measurement problem, not only a
  threshold problem.
- **Rejected:** running the strategy and portfolio tiers anyway. The tier
  contract blocks them until a model baseline is accepted.
- **Rejected:** treating the 2021 harness as a full validation. It is one year,
  two timeframes, and three instruments; the record says so.
- **Rejected:** deleting the pre-reset records in the same change as the audit
  fix. The invalidation rows exist; deletion is the separate cleanup phase.

- Rejected: preserving the retired workflow name as an alias. The human wants the
  old term gone; a term guard will fail if it reappears in active docs.
- Rejected: keeping pre-reset experiment records as comparable evidence. The
  reset changes the workflow and run layout; invalidated records will be deleted
  with tombstones.
- Rejected: full autonomous baseline acceptance. The approved boundary is
  autonomous execution and reporting with human acceptance of baselines.

## Limitations

- The AI performed the planning and implementation passes sequentially. This is
  not an independent model judgment.
- The audit phase is a separate read-only pass; it has not run at the time this
  entry was opened.
