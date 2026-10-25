# Execution Plan: Persist Events, Legs, and Level Records (ST8)

- Issue: [#72](https://github.com/quant-rule/quant-rule/issues/72) — feat: persist events, legs, and level records
- Branch: `feat/72-events-legs-levels`
- Pull request: pending
- Status: Verified
- Date: 2026-10-25 (UTC)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-25/72-events-legs-levels.md`](../../ai-transparency-log/2026-10-25/72-events-legs-levels.md)

## 1. Intent

Extend `store ingest` so a dataset also carries the model's event records and
progression leg records, and confirm that the confirmed-level records written by
#69 still round-trip. Each event is written once, at its availability bar; each
retained leg is written as its full model record; each level keeps its
`1000 + n` reference. The store copies the model's numbers. It does not detect
events, compute legs, assign levels, or alter any axis value. Events are
evidence, not a metric input.

## 2. Current state

- `main` is at `0bf7fe2`; #71 is merged and #72 is the next work item.
- Schema v1 already defines the `event` and `leg` tables. No table or column
  change is needed. Version 2 replaces the `event` unique index so coincident
  Signal threshold crossings, which differ only by `reference_value`, are
  distinct rows.
- `event` columns: `dataset_id`, `id`, `timeframe_secs`, `event_type_code`,
  `event_subtype_code`, `source_code`, `occurred_bar_index`,
  `available_bar_index`, `direction_code`, `reference_code` (`NOT NULL DEFAULT
  0`), `primary_value`, `reference_value`. Version 2's `event_key` is partial:
  it covers `(dataset_id, timeframe_secs, event_type_code, event_subtype_code,
  source_code, occurred_bar_index, reference_code)` for every event except a
  Signal threshold crossing. `event_threshold_key` covers the same columns plus
  `reference_value` for `(event_type_code = 3, event_subtype_code = 1)`.
- `leg` columns: `dataset_id`, `timeframe_secs`, `leg_id`, `scale_code`,
  `origin_price`, `origin_occurred_bar_index`, `origin_available_bar_index`,
  `origin_confirmed_code`, `leg_direction_code`, `target_price`,
  `target_basis_code`, `target_version`, `target_stable_code`, `progress`,
  `displacement_price`, `displacement_atr`, `lifecycle_code`, `stage_code`,
  `target_reached_code`. The primary key is `(dataset_id, timeframe_secs,
  leg_id)`.
- `level` is already written by #69 and covered by the #71 round-trip tests.
- `LayerSet` carries `timeframes`, `layer1`, `layer2`, `consensus`, `layer3`,
  `pivots`, and `levels`. It has no event or leg rows.
- `TimeframeSeries` already retains the full `events: Vec<EventRecord>` and the
  `ProgressionSeries`; it exposes neither.
- `ProgressionSeries::legs()` returns every retained leg in creation order. The
  ledger is bounded to 1,024 legs per scale per timeframe by the model contract.
- `IngestRows` carries bars, consensus, Layer 3, pivots, and levels.
- The research contract defines `events.csv` (10 columns) and `legs.csv` (18
  columns). The SQLite tables match those columns plus `dataset_id`, and `event`
  adds a row `id`.
- `state_m1.csv` produces 624 event lines and 8 published leg lines, so the
  checked-in fixture exercises both record kinds.
- The `store/ingest.rs` module doc says events and legs are deferred to #72–#73.
  After this issue only the higher-timeframe blocks remain deferred to #73.

## 3. Scope

Included:

- Public `EventRow` and `LegRow` types in `quantrule-analytics::layers`, with
  builders that copy the model's `EventRecord` and `ProgressionLeg` values.
- `LayerSet.events` and `LayerSet.legs`, built in `build_layers`.
- `TimeframeSeries` accessors for its events and its retained legs.
- Contract ordering: events by timeframe, availability bar, type, subtype,
  source, occurrence bar, and reference; legs by timeframe and `leg_id`.
- `IngestRows` carrying events and legs; `store ingest` passing them.
- Pre-transaction validation of event and leg shape, contract ranges,
  availability, and duplicate keys. Level validation is unchanged.
- `EVENT_INSERT` and `LEG_INSERT` bindings and inserts inside the existing
  `BEGIN IMMEDIATE` transaction.
- Event `reference_code` `None` encoded as `0`. `0` is outside the valid
  reference domain (`1..=3` for EMA references, `>= 1001` for confirmed
  levels), so it is the "no reference" sentinel.
- Analytics tests, writer tests, and a real-binary audit of the stored event,
  leg, and level rows.
- README and module-doc updates, this plan, and the transparency log.

Excluded:

- Schema changes or migrations. Schema v1 is the complete target.
- Backfilling databases written by #68–#71. A same-identity re-ingest remains a
  no-op; #74's `store verify` is the completeness check.
- The higher-timeframe block columns and `layer3_materialized = 1` (#73).
- `store verify` and the query/inspection commands (#74–#75).
- Any change to event detection, the leg state machine, level assignment, the
  four axes, or the aggregation rules.
- A new CSV export for events or legs. The research contract already defines
  those files; this issue persists the same records to SQLite.
- Any change to the `store ingest` output contract.

## 4. Assumptions and uncertainty

- `EventRecord` and `ProgressionLeg` are the complete published records. The
  analytics row types are flat copies of their fields, not new measurements.
- `TimeframeSeries.events` holds the full event history, already sorted by
  availability. The row builder re-sorts it into the research contract's order.
- `ProgressionSeries::legs()` is the retained ledger. The store writes one row
  per retained leg; older terminal legs outside the 1,024-per-scale bound are
  not stored. The per-bar published leg values remain in `bar`, and the bound is
  the contract's retention rule rather than a data loss defect.
- A leg row is the current or final state at the end of the ingested series. A
  forming or active leg is a snapshot; a terminal leg is frozen. The same input
  always produces the same snapshot.
- The `event` table's `reference_code` is `NOT NULL`; `None` maps to `0`. A
  future reader maps `0` back to `None`. `Some(0)` is never a valid model value.
- The version 2 event keys are the occurrence identity. The `id` column is a
  row identifier and is not part of the record contract.
- `state_m1.csv` produces both events and legs. If a smaller fixture produces
  none, a test uses a fixture that does; the non-empty assertion is not
  weakened.
- Existing same-identity databases are not backfilled, following the #71
  decision. #74 detects incomplete datasets.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Where event and leg rows live | Map model records in the shell writer; add analytics row types | Add `EventRow` and `LegRow` to `layers/records.rs` | Matches `PivotRow`/`LevelRow`; the store copies one flat row type and never reaches into the model |
| D2 | How records reach the writer | Separate record sets; extend `LayerSet` | Extend `LayerSet` with `events` and `legs` | One deterministic build pass; the existing reordering test then covers the records too |
| D3 | Event `reference_code` with no reference | Change the schema to nullable; add a boolean column; use `0` | Encode `None` as `0` | A nullable column would need a new schema version; `0` is outside the valid reference domain and keeps the unique key meaningful |
| D4 | Event row order | Preserve detector order; order by the contract | Order by timeframe, availability, type, subtype, source, occurrence, reference, and reference value | Matches `events.csv`; the final reference-value tie-break totally orders coincident threshold crossings |
| D5 | Leg row order | Preserve creation order; order by the contract | Order by timeframe and `leg_id` | Matches `legs.csv` and the model's creation-order identifier |
| D6 | Validation depth | Recompute detection or leg state; validate shape and ranges | Validate shape, contract ranges, availability, and duplicate keys only | The writer copies values; the model owns detection and the state machine. Same boundary as #71 |
| D7 | Existing databases | Backfill on re-ingest; leave them and detect later | Same identity stays a no-op; #74 verifies completeness | Follows the approved #71 decision; no migration |
| D8 | Test placement | Grow `ingest/tests.rs`; add a new record test module | Add `store/ingest/record_tests.rs` and share helpers from `fixtures.rs`; extend `tests/store_aggregates.rs` | Keeps source files under 1,000 lines and reuses the real-binary audit |

## 6. Acceptance criteria

- `LayerSet.events` has one row per model event across the configured ladder, in
  the contract order. Every row carries the type, subtype, source, timeframe,
  occurrence and availability bars, direction, reference, and measured values.
  `available_bar_index >= occurred_bar_index` for every row. No occurrence key is
  duplicated. `reference_code` is `None`, `1..=3`, or `>= 1001`.
- `LayerSet.legs` has one row per retained model leg, ordered by timeframe and
  `leg_id`. Every row carries the full leg record: scale, origin and
  availability, confirmation, direction, target price/basis/version/stability,
  raw unclamped `progress`, both displacements, lifecycle, stage, and
  target-reached code. A forming leg has `origin_available_bar_index = NULL` and
  `origin_confirmed_code = 0`; a confirmed origin has both present. `stage_code`
  is `0..=5` or `NULL`; `lifecycle_code` is `0..=4`; `target_reached_code` is
  `0`, `1`, or `NULL`.
- The confirmed-level rows continue to round-trip exactly as in #69/#71.
- `store ingest` writes events and legs in the same transaction as the dataset,
  bars, consensus, Layer 3, pivots, and levels. Every event and leg column
  round-trips. An event with no reference stores `reference_code = 0`.
- An invalid event or leg row is rejected before the transaction opens: unknown
  type, subtype outside the type's range, source outside `1..=12`, direction
  outside the contract, reference outside its domain, non-finite value, bar out
  of range, availability before occurrence, duplicate event key, invalid scale,
  `leg_id = 0`, origin availability before occurrence, confirmed/available
  mismatch, invalid lifecycle/stage/target-reached code, or duplicate
  `(timeframe, leg_id)`. No dataset, bar, event, leg, or other record is left
  behind.
- Events do not alter any metric value: ingesting the same bars with and without
  event/leg rows produces identical `bar` and `layer3` rows.
- A failed event or leg insert rolls back the dataset and every record.
- A same-identity re-ingest remains a no-op and does not duplicate events or
  legs.
- A real-binary ingest of `state_m1.csv` stores non-empty `event`, `leg`, and
  `level` tables, with zero availability violations and zero duplicate event
  keys or leg identifiers.
- The command output stays `ingested <name> bars=<n> timeframes=<k>` or
  `unchanged <name> bars=<n> timeframes=<k>`.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --workspace`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
  are clean.

## 7. Work breakdown

1. **Analytics record types.** Add `EventRow` and `LegRow` to
   `crates/quantrule-analytics/src/layers/records.rs`, with builders from
   `EventRecord` and `ProgressionLeg`. Sort events into contract order and legs
   by `leg_id`. Export both types. Affected: `layers/records.rs`,
   `layers/mod.rs`.
2. **Layer set and series.** Add `events` and `legs` to `LayerSet`; add
   `TimeframeSeries::events()` and `TimeframeSeries::legs()`; collect the rows
   in `build_layers`. Update the module docs. Affected: `layers/series.rs`,
   `layers/mod.rs`.
3. **Test literal.** Add empty `events` and `legs` to the `LayerSet` literal in
   `crates/quantrule-analytics/src/summary.rs`. Affected: `summary.rs`.
4. **Writer rows and validation.** Extend `IngestRows` with events and legs;
   validate event type/subtype/source ranges, direction, reference domain,
   finite values, bar range, availability, and duplicate occurrence keys;
   validate leg scale, direction, `leg_id`, origin availability, confirmation,
   target fields, finite measurements, lifecycle/stage/target-reached ranges, and
   duplicate `(timeframe, leg_id)`. Affected: `store/ingest.rs`,
   `store/ingest/validate.rs`.
5. **Writer SQL and inserts.** Add `event_params`/`EVENT_INSERT` and
   `leg_params`/`LEG_INSERT`; insert events then legs after the existing
   records, inside the transaction. Encode a missing event reference as `0`.
   Affected: `store/ingest/sql.rs`, `store/ingest.rs`.
6. **Command wiring.** Pass `layers.events` and `layers.legs` from
   `run_store_ingest`. Affected: `command.rs`.
7. **Tests.** Add analytics tests for row agreement with the model, contract
   ordering, availability, uniqueness, every timeframe, determinism under
   reordering, and event stability when later bars are appended. Add
   `store/ingest/record_tests.rs` for round-trip, invalid-row rejection, rollback,
   no-metric-change, no-duplicate, and insert-column/DDL order. Move the shared
   `stored_rows`, `assert_params_match`, and `insert_columns` helpers to
   `fixtures.rs`; extend `tests/store_aggregates.rs` with the real-binary record
   audit. Affected: `layers/tests.rs`, `store/ingest/record_tests.rs`,
   `store/ingest/fixtures.rs`, `store/ingest/tests.rs`,
   `store/ingest/aggregate_tests.rs`, `tests/store_aggregates.rs`.
8. **Documentation.** Update the README's `store ingest` row and prose, the
   `store/ingest.rs` module doc, and the `layers`/`records` module docs. Add this
   plan's evidence and the transparency log. Affected: `README.md`,
   `docs/ai-transparency-log/2026-10-25/72-events-legs-levels.md`, this plan.
9. **Evidence and handoff.** Run the checks, record the observed results here,
   open the pull request, move #72 to `Done` on the board once the PR is open,
   and stop for human review. Do not merge.

## 8. Verification

| Check | Command | Expected evidence |
|---|---|---|
| Format | `cargo fmt --all -- --check` | No output, exit 0 |
| Lints | `cargo clippy --all-targets -- -D warnings` | No warnings, exit 0 |
| Tests | `cargo test --workspace` | Record the exact passed/failed/ignored counts |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | No warnings, exit 0 |
| Analytics focus | `cargo test -p quantrule-analytics layers::` | New record tests pass |
| Writer focus | `cargo test -p quantrule-shell store::ingest` | New writer tests pass |
| Binary audit | `cargo test -p quantrule-shell --test store_aggregates` | Event, leg, and level tables non-empty; zero availability violations; zero duplicate keys |
| Determinism | The reordering test and the new record determinism test | Identical `LayerSet` across repeated and reordered builds |
| Causality | The new event-prefix test | Appending a bar never changes an earlier event row |

Limitations to report honestly:

- The integration test uses the checked-in `state_m1.csv` fixture, not a live
  source.
- The 100k benchmark is #74's scope; this change adds rows but no new
  performance-sensitive computation.
- Existing #68–#71 databases are not backfilled; the no-op behavior is tested,
  and #74 owns the completeness verifier.

## 9. Risks and recovery

- **Reference sentinel.** A reader that treats `event.reference_code = 0` as a
  real reference would misreport an event. The README and the row docs state the
  sentinel; #75 must map `0` to `None` when it queries events.
- **Leg retention.** The store writes the retained ledger, not every leg ever
  created. The contract's 1,024-per-scale bound is documented; the per-bar
  published leg remains in `bar`, so no published per-bar value is lost.
- **Snapshot semantics.** A forming or active leg can change before it becomes
  terminal. The stored row is the state at the end of the ingested series; a
  fixed input produces a fixed row. The contract does not require historical
  leg snapshots.
- **Schema drift.** The insert-column tests compare `EVENT_INSERT` and
  `LEG_INSERT` against the DDL order, so a future schema edit fails loudly.
- **Partial write.** Events and legs are inserted in the existing transaction.
  A forced event or leg failure rolls back the dataset and every record; the
  test asserts this.
- **Recovery.** Schema version 2 migrates a version 1 database in one
  transaction; a failure rolls back to version 1. There is no in-place repair
  of an incomplete dataset. A dataset that is incomplete or wrong is
  re-ingested into a fresh database. A same-identity re-ingest into the existing
  database is a no-op by design; #74 reports it.
- **Data growth.** Events and legs add rows proportional to the model's
  detections and the retained ledger. The 100k M1 cap still bounds the ladder.
  If the added rows make ingest materially slower, report the measurement; a
  benchmark belongs to #74.

## 10. Stop conditions

- A schema change beyond the approved version 2 event-index migration appears
  necessary, for example a nullable event reference or a new leg column. Stop
  and ask.
- A model record field cannot be stored without loss, or the model exposes no
  complete event or leg record. Stop and reconcile the contract.
- Validation would reject output the model legitimately produces. Stop and
  reconcile the rule rather than weakening the model or the test.
- A fixture cannot produce events or legs without changing model logic. Stop and
  choose a different fixture.
- The work requires a change to event detection, the leg state machine, level
  assignment, axis semantics, the command contract, or a dependency. Stop and
  ask.
- The pull request is open but not reviewed or merged. Stop; the human owns
  review and merge.

## 11. Blocker (2026-10-25)

The new analytics uniqueness test found that the schema's `event_key` cannot
represent every event the model publishes.

- The 300-bar synthetic fixture produces, at bar 111, three Signal events with
  `event_type_code = 3`, `event_subtype_code = 1`, `source_code = 4` (RSI 14),
  `occurred_bar_index = available_bar_index = 111`, and `reference_code = NULL`.
  Their only distinguishing field is `reference_value`: 30.0, 50.0, and 70.0.
- The model contract (§7.4) says distinct signal conditions may coincide on one
  bar; each is stored once and they are not duplicates because each names a
  different defined relationship. The model's own test
  (`crates/quantrule-model/src/events/signal.rs:596`) asserts two threshold
  crossings on one bar.
- The schema unique index `event_key` and the writer's duplicate-key validation
  omit `reference_value`, so they treat the three events as one key. The writer
  therefore rejects valid model output. The checked-in `state_m1.csv` happens
  not to trigger the collision, so the real-binary audit passes while the defect
  remains latent.
- The failing reproducer is
  `quantrule_analytics::layers::tests::event_and_leg_records_are_ordered_and_valid`.

Resolving this requires a human decision because it changes the fixed event
identity:

1. Extend the event identity for Signal threshold crossings to include
   `reference_value`, for example with a partial unique index and a matching
   writer key. No table column changes, but the indexes and the ADR change and a
   migration is needed for existing databases.
2. Add a `variant_code` (or equivalent integer discriminator) to `event` and
   include it in `event_key`. Requires a model/analytics field or a defined
   mapping, a schema column, an index migration, and a #75 mapping.
3. Change the model so each threshold crossing already has a distinct key
   component, for example a reserved `reference_code` per threshold. No schema
   change, but it changes the model contract and #75's reference mapping.
4. Drop the unique index and enforce uniqueness only in validation. Not
   recommended: it weakens the database invariant and still needs a schema
   change.

Recommendation: option 1, because it preserves every model event and changes no
table column or model output. The plan cannot proceed to verification until the
human approves a revised identity and the corresponding migration.

### Resolution (2026-10-25)

The human approved option 1. The approved scope now includes:

- Schema version 2: `event_key` becomes partial and `event_threshold_key` adds
  `reference_value` for Signal threshold crossings. A version 1 database is
  migrated in one transaction.
- The writer's duplicate key carries the threshold value for a Signal threshold
  crossing and rejects a threshold event with no finite `reference_value`.
- The event row order adds `reference_value` as the final tie-break, using a
  total order over finite values.
- Tests cover coincident 30/50/70 crossings, a duplicate threshold, a missing
  threshold value, the version 1 to version 2 migration, and the real-binary
  audit.


## 12. Verification evidence (2026-10-25)

| Check | Command | Observed result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | Clean, exit 0 |
| Lints | `cargo clippy --all-targets -- -D warnings` | Clean, exit 0 |
| Tests | `cargo test --workspace` | 818 passed, 0 failed, 3 ignored |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean, exit 0 |
| Analytics focus | `cargo test -p quantrule-analytics layers::tests` | 22 passed, 0 failed, 1 ignored |
| Writer focus | `cargo test -p quantrule-shell --bin quantrule store::ingest` | 34 passed, 0 failed |
| Binary audit | `cargo test -p quantrule-shell --test store_aggregates` | 1 passed, 0 failed |

The binary audit over `state_m1.csv` stores 624 `event`, 217 `leg`, and 94
`level` rows. It reports zero availability violations for events, levels, and
leg origins; zero event occurrences duplicated under the version 2 identity;
zero threshold crossings without a threshold; and zero duplicate leg
identifiers. A second ingest prints `unchanged demo bars=333 timeframes=4` and
leaves the event, leg, and level counts unchanged.

The analytics focus covers row agreement with the model, contract order,
availability, uniqueness, every timeframe, determinism under reordering, and
event-prefix causality. The writer focus covers exact event/leg round-trips,
the coincident 30/50/70 threshold crossings, a duplicate threshold, a missing
threshold value, invalid rows, rollback, the no-metric-change check, the
same-identity no-op, and the insert/DDL column order. The store focus covers
the version 1 to version 2 migration and the failed-migration rollback.

Limitations: the binary audit uses the checked-in `state_m1.csv` fixture, not a
live source. The 100k benchmark remains #74's scope. Existing #68–#71
databases are not backfilled; #74 owns the completeness verifier.
