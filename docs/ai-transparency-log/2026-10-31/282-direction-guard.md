# AI Transparency Log — Divergence Direction Guard

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #282, branch `fix/282-direction-guard`
- Plan: `docs/execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md` (W3)
- Audit: `docs/audits/2026-10-31/remediation-and-diagnosis-v1-audit.md` (AUD-2026-10-31-33)

## Intent and boundary

W3 fixes the Minor finding that the divergence direction guard still overflows
for `direction_code = i8::MAX`. The change is confined to the event aggregator
and its tests. It does not change a model axis, a strategy rule, execution, or
portfolio accounting, and it does not touch the signal path.

The prior audit's AUD-23 claimed an out-of-range direction test. No such test
existed; this work adds it and reproduces the panic before the fix.

## Implemented

- Replaced `usize::try_from(direction + 1)` with
  `checked_add(1)` then `usize::try_from` then a three-slot bounds filter.
- Added `out_of_range_divergence_directions_do_not_panic_or_count` with
  `i8::MAX`, `i8::MIN`, `2`, `-1`, `0`, and `1` across six timeframes.
- Recorded the Quant Validator pass in
  `docs/reports/2026-10-31/w3-direction-guard-validation.md`.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo test -p quantrule-model out_of_range_divergence_directions_do_not_panic_or_count` before the fix | FAILED: `attempt to add with overflow` at `events.rs:131` |
| Same test after the fix | 1 passed, 0 failed |
| `cargo test -p quantrule-model` | all pass, 3 ignored |
| `cargo test --workspace` | 1509 passed, 0 failed, 7 ignored |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The existing aggregate test with valid `-1` and `1` directions and the model
golden suites pass without changes.

## Learning

- The fail-before test is the direct evidence AUD-23 lacked: the unfixed code
  panics in a debug build at the exact addition the audit identified. A
  verification column is a claim to reproduce, not proof.
- The checked-arithmetic form keeps the guard local and avoids a new public
  type; a validated newtype remains the stronger long-term shape but is a wider
  API decision.
- The invalid codes still count as divergence timeframes (the timeframe did
  emit a divergence); they are excluded only from the direction agreement
  denominator. The test asserts both behaviors, so the distinction is pinned.

## Limitations

- The Quant Validator pass is sequential, not independent.
- The test uses constructed snapshots, not a full backtest; the aggregate has
  no I/O or clock, so the fixture is sufficient for this guard.
- The public `i8` field remains unvalidated at construction; the residual risk
  is recorded in the validator record.
