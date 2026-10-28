# AI Transparency Log — Market-Report Command

- Date: 2026-10-27 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#151](https://github.com/quant-rule/quant-rule/issues/151)
- Container: [#148](https://github.com/quant-rule/quant-rule/issues/148)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md`](../../execution-plans/2026-10-27/141-market-analysis-and-backtest.md)
- Contract: [`docs/model/2026-10-27/market-report.md`](../../model/2026-10-27/market-report.md)
- Branch/worktree: `feat/151-market-report-command` in `quant-rule--151-market-report-command`
- Status: Implementation, tests, and documentation complete; commit and pull request pending.

## Intent and boundary

A3 exposes the pure `MarketReport` view through the `market-report` command. The shell parses the contract, loads the extended M1 range, calls the model, renders the four artifacts, and writes them atomically. It computes no coverage, histogram, event count, bucket, or axis value. The command is descriptive: it publishes no order, fill, position, trade, equity, drawdown, or P&L field, and it applies no strategy.

The work changes `quantrule-shell` only, plus the README and the program documents. It adds no dependency, store migration, model field, or existing-command contract change. The one existing message it touches is the fetch over-cap warning, whose wording D2 deferred to the first extended command; `fetch` keeps its arguments and output.

## Work performed

1. Added `DateBounds` to `crates/quantrule-shell/src/source/date.rs`. It parses inclusive `YYYY-MM-DD` bounds, rejects a reversed range, converts the end date to an exclusive second, and exposes the start and end seconds to the loader.
2. Added `load_extended_timeframes` to `crates/quantrule-shell/src/csv.rs`. It filters M1 rows by the bounds before `TimeframeSet::push`, so the ladder is built from the requested period only and no pre-range warm-up is used. The shared loader keeps the existing 100k path unchanged.
3. Added `model_run_from_set` to `crates/quantrule-shell/src/command/model.rs`, so the command calls the same model construction as the existing model commands.
4. Added `Command::MarketReport`, `MarketReportArgs`, `ParseError::MarketReport`, `ReportError`, and `RunError::Report` to `command/types.rs`.
5. Added `command/report.rs`: argument parsing, output-directory validation, the full-period run, artifact rendering, atomic writes through `export::write_atomic`, partial-set cleanup, and parser/atomic-failure unit tests.
6. Added `render_report/mod.rs` for `summary.txt` and `render_report/json.rs` for the `report.json` envelope. The summary uses relative artifact names in its `[artifacts]` section, so its bytes do not depend on the output path.
7. Added `chart/report.rs` with the 1280×960 price-and-axes chart and the 1280×720 events chart. Both draw the report's precomputed buckets and event markers.
8. Wired `market-report` into `command.rs`, `parse.rs`, `dispatch.rs`, `help.rs`, and `main.rs`.
9. Added `tests/market_report.rs` with 12 binary-level tests: artifact names and stdout order, exact text/JSON/PNG goldens, `--json` byte equality, determinism and interactive parity, date filtering before the ladder build, an existing empty directory, a non-empty directory, a file output path, an empty range, absence of execution vocabulary, bad arguments, and help.
10. Generated the four checked-in goldens from the real binary over `state_m1.csv` with `--data-version test:v1`, and regenerated `tests/fixtures/help.txt`.
11. Updated the README command table and added the `market-report` contract section; updated the help text, the program plan, `docs/README.md`, the contract status line, and the fetch over-cap warning.

## Verification and evidence

- `cargo fmt --all -- --check` — passed.
- `cargo clippy --all-targets -- -D warnings` — passed. The first run found one `write_literal` in the coverage header; the literal `last` was folded into the format string, the output stayed identical, and no lint was suppressed.
- `cargo test` — 1,089 passed, 0 failed, 6 ignored. The ignored tests are the pre-existing 100k/520k benchmarks plus the A2 520k market-report test.
- `cargo test -p quantrule-shell --test market_report` — 12 passed, 0 failed.
- `cargo test -p quantrule-shell --bin quantrule -- render_report chart::report command::report` — 15 passed, 0 failed.
- `cargo doc --no-deps` — passed without warnings.
- Manual smoke run over `tests/fixtures/state_m1.csv` with `--data-version test:v1`: four artifacts in contract order, 260 M1 bars, 52 M5, 17 M15, 4 H1; 624 events; the H1 chart has 4 buckets and no H1 events. The checked-in golden sizes are 4,413 bytes for `summary.txt`, 16,099 for `report.json`, 12,539 for `price-axes.png`, and 8,225 for `events.png`.
- The date-filter test generates 1,500 M1 rows across two UTC days. The full report publishes 1,500 M1 and 25 H1 bars; the first-day run publishes 1,440 M1 and 24 H1 bars; the second-day run publishes 60 M1, 12 M5, 4 M15, and 1 H1 bar. This proves the filter runs before the ladder build.
- An empty requested range fails before the model run and creates no output directory.
- A blocked artifact write leaves the blocker intact and removes the partial set.

## Learning

- The coverage header exposed a clippy `write_literal`: a constant `"last"` was passed through `{}`. Folding it into the format string kept the rendered bytes identical and removed the lint at the source.
- The directory-listing assertion initially compared `read_dir` output with the contract order. `read_dir` is unordered, so the test now sorts the names before comparing. The stdout assertion still checks the contract order.
- The date-filter test first expected 1,440 bars from a 1,500-row full run. The generated series spans 25 hours, so the full run is 1,500 M1 bars. A separate first-day run now proves the inclusive upper boundary instead of adjusting the expectation by convenience.
- The events chart recomputed the price bounds for every marker. Hoisting the bounds out of the loop keeps the bytes and removes repeated work. The circle marker now writes single pixels through the canvas instead of allocating a one-pixel rectangle per pixel.
- `--json` prints the exact `report.json` bytes rather than re-serializing the value, so the stdout and the file cannot drift.
- The summary's `[artifacts]` section uses relative names. The same run therefore produces the same summary bytes under any output directory, which the determinism test compares across two directories.
- `serde_json` in the shell does not enable `preserve_order`, so object keys serialize alphabetically. That order is fixed and deterministic; the contract's requirement is a fixed order, not insertion order.
- Rejected: computing a report statistic, an event count, or a chart bucket in the shell. The model view owns every number; the shell only formats and draws.
- Rejected: adding a dependency for JSON, PNG, or file writing. The workspace already has `serde_json`, `png`, and `export::write_atomic`.
- Rejected: writing artifacts directly into the target directory as each one is produced. The command builds all bytes first and writes through atomic siblings, so a failure leaves no partial set.
