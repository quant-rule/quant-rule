# Execution Plan: Fetch Dukascopy Candles into Validated CSV and PNG

- Issue: [#55](https://github.com/quant-rule/quant-rule/issues/55) — feat: fetch Dukascopy candles into validated CSV and PNG
- Branch: `feat/55-dukascopy-source`
- Worktree: `/home/zantgo/Proyectos/quant-rule--55-dukascopy-source`
- Status: Verified; amended on 2026-10-23 at the human's request to add native 5m, 15m, and 1h fetching while preserving the 1m parity result. The human accepted the review PNGs on 2026-10-23 and authorized the Git handoff.
- Date: 2026-10-23 (UTC)
- Transparency log: [`docs/ai-transparency-log/2026-10-23/55-dukascopy-source.md`](../../ai-transparency-log/2026-10-23/55-dukascopy-source.md)

## Intent

Rebuild the Python Dukascopy downloader as a Rust source adapter reachable from
the shell. A user can fetch a UTC date range of native one-minute BID candles,
have them validated by the data engine, write the series as CSV, and optionally
render a PNG. The result must be deterministic and must reproduce the existing
XAUUSD 2025 dataset exactly.

Affected users: a person acquiring historical candles for model and backtest
work, and a caller running the same command non-interactively.

## Current state

- `main` is at `7675f6b`; the task worktree is on `feat/55-dukascopy-source`.
  The worktree was clean at preflight and now holds the task changes, none of
  which is committed.
- `crates/quantrule-data` owns candle validation, ordering, and the bounded
  1000-candle window. It is pure.
- `crates/quantrule-shell` owns local CSV source access and NDJSON rendering.
- The Python tool in `~/Proyectos/market-data` uses the third-party
  `dukascopy-python` 4.0.1 package, which calls the undocumented
  `freeserv.dukascopy.com/2.0/index.php` JSON chart endpoint with BID prices.
- The existing `data/XAUUSD_M1_2025-01-01_2025-12-31.csv` holds 354,441 M1
  candles. Preflight fetched the same window through the live endpoint in 12
  pages and reproduced that file byte for byte.

## Scope

Included:

- A new `fetch` shell command:
  `fetch <symbol> <start> <end> [--csv <path>] [--png <path>] [--candles <n>]`.
- A Dukascopy source adapter in the shell: HTTP transport, JSONP parsing,
  ordered pagination, bounded retries with fixed delays, and symbol aliases.
- Native `1m` BID candles only; explicit UTC dates; end date inclusive.
- Streaming validation through the data engine's bounded window; accepted
  candles are written to an atomic temporary CSV and renamed at the end.
- Deterministic CSV matching the existing header, timestamp shape, and number
  formatting.
- An optional deterministic PNG of the last at most 1000 candles: candles,
  volume, axes, and title. No moving averages, because the shell must not
  compute indicators.
- A small pure addition to `quantrule-data`: numeric candle values accepted by
  the existing window, with the same ordering and duplicate rules.
- Fixture-based tests for pagination, overlap removal, retries, CSV bytes,
  PNG determinism, command parsing, and edge cases. No network in tests.
- README, help text, this plan, and the transparency log.

Explicit non-goals:

- Higher timeframes derived from M1 by the model engine; issue #50 builds those.
- Provider intervals outside `1m`, `5m`, `15m`, and `1h` (see the amendment below).
- The full 1,380-instrument catalogue; exact provider symbols and a curated
  alias table are accepted.
- Caching, resumable downloads, tick data, and ask-side prices.
- Moving averages or any other indicator in the shell.
- Live exchange data; Dukascopy remains the only adapter in this slice.
- Commits, pushes, and a pull request. Those need separate authorization.

## Assumptions and uncertainty

- The endpoint remains available and returns the same historical values. The
  preflight proved it for XAUUSD 2025 on 2026-10-23.
- A page holds at most 30,000 rows; a year is 12 requests.
- Volume is Dukascopy tick activity, not exchange-traded volume.
- Invalid rows are skipped and counted; the command fails only when no valid
  candle exists. This is the same tolerant policy as `candles-tolerant`.
- Gaps are market closures and are preserved; no candle is synthesised.
- A PNG with an embedded bitmap font and a pure-Rust encoder is deterministic
  for a pinned dependency set.

## Decisions

- Source access lives in the shell, per `AGENTS.md` section 3. The data engine
  gains only a pure numeric input path.
- Dependencies `ureq` 3.5, `serde_json` 1, and `png` 0.18 are approved. The
  standard library has no HTTP, JSON, or PNG support; no async runtime or
  system font is used.
- Native `1m`, `5m`, `15m`, and `1h` intervals, fetched as published by the
  provider. Timeframes derived from M1 stay in #50.
- CSV is always written; PNG is opt-in.
- The chart has no moving averages.
- The issue is worked in this worktree and branch. Commit, push, and pull
  request remain gated.

## Amendment: native 5m, 15m, and 1h

The human asked on 2026-10-23 for EURUSD from 2026-01-01 through
2026-06-30 in native `1m`, `5m`, `15m`, and `1h`. This extends the command but
does not replace the original XAUUSD M1 byte-parity requirement.

Included:

- `Timeframe::M5`, `Timeframe::M15`, and `Timeframe::H1` alongside `M1`, with
  stable user codes and CSV file codes.
- `--timeframe <1m|5m|15m|1h>` on `fetch`; the default remains `1m`.
- Provider mapping `1MIN`, `5MIN`, `15MIN`, and `1HOUR`. Each interval is
  downloaded natively from the source, never aggregated by QuantRule.
- Timeframe-aware timestamp alignment and chart titles.
- Default CSV names `data/<slug>_<M1|M5|M15|H1>_<start>_<end>.csv`.
- EURUSD 2026-01-01 through 2026-06-30 fetched in all four intervals, with one
  review PNG per interval.

Explicit non-goals:

- Aggregating 1m candles into higher timeframes. That remains issue #50.
- Provider intervals other than the four above.
- Indicators, moving averages, or a combined multi-timeframe view.
- Commits, pushes, or a pull request before the human inspects the PNGs.

Decisions:

- Native intervals are a source-acquisition concern, not derived-timeframe
  construction. Requesting the provider's own interval matches the reference
  downloader and avoids an unapproved aggregation rule.
- The provider interval is stored on `FetchRequest`; alignment uses the
  timeframe length, so a row that does not sit on the requested boundary is
  rejected before it reaches the data engine.
- The default CSV names keep the existing `M1` convention and add `M5`, `M15`,
  and `H1` for the other intervals.

Amendment acceptance criteria:

- All four native intervals are accepted by the parser and mapped to their
  provider interval.
- A timestamp aligned to 5m, 15m, or 1h is accepted; one misaligned to the
  selected interval is rejected.
- The default CSV path names the selected file code.
- The summary and chart title include the selected timeframe.
- The full test suite remains green and the original XAUUSD M1 parity result is
  unchanged.
- EURUSD 2026-01-01 through 2026-06-30 is downloaded in all four intervals with
  one PNG per interval for human review.

Amendment work breakdown:

1. Extend `quantrule-data::Timeframe` with native variants, codes, file codes,
   parsing, and unit tests.
2. Make the Dukascopy request carry the selected timeframe and provider
   interval; align timestamps to the selected length.
3. Wire `--timeframe` through parsing, default paths, summaries, charts, help,
   README, and CLI tests.
4. Run all quality gates and re-run the original parity check.
5. Fetch the four EURUSD ranges, verify shape and boundaries, and generate the
   review PNGs.

## Acceptance criteria

- `fetch gold 2025-01-01 2025-12-31` writes a CSV byte-identical to
  `data/XAUUSD_M1_2025-01-01_2025-12-31.csv`: 354,441 rows, same timestamps,
  same OHLCV values, same first and last candle.
- Two identical runs produce identical CSV, PNG, and stdout bytes.
- A fixture run with an overlapping page boundary produces one row per
  timestamp.
- Appending later candles never changes the output of an earlier window.
- Empty, single-candle, flat, zero-volume, duplicate, out-of-order, and invalid
  rows are covered without panic.
- The command is in `help` and the README, and existing commands are unchanged.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and
  `cargo doc --no-deps` are clean.
- Tests do not use the network.

## Work breakdown

1. `quantrule-data`: add `CandleValues` and `CandleWindow::push_values`, share
   the ordering path with `push`, and test numeric acceptance and rejection.
2. `quantrule-shell::source`: transport trait, `ureq` implementation, JSONP
   parser, pagination state machine, retry policy, symbol aliases, fixtures and
   tests.
3. `quantrule-shell::export`: deterministic CSV writer with atomic rename and
   in-memory tests.
4. `quantrule-shell::chart`: canvas, bitmap font, candle and volume layout, PNG
   encoder, determinism and edge-case tests.
5. `quantrule-shell::fetch`: orchestration, summary, error mapping.
6. `quantrule-shell::command` and `main`: parse the `fetch` command, wire
   interactive and non-interactive paths, update help.
7. Documentation: README command reference, CSV and PNG contracts, provider
   notes, this plan, and the transparency log.
8. Verification: complete checks, live full-year fetch, byte comparison,
   second-run determinism, and PNG inspection.

## Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
cargo run -p quantrule-shell -- fetch gold 2025-01-01 2025-12-31 \
  --csv /tmp/opencode/rust_xauusd_2025.csv \
  --png /tmp/opencode/rust_xauusd_2025.png
cmp /tmp/opencode/rust_xauusd_2025.csv \
  ../quant-rule/data/XAUUSD_M1_2025-01-01_2025-12-31.csv
```

The live comparison is manual evidence, not a test, because tests may not
reach the network. The fixture tests cover the same code paths without I/O.

## Observed evidence

Quality gates run in this worktree on 2026-10-23:

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Exit 0, no diff |
| `cargo clippy --all-targets -- -D warnings` | Exit 0, no warnings |
| `cargo test` | Exit 0: 57 data, 85 shell unit, 36 shell CLI, 2 doctests, 0 failed |
| `cargo doc --no-deps` | Exit 0, no warnings |

The same-path live run was executed twice with the same command and outputs:

```bash
target/debug/quantrule fetch gold 2025-01-01 2025-12-31 \
  --csv /tmp/opencode/parity55/xauusd.csv \
  --png /tmp/opencode/parity55/xauusd.png
```

- Both runs exited 0 and stdout was byte-identical. A repeat with `/usr/bin/time -o` redirected separately wrote zero bytes of program stderr and the same stdout.
- Observed summary: `rows=354441 accepted=354441 rejected=0 pages=12`, first
  `2025-01-01T23:00:00Z`, last `2025-12-31T21:58:00Z`.
- `cmp` against `../quant-rule/data/XAUUSD_M1_2025-01-01_2025-12-31.csv`
  reported identity. Both files have SHA-256
  `cfcc884687c1f98522c6253188524fea56d3709d5068a41e28c6b5acd1005ef3`.
- The CSV has 354,441 data rows plus the header, and a trailing newline.
- The PNG is 1280x720 RGB, 16,100 bytes, with SHA-256
  `dc8e0a2a59670ffb52d94706f287969924593e83b3052237328f860235cb9adf`.
  Repeated runs produced the same hash. The rendered image was inspected and
  showed the price and volume panels, UTC labels, and no moving average.
- Earlier independent runs with different output paths produced the same CSV
  and PNG hashes, so the result does not depend on the output path.

Acceptance criteria status:

| Criterion | Status | Evidence |
|---|---|---|
| Full-year byte parity | Met | Live `cmp` and SHA-256 above |
| Two runs identical | Met | Same-path stdout, CSV, and PNG hashes |
| Overlap page boundary | Met | `drops_the_overlapping_cursor_row_at_a_page_boundary` |
| Later candles do not change earlier output | Met | `appending_later_candles_does_not_change_retained_values` |
| Edge cases without panic | Met | Empty page, single candle, flat, zero volume, duplicate, out-of-order, and invalid-row tests |
| Help and README updated | Met | Help fixture, CLI tests, README contract |
| Formatting, lint, test, and doc gates | Met | Table above |
| No network in tests | Met | Fixture transport; live run is manual evidence |

## Amendment evidence (2026-10-23)

Quality gates re-run in this worktree after the native-timeframe change:

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Exit 0, no diff |
| `cargo clippy --all-targets -- -D warnings` | Exit 0, no warnings |
| `cargo test` | Exit 0: 61 data, 90 shell unit, 37 shell CLI, 2 doctests; 0 failed |
| `cargo doc --no-deps` | Exit 0, no warnings |

The original XAUUSD 2025 M1 parity was re-run after the change:

- Summary: `rows=354441 accepted=354441 rejected=0 pages=12`, first
  `2025-01-01T23:00:00Z`, last `2025-12-31T21:58:00Z`.
- `cmp` against `../quant-rule/data/XAUUSD_M1_2025-01-01_2025-12-31.csv`
  exited 0. Both files have SHA-256
  `cfcc884687c1f98522c6253188524fea56d3709d5068a41e28c6b5acd1005ef3`.
- The PNG was unchanged at 16,100 bytes and SHA-256
  `dc8e0a2a59670ffb52d94706f287969924593e83b3052237328f860235cb9adf`.

EURUSD live fetches, 2026-01-01 through 2026-06-30, BID:

| Timeframe | Rows | Pages | First | Last | CSV bytes | CSV SHA-256 |
|---|---|---|---|---|---|---|
| `1m` | 183,806 | 7 | `2026-01-01T22:04:00Z` | `2026-06-30T23:59:00Z` | 11,642,641 | `2756bf56b975fc345c208696b7e990190c4679217c6b29e9d8abecb77c8fceb4` |
| `5m` | 36,900 | 7 | `2026-01-01T22:00:00Z` | `2026-06-30T23:55:00Z` | 2,371,685 | `8c6aca1672249fecd2241ad4043b46cf789d3e5e6257a4408d99887aa7a2a659` |
| `15m` | 12,300 | 7 | `2026-01-01T22:00:00Z` | `2026-06-30T23:45:00Z` | 798,305 | `d72f95d54499e9fb4a6dfacaee008e7f62ecff5265531d819ab61a44b76c5cbe` |
| `1h` | 3,074 | 1 | `2026-01-01T22:00:00Z` | `2026-06-30T23:00:00Z` | 201,423 | `7eb047b00c4395462fcdd3b82465cbad85d0d84b991687bf521cb04f470ba208` |

Shape checks over the four CSVs:

- None has a duplicate timestamp, and every consecutive delta is a positive
  multiple of its timeframe length.
- Every `5m`, `15m`, and `1h` bar matches the open, high, low, close, and volume
  of the corresponding non-empty `1m` bucket. Differences: 0 for all three
  intervals. The CSVs still come from the provider's native intervals; this is
  a consistency check, not aggregation.
- A second `1m` fetch and a second `1h` fetch to different paths produced
  byte-identical CSV and PNG files. The `1m` and `1h` PNG SHA-256 values were
  `da28d10d82c0b696ac706c37836bb6aab91ac671ebdf759db072fcbbd314c7a1` and
  `82725990572e765cbd1c91ecf1d5cfb28e6157ece584ec6e7e6c83133c7a511e`.

Review PNGs, all 1280x720 RGB:

| Timeframe | Rendered tail | Bytes | SHA-256 |
|---|---|---|---|
| `1m` | `2026-06-30T07:17:00Z` to `23:59:00Z` | 16,079 | `da28d10d82c0b696ac706c37836bb6aab91ac671ebdf759db072fcbbd314c7a1` |
| `5m` | `2026-06-25T12:40:00Z` to `2026-06-30T23:55:00Z` | 16,490 | `787954375c7ceef1149134df1033cac6b3b45ffe800e02496b095b26d2bbc8de` |
| `15m` | `2026-06-16T14:00:00Z` to `2026-06-30T23:45:00Z` | 15,480 | `75ce11bc9efdde6dff3efe3d83ba687fad73b6228ad84a18a3cf0a452a25a61e` |
| `1h` | `2026-05-04T08:00:00Z` to `2026-06-30T23:00:00Z` | 15,739 | `82725990572e765cbd1c91ecf1d5cfb28e6157ece584ec6e7e6c83133c7a511e` |

All four images were rendered by the shell and inspected: price and volume
panels, UTC labels, no moving average. The 1m, 5m, and 15m images show the last
1,000 candles, not the full six months, because the command caps the chart at
1,000 candles. The 1h image shows the last 1,000 hourly bars, covering about two
months. The human accepted all four PNGs on 2026-10-23 and authorized the Git
handoff.

Amendment acceptance criteria status:

| Criterion | Status | Evidence |
|---|---|---|
| Four native intervals parsed and mapped | Met | `parses_each_supported_timeframe`, `requests_each_native_interval` |
| Alignment per interval | Met | `rejects_an_unaligned_timestamp`, `rejects_a_timestamp_unaligned_to_an_hour` |
| Default path names the file code | Met | `default_csv_path_uses_the_timeframe_code` |
| Summary and chart include the timeframe | Met | `summary_has_a_fixed_shape`, `the_timeframe_changes_the_rendered_title` |
| Full suite and original parity green | Met | Gates and re-run above |
| EURUSD four intervals and PNGs | Met | Tables above; accepted by the human on 2026-10-23 |

Limitations and residual risks:

- The endpoint is undocumented and historical values can be revised. The
  adapter reports failures instead of hiding them; the reference CSV remains a
  fallback.
- Only native `1m`, `5m`, `15m`, and `1h` BID candles and the curated symbol
  aliases are in scope. Timeframes derived by the model engine remain #50.
- The PNG is deterministic for the pinned dependency set and this platform; a
  dependency change that alters PNG encoding would require re-verification.
- The tests use fixtures, not the live provider. The byte-parity claim is
  supported by the manual run recorded above, not by `cargo test`.

The plan is `Verified`. The human accepted the review PNGs on 2026-10-23 and
authorized the Git handoff. The actual commit, push, and pull request are
recorded in the transparency log.

## Risks and recovery

- The endpoint is undocumented and can change. The adapter is isolated; the
  existing CSV remains a fallback and the failure is reported, not hidden.
- Historical values can be revised. The live comparison reports any difference
  rather than silently accepting it.
- Large series use disk, not memory: the 7-year XAUUSD file is 166 MB.
- New dependencies increase the build surface. Cargo.lock pins them and the
  diff is reviewed.
- The chart is a data-integrity view, not a trading chart; it deliberately
  omits indicators.

## Stop conditions

- The approved layering or dependency set is rejected.
- The provider becomes unavailable or changes its payload during verification.
- The Rust output differs from the existing CSV in a way that is not a
  formatting defect.
- A required decision falls outside this scope.
