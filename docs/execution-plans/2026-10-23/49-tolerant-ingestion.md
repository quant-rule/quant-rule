# Execution Plan: Add Candle Quality Reporting and Tolerant Ingestion

- Issue: [#49](https://github.com/quant-rule/quant-rule/issues/49) — feat: add candle quality reporting and tolerant ingestion
- Branch: `feat/49-tolerant-ingestion`
- Worktree: `/home/zantgo/Proyectos/quant-rule--49-tolerant-ingestion`
- Status: Verified — acceptance criteria met; PR #54 open, merge pending human authorization
- Date: 2026-10-23 (UTC)
- Pull request: [#54](https://github.com/quant-rule/quant-rule/pull/54)
- Transparency log: [`docs/ai-transparency-log/2026-10-23/49-tolerant-ingestion.md`](../../ai-transparency-log/2026-10-23/49-tolerant-ingestion.md)

## Intent

Make source quality visible without weakening the strict default. A caller can
choose an explicit tolerant command that skips row-level failures, retains the
valid candles, and prints a deterministic report of what was read, accepted,
rejected, duplicated, and evicted.

Affected users: a person running the shell interactively and a caller running
the same command non-interactively over a noisy source file.

## Current state at preflight

- `main` is at `b2876fd`, after PR #53 merged the shell NDJSON slice.
- `crates/quantrule-shell/src/csv.rs` streams a fixed six-column CSV into
  `CandleWindow`; the first row-level error aborts the command.
- `candles <csv-path>` prints the retained window as NDJSON and keeps stderr
  empty on success.
- `quantrule-data` already rejects invalid candles, duplicate timestamps, and
  out-of-order timestamps, and preserves a real zero volume as `Some(0.0)`.
- The bounded window retains 1000 candles and evicts the oldest accepted candle.
- #49 has no implementation, tests, fixtures, or documentation yet.

## Scope

Included:

- A new `candles-tolerant <csv-path>` command.
- Strict `candles <csv-path>` behavior and output remain unchanged.
- Tolerant ingestion skips row-level failures: wrong field count, invalid
  candle, non-finite value, bad timestamp, duplicate timestamp, and out-of-order
  timestamp.
- NDJSON on stdout, identical in schema and order to `candles`, for the retained
  accepted candles.
- A deterministic quality report on stderr after the NDJSON.
- Counts for rows read, accepted, rejected, duplicate, out-of-order, invalid,
  evicted, and retained.
- One `skipped line=<n> reason=<reason>` line per skipped row, in source order.
- Preserve a real zero volume as `0`; absent volume remains `null`.
- Checked-in fixtures and binary-level tests, including exact report bytes.
- README and help text updates.

Explicit non-goals:

- No change to the default `candles` command's parsing, errors, stdout, or
  stderr.
- No `--tolerant` flag on `candles`; tolerant behavior is a separate command.
- No tolerant header handling. A missing or wrong header is a file-level error.
- No tolerance for open/read failures.
- No indicators, model axes, events, higher timeframes, or gaps. Those remain
  in later issues.
- No network, database, GUI, service, or library surface.
- No third-party dependency.

## Assumptions and uncertainty

- The source header remains exact and ordered:
  `Date,Open,High,Low,Close,Volume`.
- `rows_read` counts data rows after the header, including rejected rows.
- A header-only file is a valid empty series: zero counts and an empty report
  body.
- A file with only invalid rows succeeds in tolerant mode, prints no NDJSON, and
  reports the rejects. This is the "never fails the whole file for one bad row"
  requirement.
- A duplicate timestamp is compared with the last accepted timestamp. An
  out-of-order timestamp is any earlier timestamp. Neither consumes a series
  index.
- A real zero volume is a value and must remain `0`; an empty volume field is
  absent and must remain `null`.
- The report is written after the window is built, so a source read failure
  produces no partial stdout and no completed report.
- The report is human-readable stderr output; stdout remains the NDJSON stream.
- The report stores one skipped-row record per rejection. A file with an
  extreme number of bad rows can grow that list; this slice prioritizes complete
  attribution over a cap.

## Decisions

| # | Decision | Choice | Rationale |
|---|---|---|---|
| T1 | Command shape | Add `candles-tolerant <csv-path>` | A new capability arrives as a new command; the default contract stays untouched |
| T2 | Report channel | NDJSON on stdout, report on stderr | stdout stays machine-readable and the default stdout schema is reused |
| T3 | Report summary | `quality rows_read=... accepted=... rejected=... duplicate=... out_of_order=... invalid=... evicted=... retained=...` | Fixed key order gives byte-stable output |
| T4 | Skipped rows | `skipped line=<n> reason=<reason>` in source order | Every rejection is attributable to a source line and a reason |
| T5 | Row-level failures | Skip and count; continue | Tolerant mode must not fail the file for one bad row |
| T6 | File-level failures | Fail with exit 1 and no stdout | A bad header or unreadable file cannot be attributed as a row |
| T7 | Count semantics | `rejected = duplicate + out_of_order + invalid`; `rows_read = accepted + rejected`; `accepted = evicted + retained` | Counts are auditable and testable |
| T8 | Zero volume | Preserve as `0` | A real zero is a measurement, not a missing value |
| T9 | Report ordering | Summary first, then skipped rows in source order | Deterministic bytes independent of map or thread order |
| T10 | Tests | Checked-in fixtures and binary-level assertions | Tests never read `data/` and assert observable behavior |

## Acceptance criteria

- `quantrule candles <path>` is unchanged: strict errors still fail with exit 1
  and no stdout.
- `quantrule candles-tolerant <path>` exits 0 for row-level failures and prints
  the retained valid candles as NDJSON.
- The report is deterministic and contains the eight summary counts in the fixed
  order.
- Every skipped row appears exactly once as `skipped line=<n> reason=<reason>`,
  in source order.
- Invalid OHLC, non-finite values, bad timestamps, duplicates, out-of-order
  timestamps, and wrong field counts are skipped and counted.
- A real zero volume is printed as `0`; an absent volume is printed as `null`.
- A 1001-row valid file reports `evicted=1` and `retained=1000`.
- A header-only file reports zero counts; an all-invalid file reports the rejects
  and still exits 0.
- Missing files and bad headers still fail with exit 1 and no stdout.
- Interactive and non-interactive tolerant runs produce byte-identical stdout;
  repeated runs produce byte-identical stdout and report bytes.
- Tests use checked-in fixtures only and never read `data/`.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, and `cargo doc --no-deps` are clean.
- No third-party dependency is added.

## Work breakdown

| Step | Action | Role | Paths |
|---|---|---|---|
| 1 | Write this plan and log | Orchestrator | `docs/` |
| 2 | Add tolerant loading, skip reasons, and report rendering | Implementation | `crates/quantrule-shell/src/csv.rs` |
| 3 | Add `candles-tolerant` parsing, execution, and help | Implementation | `crates/quantrule-shell/src/command.rs` |
| 4 | Wire interactive and non-interactive entrypoints | Implementation | `crates/quantrule-shell/src/main.rs` |
| 5 | Add checked-in fixtures and binary-level tests | Test Engineer | `crates/quantrule-shell/tests/` |
| 6 | Update README and help fixture | Implementation | `README.md`, `crates/quantrule-shell/tests/fixtures/help.txt` |
| 7 | Run quality gates and record evidence | Test Engineer | `crates/` |
| 8 | Review the complete diff | Code Reviewer | all |
| 9 | Commit, push, open PR after authorization | Orchestrator | git state, PR |

## Verification

Commands to run from the worktree with `~/.cargo/bin` on `PATH`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

Expected evidence: every command exits zero. Tests cover exact NDJSON and report
bytes for a mixed fixture, zero volume, all-invalid input, header-only input,
eviction, repeated runs, interactive/non-interactive identity, default strict
behavior, and tolerant source errors. No result is claimed until it is observed.

Observed evidence on 2026-10-23:

- `cargo fmt --all -- --check` exited 0.
- `cargo clippy --all-targets -- -D warnings` checked all three crates and exited 0 with no warnings.
- `cargo test` exited 0: 50 `quantrule-data` unit tests, 10 shell parser unit tests, 32 shell integration tests, and 2 doctests passed.
- `cargo doc --no-deps` generated documentation for all three crates and exited 0.
- A manual tolerant run against `/home/zantgo/Proyectos/quant-rule/data/BTCUSD_M1_2018-01-01_2024-12-31.csv` exited 0 and printed 1000 NDJSON lines. Its report was `quality rows_read=3393175 accepted=3393175 rejected=0 duplicate=0 out_of_order=0 invalid=0 evicted=3392175 retained=1000`. The first output was `series_index` 3,392,175 / `window_index` 0 / `2024-12-30T22:19:00Z`; the last was 3,393,174 / 999 / `2024-12-31T14:58:00Z`.
- A manual strict run over the same file exited 0, printed 1000 lines, wrote nothing to stderr, and produced stdout byte-identical to the tolerant run.
- The shell integration tests use only `crates/quantrule-shell/tests/fixtures/`; no test reads `data/`. The manual real-file check is not a test.
- `Cargo.lock` is unchanged and lists only the three workspace packages.

The acceptance criteria are met. The change was committed as `4208c99`, pushed, and opened as PR #54. Merge remains subject to human authorization.

## Risks and recovery

- **A report could grow with a pathological file.** Mitigation: one record per
  skipped row is required for complete attribution; the candle window itself
  remains bounded. A later issue can cap detail if the product needs it.
- **The report could drift from the counts.** Mitigation: exact fixture bytes pin
  the counts and their documented relationships (`rows_read = accepted +
  rejected`, `rejected = duplicate + out_of_order + invalid`, `accepted = evicted
  + retained`).
- **The tolerant command could change default behavior.** Mitigation: keep the
  strict loader and `candles` tests unchanged, and add a test that `candles`
  still fails on the mixed fixture.
- **A skipped row could be silently dropped.** Mitigation: every rejection is
  counted and written with its source line and reason.
- **Report bytes could depend on iteration order.** Mitigation: skipped rows are
  stored in source order and the summary order is fixed.
- **The additive command/report shape needs PR review.** Mitigation: the shape is
  documented in the plan, help text, and README; rejecting it does not require
  changing the default `candles` contract.
- **Recovery.** The change is additive to the shell crate, README, and help. It
  can be reverted with a new commit on `main`; the worktree is preserved until
  the PR is accepted.

## Stop conditions

Stop and request human direction if:

- The additive command name or report shape is not acceptable.
- The default `candles` contract would need to change.
- Tolerant mode would need to swallow a file-level error or invent data.
- The shell would need to compute indicators or model state.
- A dependency is needed.
- Verification cannot be completed with the installed toolchain.
