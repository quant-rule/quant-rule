# AI Transparency Log — MVP v1 Release

- Date: 2026-10-28 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#178](https://github.com/quant-rule/quant-rule/issues/178)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md)
- Branch/worktree: `docs/178-mvp-v1-release` in `quant-rule--178-mvp-v1-release`
- PR: pending
- Status: Implementation complete; checks pass; PR and tag not yet done.

## Intent and boundary

V2 closes MVP v1: bump the workspace to `0.2.0`, write the release notes, bring
the README, docs index, program plan, and old plan notes in step with what was
merged, and tag `main` as `v0.2.0` with the release title **MVP v1**. No behavior
change is part of the release commit.

## Work performed

1. `[workspace.package] version` and the five `Cargo.lock` entries move from
   `0.1.0` to `0.2.0`.
2. The three fixtures that embed the package version — `dashboard.json`,
   `dashboard.txt`, and `market-report.json` — were regenerated with `0.2.0`.
3. The research-export test asserted `model_version: "0.1.0"` as a literal. It
   now asserts `0.2.0` and additionally that the manifest matches
   `env!("CARGO_PKG_VERSION")`, so the release literal and the crate version
   cannot drift.
4. `docs/releases/2026-10-28/v0.2.0.md` records the commands, configs, reports,
   accounting rules, verification evidence, and compatibility statement.
5. `README.md` gains a Releases table, a full-run example, and a Scope section
   that describes the delivered backtest instead of saying it is not
   implemented.
6. `docs/README.md` gains a Releases table and marks the program delivered;
   the plan's status and criterion 15 point at the tag; the 2026-10-27 plan
   records the delivery.
7. The four checks were run at `0.2.0`.

## Decisions and corrections

- **The version is pinned twice in one assertion.** A literal `0.2.0` alone
  would pass after a future drift between the literal and the crate; the
  package-version half alone would pass if the release number were wrong. The
  two together pin the release and the binding.
- **Fixtures are regenerated, not loosened.** The dashboard and market-report
  fixtures are golden byte comparisons of output that includes the package
  version. A version release is the deliberate case the testing rules name, and
  the diff is exactly the version string.
- **`0.1.0` was a marker, not a release.** The repository had no tags; `v0.2.0`
  is the first tag, and the release notes state that so a reader does not look
  for a missing `v0.1.0`.
- **The Scope section had gone stale.** It still said backtests, strategies, and
  portfolio execution were not implemented. V2 rewrites it, because a release
  that leaves the README describing an earlier slice is the stale-documentation
  defect the instructions name.
- **The tag is created after the merge, on `main`.** The release commit cannot
  contain its own tag; the tag is an annotated tag on the merge commit.

## Verification and evidence

Commands were run in `quant-rule--178-mvp-v1-release` on 2026-10-28.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Passed; no diff after `cargo fmt` |
| `cargo clippy --all-targets -- -D warnings` | Passed; no warning |
| `cargo test` | 1,349 passed, 0 failed, 1 ignored (`store_100k`) |
| `cargo doc --no-deps` | Passed; exit 0; no warning |

The first test run after the bump failed only `research_export_writes_the_dataset_and_repeats_byte_for_byte`
on the literal `0.1.0`; the assertion was updated as described, not removed.

## Learning

- A version bump is a search problem, not a one-line edit: the package version
  leaks into JSON fixtures and a manifest assertion. Searching the test tree for
  the old literal found all three.
- The stale Scope paragraph was not caught by a test. Release work is the last
  chance to read the front page as a user would.
