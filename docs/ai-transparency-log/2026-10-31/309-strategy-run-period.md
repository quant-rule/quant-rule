# AI Transparency Log — Strategy-Run Period

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #309, branch `fix/309-strategy-run-period`
- Plan: `docs/execution-plans/2026-10-31/full-system-audit.md` (W2)
- Audit: `docs/audits/2026-10-31/full-system-audit.md` (AUD-2026-10-31-42, -49)

## Intent and boundary

The human approved the full-system audit plan on 2026-10-31. W2 closes the Major
provenance finding: `strategy-run --config` applied the run's `data.from`/`data.to`
bounds but the JSON envelope recorded only the CLI `--from`/`--to`, so the
envelope could claim a different period from the one analyzed. It also makes the
empty-result contract explicit.

The change is confined to the shell's strategy-run envelope, command
documentation, and tests. It does not touch the strategy rule, an indicator, the
model, execution, portfolio accounting, or any diagnosis threshold. It adds no
dependency.

## Implemented

- Added `effective_period` to the JSON envelope: the CLI dates when either is
  supplied, otherwise the run configuration's `data.from`/`data.to`; `null`
  endpoints mean the full file.
- Kept `requested_period` as the exact CLI request, so the two are distinguishable.
- Added `actual_period` (first and last analyzed execution bar) and `bars` (the
  execution bar count). A missing execution timeframe reports `bars: 0` with
  null actual bounds.
- Updated the README and `help all` to document the envelope fields, the
  `--config` date-bounds behavior, and the empty-result contract: a missing
  context holds; a missing execution timeframe is an empty success, not an
  error.
- Added tests for config bounds, CLI dates, the zero-bar case, and the existing
  full envelope. The five-row short-series test remains the context-missing
  case: one execution bar, no records.

## Verification and evidence

Commands actually run, with observed results:

| Command | Observed |
|---|---|
| `cargo test -p quantrule-shell --test strategy_run` | 15 passed, 0 failed |
| `cargo test -p quantrule-shell --test cli help` | 15 passed, 0 failed |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace` | 1,528 passed, 0 failed, 7 ignored; 47 suites |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The config-bounds test uses a copied run configuration with a one-day range; it
asserts `effective_period` is that day, `bars` is 288, and the actual first and
last execution bars are inside the day. The zero-bar test uses a single-candle
source, which cannot build the 5m execution series.

## Learning and rejected suggestions

- The existing `short_series` (five M1 rows) is not a zero-bar case: it builds
  one 5m execution bar and the 15m context is missing, so the strategy holds
  with `bars: 1`. I added a separate one-candle source for `bars: 0` rather than
  mislabelling the five-row fixture.
- The first assertion used the CSV timestamp shape
  (`YYYY-MM-DD HH:MM:SS+00:00`); `format_utc_timestamp` is the ISO shape
  (`YYYY-MM-DDTHH:MM:SSZ`). The test now uses the actual form.
- The old README said the command fails when an active-ladder timeframe cannot
  be built. The W6b2 validation intentionally changed that to an empty success;
  the README now says so and distinguishes the missing-M1 error from the
  missing-context hold.

## Limitations

- The command remains a virtual diagnostic walk; it does not create fills or
  P&L. No Quant Validator record is required because the strategy decision rule
  is unchanged.
- The tests use deterministic generated M1 series, not live market data.
- `bars` counts execution bars, not decisions; a run with bars and no records
  is a valid all-Hold result.
