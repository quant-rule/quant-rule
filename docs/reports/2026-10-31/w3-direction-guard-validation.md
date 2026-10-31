# W3 — Divergence Direction Guard Validation

- Date: 2026-10-31 (UTC)
- Issue: #282
- Finding: AUD-2026-10-31-33
- Plan: `docs/execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md` (W3)
- Validator: Quant Validator role, sequential pass by the implementation agent (not independent)
- Claim type: model aggregate correctness; no strategy-rule or signal-path change

## Claim

`CrossTimeframeAggregates::compute` ignores a divergence direction outside
`-1..=1` instead of overflowing. The three valid codes are still counted, and
the aggregate output for valid snapshots is unchanged.

## Fail-before

The regression test was added before the guard changed. On the unfixed code:

```text
cargo test -p quantrule-model out_of_range_divergence_directions_do_not_panic_or_count
test aggregate::tests::out_of_range_divergence_directions_do_not_panic_or_count ... FAILED
panicked at crates/quantrule-model/src/aggregate/events.rs:131:52:
attempt to add with overflow
```

The panic is `direction + 1` evaluated as `i8` with `direction = i8::MAX`.

## Fix

```rust
direction
    .checked_add(1)
    .and_then(|value| usize::try_from(value).ok())
    .filter(|index| *index < divergence_directions.len())
```

`i8::MAX` yields `None` from `checked_add`; `i8::MIN` and other negatives fail
`try_from`; `2` and above are filtered by the three-slot length. `-1`, `0`, and
`1` map to the three bins.

## Reproduction

| Command | Observed |
|---|---|
| `cargo test -p quantrule-model out_of_range_divergence_directions_do_not_panic_or_count` | 1 passed, 0 failed |
| `cargo test -p quantrule-model` | all pass, 3 ignored |
| `cargo test --workspace` | 1509 passed, 0 failed, 7 ignored |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |

The new test carries `i8::MAX`, `i8::MIN`, `2`, `-1`, `0`, and `1` in six
timeframes; it asserts six divergence timeframes and a direction agreement
share of `1/3` from the three valid directions. The existing
`event_counts_use_the_five_bar_window` test and the model golden suites pass
without changes, which is the no-change evidence for valid input.

## Checks

- Signal path: the aggregate is a reporting view; no strategy decision reads it.
- Determinism: the guard is pure integer arithmetic; no clock, randomness, or
  unordered iteration was added.
- API: the public `direction_code: i8` field is unchanged; the guard is
  defensive at the aggregation boundary.

## Residual risk

- The public snapshot still accepts any `i8` direction. A validated newtype or
  enum (finding option C) would prevent the malformed value at construction,
  but that is a wider API change and was not selected.
- The validator pass is sequential, not independent. The human remains the
  final acceptance authority.
