# ADR: The SQLite Per-Bar Store and Research Schema

- Date: 2026-10-24 (UTC)
- Issue: [#65](https://github.com/quant-rule/quant-rule/issues/65) — arch: define the SQLite per-bar store and research schema
- Status: Accepted. The store implementation follows this document.
- Related: [research output contract](../../analytics/2026-10-24/research-output.md), [model contract](../../model/2026-10-21/four-axis-analytical-model.md), [store epic #64](https://github.com/quant-rule/quant-rule/issues/64)

## 1. Context

The model and the research export are complete. The export is a deterministic file artifact; it is not queryable, and it cannot tell whether a model run finished. The store gives QuantRule a durable, queryable record of one dataset: the candles, readings, per-axis metrics, resolutions, pivots, levels, events, legs, consensus, aggregates, and the materialized Layer 3.

The store is evidence. A dataset row exists only after every part of the run is committed, and `store verify` compares the stored values with a fresh in-memory build. The store is not a second model: it persists values the engines publish, and it never recomputes an indicator or a rule.

## 2. Decision

- **SQLite through `rusqlite` with the `bundled` feature.** SQLite is a single-file, transactional, SQL database with no server. `bundled` embeds the library, so the build does not depend on a system SQLite. This is a new shell-only dependency; it is approved for the store epic (#64).
- **The store is a shell module, not a crate.** It lives in `crates/quantrule-shell/src/store/`. The analytics, data, and model crates stay pure; only the shell performs I/O. The crate list and the dependency direction in `AGENTS.md` do not change.
- **Schema version 1** is the complete target table schema; **version 2** revises one unique index. The implementation sub-issues add writers and queries, not tables. `PRAGMA user_version` carries the version.
- **A wide per-bar table.** `bar` has one row per `(dataset, timeframe, bar_index)` with the candle, the six readings, the 47 metrics, and the 12 resolution columns: the research contract's Layer 2 record. Layer 1 and Layer 2 are projections of that table, not copies.
- **Records are separate tables.** `pivot`, `level`, `event`, `leg`, and `consensus` keep the full records. Events and levels carry both occurrence and availability bars.
- **Layer 3 is materialized without duplicating the base.** `layer3` holds the three higher-timeframe blocks and the 82 aggregates, keyed by `(dataset, bar_index)`. The M1 base is the `bar` row; `layer3_full` is a view that joins it.
- **A hard 100,000 M1 candle cap per dataset.** The `dataset.m1_bars` column has a `CHECK`, and the ingest rejects over-cap input. Derived timeframes add rows but cannot exceed the cap.
- **One transaction per dataset.** A failed ingest rolls back every table. A re-run with the same identity is a no-op; a different identity for an existing name is an error, not an overwrite.

## 3. Alternatives considered

| Alternative | Why it was rejected |
|---|---|
| CSV/JSON files only | No transactions, no indexes, no atomic commit, and `store verify` would need a second parser. The research export already covers the file artifact. |
| DuckDB | A large C++ dependency and an analytic engine the shell does not need; the queries are simple lookups by dataset and bar. |
| A key-value store (sled, redb) | No SQL, no joins for the layer view, and a custom query layer would be larger than the schema. |
| A new `quantrule-store` crate | It would move I/O out of the shell or add a fourth layer. The shell already owns every read and write; a module is the smaller boundary. |
| Storing long metric rows (one row per field) | It multiplies 128k rows into millions and makes a per-bar read a join. The per-bar record is the unit the model publishes. |
| Storing Layer 3 as a full 356-column copy | It duplicates the M1 base (73 columns) in every row. The view keeps one copy and the same columns. |

## 4. Schema version 1

All tables use `INTEGER` for indices and codes, `REAL` for numbers, and `TEXT` for identity strings. A missing number is `NULL`; it is never zero. Codes are the contract's codes. Column names match the research contract, so a query and an export use the same names.

### 4.1 `dataset`

| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER | Primary key, autoincrement |
| `name` | TEXT | Unique; the dataset directory name |
| `data_version` | TEXT | Caller-supplied identity |
| `model_version` | TEXT | Workspace package version |
| `model_contract` | TEXT | `2026-10-21` |
| `research_contract` | TEXT | `2026-10-24` |
| `structure_scale_code` | INTEGER | 1 minor, 2 major |
| `m1_bars` | INTEGER | `CHECK (m1_bars BETWEEN 0 AND 100000)` |
| `layer3_materialized` | INTEGER | 0 or 1; set when Layer 3 is complete |

### 4.2 `bar`

Primary key `(dataset_id, timeframe_secs, bar_index)`. Foreign key to `dataset(id) ON DELETE CASCADE`.

| Group | Columns |
|---|---|
| Identity | `dataset_id`, `timeframe_secs`, `bar_index` |
| Candle | `open_time`, `open`, `high`, `low`, `close`, `volume` |
| Readings | `ema20`, `ema50`, `ema200`, `rsi14`, `atr14`, `rvol20` |
| Direction | `direction`, `direction_state_code`, `direction_size_percentile`, `direction_readiness_code`, `direction_unavailable_reason_code`, `direction_state_age_bars`, `direction_state_changes_50` |
| Momentum | `momentum_force`, `momentum_acceleration`, `momentum_intensification`, `momentum_state_code`, `momentum_size_percentile`, `momentum_readiness_code`, `momentum_unavailable_reason_code`, `momentum_state_age_bars`, `momentum_state_changes_50` |
| Structure | `structure_score`, `structure_code`, `structure_scale_code`, `high_relation_code`, `low_relation_code`, `ema_distance_atr_20`, `ema_distance_atr_50`, `ema_distance_atr_200`, `ema_cross_code_20`, `ema_cross_code_50`, `ema_cross_code_200`, `ema_retest_code_20`, `ema_retest_code_50`, `ema_retest_code_200`, `structure_size_percentile`, `structure_readiness_code`, `structure_unavailable_reason_code`, `structure_state_age_bars`, `structure_state_changes_50` |
| Progression | `progress`, `progression_stage_code`, `progression_leg_direction`, `progression_leg_scale_code`, `progression_size_percentile`, `progression_readiness_code`, `progression_unavailable_reason_code`, `progression_state_age_bars`, `progression_state_changes_50` |
| Context | `atr_ratio`, `rvol_ratio`, `context_code` |
| Resolutions | `direction_continuous`, `direction_ordinal`, `direction_resolution_reason_code`, `momentum_continuous`, `momentum_ordinal`, `momentum_resolution_reason_code`, `structure_continuous`, `structure_ordinal`, `structure_resolution_reason_code`, `progression_continuous`, `progression_ordinal`, `progression_resolution_reason_code` |

The `timeframe_secs + bar_index` key is the research contract's `bar_index`; the report's `bar_index` is per timeframe. Index: `bar(dataset_id, timeframe_secs, bar_index)` is the primary key; an additional index on `(dataset_id, timeframe_secs, open_time)` supports time lookups.

### 4.3 `pivot`

Primary key `(dataset_id, timeframe_secs, scale_code, kind_code, occurred_bar_index)`. The table stores candidates and confirmed pivots; `confirmed_code` is 0 or 1, and `available_bar_index` is the confirmation bar for a confirmed pivot and the occurrence bar for a candidate.

| Columns |
|---|
| `dataset_id`, `timeframe_secs`, `scale_code`, `kind_code`, `price`, `occurred_bar_index`, `available_bar_index`, `confirmed_code` |

### 4.4 `level`

Primary key `(dataset_id, timeframe_secs, reference_code)`. The registry is append-only; `reference_code` is `1000 + n`.

| Columns |
|---|
| `dataset_id`, `timeframe_secs`, `reference_code`, `scale_code`, `kind_code`, `price`, `occurred_bar_index`, `available_bar_index` |

Index: `(dataset_id, timeframe_secs, available_bar_index)`.

### 4.5 `event`

Primary key `(dataset_id, id)`, with `id` autoincrement. A unique index covers `(dataset_id, timeframe_secs, event_type_code, event_subtype_code, source_code, occurred_bar_index, reference_code)`; `reference_code` stores 0 for "none", because SQLite treats NULLs as distinct in a unique index.

| Columns |
|---|
| `dataset_id`, `id`, `timeframe_secs`, `event_type_code`, `event_subtype_code`, `source_code`, `occurred_bar_index`, `available_bar_index`, `direction_code`, `reference_code`, `primary_value`, `reference_value` |

Index: `(dataset_id, timeframe_secs, available_bar_index, event_type_code)`.

### 4.6 `leg`

Primary key `(dataset_id, timeframe_secs, leg_id)`. The retained ledger is what the model exposes; a terminal leg is not deleted.

| Columns |
|---|
| `dataset_id`, `timeframe_secs`, `leg_id`, `scale_code`, `origin_price`, `origin_occurred_bar_index`, `origin_available_bar_index`, `origin_confirmed_code`, `leg_direction_code`, `target_price`, `target_basis_code`, `target_version`, `target_stable_code`, `progress`, `displacement_price`, `displacement_atr`, `lifecycle_code`, `stage_code`, `target_reached_code` |

Index: `(dataset_id, timeframe_secs, origin_occurred_bar_index)`.

### 4.7 `consensus`

Primary key `(dataset_id, timeframe_secs, bar_index)`.

| Columns |
|---|
| `dataset_id`, `timeframe_secs`, `bar_index`, `bullish_axis_count`, `bearish_axis_count`, `neutral_axis_count`, `weakest_directional_axis_code` |

### 4.8 `layer3`

Primary key `(dataset_id, bar_index)`. The base is the M1 `bar` row; the block and aggregate columns are the research contract's names.

| Group | Columns |
|---|---|
| Identity | `dataset_id`, `bar_index` |
| M5 block | `m5_bar_index`, `m5_open_time`, `m5_open`, `m5_high`, `m5_low`, `m5_close`, `m5_volume`, `m5_<metric>`, `m5_<resolution>`, `m5_staleness_secs` (67 columns) |
| M15 block | the same 67 columns with the `m15_` prefix |
| H1 block | the same 67 columns with the `h1_` prefix |
| Aggregates | the 82 `agg_*` columns |

`layer3_full` is a view:

```sql
CREATE VIEW layer3_full AS
SELECT b.*, l.m5_*, l.m15_*, l.h1_*, l.agg_*
FROM bar AS b
JOIN layer3 AS l
  ON l.dataset_id = b.dataset_id AND l.bar_index = b.bar_index
WHERE b.timeframe_secs = 60;
```

The block columns are nullable; a timeframe with no closed bar yet is a null block. The aggregate columns are the model's 82 fields.

### 4.9 Indexes

The primary keys cover the per-dataset reads. Additional indexes:

| Table | Index |
|---|---|
| `bar` | `(dataset_id, timeframe_secs, open_time)` |
| `pivot` | `(dataset_id, timeframe_secs, available_bar_index)` |
| `level` | `(dataset_id, timeframe_secs, available_bar_index)` |
| `event` | `(dataset_id, timeframe_secs, available_bar_index, event_type_code)` |
| `leg` | `(dataset_id, timeframe_secs, origin_occurred_bar_index)` |

## 5. Migrations and PRAGMAs

- `PRAGMA user_version` is the schema version. A fresh database has 0; migration 1 creates the schema and sets 1. Migration 2 replaces the `event` unique index and sets 2.
- Each migration runs in one transaction. A failed statement rolls the migration back and leaves the previous version.
- A database whose `user_version` is newer than the binary's fails with a clear error before any persistent PRAGMA; its journal mode and bytes are unchanged. A non-zero `application_id` that differs from the QuantRule id is refused as not a store, and a version-0 SQLite file with tables is refused instead of migrated.
- A new store sets `PRAGMA application_id` to the QuantRule id (`QRUL`) in the migration transaction. A legacy store with a zero id adopts it on the next write open.
- A corrupt database fails on open or on `PRAGMA integrity_check`; the error names the path.
- PRAGMAs on every connection: `foreign_keys=ON`, `journal_mode=WAL` for a file database, `synchronous=NORMAL`, `busy_timeout=5000`. In-memory databases keep the default journal mode.
- The schema DDL is versioned, not `IF NOT EXISTS`; the migration runs once.
- No column is renamed, reordered, or removed in a migration. A new column is appended; a change of meaning is a new column and a new version.

## 6. Identity, idempotency, and transactions

The dataset identity is `(name, data_version, model_version, structure_scale_code, ladder)`. The ladder is the four timeframes `60, 300, 900, 3600`.

- `store ingest` opens one `BEGIN IMMEDIATE` transaction, inserts the dataset row, every bar, every record, and Layer 3, then commits.
- If `name` exists with the same `data_version`, `model_version`, `structure_scale_code`, and `m1_bars`, the command is a no-op and reports the existing row counts.
- If `name` exists with a different identity, the command fails and changes nothing. Replacing a dataset requires deleting it explicitly; no silent overwrite.
- A failed ingest rolls back every table; no partial dataset is visible.
- `layer3_materialized` is set in the same transaction that inserts the last Layer 3 row.

## 7. The command surface

| Command | Meaning |
|---|---|
| `store init [--db <path>]` | Create or open the database and apply migrations |
| `store ingest <csv-path> --dataset <name> --data-version <id> [--db <path>]` | Build the layers and insert the full dataset in one transaction |
| `store list [--db <path>]` | One line per dataset: identity, counts, Layer 3 state |
| `store show <dataset> <table> [filters] [--db <path>]` | Rows from one stored table, bounded by a limit |
| `store verify <csv-path> --dataset <name> --data-version <id> [--db <path>]` | Rebuild the dataset from source and compare every stored value; report zero mismatches or the first mismatch |

`store show` accepts `bar`, `pivot`, `level`, `event`, `leg`, `consensus`, and `layer3`, with `--timeframe`, `--bar`, and `--limit` filters as applicable. The exact query flags are fixed in [#75](https://github.com/quant-rule/quant-rule/issues/75); every table is reachable. The displayed rows omit the internal `dataset_id` and the event row `id`; they are storage identity, not dataset content, and the same logical dataset dumps identically across a remove and re-ingest. `db series`, `db bars`, `db metrics`, and `db layer3` are per-bar projections over the same read-only query layer.

The default database path is `data/db/quantrule.db`. `--db` overrides it. Tests use a file under `target/` or an in-memory database; no test writes outside the repository.

## 8. What is stored and what is derived

| Layer | Store | Derivation |
|---|---|---|
| Layer 1 | `bar` columns (identity, candle, readings, metrics) | Read from the model's per-bar record |
| Layer 2 | `bar` resolution columns | Derived from Layer 1 by the analytics mapping; stored once |
| Layer 3 | `layer3` blocks and aggregates | Materialized from the as-of blocks and the model aggregates |
| Records | `pivot`, `level`, `event`, `leg`, `consensus` | Stored from the model's batch output |
| Research CSVs | Not stored | Rendered from the in-memory layers by `research export` |

The store does not store the event, leg, level, or consensus CSV files. The research export remains the file artifact; the store is the queryable record. `store verify` re-reads the source, rebuilds the layers, and compares the stored rows with the expected rows through the same writer parameter functions. It proves writer/reader consistency and catches drift; it is not an independent oracle for the model or the layer computation. A stored dataset is addressed by its name, which uses only letters, digits, `.`, `-`, and `_`, does not start with `-`, and is not `.` or `..`.

## 9. Determinism

- The store writes are deterministic in their logical content: the same input and configuration produce the same rows, in the same order, with the same values.
- SQLite's file bytes are not part of the reproducible artifact. The deterministic artifact remains the research CSV export; `store verify` compares logical rows.
- No wall-clock value is stored. A dataset's coverage is the candle timestamps; its identity is the caller's data version.
- No `HashMap` iteration reaches the database. Rows are inserted in the layer order.

## 10. Consequences

Positive:

- One file per database, one transaction per dataset, and a SQL query surface for every stored table.
- The 100k dataset is a wide table with 128,332 bar rows, not millions of long rows.
- The store is isolated in the shell; the engines remain pure and dependency-free of SQLite.

Negative:

- A new dependency (`rusqlite` with `bundled`), a larger binary, and a C build at compile time.
- SQLite has no built-in decimal type; `REAL` is IEEE-754, and the values are the model's `f64` bits. Exact round-trip is tested.
- A wide `bar` table with 71 data columns and a `layer3` table with 301 columns are large; the 100k measurement is reported in #74.
- One writer at a time. The shell is single-threaded, and WAL lets readers proceed during a write.

## 11. AGENTS.md and dependency approval

- No crate or layer change: the store is a module of `crates/quantrule-shell`, and the dependency direction is unchanged.
- The dependency `rusqlite` with the `bundled` feature is approved for this store; it is used by the shell only. The exact version is pinned in `crates/quantrule-shell/Cargo.toml` when #67 lands.
- The shell-boundary rule already permits I/O in the shell; the store does not move domain logic out of the engines.

## 12. Verification

- #67: a fresh database opens idempotently; a failed migration rolls back; a newer schema fails; a corrupt file fails.
- #68–#73: every table round-trips the in-memory values, including nulls and valid zeros.
- #74: a 100k M1 dataset round-trips; `store verify` reports zero mismatches; Layer 3 is causal; the time and size are reported.
- #75: every table is reachable from a command, and the output is deterministic.

## 13. Addendum: version 2 event identity

Version 1 identified an event by `(dataset_id, timeframe_secs, event_type_code, event_subtype_code, source_code, occurred_bar_index, reference_code)`. A Signal threshold crossing additionally needs its threshold (`reference_value`). When RSI crosses 30, 50, and 70 on the same bar, the three events share every version 1 key column, so the unique index rejected the second one.

Decision: keep the version 1 key for every event, but make it partial (`WHERE NOT (event_type_code = 3 AND event_subtype_code = 1)`) and add a second partial unique index over the same columns plus `reference_value` for Signal threshold crossings. The DB-migration path replaces `event_key` and creates `event_threshold_key` in one transaction.

| Alternative | Consequence |
|---|---|
| Add `reference_value` to the single key for every event | Non-threshold events with `NULL` values would still collide, and a spurious measured value would silently create a second identity. |
| Drop the unique index | Duplicate occurrences would become undetectable. |
| Emit one threshold event per bar | Violates the contract: each crossed threshold is a distinct event. |

Consequences: schema version 2; a version 1 database is migrated in place; the writer requires a finite `reference_value` for a Signal threshold crossing; the writer's duplicate check and the row builder's order use the same threshold identity. #75 presents the version 2 key.
