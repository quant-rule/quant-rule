# Execution Plan: Persist Pivots and Confirmed Levels (ST5)

- Issue: [#69](https://github.com/quant-rule/quant-rule/issues/69) — feat: persist pivots and confirmed levels
- Branch: `feat/69-pivot-levels`
- Pull request: pending
- Status: Verified
- Date: 2026-10-25 (UTC)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-25/69-pivot-levels.md`](../../ai-transparency-log/2026-10-25/69-pivot-levels.md)

## 1. Intent

Extend `store ingest` so the transaction that writes the dataset and every
ladder bar also writes every pivot candidate, every confirmed pivot, and every
confirmed level. The stored records must be the model's own output, must keep
the occurrence and availability bars distinct, and must never expose a pivot
before the bar on which it became knowable.

## 2. Current state

- `main` is at `d45f386`; #68 is merged.
- `store ingest` builds M1/M5/M15/H1 through
  `command::load_layers` and writes one `dataset` row plus one `bar` row per
  Layer 1 row in one `BEGIN IMMEDIATE` transaction.
- Schema v1 already has the `pivot` and `level` tables and their indexes. This
  issue adds writers, not tables or migrations.
- `quantrule-model` already computes `PivotSeries` and
  `ConfirmedLevelRegistry`; `quantrule-analytics::layers::TimeframeSeries`
  already holds both.
- The analytics `LayerSet` publishes Layer 1, Layer 2, and Layer 3 rows only.
  Pivots and levels are currently discarded after the layer build.

## 3. Scope

Included:

- `PivotRow` and `LevelRow` records in `quantrule-analytics::layers`, built
  from the exact `PivotSeries` and `ConfirmedLevelRegistry` the layer build
  already uses.
- `LayerSet::pivots` and `LayerSet::levels`, ordered by timeframe and then by
  the record's contract order.
- An `IngestRows` bundle passed to the writer; `store ingest` writes bars,
  pivots, and levels in the existing transaction.
- Writer validation for finite prices, valid scale/kind codes, reference
  codes, bar ranges, availability ordering, and duplicate record keys.
- Analytics, writer, and CLI tests for round-trip, availability, duplication,
  and rollback.
- README and plan/transparency-log updates.

Excluded:

- Resolutions, events, legs, consensus, aggregates, and Layer 3 (#70–#73).
- `store list`, `store show`, and `store verify` (#74–#75).
- Schema changes or migrations; schema v1 is complete.
- Any change to the data or model crates.

## 4. Assumptions and uncertainty

- A pivot occurrence is stored once. A candidate that later confirms becomes a
  single row with `confirmed_code = 1` and `available_bar_index` set to the
  confirmation bar; it is not stored twice under the table's primary key.
- Candidate availability is the occurrence bar; confirmed availability is the
  confirmation bar. The model guarantees both.
- The store writes the full model output, not a user-selected subset. An empty
  dataset has no pivot or level rows.
- The command output stays `ingested <name> bars=<n> timeframes=<k>` or
  `unchanged <name> bars=<n> timeframes=<k>`; record counts are not part of the
  #68 contract and are left to `store list` in #74.
- A #68-era database with bars but no pivots is not rejected by the identity
  no-op. The ADR defines the no-op by identity only; `store verify` in #74 is
  the completeness check.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Record source | Recompute pivots in the shell; expose the layer build's records | Expose | One computation path, no drift |
| D2 | Candidate plus confirmation | Two rows per confirmed pivot; one row per occurrence | One row | The table's primary key is per occurrence |
| D3 | Record order | Availability order; occurrence order | Occurrence order, then scale and kind | Stable and independent of unordered iteration |
| D4 | Writer input | Separate calls; one bundle | `IngestRows` | Bars, pivots, and levels share one transaction |
| D5 | Validation | Rely only on SQLite; validate before the transaction | Validate | A bad record never creates a dataset |
| D6 | Duplicate handling | Silently ignore; reject and roll back | Reject | A duplicate is a defect, not a merge |
| D7 | Output | Add record counts; keep the #68 line | Keep | Existing command contract stays stable |

## 6. Acceptance criteria

- Every pivot candidate and confirmed pivot matches the in-memory
  `PivotSeries`.
- Every confirmed level matches `ConfirmedLevelRegistry`, including contiguous
  `1000 + n` reference codes within each timeframe.
- A candidate row has `available_bar_index == occurred_bar_index`; a confirmed
  row has `available_bar_index` at or after its occurrence.
- Availability violations are rejected before anything is written.
- A pivot is never duplicated; a duplicate record fails the transaction and
  leaves no dataset, bar, pivot, or level row.
- The existing 333-bar/4-timeframe ingest output is unchanged.
- `fmt`, `clippy -D warnings`, `cargo test --workspace`, and `doc -D warnings`
  are clean.

## 7. Work breakdown

1. Add `layers/records.rs` with `PivotRow`, `LevelRow`, and deterministic
   builders from `PivotSeries` and `ConfirmedLevelRegistry`.
2. Expose the built pivots and levels from `TimeframeSeries`; add them to
   `LayerSet` before the Layer 3 M1-first swap.
3. Extend `store/ingest.rs` with `IngestRows`, record validation, and the
   `pivot`/`level` inserts in the existing transaction.
4. Update `run_store_ingest` to pass all three record sets.
5. Add analytics tests for model agreement, ordering, availability, and empty
   input; add writer tests for round-trip, validation, duplicates, and
   rollback; add CLI assertions for nonzero records and zero violations.
6. Update the README description of what `store ingest` writes.
7. Run the checks, write the transparency log, mark this plan `Verified`,
   commit, push, open the PR, and move #69 to Done.

## 8. Verification

| Check | Expected evidence |
|---|---|
| Analytics record tests | Pivot/level rows match the model; candidate and confirmed availability |
| Writer unit tests | Eight-column round-trip, invalid availability, invalid code, duplicate pivot/level, rollback |
| CLI tests | `state_m1.csv` stores nonzero pivots and levels; zero availability violations; contiguous level codes |
| `cargo test --workspace` | All tests pass; no test is skipped or weakened |
| `cargo fmt --all -- --check` | Clean |
| `cargo clippy --all-targets -- -D warnings` | Clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean |

## 9. Risks and recovery

- A record insert can fail after the bars are written. The transaction rolls
  back the whole dataset, which the tests exercise.
- The pivot table's primary key cannot represent a candidate and its later
  confirmation as two rows. The chosen one-row form preserves the confirmation
  without duplication.
- The layer build is unchanged, so a failed record write cannot alter the
  research export.
- No migration is needed; an existing schema-v1 file opens unchanged.

## 10. Stop conditions

Stop and ask the human if the pivot representation requires two rows, if the
schema must change, or if the command output contract must expose record counts.

## 11. Verification evidence

Observed on 2026-10-25 (UTC) in the task worktree:

| Check | Result |
|---|---|
| `cargo test -p quantrule-analytics layers::tests` | 16 passed, 1 ignored |
| `cargo test -p quantrule-shell store::ingest` | 16 passed |
| `cargo test -p quantrule-shell --test cli store_ingest` | 4 passed |
| `cargo test --workspace` | 791 passed, 0 failed, 3 ignored |
| `cargo fmt --all -- --check` | Clean |
| `cargo clippy --all-targets -- -D warnings` | Clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean |

The `state_m1.csv` ingest stores 354 pivots (94 confirmed, 260 candidates) and
94 levels (M1 68, M5 21, M15 5; H1 has no confirmed level). The stored records
have zero availability violations, every level reference range starts at 1001
and is contiguous within its timeframe, and the command output remains
`ingested demo bars=333 timeframes=4`.

Duplicate pivot and level keys are rejected by the pre-transaction validation
chosen in D5, so the transaction is never opened for that case. The rollback
test forces a duplicate bar primary key inside the transaction and confirms
that the dataset, bar, pivot, and level tables are all left empty.
