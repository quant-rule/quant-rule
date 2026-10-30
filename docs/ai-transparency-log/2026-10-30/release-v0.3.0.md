# AI Transparency Log — Release v0.3.0

- Date: 2026-10-30 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: release of the delivered program
  [#208](https://github.com/quant-rule/quant-rule/issues/208)
- Plan:
  [`docs/execution-plans/2026-10-30/release-v0.3.0.md`](../../execution-plans/2026-10-30/release-v0.3.0.md)
- Release notes:
  [`docs/releases/2026-10-30/v0.3.0.md`](../../releases/2026-10-30/v0.3.0.md)
- Branch/worktree: `docs/release-v0.3.0` in `quant-rule--release-v0.3.0`
- Status: release branch verified; PR, merge, tag, and GitHub release follow.

## Intent and boundary

Tag the delivered `lab summary`, tutorial, role manuals, and help redesign as
`v0.3.0` with the title **Lab Summary and Help**. The workspace version and the
reports' `model_version` move from `0.2.0` to `0.3.0`, so a run records the
release identity. No model formula, strategy, execution, portfolio,
configuration, report schema, or CSV schema changes.

## Work performed

1. `[workspace.package] version` and the five workspace entries in `Cargo.lock`
   move from `0.2.0` to `0.3.0`.
2. The version-bearing fixtures `dashboard.json`, `dashboard.txt`, and
   `market-report.json` were updated with `0.3.0`.
3. The research-export test's literal version assertion was updated from
   `0.2.0` to `0.3.0`; the test still checks it against
   `env!("CARGO_PKG_VERSION")`.
4. The tutorial's current version references and the README backtest envelope
   were updated from `0.2.0` to `0.3.0`.
5. `docs/releases/2026-10-30/v0.3.0.md` records the commands, help contract,
   tutorial and manuals, configuration and report compatibility, and
   verification evidence.
6. The README and docs-index release tables and the docs-index program table
   were updated.
7. The release plan and this log were added.

## Verification and evidence

Commands were run in `quant-rule--release-v0.3.0` on 2026-10-30.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Clean |
| `cargo clippy --all-targets -- -D warnings` | No warnings |
| `cargo test` | 1,391 passed, 0 failed, 6 ignored |
| `cargo doc --no-deps` | Built without warnings |
| `cargo test -p quantrule-shell --test cli` | 90 passed |
| `cargo test -p quantrule-shell --test market_report` | 12 passed |
| Full-year `backtest` plus `lab summary` at `0.3.0` | Five bundles, 29 bundle artifacts, root summary; `model_version: 0.3.0`; 48 trades, final equity `8016.143910`, max drawdown `0.203150`, `halted: true`, commission `885.610499` |
| `quantrule help` / `help all` | 43 / 504 lines |
| `quantrule help glossary` / `tutorial` / `roles` | 118 / 30 / 23 lines |

The full-year run is illustrative. It is in-sample only, halts at the configured
20% drawdown limit, and makes no profitability claim.

## Learning

- The version literal lives in four test artifacts, not one: `dashboard.json`,
  `dashboard.txt`, `market-report.json`, and the research-export assertion in
  `cli.rs`. A search of the test tree for the old literal found them all before
  the run.
- `model_version` is the package version. The release notes state that this is an
  identity change, not a formula change; the report and CSV schemas are
  unchanged.
- The tag cannot be created before the release PR merges. The plan puts the
  annotated tag on the merge commit, and the release notes distinguish the
  observed release-branch checks from the tag step.
- The help output is unaffected by the version bump; the dashboard, report, and
  store fixtures are the only version-bearing outputs.

## Pull request and tag

The release PR, merge, annotated tag `v0.3.0`, and GitHub release are pending.
Each requires the human's explicit authorization; the user's request authorizes
the release and tag.
