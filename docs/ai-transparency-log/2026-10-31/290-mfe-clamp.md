# AI Transparency Log — MFE/MAE Clamp

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #290, branch `fix/290-mfe-clamp`
- Plan: `docs/execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md` (W5)
- Audit: `docs/audits/2026-10-31/remediation-and-diagnosis-v1-audit.md` (AUD-2026-10-31-37)

## Intent and boundary

W5's second leaf fixes the Minor finding that enriched MFE could be negative when
the entry fill sits outside the entry bar's range. The human approved D7=A:
clamp MFE at zero and MAE at zero, conforming to the diagnosis contract, while
keeping the measured favourable and adverse prices. The change is confined to
the excursion calculation and its tests.

## Implemented

- `excursions` clamps MFE at `0.0` with `.max(0.0)` and MAE at `0.0` with
  `.min(0.0)` for both sides.
- `max_favorable_price` and `max_adverse_price` remain the measured prices, so
  the fill-outside-the-bar anomaly is still visible.
- The `trade` test helper takes an explicit entry price.
- Added three tests: long fill above the bar high, short fill below the bar low,
  and an in-range fill that keeps its measured excursions.
- Updated the W5 validator record for #290.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo test -p quantrule-analytics --test diagnosis` | 12 passed, 0 failed |
| `cargo test --workspace` | 1516 passed, 0 failed, 7 ignored |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The long and short out-of-range cases assert `mfe = 0` and keep `mae = -3`; the
in-range case asserts `mfe = 1`, `mae = -1`, with the measured prices 111 and
109.

## Learning

- Clamping preserves the published invariant but hides the fact that the fill
  was outside the bar. Keeping the measured prices is what lets a reader see the
  anomaly; without them the clamp would erase it.
- D7=B (amend the contract to fill-relative signs) would have preserved the sign
  information but changed the contract. The human selected A, so the contract and
  the code now agree.
- The old formula yields `-1` in the two out-of-range cases; the new assertions
  would fail against it. The fail-before was derived from the formula rather than
  run, because the fix and the tests were committed together.

## Limitations

- The Quant Validator pass is sequential, not independent.
- The tests cover representative out-of-range and in-range fills, not a property
  over every possible price.
- The clamp is diagnosis-only; no strategy result changes.
