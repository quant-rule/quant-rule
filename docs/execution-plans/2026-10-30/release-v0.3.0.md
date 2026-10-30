# Release v0.3.0 — Lab Summary and Help

- Date: 2026-10-30 (UTC)
- Status: **Approved — 2026-10-30.** The human asked to tag the completed
  program as the next version.
- Work item: no new issue; release of the delivered program
  [#208](https://github.com/quant-rule/quant-rule/issues/208)
- Baseline: `main` at `5766ec6`; previous tag `v0.2.0`
- Release notes:
  [`docs/releases/2026-10-30/v0.3.0.md`](../../releases/2026-10-30/v0.3.0.md)
- Transparency log:
  [`docs/ai-transparency-log/2026-10-30/release-v0.3.0.md`](../../ai-transparency-log/2026-10-30/release-v0.3.0.md)
- Model/strategy behavior: unchanged

## Intent

Tag the delivered `lab summary`, tutorial, role manuals, and help redesign as
the next release. The workspace version and every report's `model_version` move
from `0.2.0` to `0.3.0`, so a run records the release identity. No formula,
strategy, execution, portfolio, configuration, or schema behavior changes.

## Current state

- `main` is at `5766ec6`; #209, #210, #211, and #212 are Done and closed, and
  container #208 is Done and closed.
- The workspace version is `0.2.0`; the only tag is `v0.2.0` (MVP v1).
- `model_version` in reports, manifests, dashboards, and the store is
  `env!("CARGO_PKG_VERSION")`.
- The program added `lab summary`, the full-pipeline tutorial, three quant role
  manuals, and the metadata-driven help with topics.

## Scope

Included:

- Bump `[workspace.package] version` and the five workspace entries in
  `Cargo.lock` to `0.3.0`.
- Update the version-bearing fixtures (`dashboard.json`, `dashboard.txt`,
  `market-report.json`) and the research-export version assertion.
- Update the tutorial's current version references and the README backtest
  envelope.
- Add the v0.3.0 release notes and the README/docs-index release table entries.
- Add the one-line shell entry points already delivered by the program.

Excluded:

- Model formulas, strategy rules, execution, portfolio accounting,
  configuration schemas, report or CSV schemas, fixtures other than the
  version-bearing ones, the oracle, and CI behavior.
- A new feature or a behavior change.
- A profitability claim.

## Decisions

| ID | Decision | Status |
|---|---|---|
| D1 | The next version is `v0.3.0`, a minor release: the program adds a command and a new help surface. | Approved by the tag request |
| D2 | `model_version` follows the package version, as it did for `v0.2.0`. The release notes state that this is an identity change, not a formula change. | Approved |
| D3 | The tag is an annotated `v0.3.0` on the merge commit, created after the release PR merges. | Approved |
| D4 | The GitHub release title is **Lab Summary and Help**, naming the two user-facing highlights. | Selected |

## Acceptance criteria

- `Cargo.toml`, `Cargo.lock`, and every version-bearing fixture report `0.3.0`.
- `cargo fmt`, `clippy`, `test`, and `doc` pass.
- A full-year backtest at `0.3.0` writes five bundles plus the root summary and
  reports `model_version: 0.3.0`; the observed result is unchanged.
- The release notes and release tables are current.
- The annotated tag `v0.3.0` exists on `main` and is pushed.
- The GitHub release **Lab Summary and Help** points at `v0.3.0`.
- No model or strategy behavior changed.

## Work breakdown

1. Bump the workspace and lockfile versions.
2. Update the four version-bearing test artifacts and the assertion.
3. Update the tutorial and README current version examples.
4. Write the release notes and update both release tables.
5. Run the full checks and the full-year run at `0.3.0`.
6. Open the release PR, merge it, tag `main`, and publish the GitHub release.
7. Record the evidence in this plan and the transparency log.

## Verification

| Layer | Check | Expected |
|---|---|---|
| Build | `cargo build --release -p quantrule-shell` | Succeeds at `0.3.0` |
| Format | `cargo fmt --all -- --check` | No diff |
| Lints | `cargo clippy --all-targets -- -D warnings` | No warnings |
| Tests | `cargo test` | All pass |
| Docs | `cargo doc --no-deps` | No warnings |
| Fixtures | `cargo test -p quantrule-shell --test cli`, `--test market_report` | Golden files match |
| Run | `backtest` plus `lab summary` on the full-year example | Five bundles, 29 artifacts, root summary, `model_version: 0.3.0`, 48 trades, final equity 8016.143910, `halted: true` |
| Tag | `git tag --list v0.3.0` and `git ls-remote --tags origin v0.3.0` | The annotated tag is present locally and remotely |
| Release | `gh release view v0.3.0` | Title **Lab Summary and Help** |

## Risks and recovery

| Risk | Mitigation |
|---|---|
| `model_version` is a visible output change | Document it in the release notes as an identity change; no formula change. |
| A version-bearing fixture is missed | Search the test tree for the old literal; the release test run fails on a mismatch. |
| The tag points at the wrong commit | Tag the merge commit on `main` after the release PR merges; verify `git rev-parse v0.3.0`. |
| The release notes describe stale behavior | Run the final checks and the full-year run before writing the evidence. |
| The version and tag disagree | Check `Cargo.toml`, `Cargo.lock`, the fixtures, and `git tag` together. |

## Stop conditions

Stop and ask if:

- A test fails for a reason other than the expected version string.
- A version-bearing file cannot be identified or regenerated.
- The tag or release cannot be created on the intended merge commit.
- A merge or release is not authorized.
- A behavior change, schema change, or model/strategy change would be required.

## Evidence

Observed on the release branch on 2026-10-30:

| Check | Result |
|---|---|
| Version | `Cargo.toml`, `Cargo.lock`, and the version-bearing fixtures report `0.3.0` |
| Format | `cargo fmt --all -- --check` clean |
| Lints | `cargo clippy --all-targets -- -D warnings` no warnings |
| Tests | `cargo test` 1,391 passed, 0 failed, 6 ignored |
| Docs | `cargo doc --no-deps` built without warnings |
| Full-year run | Five bundles, 29 bundle artifacts, root summary; `model_version: 0.3.0`; 48 trades, final equity `8016.143910`, max drawdown `0.203150`, `halted: true`, commission `885.610499` |
| Help | `help` 43 lines, `help all` 504, glossary 118, tutorial 30, roles 23 |

The release PR, merge, annotated tag, and GitHub release are pending. The tag
and release are created only after the merge, on the merge commit.
