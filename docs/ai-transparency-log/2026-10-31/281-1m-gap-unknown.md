# AI Transparency Log — 1m Gap Measurement

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #281, branch `fix/281-1m-gap-unknown`
- Plan: `docs/execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md` (W2)
- Audit: `docs/audits/2026-10-31/remediation-and-diagnosis-v1-audit.md` (AUD-2026-10-31-32)

## Intent and boundary

W2 fixes the Minor finding that `data.long_gap` passed with `0.0` when the run
had no 1m series. The fix is confined to diagnosis evidence; it does not touch
the signal path, a threshold, a model reading, or a strategy rule.

## Implemented

- `DataQuality.raw_gaps` is now `Option<Vec<Gap>>`: `None` means the run has no
  1m series, `Some(vec![])` means the series has no gaps.
- `data.long_gap` publishes `unknown` with no value and evidence "no 1m series"
  when the series is absent; a present series with no gaps still passes with
  `0.0`.
- The triage action for an unknown gap check now says to add a 1m series or
  record why the run has none.
- The text summary prints "raw 1m gaps: unavailable (no 1m series)"; the
  data-quality chart renders "no 1m series"; `diagnosis.json` publishes
  `raw_gaps: null`.
- Added analytics tests for the absent and present-with-no-gaps cases; the
  fixture gained a two-timeframe constructor (1m and 5m).

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo test -p quantrule-analytics --test diagnosis` | 6 passed, 0 failed |
| `cargo test -p quantrule-shell --test diagnosis` | 5 passed, 0 failed |
| `cargo test --workspace` | 1508 passed, 0 failed, 7 ignored |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The analytics tests assert `unknown`/`None`/`triage names the missing series`
for the absent case and `pass`/`Some(0.0)` for the present case. The existing
diagnosis summary test runs the `None` text branch on the 300s-only fixture.

## Learning

- The defect was a collapsed distinction, not a wrong threshold: an absent
  series became an empty gap list before the check was built. Making the list
  optional preserved "no gaps" as a valid zero and made "no series" unknown.
- The check's triage action was static, so an unknown gap check still told the
  reader to fill a gap. It now branches on the status.
- The shell's backtest path always has a 1m series, so the absent branch is
  exercised at the analytics layer. The chart's absent branch is compile- and
  review-checked, not rendered in a shell test.

## Limitations

- No independent verifier; this is a sequential implementation pass.
- The tests use checked-in fixtures, not live market data.
- The change is evaluation evidence; no strategy result changes.
