# AI Transparency Log — Current Docs and Bounds

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential implementation and verification
- Work item: #253 (W7)
- Findings: AUD-2026-10-31-06, -18, -29
- Plan: `docs/execution-plans/2026-10-31/audit-unified-ladder.md` (W7)

## Intent and boundary

Make the current docs match the merged system and make `strategy-run --config`
honor the run's data bounds when no CLI bound is supplied. Also point the
portfolio code at the current contract and correct the stale M15 statement.

## Implemented

- `strategy-run` uses `run.data.from/to` when neither `--from` nor `--to` is
  supplied; CLI bounds still override.
- Added a current portfolio schema section to `policy-and-strategy.md`; the
  portfolio module links to it and the execution module no longer says M15.
- `docs/README.md` records the unified ladder as complete (PR #246 merged as
  `7c865de`); the per-timeframe model status says the context builder is live.
- README marks the old 15m `axis-agreement` envelope historical.

## Verification

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | 45 suites, 0 failures |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Passed |
| `git diff --check` | Clean |

## Learning and rejected suggestions

- Rejected regenerating the README envelope. The full-year SOLUSD source is not
  committed; the old envelope is marked historical rather than claimed current.
- Kept CLI bounds as the override; the run config is the default only when the
  command supplies no bound.
