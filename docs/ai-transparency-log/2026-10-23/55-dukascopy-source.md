# AI Transparency Log — Fetch Dukascopy Candles into Validated CSV and PNG

- **Issue:** [#55](https://github.com/quant-rule/quant-rule/issues/55)
- **Date:** 2026-10-23 (UTC)
- **Pull request:** To be opened from `feat/55-dukascopy-source` after human acceptance; this record is updated when it exists.
- **Execution plan:** [`docs/execution-plans/2026-10-23/55-dukascopy-source.md`](../../execution-plans/2026-10-23/55-dukascopy-source.md)

## Entry: planning and preflight

- **Date and tool:** 2026-10-23 (UTC). opencode (DeepSeek V4.1 Flash Free), running in a local shell with `git`, `gh`, and filesystem tools.
- **Intent and boundary:** Rebuild the Python Dukascopy downloader as a Rust source adapter in the shell. The boundary is native 1m BID candles, a deterministic CSV, and an optional deterministic PNG. Higher timeframes, indicators, and live exchange data are out of scope. No commit, push, or pull request is included.
- **Verification and evidence:**
  - The reference implementation in `~/Proyectos/market-data` and its `dukascopy-python` dependency were inspected. Its chart JSON3 endpoint uses BID prices, a browser `User-Agent`, a `Referer`, a fixed callback, and `last_update` pagination.
  - The existing `data/XAUUSD_M1_2025-01-01_2025-12-31.csv` has 354,441 rows, a header, and a trailing newline. Preflight fetched the same window through the live endpoint in 12 pages and reproduced the file byte for byte.
  - Issue #55 was opened, branch `feat/55-dukascopy-source` and worktree `quant-rule--55-dukascopy-source` were created, and the board item was moved to `In Progress` with start date 2026-10-23.
  - The human approved the execution plan on 2026-10-23. This approval did not authorize commits, pushes, or a pull request.
  - No implementation or passing-test result is claimed in this entry; implementation had not been written.
- **Learning:**
  - Source access belongs in `crates/quantrule-shell`, not in the pure data engine.
  - A new `fetch` command is required. Changing an existing command's contract would be a separate decision.
  - The CSV is the primary artifact; the PNG must be opt-in.
  - The provider is undocumented, so its protocol and retry behavior belong behind a small, fixture-tested adapter.

## Entry: implementation

- **Date and tool:** 2026-10-23 (UTC). opencode (DeepSeek V4.1 Flash Free), running in the #55 worktree.
- **Intent and boundary:** Implement the approved plan only. No indicators, higher timeframes, moving averages, clock-derived default dates, or Git operations were added.
- **Implemented:**
  - `crates/quantrule-data/src/window.rs`: added `CandleValues` and `CandleWindow::push_values`, reusing the existing validation, duplicate, and ordering path.
  - `crates/quantrule-shell/src/source/`: date range parsing, symbol aliases, `ureq` transport, JSONP parsing, Dukascopy pagination, overlap removal, and fixed retry policy.
  - `crates/quantrule-shell/src/export.rs`: exact CSV header, UTC timestamp shape, shortest-round-trip numbers, atomic temporary-file writes, and cleanup on failure.
  - `crates/quantrule-shell/src/chart/`: deterministic RGB canvas, embedded 8x8 bitmap font, price and volume panels, PNG encoding, and no moving average.
  - `crates/quantrule-shell/src/fetch.rs`: validation through the bounded window, streaming CSV, optional tail chart, summary, and failure handling.
  - `crates/quantrule-shell/src/command.rs`, `src/main.rs`, `tests/cli.rs`, and `tests/fixtures/`: `fetch` parsing, defaults, help, interactive and non-interactive dispatch, and argument-error tests.
  - `README.md`: command table, options, fetch contract, summary shape, and limitations.
  - `docs/execution-plans/2026-10-23/55-dukascopy-source.md`: approved plan and observed evidence.
  - The embedded font notice records the upstream MIT license; no font dependency was added.
- **Verification and evidence:**
  - Implementation was checked incrementally with `cargo fmt`, `cargo clippy`, `cargo test`, and `cargo doc`; the final observed results are in the next entry.
  - No test reaches the network. Pagination, retries, JSONP parsing, CSV shape, chart determinism, and argument errors use scripted transports and checked-in fixtures.
- **Learning:**
  - **Corrected:** fixture stop rows initially used `86,400,000` ms for a two-day inclusive range. `DateRange::parse("1970-01-01", "1970-01-02")` ends at `172,800,000` ms, so the fixtures were changed to the actual exclusive end.
  - **Corrected:** the first page may contain only the candle at the start timestamp. Rejecting cursor equality there truncated legitimate data; equality is now allowed on the first page and later pages still drop the repeated cursor row.
  - **Corrected:** CSV cleanup tests checked `path.with_extension("tmp")`, which is not the writer's `path.tmp` temporary path. The tests now check the real path and cover drop and source-failure cleanup.
  - **Corrected:** a PNG target equal to the default CSV path could overwrite the CSV after a successful fetch. `run_fetch` now rejects that conflict before creating either output; the parser rejects an explicit duplicate and a unit test covers the default-path case.
  - **Rejected:** adding the `font8x8` crate. The approved dependency set did not include it, so a small embedded table with the upstream MIT notice avoids a new dependency.
  - **Rejected:** moving averages or any indicator in the chart. The shell must not compute indicators.
  - **Rejected:** defaulting dates from the wall clock. The range is always supplied explicitly in UTC.
  - **Rejected:** synthesizing gaps or filling zero volume. Gaps are preserved and a real zero stays `Some(0.0)`.

## Entry: native 5m, 15m, and 1h extension

- **Date and tool:** 2026-10-23 (UTC). opencode (DeepSeek V4.1 Flash Free), running in the #55 worktree.
- **Intent and boundary:** Add native `5m`, `15m`, and `1h` fetching so EURUSD 2026-01-01 through 2026-06-30 can be downloaded in all four intervals for review. The original XAUUSD M1 parity requirement remains. No aggregation, indicators, moving averages, commit, push, or pull request was added.
- **Implemented:**
  - `crates/quantrule-data/src/timeframe.rs`: `M5`, `M15`, and `H1` variants, stable user codes, CSV file codes, case-insensitive parsing, and unit tests.
  - `crates/quantrule-shell/src/source/dukascopy.rs`: the request carries the timeframe and provider interval; timestamps are aligned to the selected length; provider mapping is `1MIN`, `5MIN`, `15MIN`, and `1HOUR`.
  - `crates/quantrule-shell/src/command.rs`, `src/fetch.rs`, and `src/chart/mod.rs`: `--timeframe` parsing, timeframe-aware default CSV paths, summaries, and chart titles.
  - `README.md`, the help text and fixture, and CLI coverage for valid, invalid, duplicate, and missing timeframe arguments.
- **Verification and evidence:**
  - Final gates: `cargo fmt --all -- --check` exit 0; `cargo clippy --all-targets -- -D warnings` exit 0; `cargo test` exit 0 with 61 data, 90 shell unit, 37 shell CLI, and 2 doctests passed; `cargo doc --no-deps` exit 0 with no warnings.
  - The original XAUUSD 2025 M1 live fetch still produced 354,441 rows in 12 pages and was byte-identical to the reference with SHA-256 `cfcc884687c1f98522c6253188524fea56d3709d5068a41e28c6b5acd1005ef3`.
  - Live EURUSD fetches produced 183,806 `1m`, 36,900 `5m`, 12,300 `15m`, and 3,074 `1h` rows. The four CSVs had no duplicate timestamps and all deltas were positive multiples of the selected interval.
  - Every native `5m`, `15m`, and `1h` bar matched the corresponding non-empty `1m` bucket on OHLC and volume, with zero differences. This checks consistency; the CSVs were still fetched from provider-native intervals.
  - A second `1m` fetch and a second `1h` fetch to different paths produced byte-identical CSV and PNG files.
  - The four review PNGs were rendered and inspected locally. The `1m`, `5m`, and `15m` charts each show the last 1,000 candles rather than the full six months; the `1h` chart covers about the last two months. The human accepted all four images on 2026-10-23.
- **Learning:**
  - **Corrected:** the help fixture was stale after the help text changed, causing four CLI failures. The fixture now matches the command reference exactly.
  - **Corrected:** the first duplicate-detection draft used the default `M1` as a sentinel and so could not detect `--timeframe 1m --timeframe 5m`. The parser now stores an `Option<Timeframe>` and rejects every repeated option.
  - **Corrected:** a test sink closure needed an explicit `Result<(), ()>` type because the error type was not inferable.
  - **Observed:** the provider's intraday pages cover roughly one month each for `1m`, `5m`, and `15m`, while `1h` returned the whole requested history in one page. The adapter's cursor pagination still covered the full range.
  - **Rejected:** aggregating 1m candles into the higher intervals. The reference downloader requests provider-native intervals, and derived timeframes remain #50.
  - **Rejected:** raising the chart cap or adding moving averages to show the full range. The 1,000-candle limit is the existing command contract; the review images state their rendered tail.
  - **Accepted:** committing, pushing, and opening a pull request after the human inspected and accepted the four PNGs on 2026-10-23.
  - This was a sequential implementation and verification pass by one agent. No independent reviewer or separate model judgment was used.

## Entry: original 1m verification

- **Date and tool:** 2026-10-23 (UTC). opencode (DeepSeek V4.1 Flash Free), running in the #55 worktree.
- **Intent and boundary:** Verify the original approved 1m acceptance criteria and record actual evidence. This entry does not claim an independent review or a Git handoff.
- **Relationship to later work:** This entry records the state before the native-timeframe extension. The extension entry above records the current four-interval gates and evidence.
- **Verification and evidence (observed in this worktree):**
  - `cargo fmt --all -- --check` exited 0 with no diff.
  - `cargo clippy --all-targets -- -D warnings` exited 0 with no warnings.
  - `cargo test` exited 0: 57 `quantrule-data` tests, 85 shell unit tests, 36 shell CLI tests, and 2 doctests passed; 0 failed.
  - `cargo doc --no-deps` exited 0 with no warnings.
  - The same command was run twice with the same output paths:
    `target/debug/quantrule fetch gold 2025-01-01 2025-12-31 --csv /tmp/opencode/parity55/xauusd.csv --png /tmp/opencode/parity55/xauusd.png`.
    Both runs exited 0 and printed byte-identical stdout. A repeat with `/usr/bin/time -o` redirected separately wrote zero bytes of program stderr and the same stdout. The observed summary was `rows=354441 accepted=354441 rejected=0 pages=12`, first `2025-01-01T23:00:00Z`, last `2025-12-31T21:58:00Z`.
  - `cmp` reported the generated CSV identical to `../quant-rule/data/XAUUSD_M1_2025-01-01_2025-12-31.csv`. Both SHA-256 values are `cfcc884687c1f98522c6253188524fea56d3709d5068a41e28c6b5acd1005ef3`. The generated file has 354,441 data rows plus the header and a trailing newline.
  - The PNG is 1280x720 RGB, 16,100 bytes, SHA-256 `dc8e0a2a59670ffb52d94706f287969924593e83b3052237328f860235cb9adf`. Independent runs and the same-path runs produced the same hash. The image was inspected and shows price and volume panels, UTC labels, and no moving average.
  - The live runs are manual evidence, not part of `cargo test`; the test suite does not use the network.
- **Unresolved items and limitations:**
  - The endpoint is undocumented and historical values can be revised. The adapter reports failures and does not hide them.
  - At this point only native 1m BID candles and curated symbol aliases were supported. The later extension adds 5m, 15m, and 1h; timeframes derived by the model engine remain #50.
  - PNG byte determinism is verified for the pinned dependency set and this platform. A dependency or encoder change requires re-verification.
  - This was a sequential implementation and verification pass by one agent. No independent code reviewer, test engineer, or separate model judgment was used.
  - At that point the worktree held uncommitted changes and no commit, push, or pull request existed, so issue #55 remained `In Progress` on the board.
- **Learning:**
  - The full-year parity check is the strongest evidence for the provider mapping and CSV formatting; the fixture tests alone would not prove the live values.
  - The provider repeats the cursor row at page boundaries, so the duplicate row must be filtered before it reaches the data engine.
  - A deterministic PNG requires a fixed font, no clock metadata, no unordered iteration, and a pinned encoder.
