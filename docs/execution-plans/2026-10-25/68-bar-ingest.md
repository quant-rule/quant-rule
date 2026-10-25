# Execution Plan: Persist Candles, Readings, and Four-Axis Metrics (ST4)

- Issue: [#68](https://github.com/quant-rule/quant-rule/issues/68) — feat: persist candles, readings, and four-axis metrics per bar
- Branch: `feat/68-bar-ingest`
- Pull request: pending
- Status: Verified
- Date: 2026-10-25 (UTC)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-25/68-bar-ingest.md`](../../ai-transparency-log/2026-10-25/68-bar-ingest.md)

## 1. Intent

Add the first store writer: `store ingest` reads one M1 CSV, builds the
M1/M5/M15/H1 layers in memory, and writes the dataset row plus one wide `bar`
row per Layer 1 row in a single transaction. This is the base every later store
issue extends; it must round-trip exactly, reject a different identity without
overwriting, and leave no partial dataset on failure.

## 2. Current state

- `main` is at `02e24fb`; the store foundation (#67) is merged.
- `crates/quantrule-shell/src/store/` owns the connection, schema v1, migration,
  PRAGMAs, errors, and `store init`.
- `crates/quantrule-analytics/src/layers/` already builds Layers 1, 2, and 3 from
  the model output; `crates/quantrule-shell/src/command.rs::load_layers` is the
  shared in-memory build used by the research export.
- Schema v1 already has all 74 `bar` columns; #68 writes the 62 non-resolution
  columns and leaves the 12 resolution columns null for #70.
- `load_full_timeframes` enforces the 100k M1 cap and whole-minute timestamps.

## 3. Scope

Included:

- `store ingest <csv-path> --dataset <name> --data-version <id> [--db <path>]`.
- A `store` writer that inserts one `dataset` row and one `bar` row per Layer 1
  row in one `BEGIN IMMEDIATE` transaction.
- Identity checks: same identity is a no-op; a different identity is a conflict.
- Nulls preserved, valid zeros preserved, no partial dataset on failure.
- Unit tests for the writer, CLI tests for the command, help/README updates,
  this plan, and the transparency log.

Excluded:

- Pivots, levels, resolutions, events, legs, consensus, aggregates, and Layer 3
  (#69–#73).
- `store list`, `store show`, and `store verify` (#74–#75).
- Schema changes or a migration; schema v1 is complete.
- Any change to the data, model, or analytics crates.
- The 100k performance measurement, which belongs to #74.

## 4. Assumptions and uncertainty

- `Layer1Row` is the authoritative per-bar record; the writer copies it and
  computes nothing.
- The store holds every ladder timeframe, not only M1. The issue's "stored bar
  count equals the accepted source rows" is interpreted as the M1 count; the
  full `bar` count is asserted separately. The verified 100k dataset has 128,332
  Layer 1 rows for 100,000 M1 source rows.
- `structure_scale_code` is the research default, major (2), because the ingest
  command has no scale option in the ADR.
- An empty CSV produces an empty Layer 1 set. The schema permits `m1_bars = 0`;
  the writer will accept it only when the M1 row count is also zero.
- The command output is part of the contract and must be deterministic:
  `ingested <name> bars=<n> timeframes=<k>` or
  `unchanged <name> bars=<n> timeframes=<k>`.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Writer location | A new crate; a shell module | `store/ingest.rs` | The shell owns SQLite; the engines stay pure |
| D2 | Layer build | Rebuild per store table; reuse `load_layers` | Reuse | The store and the CSV cannot drift silently |
| D3 | Transaction | Per row; one per dataset | One per dataset | The ADR requires an all-or-nothing dataset |
| D4 | Identity | Name only; the ADR tuple | ADR tuple | A different data version is not an overwrite |
| D5 | No-op | Error; success with counts | Success with counts | Re-running the same ingest is safe |
| D6 | Resolution columns | Write nulls explicitly; omit them | Omit | SQLite defaults them to null; #70 fills them |
| D7 | Layer 3 flag | Set 1; leave 0 | Leave 0 | Layer 3 is not materialized until #73 |
| D8 | Empty input | Reject; accept | Accept when M1 is zero | The schema and the contract allow an empty series |
| D9 | Output | Verbose; one line | One line | Stable, parseable, and easy to assert |

## 6. Acceptance criteria

- `store ingest` writes one `dataset` row and one `bar` row per Layer 1 row.
- The M1 `bar` count equals the accepted M1 source rows; the full count equals
  the Layer 1 rows across M1/M5/M15/H1.
- Every non-resolution `bar` column round-trips exactly, including nulls and
  valid zeros.
- The 12 resolution columns are null after #68.
- A failed write rolls back the dataset row and every bar row.
- Re-running the same identity is a no-op and reports the existing counts.
- A different identity for an existing name is a conflict and changes nothing.
- An over-cap input (100,001 M1 bars) is rejected before any write.
- `fmt`, `clippy -D warnings`, `cargo test --workspace`, and `doc -D warnings`
  are clean.
- Help, README, and the command reference describe the new command.

## 7. Work breakdown

1. Add `store/ingest.rs`: `DatasetRecord`, `IngestReport`, validation, identity
   lookup, the dataset insert, the prepared bar insert, and the transaction.
2. Add `StoreError::Conflict`, `StoreError::InvalidDataset`, and
   `StoreError::Write`.
3. Add the parser, `Command::StoreIngest`, the run function, and the interactive
   and non-interactive dispatch.
4. Add writer unit tests: full round-trip, null/zero, no-op, conflict, rollback,
   cap, count mismatch, reordered rows, and empty input.
5. Add CLI tests: ingest, no-op, conflict, invalid source, and interactive
   output equality.
6. Update `HELP`, `tests/fixtures/help.txt`, and `README.md`.
7. Write this plan and the transparency log; run the checks; commit, push, open
   the PR, and move #68 to Done.

## 8. Verification

| Check | Expected evidence |
|---|---|
| Writer unit tests | Round-trip, null/zero, no-op, conflict, rollback, cap, count, order, empty |
| CLI tests | Ingest, no-op, conflict, invalid source, interactive equality |
| `cargo test --workspace` | All tests pass; no test is skipped or weakened |
| `cargo fmt --all -- --check` | Clean |
| `cargo clippy --all-targets -- -D warnings` | Clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean |

The 100k timing and the full `store verify` are measured in #74, not here.

Observed evidence (2026-10-25 UTC):

| Check | Result |
|---|---|
| Writer unit tests | 13 passed: 62-column round-trip, null/zero, dataset row, no-op, conflict, rollback, cap, count mismatch, empty, row order, timeframe count, placeholder count, empty identity |
| Parser tests | 6 new `store ingest`/`store init` tests; the `command::tests` module has 36 passing tests |
| CLI tests | 4 new ingest tests: ladder counts and null resolutions, no-op, conflict, invalid source leaves no file, interactive output equality |
| `cargo test --workspace` | 786 passed, 0 failed, 3 ignored |
| `cargo fmt --all -- --check` | Clean |
| `cargo clippy --all-targets -- -D warnings` | Clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean |

The CLI fixture `state_m1.csv` stores 333 bars across 4 timeframes, 260 of them M1, matching the research export. The 12 resolution columns and `layer3_materialized` are asserted null/zero.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| A 62-column insert maps a value to the wrong column | The unit test compares every column by name with unique values |
| A failed insert leaves a partial dataset | One transaction; the rollback test forces a failure after the first row |
| The store and the research export drift | Both use `load_layers`; the round-trip test compares the stored values to the layer |
| A conflict overwrites an existing dataset | The identity is checked inside the transaction; the conflict test asserts unchanged counts |
| The 100k cap is bypassed | The loader rejects the 100,001st M1 row; the writer also checks `m1_bars` |
| The output differs between interactive and non-interactive | One render function; the CLI test compares both byte for byte |

## 10. Stop conditions

- A required column is missing from schema v1.
- The writer needs a schema change or a new dependency.
- The identity tuple cannot be compared without a ladder column.
- A check fails and the fix is outside this scope.

## 11. Delivery

| Item | Value |
|---|---|
| Commit | `feat: persist candles, readings, and metrics per bar` |
| Branch | `feat/68-bar-ingest` |
| Pull request | pending |
| Board | #68 is `In Progress`; it moves to `Done` when the PR is open |

Merge, release, tag, and deployment remain with the human.
