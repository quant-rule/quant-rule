# AI Transparency Log — README and Help Refresh

- Date: 2026-10-29 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: post-release documentation correction; no issue
- Related: [#141](https://github.com/quant-rule/quant-rule/issues/141), release `v0.2.0`
- Branch/worktree: `docs/readme-report-bundles` in `quant-rule--readme-report-bundles`
- PR: [#193](https://github.com/quant-rule/quant-rule/pull/193)
- Status: PR open; merge after checks pass.

## Intent and boundary

The README is the user manual: there is no separate guide, and `help` is the
in-shell reference. A read-only review after the `v0.2.0` tag found three stale
statements and a missing command reference for the config-driven surface: the
five bundles appeared only as a paragraph inside the market-report section, with
no heading, config table, artifact table, or worked example. This change fixes
the manual; it changes no command behavior, no report field, and no engine.

## Work performed

1. Added `### Report bundles and backtest`: the five engines with their
   artifacts, the seven config files, path resolution, validation timing, the
   `--out` default, bundle availability, `--json` differences, and provenance.
2. Added the onboarding prerequisite: the example run needs its M1 file, which
   `data/` does not commit; the `fetch sol/usd` command produces it. The command
   was exercised for one day against the provider.
3. Added a worked example: the five printed bundle paths and the full-year
   `--json` envelope, taken from the B4 run and checked for equality after
   normalizing the root and the release version.
4. Corrected the two stale claims that the analytical pass materializes layers
   or is computed alongside the model. Since #189 the summary streams one bar at
   a time.
5. Replaced the single-cap sentence with a per-command table: 100,000 for
   analysis, store, and research; 600,000 for `market-report` and
   `strategy-run`; `data.json.max_candles` for `report` and `backtest`.
6. Reworded the exhaustive command-name sentence, which no longer listed
   `report` and `backtest`, into the general command-contract statement.
7. `docs/README.md`: the config, report, and example-run rows now say what was
   implemented and verified; a `Current evidence` table links the verification
   report.
8. `help`: added non-interactive examples for `report` and `backtest`, and
   replaced the contradictory cap line; the checked-in help fixture was updated
   in step.

## Decisions and corrections

- **The README is the manual; no new document.** A separate guide would repeat
  the README and drift. The durable contracts stay in `docs/config/` and
  `docs/reports/`, linked from the new section.
- **The example envelope is real, not illustrative.** It is the B4 full-year
  envelope with the output root and the package version adjusted; the JSON was
  compared for structural equality before it was written into the README.
- **The fetch prerequisite is verified.** `fetch sol/usd 2025-01-01 2025-01-01`
  returned 595 accepted rows and the expected CSV header, so the documented
  instruction is not a guess.
- **The help change is a fixture change.** The CLI test compares the help text
  byte for byte; both `help.rs` and `tests/fixtures/help.txt` were updated, and
  the test passes.
- The first proposal listed "model and analytical layers are computed once" as
  merely imprecise; it is false after #189, because no layer set is built. The
  wording now states what the code does.

## Verification and evidence

Commands were run in `quant-rule--readme-report-bundles` on 2026-10-29.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; no warning |
| `cargo test` | 1,349 passed, 0 failed, 1 ignored (`store_100k`) |
| `cargo doc --no-deps` | Passed; exit 0; no warning |
| `cargo test -p quantrule-shell --test cli help` | 3 passed, 0 failed |
| `fetch sol/usd 2025-01-01 2025-01-01` | 595 rows accepted, 0 rejected; header as documented |

The full-suite count is unchanged because the change adds no test; the existing
help and CLI tests cover the help fixture, and the README has no test.

## Learning

- The README had drifted in exactly the place where the implementation changed
  last: the analytical path. A release check that compares the README's claims
  to the final code would have caught it before the tag.
- A worked example needs an input the reader can actually obtain. The command
  was correct, but the example could not run on a fresh clone; the missing step
  was data acquisition, not syntax.
- The docs index is part of the manual's accuracy. "Approved" and "implemented"
  are different states, and the index should say which one is true.
