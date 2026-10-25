# Execution Plan: Chart the Last 1000 Candles with the Four Axes

- Issue: [#106](https://github.com/quant-rule/quant-rule/issues/106) — feat: chart the last 1000 candles with the four axes
- Branch: `feat/106-market-chart`
- Pull request: [#108](https://github.com/quant-rule/quant-rule/pull/108)
- Status: Verified
- Date: 2026-10-25 (UTC)
- Transparency log: [`docs/ai-transparency-log/2026-10-25/106-market-chart.md`](../../ai-transparency-log/2026-10-25/106-market-chart.md)

## 1. Intent

Add a deterministic PNG view of the last OHLCV bars with four aligned panels
for the continuous Direction, Momentum, Structure, and Progression resolutions.
The chart is a presentation layer over values the analytics engine already
publishes; it does not recompute an axis, an indicator, or an event.

## 2. Current state

- `LayerSet.layer2` already carries every bar's candle and four axis
  resolutions.
- The shell has a deterministic candle + volume PNG renderer for `fetch`.
- `crates/quantrule-shell/src/chart/mod.rs` is 759 lines and owns the canvas,
  embedded font, candle chart, and PNG encoder.
- No command renders the four axis panels.
- `DEFAULT_CHART_CANDLES` is 1,000; the source loader already builds the full
  M1/M5/M15/H1 ladder.

## 3. Scope

Included:

- A pure `ChartSeries` selector in `quantrule-analytics` over `LayerSet.layer2`.
- A new `market-chart` command writing a PNG with a candle/volume panel and four
  axis panels.
- A new `chart/market.rs` renderer using the existing canvas and embedded font.
- Deterministic output, parser errors, help, README, and tests.

Excluded:

- Interactive or HTML charts.
- Charting from the SQLite store.
- Overlaying multiple timeframes; one `--timeframe` is charted at a time.
- Indicators beyond the four published axes.
- New dependencies or a new crate.
- Golden PNG fixtures; the test asserts dimensions, determinism, and the
  command contract without pinning encoder bytes across versions.

## 4. Assumptions and uncertainty

- The four continuous resolutions share the `[-1, 1]` scale and are the correct
  visual summary; raw metrics remain available through the research output and
  the store.
- The last 1,000 bars of the selected timeframe are enough for inspection.
- The existing canvas text and line primitives are deterministic and reusable by
  a child module.
- `chart/mod.rs` remains below 1,000 lines if the new renderer lives in
  `chart/market.rs`. This is a deliberate deviation from the roadmap's proposed
  full split: the measured file is 759 lines, and a mechanical split would add
  regression risk without a size need.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Command | `market-state --png`; `chart`; `market-chart` | `market-chart` | Keeps the JSON command stable and distinguishes the model chart from the fetch chart |
| D2 | Panel values | Raw metrics; continuous resolutions; both | Continuous resolutions in `[-1, 1]` | One comparable scale and the exact Layer 2 value |
| D3 | Timeframe | One selected; overlay all | One `--timeframe`, default `1m` | Higher timeframes have different x positions; one series is unambiguous |
| D4 | Window | Fixed 1000; configurable | `--candles` 1..=1000, default 1000 | Matches the requested inspection window and the existing chart cap |
| D5 | Module placement | Split the whole chart module; add a child module | Add `chart/market.rs`; keep the existing canvas in `mod.rs` | Keeps the file under 1,000 lines and the fetch chart unchanged |
| D6 | Volume | Omit; separate panel; strip inside the candle panel | A slim strip inside the candle panel | Preserves OHLCV while keeping the five-panel layout |
| D7 | Output | No summary; one deterministic line | `png=<path> timeframe=<tf> candles=<n> bytes=<n>` | Discoverable result with a stable contract |

## 6. Acceptance criteria

- `market-chart <csv-path> --png <path> [--timeframe <tf>] [--candles <n>]`
  writes a PNG with a candle/volume panel and Direction, Momentum, Structure,
  and Progression panels.
- The chart uses the last `n` bars of the selected timeframe, oldest first; a
  shorter series is drawn in full.
- Missing axis values leave gaps; no axis value is replaced by zero.
- The same input and configuration produce byte-identical PNG bytes and the
  same stdout.
- Interactive and non-interactive output are identical for the same path.
- Missing `--png`, an unknown timeframe, a duplicate option, or a candle count
  outside `1..=1000` fails before writing a file.
- Help and README document the command.
- `cargo fmt`, `cargo clippy`, `cargo test --workspace`, and `cargo doc` are
  clean.

## 7. Work breakdown

1. **Analytics selector.** Add `crates/quantrule-analytics/src/chart.rs` with
   `ChartBar` and `ChartSeries::from_layers`; export the module and add unit
   tests. Affected: `analytics/src/lib.rs`, `analytics/src/chart.rs`.
2. **PNG renderer.** Add `chart/market.rs` with the five-panel layout, shared
   x-axis, volume strip, axis lines, and deterministic labels. Export
   `render_market_png` from `chart/mod.rs`. Affected: `shell/src/chart/mod.rs`,
   `shell/src/chart/market.rs`.
3. **Command.** Add `MarketChartArgs`, parsing, help, interactive and
   non-interactive dispatch, and `run_market_chart`. Affected:
   `shell/src/command.rs`, `shell/src/main.rs`.
4. **Tests.** Analytics selection tests; renderer dimensions/determinism/empty
   tests; parser tests; binary test for PNG signature, dimensions, output, and
   repeated-run byte identity. Update the help fixture.
5. **Documentation.** README command table and prose, this plan, and the
   transparency log.
6. **Evidence.** Run focused and full checks; record results; open the pull
   request; move #106 to Done; stop for review.

## 8. Verification

| Check | Command | Expected evidence |
|---|---|---|
| Format | `cargo fmt --all -- --check` | Clean |
| Lints | `cargo clippy --all-targets -- -D warnings` | Clean |
| Tests | `cargo test --workspace` | Exact counts recorded |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean |
| Analytics | `cargo test -p quantrule-analytics chart` | Selection, limit, empty, determinism |
| Shell chart | `cargo test -p quantrule-shell chart` | Dimensions, pixels, determinism, empty |
| Binary | `cargo test -p quantrule-shell --test market_chart` | PNG signature/dimensions, output, repeated runs |

## 9. Risks and recovery

- **Chart readability.** The four panels use a fixed `[-1, 1]` scale; a missing
  value is a gap, not a zero. The binary test decodes the PNG and checks its
  dimensions; visual quality is reviewed by the human.
- **x-axis alignment.** Candles, volume, and all four panels share one x
  mapping. A mismatch would misalign evidence, so one `x_for(index, count)` is
  used by every panel.
- **File size.** The new renderer is a child module; `chart/mod.rs` stays under
  the limit. If it grows past 1,000 lines, split the canvas into its own module.
- **Encoder drift.** PNG bytes can change across `png` crate versions. The
  deterministic-byte test is within one build; no cross-version golden is
  asserted.
- **Overwrite.** The command writes atomically and rejects a PNG path equal to
  the input CSV path.
- **Recovery.** The command writes one file; a failure leaves the previous file
  intact through the atomic write.

## 10. Stop conditions

- A new dependency is required.
- The chart needs a system font, clock, or non-deterministic input.
- The requested timeframe cannot be selected from `LayerSet.layer2`.
- The command would need a store dependency.
- The pull request is open but not reviewed or merged.

## 11. Verification evidence (2026-10-25)

| Check | Command | Observed result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | Clean, exit 0 |
| Lints | `cargo clippy --all-targets -- -D warnings` | Clean, exit 0 |
| Tests | `cargo test --workspace` | 840 passed, 0 failed, 3 ignored |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean, exit 0 |
| Analytics focus | `cargo test -p quantrule-analytics chart` | 6 passed, 0 failed |
| Shell chart focus | `cargo test -p quantrule-shell chart::` | 13 passed, 0 failed |
| Binary focus | `cargo test -p quantrule-shell --test market_chart` | 6 passed, 0 failed |

The binary test over `state_m1.csv` with `--candles 3` writes a 1280x960 RGB
PNG and reports `timeframe=1m candles=3`; two separate output paths receive
byte-identical PNGs. The `5m`/2-bar case reports `timeframe=5m candles=2`. A
missing `--png`, an unknown timeframe, and an empty source fail without writing
a PNG. Interactive and non-interactive output match.

The full 260-bar `state_m1.csv` chart was rendered for visual inspection:
1280x960, 22,277 bytes, with a candle/volume panel and four aligned axis panels.
The Direction panel begins where the axis becomes available; missing values are
gaps, not zeros.

Limitations: the chart is intentionally a single timeframe and a fixed
`[-1, +1]` axis scale. It does not overlay timeframes or chart from the store.
No golden PNG is checked in because PNG bytes can change with the encoder
version; same-build determinism is tested.
