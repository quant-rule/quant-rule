# AI Transparency Log — Print the Candle Window as NDJSON from the Shell

- **Issue:** [#48](https://github.com/quant-rule/quant-rule/issues/48)
- **Date:** 2026-10-23 (UTC)
- **Execution plan:** [`docs/execution-plans/2026-10-23/48-shell-ndjson.md`](../../execution-plans/2026-10-23/48-shell-ndjson.md)

## Entry: planning and preflight

- **Date and tool:** 2026-10-23 (UTC). opencode (DeepSeek V4.1 Flash Free), running in a local shell with `git`, `gh`, and filesystem tools.
- **Intent and boundary:** Start #48 after #47 merged. The boundary is the first end-to-end shell slice: CSV source access, `candles`/`help`/`quit`, and NDJSON. No indicators, higher timeframes, or model behavior. No implementation had started when this entry was written.
- **Verification and evidence:**
  - `main` was at `fac3064`, the project root was clean, and no other worktrees were present.
  - The shell crate contained only `Cargo.toml` and a stub `src/main.rs`.
  - The real source header was observed as `Date,Open,High,Low,Close,Volume`; rows use `YYYY-MM-DD HH:MM:SS+00:00`.
  - Issue #48 was moved to `In Progress` with start date 2026-10-23.
  - The worktree `quant-rule--48-shell-ndjson` was created on `feat/48-shell-ndjson` from `origin/main`.
  - No build, test, lint, or doc result is claimed in this entry; the implementation had not been written.
- **Learning:**
  - The shell must build the window before printing. Rendering as rows are read would leave partial NDJSON when a later row fails.
  - The prompt must go to stderr so stdout is only the command result.
  - The source header and timestamp shape are exact; tolerant ingestion belongs to #49, not this slice.
  - `window_index` can be derived from the first retained `series_index`, so the data engine does not need a presentation-only field.

## Entry: implementation and verification

- **Date and tool:** 2026-10-23 (UTC). opencode (DeepSeek V4.1 Flash Free), running in the PR3 worktree.
- **Intent and boundary:** Implement the first end-to-end shell slice and verify it. No indicators, higher timeframes, tolerant ingestion, or model behavior were added.
- **Implemented:**
  - `crates/quantrule-shell/src/csv.rs`: exact-header, streaming CSV source loading; line-numbered errors; one row at a time into `CandleWindow`.
  - `crates/quantrule-shell/src/command.rs`: `candles`, `help`, and `quit` parsing; help text; source and output errors.
  - `crates/quantrule-shell/src/render.rs`: oldest-first NDJSON rendering with `series_index`, `window_index`, RFC 3339 `datetime`, OHLC, and nullable volume.
  - `crates/quantrule-shell/src/main.rs`: interactive prompt on stderr and non-interactive `quantrule candles <csv-path>`.
  - `README.md`: command reference, CSV contract, NDJSON contract, error behavior, and scope.
  - Checked-in fixtures and 20 binary-level integration tests in `crates/quantrule-shell/tests/`.
- **Verification and evidence (observed in this worktree):**
  - `cargo fmt --all -- --check` exited 0.
  - `cargo clippy --all-targets -- -D warnings` exited 0 with no warnings.
  - `cargo test` exited 0: 50 data unit tests, 7 shell parser tests, 20 shell integration tests, and 2 doctests passed.
  - `cargo doc --no-deps` exited 0.
  - A manual run against `data/BTCUSD_M1_2018-01-01_2024-12-31.csv` produced 1000 lines from 3,393,175 source rows. The first retained record was `series_index` 3,392,175 / `window_index` 0; the last was 3,393,174 / 999 and matched the source's last row.
  - A Python JSON check confirmed the documented field set, contiguous window indices, and the 1000-record series span.
  - The tests read only checked-in fixtures; the manual real-file check is not a test and does not use `data/` from test code.
- **Learning:**
  - Clippy rejected the closure form of the line-ending trim with `manual_pattern_char_comparison`; the array pattern `['\r', '\n']` is the narrower form and passes.
  - **Rejected:** rendering while reading. A late malformed row would leave partial NDJSON on stdout; building the window first makes the command all-or-nothing for source errors.
  - **Rejected:** exposing raw Unix seconds in duplicate/out-of-order errors. The shell maps those engine numbers to RFC 3339 timestamps for the person reading stderr.
  - **Rejected:** adding a CSV dependency. The fixed six-column source is small enough for a std-only parser, and no dependency was approved.
  - No commit, push, or pull request was created before this entry. The human authorized the commit, push, and pull request on 2026-10-23; this verified tree is what they carry.
