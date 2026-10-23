# Execution Plan: Print the Candle Window as NDJSON from the Shell

- Issue: [#48](https://github.com/quant-rule/quant-rule/issues/48) — feat: print the candle window as NDJSON from the shell
- Branch: `feat/48-shell-ndjson`
- Worktree: `/home/zantgo/Proyectos/quant-rule--48-shell-ndjson`
- Status: Verified — acceptance criteria evaluated against observed evidence; Git handoff authorized on 2026-10-23
- Date: 2026-10-23 (UTC)
- Transparency log: [`docs/ai-transparency-log/2026-10-23/48-shell-ndjson.md`](../../ai-transparency-log/2026-10-23/48-shell-ndjson.md)

## Intent

Make the bounded window reachable from the product's only surface. The shell reads a local CSV one row at a time, hands each row to the data engine, and prints the retained window as NDJSON. This is the first end-to-end QuantRule slice: source input, data engine, and deterministic text output.

Affected users: a person running the shell interactively and a caller running the same command non-interactively.

## Current state

- `main` is at `fac3064`, after PR #52 merged the bounded window.
- `crates/quantrule-shell/src/main.rs` is a stub that prints an unimplemented message.
- `quantrule-data` exposes `CandleWindow`, `Candle`, `RawCandle`, UTC timestamp formatting, and the strict window errors.
- The real source header is `Date,Open,High,Low,Close,Volume`; timestamps use `YYYY-MM-DD HH:MM:SS+00:00`.
- There are no shell tests, fixtures, or command documentation yet.

## Scope

Included:

- Shell-side CSV source access with the exact named header, streamed rows, and one row at a time.
- `candles <csv-path>`, `help`, and `quit` commands.
- Interactive prompt on stderr.
- Non-interactive invocation `quantrule candles <csv-path>`.
- NDJSON output, oldest first, with `series_index`, `window_index`, RFC 3339 `datetime`, open, high, low, close, and volume (`null` when absent).
- Strict errors for missing files, bad headers, malformed rows, invalid candles, duplicate timestamps, and out-of-order timestamps.
- Checked-in fixtures and integration tests that run the binary and assert stdout, stderr, and exit status.
- README command reference and help text.

Explicit non-goals:

- No `first`, `last`, `limit`, or `search` commands.
- No indicators, model axes, or events.
- No timeframe construction, aggregation, or gaps.
- No tolerant ingestion or quality counts. Those are #49.
- No higher timeframes. Those are #50.
- No network, database, GUI, service, or library surface.
- No third-party dependency.

## Assumptions and uncertainty

- The CSV header is exact and ordered: `Date,Open,High,Low,Close,Volume`.
- A header-only file is a valid empty series and prints nothing.
- A zero-byte file is a missing-header error.
- A malformed row anywhere aborts the command. The shell builds the window completely before printing, so a late error produces no partial stdout.
- The window is fixed at 1000 candles; the shell prints only what the window retained.
- `window_index` is presentation state: it is `series_index - first_retained_series_index`, so the oldest printed candle is always index 0.
- Numeric values are rendered with Rust's `f64` `Display`, which is deterministic for the same value on a conforming platform.
- An empty volume field means absent (`null`); a zero volume field means a real zero.
- Non-interactive mode executes exactly one command from the process arguments. Interactive mode reads one command per line.
- The prompt belongs on stderr so stdout stays machine-readable.

## Decisions

| # | Decision | Choice | Rationale |
|---|---|---|---|
| S1 | Source ownership | CSV reading lives in `quantrule-shell` | The shell owns source access; the data engine stays pure |
| S2 | CSV header | Exact `Date,Open,High,Low,Close,Volume` | The source contract is fixed and a mismatch must fail loudly |
| S3 | Row shape | Exactly six fields | Missing or extra fields are malformed input |
| S4 | Ingestion order | Parse the whole file into the window before rendering | A late malformed row must not leave partial NDJSON on stdout |
| S5 | Window position | Derive `window_index` from the first retained `series_index` | Stable series identity stays on the candle; position is output-only |
| S6 | NDJSON order | `series_index`, `window_index`, `datetime`, `open`, `high`, `low`, `close`, `volume` | One stable schema for every line |
| S7 | Volume | `null` for absent, numeric zero for a real zero | A missing measurement is not a zero |
| S8 | Interactive prompt | `quantrule> ` on stderr | stdout remains the command result |
| S9 | Non-interactive form | `quantrule candles <csv-path>` | The same command contract is reachable without a TTY |
| S10 | Errors | stderr; exit 1 non-interactively; continue interactively | Machine callers get a status; interactive users keep the session |
| S11 | Tests | Checked-in fixtures, binary-level assertions | Tests do not read `data/` and prove observable behavior |

## Acceptance criteria

- `quantrule candles <real-m1-file>` prints the last up-to-1000 candles as NDJSON.
- Every line is valid JSON with the documented fields and RFC 3339 UTC datetime.
- The oldest retained candle has `window_index` 0; indices increase by one per retained candle.
- `volume` is `null` when absent and `0` for a real zero.
- Missing file, bad header, malformed row, invalid candle, duplicate timestamp, and out-of-order timestamp fail with a non-zero exit and no stdout.
- A header-only file succeeds with no output.
- Interactive and non-interactive runs of the same command produce byte-identical stdout.
- Repeated runs produce byte-identical stdout.
- Tests use checked-in fixtures only and never read `data/`.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `cargo doc --no-deps` are clean.
- No third-party dependency is added.

## Work breakdown

| Step | Action | Role | Paths |
|---|---|---|---|
| 1 | Write this plan and log | Orchestrator | `docs/` |
| 2 | Add CSV source loading and errors | Implementation | `crates/quantrule-shell/src/csv.rs` |
| 3 | Add command parsing and help text | Implementation | `crates/quantrule-shell/src/command.rs` |
| 4 | Add NDJSON rendering | Implementation | `crates/quantrule-shell/src/render.rs` |
| 5 | Wire interactive and non-interactive entrypoints | Implementation | `crates/quantrule-shell/src/main.rs` |
| 6 | Add checked-in fixtures and binary-level tests | Test Engineer | `crates/quantrule-shell/tests/` |
| 7 | Update README command reference | Implementation | `README.md` |
| 8 | Run quality gates and record evidence | Test Engineer | `crates/` |
| 9 | Review the complete diff | Code Reviewer | all |
| 10 | Commit, push, open PR after authorization | Orchestrator | git state, PR |

## Verification

Commands to run from the worktree with `~/.cargo/bin` on `PATH`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

Expected evidence: every command exits zero; tests cover exact stdout for empty, single, multi-row, volume-null/zero, and over-window fixtures; error cases; interactive/non-interactive identity; repeated-run identity; and help/quit behavior. No result is claimed until it is observed.

Observed evidence on 2026-10-23:

- `cargo fmt --all -- --check` exited 0.
- `cargo clippy --all-targets -- -D warnings` checked all three crates and exited 0 with no warnings.
- `cargo test` exited 0: 50 `quantrule-data` unit tests, 7 shell parser unit tests, 20 shell integration tests, and 2 doctests passed.
- `cargo doc --no-deps` generated documentation for all three crates and exited 0.
- A manual run against the real M1 file `data/BTCUSD_M1_2018-01-01_2024-12-31.csv` produced exactly 1000 NDJSON lines. The source had 3,393,176 lines including its header. The first output was `series_index` 3,392,175, `window_index` 0, `datetime` `2024-12-30T22:19:00Z`; the last was `series_index` 3,393,174, `window_index` 999, `datetime` `2024-12-31T14:58:00Z`. The last output matched the source's last row.
- A JSON schema check over the real-file output confirmed 1000 records, exactly the documented field set, contiguous `window_index` 0..999, and a 1000-record `series_index` span.
- The shell integration tests use only `crates/quantrule-shell/tests/fixtures/`; no test reads `data/`.
- `Cargo.lock` is unchanged and lists only the three workspace packages.

The acceptance criteria are met. Commit, push, and pull request remain subject to the human authorization gate.

## Risks and recovery

- **A late malformed row could leave partial stdout.** Mitigation: build the window first, then render.
- **A large source file could be buffered.** Mitigation: `BufReader::read_line` and the fixed 1000-candle window; no full-file collection.
- **The output schema could drift.** Mitigation: exact expected NDJSON fixtures and README/help text in the same change.
- **Path or source errors could be non-deterministic.** Mitigation: errors are built from the command's path and line number; no clock, environment, or absolute repository path is added by the code.
- **The shell could acquire engine logic.** Mitigation: CSV parsing and rendering only; validation and indexing stay in `quantrule-data`.
- **Recovery.** The change is additive to the shell crate and README; it can be reverted with a new commit on `main`. The worktree is preserved until the PR is accepted.

## Stop conditions

Stop and request human direction if:

- The command names or NDJSON field names need to differ from the issue.
- The source format requires tolerance or a header other than the exact one.
- The shell needs to compute indicators or model state.
- A dependency is needed.
- Verification cannot be completed with the installed toolchain.
