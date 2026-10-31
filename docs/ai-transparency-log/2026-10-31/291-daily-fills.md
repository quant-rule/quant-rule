# AI Transparency Log — Daily Entry Fills

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #291, branch `fix/291-daily-fills`
- Plan: `docs/execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md` (W5)
- Audit: `docs/audits/2026-10-31/remediation-and-diagnosis-v1-audit.md` (AUD-2026-10-31-38)

## Intent and boundary

W5's third leaf fixes the Minor finding that the daily strategy funnel counted
entry fills from the closed-trade list, so an entry still open at run end was
missing and the daily series disagreed with `funnel.entry_fills`. The change is
confined to `funnel_series` and its test.

## Implemented

- The daily series now reads `input.fills` and keeps only `reason == "entry"`.
- Each fill is keyed by its own UTC day, `time.div_euclid(86_400) * 86_400`.
- Exit, stop, and target fills are excluded from the entry count.
- Attempts still come from `input.entry_contexts`; the two series are merged as
  before.
- Added a test with a closed trade and an open entry on different days, plus an
  exit fill that must not count.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo test -p quantrule-analytics --test diagnosis the_daily_funnel_counts_open_entry_fills` before the fix | FAILED: daily sum 1, expected 2 |
| Same test after the fix | 1 passed, 0 failed |
| `cargo test -p quantrule-analytics --test diagnosis` | 13 passed, 0 failed |
| `cargo test --workspace` | 1517 passed, 0 failed, 7 ignored |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The test asserts a daily sum of 2, equality with `funnel.entry_fills`, and a
point on the open entry's day with `entry_fills = 1`.

## Learning

- An open entry is a fill, not a trade. The daily series must read the fill list,
  while the exit-reason and hold statistics must keep reading closed trades.
- The aggregate funnel already counted entry fills from `input.fills`; the daily
  series was the only place using the closed-trade list, which is why the two
  disagreed.
- Keying by the fill's own `time` avoids relying on a trade's entry time and
  keeps the series correct for a fill that never becomes a trade.

## Limitations

- The open-entry case is exercised through the analytics input; the shell
  fixture's open position is not asserted end-to-end.
- The Quant Validator pass is sequential, not independent.
- W5 closes findings 35, 37, and 38; the W5 validator record is complete.
