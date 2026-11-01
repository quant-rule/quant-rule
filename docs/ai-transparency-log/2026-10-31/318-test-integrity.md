# AI Transparency Log — Test Integrity and Guards

- Date: 2026-11-01 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #318, branch `test/318-test-integrity`; PR [#330](https://github.com/quant-rule/quant-rule/pull/330)
- Plan: `docs/execution-plans/2026-10-31/full-system-audit.md` (W11)
- Audit: `docs/audits/2026-10-31/full-system-audit.md` (AUD-63, -65)

## Intent and boundary

The human approved the full-system audit plan on 2026-10-31. W11 closes the
test-integrity gaps that let the audited defects pass: multi-context weights,
non-zero fill fees and funding, six diagnosis checks, the unreadable lab
evidence, the vacuous report/detector tests, the renderer/diagnosis architecture
guard, duplicate-option handling, and the runner metrics fixture. It changes no
model, strategy, execution, or portfolio behavior; the diagnosis error type was
extracted only to remove the command dependency.

## Implemented

- Added a two-context `compute_axes` test with distinct per-axis weights,
  shares, stage histogram, and late count.
- Added ledger coverage for a non-zero fill fee and negative funding, and a
  runner test that non-zero execution fees and funding reach the portfolio.
- Added pass/fail/unknown tests for `strategy.net_expectancy`,
  `execution.slippage`, `portfolio.cost_drag`, `portfolio.drawdown_limit`, and
  `evidence.rerun`; extracted `axis_correlation_check` and covered its 0.70
  boundary at pass/fail/unknown.
- Added the out-of-sample reader tests (absent, malformed, same version,
  distinct version) and a malformed rerun report test.
- Rewrote the report prefix test so the altered future is part of the full run;
  the detector tests now require an actual event before checking the axis
  recomputation; added a snapshot test that clears events and compares the
  published axis values.
- Expanded the architecture guard to every `render*` module and
  `diagnosis.rs`; extracted `DiagnosisError` so `diagnosis.rs` no longer imports
  `crate::command`.
- Standardized duplicate-option rejection in the model, chart, fetch, research,
  and store-show parsers, with a table test.
- Removed the stray runner-output line from `runner_metrics_golden.csv`; the
  parser now rejects unknown, duplicate, and missing metric rows and asserts the
  expected count.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo test -p quantrule-backtest --lib two_context_timeframes` | 1 passed |
| `cargo test -p quantrule-analytics --test diagnosis` | 20 passed |
| `cargo test -p quantrule-shell --bin quantrule lab_evidence::tests` | 9 passed |
| `cargo test --workspace` | 1,596 passed, 0 failed, 7 ignored; 47 suites |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `python3 tools/check_links.py` | 309 Markdown files checked; no broken relative links |
| `git diff --check` | clean |

Fail-before: restoring the stray metric line made the runner golden test fail
with the strict parser; adding a `use crate::command::Command` line to
`diagnosis.rs` made the expanded architecture guard fail; removing the fetch
duplicate check made the duplicate-option table test fail. The coverage tests
for correct behavior pass before and after; they close a gap rather than fix a
defect.

## Learning and rejected suggestions

- The report prefix test was vacuous because it trimmed the altered series
  before building the run. The test now includes the altered future in the full
  run and compares bar 199. The first version of the detector test used a
  constant RSI and produced no event; it now uses the regular-bullish RSI
  fixture and asserts an event exists.
- The architecture guard could not be demonstrated with
  `crate::command::RunError` because that re-export was removed; the
  demonstration used `crate::command::Command` instead.
- The diagnosis error extraction is a structural refactor: the command layer
  owns the user-facing `RunError`, and the diagnosis writer owns its four
  failure kinds. No error message or exit behavior changed.
- I left the metric fixture's `halted,false` value as a raw string; the strict
  parser validates names and row counts, and numeric parsing happens only when a
  numeric metric is requested. This avoids pretending a boolean is a float.
- I did not add a synthetic fail-before for every coverage test: the named
  tests exercise behavior that already worked, so a fail-before would require
  intentionally breaking the implementation.

## Limitations

- The diagnosis checks are tested against projected facts, not a full shell lab.
- The two-context test uses a cloned model bar for each context with known
  values; it exercises the weight/index path, not a full two-series model run.
- No model or strategy behavior changed, so no Quant Validator record is
  required.
