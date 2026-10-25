# Execution Plan: Verify the Stored Dataset and the 100k Cap

- Issue: [#74](https://github.com/quant-rule/quant-rule/issues/74) — test: verify the stored dataset and the 100k cap
- Branch: `test/74-store-verify`
- Pull request: pending
- Status: Verified
- Date: 2026-10-25 (UTC)
- Store ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-25/74-store-verify.md`](../../ai-transparency-log/2026-10-25/74-store-verify.md)

## 1. Intent

Prove that a stored dataset is the same dataset the model builds, including the
100k M1 cap. `store verify` rebuilds the source layers, reads the database
read-only, and compares the dataset identity, the foreign-key graph, and every
row of every record table. A 100k integration test records the observed timings
for ingest, model, verification, query, and export.

## 2. Current state

- #73 materializes Layer 3 and sets `layer3_materialized = 1` for new ingests.
- #75 provides read-only query commands and a raw table reader.
- The writer tests cover round-trip, rollback, and idempotency.
- No `store verify` command exists.
- The analytics 100k benchmark builds 100,000 Layer 3 rows in 48.164 s (debug).
- `store ingest` is capped at 100,000 M1 rows before the transaction.

## 3. Scope

Included:

- `store verify <csv-path> --dataset <name> --data-version <id> [--db <path>]`.
- Rebuild the source, compare dataset identity, `bar`, `consensus`, `layer3`,
  `pivot`, `level`, `event`, and `leg`.
- Exclude the event autoincrement `id`; compare every record column exactly.
- Reject `layer3_materialized = 0` as incomplete.
- Run `PRAGMA foreign_key_check` for orphan rows.
- Read-only opening: missing databases are rejected, never created; no
  migration or repair.
- An ignored 100k M1 integration test for ingest, model, verify, query, and
  export timings.
- Exact-output and failure tests for a changed value, wrong identity,
  incomplete Layer 3, missing dataset, missing database, and an orphan row.

Excluded:

- A repair command, an automatic backfill, or a schema migration.
- Comparing against the research CSV export instead of the in-memory layers.
- Changing the model, writer, or query contracts beyond the verify command.
- A CI-required 100k run; the 100k test is ignored and run explicitly.

## 4. Assumptions and uncertainty

- The source CSV is the authority for the dataset's input. Verification compares
  stored candles and every derived value with a fresh build from that source.
- The writer's parameter order matches the DDL order; the existing insert/DDL
  tests enforce this. Verification relies on that order and removes only the
  event `id`.
- Read-only SQLite can read a WAL database after the writer has closed; the
  command does not checkpoint or modify it.
- The 100k test may take minutes and large disk space; it is not part of the
  default test run.
- Timings are machine-specific observations, not assertions.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Verify input | Source CSV; stored candles only | Source CSV + identity | Verifies candles and every derived value, matching the issue scope |
| D2 | Open mode | `Store::open`; read-only | Read-only | Verification must not create, migrate, or repair |
| D3 | Comparison | Domain rows; writer parameter rows | Writer parameters vs raw stored cells | Reuses the exact insert mapping and catches any column drift |
| D4 | Event `id` | Compare autoincrement ids; ignore | Remove `id` from stored rows | The id is a row identifier, not a record field |
| D5 | Output | Zero-mismatch summary; full dump | `verified <name> bars=<n> tables=7`; first mismatch fails | Bounded, deterministic user output |
| D6 | Incomplete dataset | Treat flag 0 as pass; reject | Reject | #68–#72 datasets are not backfilled; #74 is the completeness gate |
| D7 | 100k evidence | Always-on test; ignored benchmark | Ignored integration test | Keeps normal tests fast while recording real timings |
| D8 | Schema/dependency | New column or crate | None | The existing schema and dependencies suffice |

## 6. Acceptance criteria

- `store verify` succeeds on a fresh `state_m1.csv` ingest and prints
  `verified demo bars=333 tables=7`.
- A second same-identity ingest prints `unchanged`, and a second verify still
  succeeds.
- A changed stored value fails with a table, row, column, stored value, and
  expected value.
- A wrong data version, missing dataset, missing database, or
  `layer3_materialized = 0` fails without writing.
- An orphan row is detected by `PRAGMA foreign_key_check`.
- Interactive and non-interactive verification output are byte-identical.
- The ignored 100k test ingests 100,000 M1 bars, verifies zero mismatches,
  reads bounded query views, verifies the 285-column Layer 3 header, exports
  the research dataset, and reports timings and database size.
- `cargo fmt`, `cargo clippy`, `cargo test --workspace`, and `cargo doc` are
  clean.

## 7. Work breakdown

1. **Expected rows and raw reads.** Expose the writer's expected parameter rows
   and a raw un-limited table read; make cell conversion reusable.
2. **Verify core.** Add `store/verify.rs` for dataset identity, Layer 3 flag,
   foreign keys, event-id removal, row counts, and exact values.
3. **Command.** Add `command/store_verify.rs`, parser, dispatch, help, README,
   and ADR updates.
4. **Tests.** Add binary success/failure/no-op/orphan/interactive tests and the
   ignored 100k benchmark.
5. **Evidence.** Run focused and full checks; record timings; open the PR;
   move #74 to Done; stop for review.

## 8. Verification

| Check | Command | Observed result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | Clean |
| Lints | `cargo clippy --all-targets -- -D warnings` | Clean |
| Tests | `cargo test --workspace` | 871 passed, 0 failed, 4 ignored |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean |
| Verify focus | `cargo test -p quantrule-shell --test store_verify` | 8 passed |
| Parser focus | `cargo test -p quantrule-shell --bin quantrule command::store_verify` | 2 passed |
| 100k | `cargo test -p quantrule-shell --test store_100k -- --ignored --nocapture` | 1 passed |

### 100k timing evidence (debug build)

| Step | Observed |
|---|---|
| Ingest (model + store) | 58.138 s |
| Model (`market-state`) | 1.293 s |
| Verify | 75.945 s |
| `store list` | 0.300 s |
| `db series` 1,000 rows | 0.014 s |
| `db layer3` header/1 row | 0.008 s |
| `research export` | 54.097 s |
| Stored bars | 128,330 |
| Layer 3 rows | 100,000 |
| Database size | 229,732,352 bytes |
| Export files | 9 |

The 100k test uses 100,000 aligned M1 rows, verifies zero mismatches, and
records the timings above. The cap itself is also covered by the existing
writer/schema cap tests.

## 9. Risks and recovery

- **Comparison cost.** Verify reads 128k bars, 100k Layer 3 rows, and every
  record twice-ish (stored plus expected in memory). The 100k timing is reported
  rather than hidden; no wall-clock assertion is made.
- **Event id.** Removing the autoincrement column is the only structural
  exception. Every other column is compared exactly.
- **Read-only compatibility.** Version 1 and 2 databases are readable; a newer
  schema is rejected. No migration runs.
- **Orphans.** `foreign_key_check` reports any row whose dataset key is missing.
- **Old datasets.** Flag 0 is an explicit incomplete error, not a silent pass.
- **Recovery.** Verification is read-only; a failed verify leaves the database
  untouched. A corrupted dataset is re-ingested into a fresh database.

## 10. Stop conditions

- A write, migration, or schema change appears necessary.
- A new dependency is required.
- The source build and stored rows cannot be aligned without recomputing a
  model value differently.
- The 100k test cannot complete in a practical window or exceeds disk/memory
  limits.
- The pull request is open but not reviewed or merged.
