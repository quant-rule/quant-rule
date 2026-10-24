# Execution Plan: The Full-Series 100k Candle Path (ST2)

- Issue: [#66](https://github.com/quant-rule/quant-rule/issues/66) — feat: add a full-series 100k candle path to the data engine
- Branch: `feat/66-full-series`
- Pull request: [#95](https://github.com/quant-rule/quant-rule/pull/95)
- Status: Verified
- Date: 2026-10-24 (UTC)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/66-full-series.md`](../../ai-transparency-log/2026-10-24/66-full-series.md)

## 1. Intent

Give the data engine a path that retains a whole dataset of up to 100,000 M1 candles instead of the last 1,000. The research export (#80) and the store ingest (#68) both need it; the existing bounded window and the commands that use it must not change.

## 2. Current state

- `CandleWindow` retains `DEFAULT_WINDOW = 1_000` candles and evicts the oldest.
- `TimeframeSet` builds M1/M5/M15/H1 from M1 into four bounded windows, so a 100k M1 input would lose the first 99,000 candles and the higher series would also be truncated.
- The shell's `candles`, `candles-tolerant`, `timeframes`, `market-state`, `axis`, `events`, and `legs` commands use the bounded path.
- No full-series consumer exists yet; #80 is paused on this issue.

## 3. Scope

Included:

- `MAX_SERIES = 100_000` and `CandleWindow::with_limit` / `CandleWindow::full` with fixed retention.
- `WindowError::CapacityExceeded { limit }`.
- `TimeframeSet::with_limit` and `TimeframeSet::full`.
- Tests for the cap, contiguous indices, a 100k series, invalid/duplicate/out-of-order rows, gaps, zero volume, and bounded/full equivalence.
- The shell's error mapping for the new variant.

Excluded:

- A `--tail` flag; that is a policy of the consuming command (#80, #68).
- Any change to the bounded path's behavior.
- The SQLite store and the research export themselves.

## 4. Assumptions and uncertainty

- The 100k cap applies to M1; the derived timeframes are smaller and are capped by the same limit.
- A full series is a `CandleWindow` in fixed-retention mode; the type name is historical, and the mode is documented.
- The trailing M5/M15/H1 buckets that are incomplete at the end are dropped and recorded as partial, exactly as in the bounded path.
- The shell's tolerant loader uses the bounded window, so its new capacity arm is unreachable today; it aborts rather than skipping, because a full series cannot accept any later row.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Full-series type | A new `CandleSeries`; a fixed-retention mode on `CandleWindow` | A mode on `CandleWindow` | The existing type already owns validation, index assignment, and ordering; a second type would duplicate those rules |
| D2 | Over-cap behavior | Evict the oldest; error | `CapacityExceeded` error | A truncated series that looks complete is worse than a failed ingest |
| D3 | Set constructor | Change `TimeframeSet::new`; add `full` | Add `full`/`with_limit` | The bounded commands must keep their behavior |
| D4 | Shell tolerant path | Add a capacity skip category; abort | Abort with the existing candle error | A capacity rejection is not a skippable row; the category would change the report for an unreachable case |
| D5 | `--tail` | Data-engine option; consumer option | Consumer option | Truncation is a source-selection policy, not candle validation |
| D6 | Equality | Compare retention in `PartialEq`; ignore it | Keep the derived `PartialEq` | The retention is part of the series configuration; the equivalence test compares candles and gaps explicitly |

## 6. Acceptance criteria

- A 100k M1 full set keeps 100,000 M1 candles with contiguous indices 0..99,999.
- The derived series have 20,000 M5, 6,666 M15, and 1,666 H1 candles for 100k aligned minutes.
- The 100,001st M1 candle is rejected and the series is unchanged.
- Invalid, duplicate, and out-of-order rows return the existing error codes.
- A gap is recorded, and a zero volume is preserved.
- The bounded and full sets derive identical candles for a short input.
- The existing shell commands and their tests are unchanged.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.

## 7. Work breakdown

1. Add the cap and the fixed-retention mode to `CandleWindow`.
2. Add `TimeframeSet::full` and `with_limit`.
3. Add the data-engine tests and the 100k report.
4. Update the shell's `WindowError` mapping.
5. Run the checks and the release benchmark, write the plan and the log, commit, push, open the PR, move #66 to Done, merge, and clean up.
6. Resume #80 on its own branch.

## 8. Verification

| Check | Result |
|---|---|
| `cargo test -p quantrule-data` | 99 passed, 0 failed |
| `cargo test --workspace` | 722 passed, 0 failed, 2 ignored |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | clean |
| 100k release benchmark | 128,332 candles, 8,213,248 bytes of candle storage, **0.008 s** |
| 100k debug run | 0.066 s |

The memory figure is the candle storage (`len * size_of::<Candle>()`), not the allocator's capacity; the `VecDeque` and the gap/partial vectors add a small overhead.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| A full series grows without bound through a direct `CandleWindow::full` user | The fixed mode rejects at `MAX_SERIES`; only `TimeframeSet::full` uses it |
| The bounded path changes | It is a separate constructor; all existing tests pass unchanged |
| The shell's tolerant report changes | The capacity arm aborts and only runs if a caller uses a fixed window there; the bounded path never hits it |
| The 100k test is slow | 0.066 s debug, 0.008 s release; it stays a regular test |
| The memory figure understates the real footprint | The plan and PR state that it counts candle storage only |

## 10. Stop conditions

- The cap cannot be enforced without changing the bounded path.
- A full-series consumer needs a different cap or a streaming design.
- A check fails and the fix is outside this scope.

## 11. Delivery

| Item | Value |
|---|---|
| Commit | `12c05da` — `feat: add a full-series 100k candle path` |
| Branch | `feat/66-full-series` |
| Pull request | [#95](https://github.com/quant-rule/quant-rule/pull/95) |
| Board | #66 moved to `Done` when the PR is open |

Merge, release, tag, and deployment remain with the human.
