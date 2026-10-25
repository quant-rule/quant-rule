# Execution Plan: Store Query and Inspection Commands

- Issue: [#75](https://github.com/quant-rule/quant-rule/issues/75) — feat: add store query and inspection commands
- Branch: `feat/75-store-query`
- Pull request: pending
- Status: Verified
- Date: 2026-10-25 (UTC)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-25/75-store-query.md`](../../ai-transparency-log/2026-10-25/75-store-query.md)

## 1. Intent

Make the stored dataset inspectable from the shell without recomputing a
reading, metric, resolution, aggregate, event, or leg. The commands read the
SQLite file and render its stored values in deterministic CSV or NDJSON.

## 2. Current state

- `store init` and `store ingest` exist; no read command exists.
- `Store` exposes only the writing surface and test-only connection access.
- The schema has `bar`, `consensus`, `layer3`, `pivot`, `level`, `event`, and
  `leg` tables; `layer3` is materialized by #73.
- `event.reference_code` stores `0` for a missing reference; the shell mapping
  must present it as `NULL`.
- The ADR names `store list` and `store show`; issue #75 names the four `db`
  views. The approved surface combines both over one query layer.

## 3. Scope

Included:

- `store list [--db <path>]`: one CSV row per dataset with identity, counts, and
  Layer 3 state.
- `store show <dataset> <table> [--timeframe <tf>] [--bar <n>] [--limit <n>]
  [--db <path>]`: every table, CSV, bounded and ordered.
- `db series <dataset> [--timeframe <tf>] [--limit <n>] [--db <path>]`:
  candle-only NDJSON.
- `db bars <dataset> [--timeframe <tf>] [--bar <n>] [--limit <n>] [--db <path>]`:
  full `bar` CSV.
- `db metrics <dataset> [--timeframe <tf>] [--bar <n>] [--limit <n>] [--db <path>]`:
  the 47 metric and 12 resolution columns as CSV.
- `db layer3 <dataset> [--bar <n>] [--limit <n>] [--db <path>]`: the materialized
  Layer 3 table as CSV.
- Read-only opening: a missing database is an error and is never created; a
  newer schema is rejected; no migration runs.
- Help, README, exact-output tests, parser tests, and a missing-file test.

Excluded:

- `store verify` and the 100k benchmark (#74).
- A SQL prompt, arbitrary SQL, writes, deletes, or updates.
- Recomputing a stored value or joining the model into the query path.
- Changing the ingest command or schema.

## 4. Assumptions and uncertainty

- `--limit` default 100 and maximum 1,000 keeps terminal output bounded.
- `--timeframe` accepts the existing `1m`, `5m`, `15m`, `1h` codes.
- `--bar` filters `bar_index`; tables without that column reject it.
- CSV uses an empty field for `NULL`, matching the research export. A stored
  `event.reference_code = 0` is rendered as an empty field.
- Layer 3 is M1-only in the schema, so `--timeframe` is invalid there.
- The read-only connection can read a version 1 or 2 schema without migrating.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Command surface | Issue-only `db`; ADR-only `store show`; combined | Combined | Keeps every table reachable and gives the per-bar views the issue asks for |
| D2 | Open mode | `Store::open`; read-only open | Read-only | A query never creates a database or migrates it |
| D3 | CSV null | `NULL` text; empty field | Empty field | Matches the research export |
| D4 | Ordering | Insertion order; explicit primary-key order | Explicit order per table | Deterministic across SQLite plans |
| D5 | Limits | Unlimited; fixed 100; configurable 1..=1000 | Configurable, default 100 | Prevents a 128k-row terminal dump |
| D6 | Event sentinel | Raw `0`; empty/null | Null | `0` is outside the valid reference domain |
| D7 | Module layout | Grow `command.rs`; add `command/store_query.rs` | Add the query-command module | `command.rs` is already over 1,000 lines; the query commands are cohesive |
| D8 | Query layer | SQL in command; store query API | Store query API | Keeps table names, ordering, and cell conversion in the store boundary |

## 6. Acceptance criteria

- `store list` prints a stable header and one row per dataset, sorted by name,
  with bar/consensus/layer3/pivot/level/event/leg counts and the Layer 3 flag.
- `store show` reaches every table, prints DDL-order columns, applies the valid
  filters, orders deterministically, and caps rows at the limit.
- `db series` prints oldest-first candle NDJSON; `db bars`, `db metrics`, and
  `db layer3` print deterministic CSV with fixed headers.
- `event.reference_code = 0` appears as an empty field in `store show event`.
- A missing database, missing dataset, unknown table, inapplicable filter, or
  invalid limit fails without creating or modifying the database.
- Interactive and non-interactive output are byte-identical for the same args.
- Help and README document every command.
- `cargo fmt`, `cargo clippy`, `cargo test --workspace`, and `cargo doc` are
  clean.

## 7. Work breakdown

1. **Store read API.** Add `Store::open_read_only`, `StoreError` query variants,
   and `store/query.rs` with `Table`, `Cell`, `TableRows`, `SeriesRow`,
   `DatasetSummary`, and the query methods. Affected: `store/mod.rs`,
   `store/error.rs`, `store/query.rs`.
2. **Query command module.** Add `command/store_query.rs` with argument parsing
   and the run functions for `store list/show` and `db series/bars/metrics/layer3`.
   Affected: `command.rs`, `command/store_query.rs`.
3. **Dispatch and help.** Add the variants, interactive/non-interactive
   dispatch, help lines, and the help fixture. Affected: `command.rs`,
   `main.rs`, `tests/fixtures/help.txt`.
4. **Tests.** Store query unit tests for ordering/filters/sentinel/limits;
   parser tests; binary integration tests for exact headers, rows, missing
   database, missing dataset, and interactive equality. Affected:
   `store/query.rs`, `command/store_query.rs`, `tests/store_commands.rs`.
5. **Documentation.** README command table/prose, this plan, and the
   transparency log.
6. **Evidence.** Run focused and full checks; open the PR; move #75 to Done;
   stop for review.

## 8. Verification

| Check | Command | Expected evidence |
|---|---|---|
| Format | `cargo fmt --all -- --check` | Clean |
| Lints | `cargo clippy --all-targets -- -D warnings` | Clean |
| Tests | `cargo test --workspace` | Exact counts recorded |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean |
| Store query focus | `cargo test -p quantrule-shell store::query` | Ordering, filters, sentinel, limits |
| Binary focus | `cargo test -p quantrule-shell --test store_commands` | Exact output and read-only behavior |

## 9. Risks and recovery

- **SQL injection.** Table names come from a closed enum; values are bound
  parameters. A free-form SQL surface is explicitly out of scope.
- **Large output.** The limit caps every command; `layer3` is wide but bounded.
- **Read-only compatibility.** A version 1 or 2 schema is readable; a newer
  version is rejected. No migration runs on a query.
- **Sentinel leakage.** The event `reference_code` conversion is tested on a
  row whose stored value is `0`.
- **Float formatting.** Use the shortest round-trip decimal and normalize
  negative zero, matching the research export.
- **Recovery.** Queries are read-only; a failed command leaves the database
  unchanged.

## 10. Stop conditions

- A write, migration, or schema change appears necessary.
- A new dependency is required.
- A table cannot be queried without recomputing a model value.
- The command surface would change an existing command's contract.
- The pull request is open but not reviewed or merged.

## 11. Verification evidence (2026-10-25)

| Check | Command | Observed result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | Clean, exit 0 |
| Lints | `cargo clippy --all-targets -- -D warnings` | Clean, exit 0 |
| Tests | `cargo test --workspace` | 861 passed, 0 failed, 3 ignored |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean, exit 0 |
| Store query focus | `cargo test -p quantrule-shell store::query` | 9 passed, 0 failed |
| Parser focus | `cargo test -p quantrule-shell command::store_query` | 5 passed, 0 failed |
| Binary focus | `cargo test -p quantrule-shell --test store_commands` | 7 passed, 0 failed |

The binary tests ingest `state_m1.csv` and verify `store list` counts:
333 bars, 333 consensus rows, 260 Layer 3 rows, 354 pivots, 94 levels,
624 events, and 217 legs, with `layer3_materialized = 1`. `store show` reaches
all seven tables with headers of 74 (`bar`), 7 (`consensus`), 285 (`layer3`),
8 (`pivot`), 8 (`level`), 12 (`event`), and 19 (`leg`) columns. `db series`
prints two oldest-first NDJSON rows, `db metrics` prints 61 columns, and
`db layer3` prints 285. The event reference sentinel test confirms a stored
`reference_code = 0` appears as an empty field.

A missing database is rejected without creating a file; a missing dataset
reports `dataset 'missing' was not found`; interactive and non-interactive
`store list` output are byte-identical.

Limitations: `store verify`, the 100k cap evidence, and query timings are #74.
The read-only path supports schema versions 1 and 2; a newer schema is rejected.
