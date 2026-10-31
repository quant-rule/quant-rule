# AI Transparency Log — Null Average Hold

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #289, branch `fix/289-null-hold`
- Plan: `docs/execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md` (W5)
- Audit: `docs/audits/2026-10-31/remediation-and-diagnosis-v1-audit.md` (AUD-2026-10-31-35)

## Intent and boundary

W5's first leaf fixes the Minor finding that an exit reason with no trades
published `0.0` as its average hold. The change is confined to the diagnosis
value and formatters. It does not touch the signal path, a model reading, a
strategy rule, execution, or portfolio accounting.

## Implemented

- `ExitReasonStats.average_hold_bars` is now `Option<f64>`.
- A reason with no trades publishes `None`; a reason with trades keeps the mean.
- `diagnosis.json` publishes `null` through the existing `Option` projection.
- `strategy-exit-reasons.csv` writes an empty cell for a missing value, the
  established convention; the text summary writes `-`.
- Added tests for the zero-trade reasons, the traded-reason mean, and both CSV
  forms.
- Started the W5 validator record; findings 37 and 38 remain pending.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo test -p quantrule-analytics --test diagnosis` | 9 passed, 0 failed |
| `cargo test --workspace` | 1513 passed, 0 failed, 7 ignored |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

## Learning

- The zero was not a threshold; it was an empty sample collapsed into a valid
  number. Making the field optional forces every consumer to say what it does
  with a missing measurement.
- The CSV and JSON conventions differ by artifact: JSON uses `null`, CSV uses an
  empty cell, and the text summary uses `-`. The tests pin the CSV form and the
  struct; the JSON projection is a direct `Option` serialization.
- A fail-before test is impossible against the old `f64` without changing the
  assertion, because `Option<f64>` and `f64` do not compare. The old `0.0`
  assignment is in the diff, and the new assertion is the guard.

## Limitations

- The Quant Validator pass is sequential, not independent.
- The shell fixture has trades, so the JSON null branch is covered at the
  analytics boundary, not by an end-to-end no-trade shell run.
- Findings 37 and 38 remain; the W5 record is updated in their leaves.
