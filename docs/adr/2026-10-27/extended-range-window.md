# ADR: The Extended-Range M1 Window

- Date: 2026-10-27 (UTC)
- Issue: [#144](https://github.com/quant-rule/quant-rule/issues/144) — feat: add the extended-range M1 window
- Status: Accepted. The 600,000-candle bound and the fixed-retention behavior are the decision; implementation follows this document.
- Related: [program plan](../../execution-plans/2026-10-27/141-market-analysis-and-backtest.md), [analysis and backtest boundaries](analysis-and-backtest-boundaries.md), [history, retention, and display policy](../2026-10-25/history-retention-and-display-policy.md), [model contract](../../model/2026-10-21/four-axis-analytical-model.md)

## 1. Context

The data engine has two window shapes:

- [`CandleWindow::new`](../../../crates/quantrule-data/src/window.rs) and `TimeframeSet::new` retain the last 1,000 candles and evict the oldest. They serve the shell's inspection views.
- `CandleWindow::full` and `TimeframeSet::full` retain every candle up to `MAX_SERIES = 100,000` and reject the next one. They serve the existing model commands, the store, and research.

A full-year SOLUSD 2025 M1 series is roughly 520,000 rows. The existing full path rejects it at row 100,001, and `--window latest` streams it but keeps only the last 1,000 candles per timeframe. Neither is a full-year analysis.

The program plan delegates the choice between an explicit larger cap and a streaming path to this work item, backed by a benchmark. The uncertainty is memory, not throughput: the model computes and retains one series per timeframe for the whole run.

## 2. Benchmark

The benchmark mirrors the shell's full-series path: 520,000 M1 candles are pushed through the extended set, M5/M15/H1 are derived, the four series are copied into the model input vectors, and the model runs over the ladder. It is an ignored integration test:

```bash
cargo test -p quantrule-model --test extended_bench --release -- --ignored --nocapture
```

Observed on 2026-10-27:

| Item | Value |
|---|---|
| Input | 520,000 M1 candles, 2025-01-01 00:00 UTC onward |
| Derived series | 104,000 M5, 34,666 M15, 8,666 H1 |
| Model bars | 667,332 across four timeframes |
| Extended-set load | 0.042 s |
| Model run | 16.047 s |
| Total | 16.089 s |
| Peak resident set size | 841,580 KiB (822 MiB) |
| Profile | `--release` |
| Toolchain | rustc 1.99.1 (58cf8f9c3 2026-10-13) |
| Machine | AMD Ryzen 3 7320U, 4 cores / 8 threads, 5.6 GiB RAM, Linux x86_64 |

The peak includes the retained timeframe set, the copied model-input vectors, and the model's own per-timeframe series. The memory is dominated by the per-bar model series, not by the candles: 667,332 bars cost about 1.2 KiB each.

The benchmark is evidence for one instrument-year on this machine. It does not promise the same figure on a smaller machine, and it does not cover the market-report or backtest report, which do not exist yet.

## 3. Decision

### 3.1 Explicit bound

Add `EXTENDED_SERIES = 600_000` M1 candles. It covers a 365-day year (525,600 minutes) and a leap year (527,040) with headroom for provider padding, while still refusing a multi-year file.

`TimeframeSet::extended()` builds a fixed-retention set with that bound. Like `TimeframeSet::full()`, it never evicts: the 600,001st M1 candle is rejected with `WindowError::CapacityExceeded`, so a truncated series can never look complete.

### 3.2 The bound is not an allocation

A fixed-retention window is a refusal bound, not a reservation. `CandleWindow::with_limit` now starts with at most `DEFAULT_WINDOW` capacity and grows as candles arrive. Reserving 600,000 slots in all four series up front would allocate about 134 MiB before the first row, most of it for series that can never reach that length.

### 3.3 Existing limits stay

| Consumer | Bound |
|---|---|
| `market-state`, `axis`, `events`, `legs`, `model-series`, `market-dashboard`, `market-chart` (full) | `MAX_SERIES = 100,000` |
| `research export`, `store ingest` | `MAX_SERIES = 100,000` |
| `market-report`, `backtest` | `EXTENDED_SERIES = 600,000` |

No existing command gains a new window mode, and no existing command's accepted values change.

### 3.4 Wiring

The shell CSV reader is already streaming and size-agnostic: `load_timeframe_set` takes the set it fills. The extended shell loader is added with the first consumer (`market-report`, work item A3) and reused by `backtest` (B3). Adding an unused loader now would be dead code, and advertising the extended range in the fetch warning before a command accepts it would describe a capability the user cannot type.

The fetch warning keeps its current, accurate wording until the first extended command ships; that command's pull request updates it to name the new bound.

## 4. Alternatives considered

| Alternative | Why rejected |
|---|---|
| Stream the model over the input | The model computes per-timeframe series, confirmed levels, events, and progression state for the whole run; a streaming model would need a different contract and a different report assembly. The measured 16 s / 822 MiB fits one instrument-year, so the rewrite is not justified for v1. |
| Raise `MAX_SERIES` to 600,000 for every command | It would change the existing model, store, and research contracts and their tests. A full-year input would silently become valid for paths that are not designed or benchmarked for it. |
| Add `--window extended` to the existing `market-*` commands | It changes an existing command contract. The program's rule is that a new capability arrives as a new command. |
| Keep the 100k cap and require the user to split the year | The requested use case is a full-year run; splitting introduces boundary effects and makes the report's coverage a user responsibility. |
| A cap of 1,000,000 or unlimited | A multi-year file would be accepted, and the memory cost would grow without a published bound. 600,000 is the smallest bound that covers one instrument-year with headroom. |

## 5. Consequences

- A 2025 SOLUSD M1 file loads into the extended set without truncation; the cap refusal is deterministic and reports the limit.
- The market-report path needs about 0.85 GiB of peak memory and about 16 s of model time on the benchmark machine. The backtest can run the model over the timeframes its strategy needs; that choice belongs to the runner, not to this window.
- The extended bound is a data-engine constant, not a shell flag. A future instrument-year that exceeds 600,000 M1 rows requires a new decision and an ADR update.
- The store and research still reject a full-year file. Persisting a full-year backtest to the store is out of scope for this program.
- `CandleWindow::with_limit` no longer reserves the full limit. Existing fixed windows grow to their data instead of preallocating, which lowers memory for every full-series command; semantics are unchanged.

## 6. Verification

- `cargo test -p quantrule-data` — the window and timeframe-set suites pass, including the new cap test.
- The ignored benchmark above loads 520,000 M1 candles, derives 104,000 M5 / 34,666 M15 / 8,666 H1, and runs the model over 667,332 bars.
- The existing 100,000-candle boundary tests are unchanged and pass.
- The extended refusal is the same code path as the 100,000 refusal; `a_fixed_window_rejects_the_candle_over_its_limit` and `a_full_set_rejects_the_candle_over_its_cap` cover it without generating 600,000 rows.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `cargo doc --no-deps` pass in the work item's pull request.

## 7. Non-goals

- Multi-year or multi-instrument extended inputs.
- A streaming model, a chunked model, or an out-of-core report.
- Raising the store or research cap.
- Changing an existing command's window behavior.
- Persisting a full-year run to the store.
