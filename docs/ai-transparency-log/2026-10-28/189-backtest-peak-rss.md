# AI Transparency Log — Backtest Peak RSS

- Date: 2026-10-28 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#189](https://github.com/quant-rule/quant-rule/issues/189)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md) §9.5
- Branch/worktree: `perf/189-backtest-peak-rss` in `quant-rule--189-backtest-peak-rss`
- PR: [#191](https://github.com/quant-rule/quant-rule/pull/191)
- Status: PR open; merge after checks pass.

## Intent and boundary

#189 brings the full-year `backtest` peak RSS under the plan's 2 GiB budget.
B4 measured 3.71–3.85 GiB, so the release was blocked. The work is confined to
the analytics crate's layer builder and summary, and to the shell's analytical
bundle; it changes no report field, no command contract, and no engine
computation. The five bundles must stay byte-identical.

## Measurement before the change

The release binary was measured per report to locate the peak:

| Command | Peak RSS | Wall time |
|---|---|---|
| `report model` | 795,632 KiB | 19.89 s |
| `report portfolio` | 811,228 KiB | 19.54 s |
| `report execution` | 811,904 KiB | 19.91 s |
| `report analytical` | 4,040,568 KiB | 28.54 s |
| `backtest` (B4) | 3,890,928 KiB | 29.14 s |

The pipeline peaked under 1 GiB; the analytical pass alone was 4 GiB. So the
budget was an analytical-report problem, not a backtest problem. The plan's
§9.5 fallback (run the analytical pass in a second pass) would have halved
nothing: the single analytical pass was the peak.

## Work performed

1. `TimeframeSeries` now borrows `&TimeframeModel` instead of owning a clone.
   `build_layers` keeps its models alive and lends them to the series;
   `build_layers_from_run` lends the run's models. This removes a full clone of
   the model run, which was the largest single allocation.
2. Added `summary::DatasetSummaryBuilder` and rewrote `summarize(&LayerSet)` on
   top of it, so the per-axis and per-pair accumulation has one implementation.
3. Added `layers::summarize_model_run(run, timeframes)`. It streams one bar at a
   time from the run, converts it to `AxisResolutions`, feeds the builder, and
   never materializes Layers 1–3. Layer 3's row count is the number of M1 bars
   that publish a snapshot, which is the condition the layer build uses.
4. Restored the `analytics.json.timeframes` filter that B3 dropped when it
   switched to `build_layers_from_run`. The function now takes the selected
   timeframes, and the analytical bundle passes the configured subset.
5. The shell's analytical bundle uses `summarize_model_run` and takes the
   actual period from the selected models rather than from layer rows.
6. Tests: the streamed summary equals the full-layer summary on a two-timeframe
   fixture; a subset keeps only the selected timeframe and reports zero Layer 3
   rows without an M1 base; the binary-level analytical test now asserts that a
   one-timeframe config produces four axis rows and six relation rows.

## Measurement after the change

| Command | Peak RSS | Wall time |
|---|---|---|
| `report analytical` | 795,748 KiB | 19.51 s |
| `report portfolio` | 811,608 KiB | 19.28 s |
| `backtest` | 811,760 KiB | 19.88 s |

Peak RSS fell from 3.89 GiB to 0.81 GiB (−79%), and wall time from 29.14 s to
19.88 s (−32%): assembling Layer 3 for 522,633 M1 bars was also the slow part.

## Decisions and corrections

- **Fix the cause, not the schedule.** The plan allowed a second pass, but the
  analytical pass was the peak, so splitting would not have met the budget. The
  streaming summary does, with no extra model computation.
- **Borrow instead of clone.** `build_layers_from_run` cloned every
  `TimeframeModel`; the clone existed only because `TimeframeSeries` owned its
  model. A lifetime removes it without changing any call site's meaning.
- **One accumulation implementation.** `summarize(&LayerSet)` and
  `summarize_model_run` share `DatasetSummaryBuilder`, so the two paths cannot
  drift. The equivalence test compares them on the same run.
- **Layer 3 is counted, not built.** The report needs `layer3_rows`, not the
  rows. Counting M1 bars whose snapshot is present is the same predicate
  `assemble` uses, and the full-year count (522,633) matches B4.
- **The timeframe filter is a defect fix, not a new feature.** B3's switch to
  `build_layers_from_run` silently published the whole ladder regardless of
  `analytics.json`. The subset test pins the contract back.
- `TimeframeSeries::snapshot_at` lost its `&mut self`; the model's
  `snapshot_at` was always immutable and the borrow exposed it.

## Verification and evidence

Commands were run in `quant-rule--189-backtest-peak-rss` on 2026-10-28.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; no warning |
| `cargo test` | 1,349 passed, 0 failed, 1 ignored (`store_100k`) |
| `cargo doc --no-deps` | Passed; exit 0; no warning |
| Full-year `backtest` vs B4 `run-a` | All five bundles identical (`diff -r`) |
| Full-year `--json` summary vs B4 | Identical after normalizing the output root |
| Full-year `backtest` peak RSS | 811,760 KiB against a 2 GiB budget |
| Full-year `report analytical` peak RSS | 795,748 KiB |

The test count rose from 1,346 to 1,349: the streamed-summary equivalence, the
subset, and the binary-level subset test. No existing test was weakened or
removed; the analytical artifact assertion still requires the same five files.

## Learning

- Measure per command before designing a fix. The assumption that the pipeline
  was the peak was wrong; one `time -v` per report located it in five minutes.
- A clone that exists for ownership convenience can cost hundreds of megabytes.
  A lifetime was the cheaper fix.
- A report needs statistics, not rows. The layer set is the right shape for the
  research export and the wrong shape for a summary over 670,000 bars.
