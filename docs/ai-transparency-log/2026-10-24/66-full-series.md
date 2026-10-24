# AI Transparency Log: The Full-Series 100k Candle Path (ST2)

- Work item: [#66](https://github.com/quant-rule/quant-rule/issues/66) — feat: add a full-series 100k candle path to the data engine
- Plan: [`docs/execution-plans/2026-10-24/66-full-series.md`](../../execution-plans/2026-10-24/66-full-series.md)
- Date: 2026-10-24 (UTC)

## Entry: the full-series path

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Add a data-engine path that retains up to 100,000 M1 candles without eviction, leaving the bounded path and the existing shell commands unchanged.
- **Verification and evidence:**
  - Added `MAX_SERIES`, `CandleWindow::with_limit`/`full`, `CapacityExceeded`, and `TimeframeSet::with_limit`/`full`.
  - 9 data-engine tests pass: the 100k series with contiguous indices, the over-cap rejection, invalid/duplicate/out-of-order rows, a gap, zero volume, and bounded/full equivalence.
  - `cargo test --workspace`: 722 passed, 0 failed, 2 ignored.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
  - The 100k release run reports 128,332 candles, 8,213,248 bytes of candle storage, and 0.008 s; the debug run is 0.066 s.
- **Learning:**
  - The first plan was a separate `CandleSeries` type. Reading `window.rs` showed that it already owns validation, stable index assignment, duplicate/out-of-order rejection, and the eviction boundary. A second type would have duplicated those rules and could drift; a fixed-retention mode on the existing type is the smaller and safer change (D1).
  - The cap had to be an error, not eviction. A full window that evicts would publish a 100k series whose first candle is not the dataset's first candle, and no count would reveal it (D2).
  - The shell's exhaustive `WindowError` match forced a decision in the tolerant loader. A capacity rejection is not a row-level skip: skipping it would reject every later row for the same reason. The loader now aborts with the existing candle error, and the arm is unreachable for the bounded window it creates (D4).
  - `TimeframeSeries` derives `PartialEq`, so the new retention fields make a bounded and a full series unequal even when their candles match. The equivalence test compares the candles and gaps explicitly rather than weakening the type's equality (D6).
  - The derived counts are exact for aligned input: 100,000 M1 minutes give 20,000 M5, 6,666 M15, and 1,666 H1 candles; the trailing partial buckets are dropped and recorded, as in the bounded path.
  - `--tail` was not added. It is a source-selection policy for the consuming command; putting it in the data engine would make the engine choose how much of a source to ignore (D5).

## Final verification

- `cargo test -p quantrule-data`: 99 passed, 0 failed.
- `cargo test --workspace`: 722 passed, 0 failed, 2 ignored.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- 100k release: 8,213,248 bytes of candle storage in 0.008 s.
- Status: `Verified`. Commit `12c05da` is pushed on `feat/66-full-series`, pull request [#95](https://github.com/quant-rule/quant-rule/pull/95) is open, and #66 is `Done` on the board.
