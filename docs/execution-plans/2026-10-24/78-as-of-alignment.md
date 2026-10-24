# Execution Plan: As-Of Timeframe Alignment (AN3)

- Issue: [#78](https://github.com/quant-rule/quant-rule/issues/78) — feat: add as-of timeframe alignment
- Branch: `feat/78-as-of-alignment`
- Pull request: pending
- Status: Verified
- Date: 2026-10-24 (UTC)
- Contract: [`docs/analytics/2026-10-24/research-output.md`](../../analytics/2026-10-24/research-output.md) §7.2
- Transparency log: [`docs/ai-transparency-log/2026-10-24/78-as-of-alignment.md`](../../ai-transparency-log/2026-10-24/78-as-of-alignment.md)

## 1. Intent

Implement the causal as-of rule that Layer 3 uses to attach the latest closed higher-timeframe bar to each M1 bar: choose the greatest higher open time whose close is at or before the base close, and publish the staleness.

## 2. Current state

- `main` is at `02262e3`; the analytics crate and resolution are merged (#77).
- The crate has `resolution` only; no alignment code exists.
- The contract fixes the formula and the boundary examples; the store's Layer 3 materialization (#73) depends on this rule.

## 3. Scope

Included:

- `alignment::latest_closed_index` and `alignment::staleness_secs`.
- Unit tests for the contract's boundary examples, the open containing bar, the first closed bar, an unaligned boundary, no closed bar, non-negative staleness, and causality.
- Rustdoc examples for both functions.

Excluded:

- The Layer 3 row and block types (#79).
- The store tables (#73).
- Any shell command or export code.

## 4. Assumptions and uncertainty

- The higher open times are sorted ascending; the function binary-searches and documents the requirement.
- Timestamps are UTC Unix seconds, and a timeframe's close is `open_time + timeframe_secs`.
- Realistic timestamps do not overflow; the function still uses checked arithmetic and returns `None` instead of wrapping.
- The issue says "staleness minutes"; the merged contract fixes `staleness_secs`. Seconds are published and minutes remain derivable. The contract supersedes the issue's unit.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Staleness unit | Minutes; seconds | Seconds | AN1 §7.2 and the merged contract fix `staleness_secs`; the issue's "minutes" is superseded |
| D2 | Return shape | Open time; index | Index | The layer builder needs the bar index to read the block and the open time from the same slice |
| D3 | Sorted input | Sort inside; require sorted | Require sorted and document it | Sorting on every call would hide a caller defect and add cost at 100k rows |
| D4 | Future higher bar | Negative staleness; `None` | `None` | The contract's staleness is non-negative; a future bar is not an as-of candidate |
| D5 | Overflow | Wrap; saturate; `None` | `None` via checked arithmetic | A wrapped timestamp would leak a future bar into a past row |
| D6 | Clock | Read the clock; take timestamps as parameters | Parameters only | The engine is pure; no clock in the signal path |

## 6. Acceptance criteria

- 09:03 uses the previous M5 bar and 09:04 uses 09:00–09:05.
- The staleness is non-negative and equals the contract's boundary values (0, 60, 240, 600 seconds).
- A causality test proves later bars do not change an earlier answer.
- An empty higher series and a base bar before the first higher close return `None`.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.

## 7. Work breakdown

1. Implement `alignment`.
2. Add the 10 boundary and causality tests.
3. Add the two rustdoc examples.
4. Run the checks, write the plan and the log, commit, push, open the PR, move #78 to Done, merge, and clean up.
5. Continue with #79 on its own branch.

## 8. Verification

| Check | Result |
|---|---|
| `cargo test -p quantrule-analytics` | 20 passed, 0 failed; 2 doctests passed |
| `cargo test --workspace` | 699 passed, 0 failed, 1 ignored |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | clean |
| Boundary assertions | 09:03 → 240 s; 09:04 → 0 s; 08:59 → 0 s; 09:00 → 60 s; unaligned M5/H1 → 600 s |

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| A caller passes unsorted opens | The requirement is documented; #79 sorts its inputs |
| The minutes/seconds wording is read as a defect | The plan and the PR record that the merged contract supersedes the issue text |
| Overflow wraps and picks a future bar | Checked arithmetic returns `None`; the causality test covers the normal path |
| The alignment is recomputed per row at 100k | It is a binary search per row; #79 measures the layer build and #82 reports the dataset run |

## 10. Stop conditions

- The contract's boundary examples cannot be reproduced.
- A competing staleness unit is required.
- A check fails and the fix is outside this scope.

## 11. Delivery

| Item | Value |
|---|---|
| Commit | pending |
| Branch | `feat/78-as-of-alignment` |
| Pull request | pending |
| Board | #78 moved to `Done` when the PR is open |

Merge, release, tag, and deployment remain with the human.
