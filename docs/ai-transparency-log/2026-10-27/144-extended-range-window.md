# AI Transparency Log — Extended-Range M1 Window

- Date: 2026-10-27 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017
- Work item: [#144](https://github.com/quant-rule/quant-rule/issues/144)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md`](../../execution-plans/2026-10-27/141-market-analysis-and-backtest.md)
- ADR: [`docs/adr/2026-10-27/extended-range-window.md`](../../adr/2026-10-27/extended-range-window.md)
- Branch/worktree: `feat/144-extended-range-window` in `quant-rule--144-extended-range-window`
- PR: [#147](https://github.com/quant-rule/quant-rule/pull/147)
- Status: Implementation complete; PR #147 open.

## Intent and boundary

The human approved the program plan and asked to proceed after D0 and D1 merged. This work item removes the 100,000-candle blocker for a full-year analysis without changing the existing command contracts:

- Measure a 520,000-row M1 model run before choosing between a larger cap and streaming.
- Add an explicit extended cap to the data engine.
- Keep `market-*`, the store, and research at 100,000.
- Record the decision and the measurement in an ADR.

No command is added or changed. The shell loader is deferred to the first consumer (A3), and the fetch warning wording follows that command.

## Work performed

1. Read the window, timeframe-set, CSV loader, model run, and existing 100k benchmark paths.
2. Added an ignored 520k integration benchmark that mirrors the shell's full-series path: 520,000 M1 candles through the extended set, derived M5/M15/H1, copied model inputs, and a full `ModelRun`.
3. Ran the benchmark in release mode and measured wall time and peak RSS.
4. Added `EXTENDED_SERIES = 600_000` and `TimeframeSet::extended()`.
5. Changed `CandleWindow::with_limit` to start with at most `DEFAULT_WINDOW` capacity instead of reserving the full refusal bound in all four series.
6. Added the cap test and documented the extended constructor.
7. Wrote the ADR with the benchmark, the alternatives, the 100k/600k consumer table, and the wiring decision.
8. Indexed the ADR and updated the program plan's D1/D2 status and first-action line.

## Verification and evidence

- Benchmark command: `cargo test -p quantrule-model --test extended_bench --release -- --ignored --nocapture`.
- Observed: load 520,000 M1 candles in 0.042 s; model 667,332 bars in 16.047 s; total 16.089 s.
- Peak RSS: 841,580 KiB (822 MiB), measured with `/usr/bin/time -v` on the test binary.
- Machine: AMD Ryzen 3 7320U, 4 cores / 8 threads, 5.6 GiB RAM, Linux x86_64; rustc 1.99.1.
- `cargo test -p quantrule-data` — 101 unit tests and 2 doc tests passed.
- `cargo fmt --all -- --check` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo test` — passed; 0 failed. The 520k benchmark and the existing 100k benchmarks remain ignored by design.
- `cargo doc --no-deps` — passed.
- Relative documentation links were checked and resolve.

## Learning

- The 520k run is feasible as an in-memory fixed-retention set: 16 s and 822 MiB. Streaming would require a different model contract for little benefit at one instrument-year.
- Memory is dominated by the per-bar model series, not the candles. The two retained progression legs per bar are a large part of the per-bar cost; optimizing that is a model-contract decision, not part of this work item.
- A fixed-retention limit should be a refusal bound, not an up-front allocation. Reserving 600,000 slots in four series would cost about 134 MiB before the first row.
- Rejected: raising the global `MAX_SERIES`. It would change the existing model, store, and research contracts and their tests.
- Rejected: adding the shell loader now. It would be dead code until A3, and the fetch warning would advertise a command that does not exist.
