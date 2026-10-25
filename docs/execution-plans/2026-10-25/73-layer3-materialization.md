# Execution Plan: Materialize Layer 3 for M1

- Issue: [#73](https://github.com/quant-rule/quant-rule/issues/73) — feat: materialize layer 3 for M1
- Branch: `feat/73-layer3-materialization`
- Pull request: [#107](https://github.com/quant-rule/quant-rule/pull/107)
- Status: Verified
- Date: 2026-10-25 (UTC)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-25/73-layer3-materialization.md`](../../ai-transparency-log/2026-10-25/73-layer3-materialization.md)

## 1. Intent

Complete the `layer3` table by writing the as-of M5, M15, and H1 blocks with
the existing 82 aggregate columns for every M1 bar, and mark a new dataset as
materialized. The analytics engine already computes the blocks; the writer must
copy them without recomputing the as-of selection.

## 2. Current state

- `layer3` has 285 columns: `dataset_id`, `bar_index`, three 67-column blocks,
  and 82 aggregate columns.
- `Layer3Row.higher` already carries the as-of blocks in fastest-first order.
- `LAYER3_AGGREGATE_INSERT` binds only `dataset_id`, `bar_index`, and the 82
  aggregate columns; block columns stay `NULL`.
- `dataset.layer3_materialized` is inserted as `0`.
- Analytics tests already cover the 09:03/09:04 as-of rule, block causality, and
  100k Layer 3 rows.
- #71 deliberately left the blocks `NULL`; this issue replaces that partial
  write.

## 3. Scope

Included:

- A 285-column Layer 3 insert, with each of the three blocks mapped to its
  fixed schema prefix.
- `layer3_materialized = 1` for a new dataset.
- Pre-transaction validation of the block set, block-to-bar consistency, base
  consistency, and close/staleness consistency.
- Updated writer, real-binary, and aggregate tests.
- README and module documentation updates.

Excluded:

- Schema changes or migrations.
- Backfilling #68–#72 datasets. A same-identity re-ingest remains a no-op.
- Changing the analytics block computation, the model, the command output, or
  any other table.
- `store verify`, query commands, or the 100k store benchmark (#74–#75).

## 4. Assumptions and uncertainty

- The block order is fixed by the schema: M5, M15, H1. The ingest command always
  builds that ladder.
- An absent block is legitimate before the first higher-timeframe close; it is
  written as `NULL` in all 67 columns.
- The analytics block's candle, metrics, and resolutions equal the corresponding
  stored Layer 2 row. A mismatch is a writer input defect, not a value to repair.
- The 285-column insert performs acceptably for a 100k M1 dataset; #74 measures
  the full 100k path.
- Existing #68–#72 datasets are incomplete by definition and must not be
  silently upgraded.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Insert strategy | One 285-column insert; aggregate insert plus block updates | One insert | Atomic, one statement per row, and the column order is testable against the DDL |
| D2 | Block mapping | Map by timeframe; rely on vector position | Map 300/900/3600 to `m5_`/`m15_`/`h1_` | The schema prefixes are fixed; an unexpected timeframe is invalid |
| D3 | Validation source | Recompute the as-of bar; compare with stored Layer 2 | Compare with stored Layer 2 and check close/staleness | Keeps as-of logic in analytics and catches copy drift |
| D4 | Flag | Set `1` for every new ingest; backfill old rows | Set `1` for new rows only | Preserves the #71 no-op/backfill decision |
| D5 | Old datasets | Leave them; update them on re-ingest | Leave them | Same identity remains a no-op; #74 detects incompleteness |
| D6 | File size | Grow `sql.rs`/`validate.rs`; split | Keep the current seam unless either passes 1,000 lines | Both files remain under the limit after this change |

## 6. Acceptance criteria

- Every new ingest writes one `layer3` row per M1 bar with all 285 columns.
- An M5/M15/H1 block is written when present; an absent block writes `NULL` to
  all 67 of its columns.
- The 09:03 base uses the previous M5 bar; the 09:04 base uses the
  09:00–09:05 M5 bar.
- Appending later bars changes no earlier `layer3` row.
- 100k M1 input produces 100k Layer 3 rows (analytics evidence; #74 covers the
  stored path).
- `layer3_materialized = 1` for a new dataset; old datasets remain `0` and are
  not silently changed.
- The writer rejects an unexpected block timeframe, an unsorted or duplicate
  block, a missing higher bar, a block that differs from its stored Layer 2 row,
  a base that differs from its stored M1 row, or a close/staleness mismatch.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --workspace`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
  are clean.

## 7. Work breakdown

1. **Layer 3 SQL.** Add the three block parameter groups, change
   `layer3_params` to bind 285 values, and replace the aggregate-only insert with
   the full insert. Affected: `store/ingest/sql.rs`, `store/ingest.rs`.
2. **Block validation.** Validate the base against the stored M1 row, the block
   set against the fixed higher timeframes, and each block against its stored
   Layer 2 row and close/staleness. Affected: `store/ingest/validate.rs`.
3. **Dataset flag.** Insert `layer3_materialized = 1`. Affected:
   `store/ingest.rs`.
4. **Tests.** Update the #71 aggregate tests and the real-binary audit; add
   round-trip, absent-block, invalid-block, and flag tests. Affected:
   `store/ingest/aggregate_tests.rs`, `store/ingest/tests.rs`,
   `tests/store_aggregates.rs`.
5. **Documentation.** Update the README, the writer module doc, this plan, and
   the transparency log.
6. **Evidence.** Run focused and full checks; record observed results; open the
   pull request; move #73 to Done; stop for review.

## 8. Verification

| Check | Command | Expected evidence |
|---|---|---|
| Format | `cargo fmt --all -- --check` | Clean, exit 0 |
| Lints | `cargo clippy --all-targets -- -D warnings` | Clean, exit 0 |
| Tests | `cargo test --workspace` | Record exact counts |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean, exit 0 |
| Writer focus | `cargo test -p quantrule-shell store::ingest` | New block/flag tests pass |
| Binary audit | `cargo test -p quantrule-shell --test store_aggregates` | Non-null blocks, flag 1, block joins, zero staleness violations |

Limitations: the 100k stored path is #74; this issue reuses the existing 100k
analytics test and does not benchmark the 285-column insert.

## 9. Risks and recovery

- **Column drift.** A wrong prefix or order corrupts every Layer 3 row. The
  insert/DDL order test and the 285-parameter round-trip fail loudly.
- **Validation scope.** Recomputing the as-of selection in the shell would
  duplicate analytics logic. Validation only checks the stored-copy invariants.
- **Old datasets.** No backfill means a same-identity database can remain
  incomplete. `layer3_materialized = 0` is the explicit marker; #74 reports it.
- **Performance.** The insert adds 201 bound values per row. If the 100k path is
  materially slower, report it in #74 and revisit the insert strategy rather
  than dropping the blocks.
- **Recovery.** No schema migration. A bad ingest is re-run into a fresh
  database; the dataset transaction rolls back on failure.

## 10. Stop conditions

- A new schema column, table, or migration appears necessary.
- The model or analytics block output cannot be stored without loss.
- Validation would reject output the analytics engine legitimately produces.
- The 285-column insert cannot complete the focused tests.
- The work requires a change to the command contract, event detection, axis
  semantics, or a dependency.
- The pull request is open but not reviewed or merged.

## 11. Verification evidence (2026-10-25)

| Check | Command | Observed result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | Clean, exit 0 |
| Lints | `cargo clippy --all-targets -- -D warnings` | Clean, exit 0 |
| Tests | `cargo test --workspace` | 819 passed, 0 failed, 3 ignored |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean, exit 0 |
| Writer focus | `cargo test -p quantrule-shell store::` | 44 passed, 0 failed |
| Binary audit | `cargo test -p quantrule-shell --test store_aggregates` | 1 passed, 0 failed |

The binary audit over `state_m1.csv` stores 260 Layer 3 rows. The M5 block is
populated on 256 rows, M15 on 246, and H1 on 201; every populated block joins a
stored higher bar, every block closes at or before its M1 base, and every
`staleness_secs` equals the base close minus the block close. `layer3` is marked
`materialized = 1`. The same audit reports zero consensus mismatches and keeps
the existing event, leg, and level checks.

The analytics tests already prove the 09:03/09:04 boundary and block causality.
The ignored 100k analytics benchmark was run explicitly: 128,332 Layer 1 rows
and 100,000 Layer 3 rows in 48.164 s (debug). The writer round-trip test covers
all three block prefixes and the absent-block case; the invalid-block cases cover
a duplicate, unexpected timeframe, missing bar, wrong staleness, mismatched
candle, wrong base, and a block that closes after the base.

Limitations: the stored 100k path and its timing are #74's scope. No schema
migration or backfill was added; existing #68–#72 datasets remain
`layer3_materialized = 0`.
