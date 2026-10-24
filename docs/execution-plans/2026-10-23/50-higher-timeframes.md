# Execution Plan: Derive M5, M15, and H1 from M1

- Issue: [#50](https://github.com/quant-rule/quant-rule/issues/50) — feat: build higher timeframes and detect gaps
- Branch: `feat/50-higher-timeframes`
- Worktree: `/home/zantgo/Proyectos/quant-rule--50-higher-timeframes`
- Status: Verified on 2026-10-23. All acceptance criteria were checked against observed local evidence. Commits, pushes, and a pull request remain separate human-authorized steps.
- Date: 2026-10-23 (UTC)
- Transparency log: [`docs/ai-transparency-log/2026-10-23/50-higher-timeframes.md`](../../ai-transparency-log/2026-10-23/50-higher-timeframes.md)

## Intent

Build M5, M15, and H1 as first-class candle series from the M1 series, inside
the data engine, and expose the result through the shell. The analysis path must
use derived timeframes, not provider-native higher intervals. A bucket that is
missing source candles is dropped and counted; a gap between published candles
is recorded. The same input and configuration must produce the same output.

Affected users: a person inspecting a multi-timeframe candle series from the
shell, and a later model or backtest stage that consumes the derived series.

## Current state

- `main` is at `883f63f`; the task worktree starts from that commit.
- `crates/quantrule-data` owns validation, ordering, and the bounded
  1000-candle window. It has no timeframe construction yet.
- `crates/quantrule-shell` owns CSV source access, command parsing, and NDJSON
  rendering. It can print one strict or tolerant candle window.
- `fetch` can download native `1m`, `5m`, `15m`, and `1h` candles. Those higher
  intervals are source references; they are not the canonical analysis input.
- Issue #50 originally included H4 and D1. The human asked on 2026-10-23 to
  remove them; this plan covers M5, M15, and H1 only.

## Scope

Included:

- M1-derived M5, M15, and H1 in `quantrule-data`, using the existing `Candle`
  shape and bounded window.
- Bucket rules: a bucket is complete only when all expected M1 candles are
  present; otherwise it is dropped and recorded as partial.
- Per-series gap records with `after`, `before`, and `missing`.
- Volume aggregation: sum when every source volume is known, otherwise `None`;
  a real zero volume remains zero.
- A `timeframes <csv-path>` shell command: strict M1 CSV input, NDJSON on
  stdout, aggregation report on stderr.
- Deterministic rendering, interactive/non-interactive parity, help text, and
  README documentation.
- Unit, edge-case, causality, determinism, retention, and end-to-end shell
  tests.

Explicit non-goals:

- H4 and D1.
- Rewriting provider-native higher-timeframe fetching or using it as canonical
  model input.
- Committing derived higher-timeframe CSV datasets.
- Indicators, the four-axis model, strategies, and backtests.
- A market-session calendar or synthetic gap filling.
- New dependencies.
- Commits, pushes, and pull requests before separate human authorization.

## Assumptions and uncertainty

- M1 timestamps are UTC and aligned to whole minutes. A misaligned row is an
  error, not a value to round.
- A complete M5/M15/H1 bucket has exactly 5/15/60 M1 candles. A duplicate,
  out-of-order, or missing minute makes the bucket incomplete.
- The M1 source may contain gaps; no calendar is inferred from the gaps.
- Volume is unknown as a whole when any source volume is absent. A zero source
  volume is known data.
- The shell's strict CSV reader is sufficient; tolerant ingestion is a separate
  command and is not extended by this task.
- The bounded window is 1000 candles for every timeframe.

## Decisions

- Derive higher timeframes in the data engine, where candle normalization and
  ordering belong. The shell only supplies M1 rows and renders the result.
- Publish a bucket on transition to the next bucket and at `finish`. This makes
  a bucket's publication depend only on bars up to and including its last
  expected minute.
- Drop incomplete buckets and record `(bucket_start, present, expected)`.
- Record gaps between consecutive published candles of each series. Do not
  synthesize a missing candle.
- Keep the four series in a fixed M1, M5, M15, H1 order.
- Add one new command rather than changing `candles` or `candles-tolerant`.
- No new crate dependency; the implementation uses the standard library and the
  existing workspace.

The human approved this direction on 2026-10-23 and explicitly removed H4 and
D1 from the issue.

## Acceptance criteria

- Feeding M1 candles produces M1, M5, M15, and H1 series with the same candle
  shape; each retains at most 1000 candles.
- A complete bucket has first open, maximum high, minimum low, last close, and
  the sum of known volumes.
- A partial bucket is absent from the candle series and appears in the partial
  records with the correct present and expected counts.
- Gaps are recorded per series with the timestamps on both sides and the number
  of missing candles.
- The same input produces identical series, gaps, partial records, stdout, and
  stderr across repeated runs and interactive/non-interactive invocation.
- Appending later M1 candles does not change an already published higher
  timeframe candle.
- `timeframes <csv-path>` prints only NDJSON on stdout and the report on stderr;
  a source error produces no partial stdout.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` pass.

## Work breakdown

1. Data engine: add internal bucket accumulation, `TimeframeSet`,
   `TimeframeSeries`, `Gap`, `PartialBucket`, and `TimeframeError`; export them
   from `lib.rs`.
2. Data engine tests: complete and partial buckets, volume rules, M1 alignment,
   gaps, all four timeframes, retention, causality, determinism, and empty and
   single-candle inputs.
3. Shell source: add `load_timeframes`, which streams strict M1 rows into the
   set and finishes it.
4. Shell rendering: add timeframe NDJSON and the aggregation report.
5. Shell command: parse and dispatch `timeframes`, update help, and add parse
   and CLI tests.
6. Documentation: README command reference, this plan, and the transparency log.
7. Verification: format, lint, tests, docs, a focused real-data smoke test, and
   a complete diff review.

## Verification

Expected evidence:

- `cargo fmt --all -- --check` exits 0 with no diff.
- `cargo clippy --all-targets -- -D warnings` exits 0 with no warnings.
- `cargo test` exits 0; record the observed test counts.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` exits 0 with no warnings.
- Shell CLI fixtures prove exact stdout and stderr for complete aggregation,
  partial buckets, gaps, and empty input.
- Repeated runs and interactive/non-interactive runs are byte-identical.
- A manual `timeframes` run over the available EURUSD M1 file completes and
  reports the expected series and gaps. This is evidence, not a checked-in
  dataset.

Limitations:

- Tests do not use the network or the wall clock.
- The manual real-data smoke test is platform- and data-dependent; it is not a
  substitute for the deterministic fixtures.
- Verification is a sequential pass by one agent unless a separate reviewer is
  explicitly used.

## Risks and recovery

- A bucket-boundary mistake could leak a future bar or publish a short bucket.
  Tests cover transition, finish, partial, and causality paths.
- Volume overflow or invalid aggregated values could reject a higher candle.
  The engine returns an error rather than panicking; no silent zero is inserted.
- Large gap and partial lists are retained for reporting. They are not bounded
  by the candle window; this is a reporting cost, not a candle-retention
  violation.
- A misaligned M1 row aborts the command. The shell builds the set before
  writing, so no partial NDJSON is emitted.
- Rollback is a source revert; no schema, migration, or persisted dataset is
  involved.

## Stop conditions

Stop and ask the human if:

- H4 or D1 becomes required.
- Provider-native higher intervals must become canonical input.
- A new dependency or a command-contract change is needed.
- The implementation requires a schema, persistence, or concurrency decision
  outside this plan.
- A gate fails for a reason that cannot be corrected within this scope.

## Verification evidence

Observed in the task worktree on 2026-10-23:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Exit 0, no diff |
| `cargo clippy --all-targets -- -D warnings` | Exit 0, no warnings |
| `cargo test --quiet` | 232 passed, 0 failed: 90 data, 93 shell unit, 47 shell CLI, 2 doctests |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Exit 0, no warnings |
| CLI fixtures | Exact stdout and stderr for complete M1/M5/M15/H1, partial buckets, gaps, empty input, invalid candle, duplicate timestamp, and misaligned M1 |
| Determinism | Repeated CLI runs and interactive/non-interactive stdout are byte-identical |
| Causality | Appending later M1 candles leaves earlier M5 output unchanged |

Manual real-data smoke test:

- Input: `../quant-rule/data/EURUSD_M1_2026-01-01_2026-06-30.csv`, 183,806 M1
  rows.
- Command: `target/debug/quantrule timeframes <input>`, stdout and stderr
  redirected to `/tmp/opencode/timeframes50/`.
- stdout: 4,000 NDJSON lines, 1,000 retained candles in each of M1, M5, M15,
  and H1, in that order.
- stderr: 2,713 report lines. Summary:
  `1m retained=1000 partial=0 gaps=560`,
  `5m retained=1000 partial=526 gaps=439`,
  `15m retained=1000 partial=429 gaps=311`,
  `1h retained=1000 partial=275 gaps=169`.
- Two runs produced byte-identical stdout and stderr (`cmp` exit 0).
- An independent Python calculation grouped the source by each timeframe,
  selected complete buckets, retained the last 1,000, and compared all 4,000
  candles, series indices, window indices, OHLC values, volumes, and the
  reported partial/gap counts. It matched.
- The source has real missing minutes, including 59-minute hours near the end;
  the dropped partial H1 buckets are therefore expected, not an aggregation
  defect.

Limitations:

- The real-data smoke test is manual evidence; `cargo test` uses checked-in
  fixtures and no network.
- The shell fixtures were generated by an independent Python implementation of
  the documented rules, then compared by the Rust CLI tests.
- Verification is a sequential pass by one agent. No independent code reviewer,
  test engineer, or separate model judgment was used.
- No commit, push, or pull request exists yet; issue #50 remains `In Progress`
  on the board until those are authorized and the pull request is open.
