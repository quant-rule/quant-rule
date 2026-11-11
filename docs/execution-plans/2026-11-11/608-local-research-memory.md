# P3.3 — Local Research Memory

- Date: 2026-11-11 (UTC)
- Issue: [#608](https://github.com/quant-rule/quant-rule/issues/608)
- Parent plan: [`588-tauri-platform.md`](588-tauri-platform.md)
- Status: Verified
- Claim type: Behaviour Claim
- Owner: Human
- Workflow: Execution Plan
- Note (2026-11-11): shipped in v0.9.0; the human accepted the release and the
  audit decision on 2026-11-11. The memory-use-case parity follow-up is tracked
  by the [audit remediation plan](audit-app-service-native-remediation.md).

## Intent

Add the local SQLite research memory behind the application service. It stores
workspaces, datasets, configurations, runs, metrics, claims, and notes on the
researcher's machine; it can export, import, back up, and restore itself. The
desktop UI and future commands read the same store.

## Current state

`crates/quantrule-app` owns workspace, dataset, configuration, model-run, and
export I/O. It has no database. The approved Tauri plan names SQLite and
`quantrule.research-memory/v1`. No system SQLite is installed, so the binding
must bundle its own engine.

## Scope

Included:

- Add `rusqlite` with the `bundled` feature (human-approved 2026-11-11).
- Add `crates/quantrule-app/src/memory/` with the v1 schema and migrations.
- CRUD for workspaces, datasets, configs, runs, metrics, claims, and notes.
- Deterministic JSON export and import.
- Consistent backup and restore.
- Unit and integration tests, including migration idempotence and round trips.

Excluded:

- Desktop UI wiring (Phase 3D).
- Shell commands (the memory is an application-service capability until the UI
  exposes it).
- Cloud sync, accounts, and multi-user access.
- Run orchestration changes.

## Assumptions and uncertainty

- `rusqlite` 0.42 with bundled SQLite builds on the pinned Rust 1.99 toolchain.
- The caller supplies every timestamp; the memory never reads the clock. This
  keeps exports and tests reproducible.
- The v1 schema is additive. A future schema change adds a numbered migration;
  it does not rewrite v1 in place.
- Export/import is a logical backup. File backup uses SQLite's `VACUUM INTO`.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| SQLite binding | `rusqlite` + `bundled` | Self-contained Linux build; no system library; synchronous service. Approved by the human. |
| Schema identity | `quantrule.research-memory/v1` in `meta` | A stored schema value is visible to export/import and migrations. |
| Time | Caller-supplied UTC Unix seconds | No clock in the service; deterministic tests and exports. |
| Metric value | Typed `kind` + `value`/`text_value` | A null is a stored null, never zero; numbers and text stay distinct. |
| Import target | Empty memory only | Prevents silent merges; backup/restore is the copy path. |
| Backup | `VACUUM INTO` | One consistent SQLite file, not a hand-copied WAL set. |
| Restore | Copy backup to a new path, then open | Explicit, testable, and safe while the target is not open. |

## Acceptance criteria

- A new database creates the v1 schema and records its version.
- Reopening applies no duplicate migration and preserves data.
- Every record type round-trips through CRUD in deterministic order.
- A null metric stays null; a zero metric stays zero.
- Export is byte-identical for the same database state.
- Import into an empty database reproduces the export.
- Backup then restore reproduces the export.
- Foreign keys reject an orphan run, metric, claim, or note.
- The full workspace gate passes.

## Work breakdown

| Step | Work | Role | Paths |
|---:|---|---|---|
| 1 | Add the approved dependency and memory module skeleton | Implementation | `crates/quantrule-app/Cargo.toml`, `src/memory/` |
| 2 | Schema, migrations, and typed records | Implementation | `src/memory/schema.rs`, `types.rs` |
| 3 | CRUD and ordered queries | Implementation | `src/memory/store.rs` |
| 4 | Export, import, backup, restore | Implementation | `src/memory/transfer.rs` |
| 5 | Tests and edge cases | Test | `src/memory/tests.rs` |
| 6 | Docs, changelog, AI log | Implementation | `docs/`, `CHANGELOG.md` |

## Verification table

Commands are run from the task worktree with
`CARGO_TARGET_DIR=/home/zantgo/Proyectos/quant-rule/target`.

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Dependency | Bundled SQLite builds | `cargo check -p quantrule-app`; rusqlite 0.42 bundled builds | pass | pass | Pass | Agent | 2026-11-11 | Human review pending |
| V2 | Schema | v1 tables and version exist | `cargo test -p quantrule-app memory::`; 22 tests | pass | 22 passed | Pass | Agent | 2026-11-11 | Human review pending |
| V3 | Migrations | Reopen is idempotent | schema migration tests; reopen and newer-version refusal | pass | pass | Pass | Agent | 2026-11-11 | Human review pending |
| V4 | CRUD | Every record type round-trips | CRUD and ordered-query tests; every record type | pass | pass | Pass | Agent | 2026-11-11 | Human review pending |
| V5 | Null | Null metric is not zero | null metric and zero metric test; integer precision test | pass | pass | Pass | Agent | 2026-11-11 | Human review pending |
| V6 | Export | Same state, same bytes | two exports compared byte for byte | pass | pass | Pass | Agent | 2026-11-11 | Human review pending |
| V7 | Import | Export/import round-trip | export/import round-trip and non-empty-target refusal | pass | pass | Pass | Agent | 2026-11-11 | Human review pending |
| V8 | Backup | Backup/restore round-trip | `VACUUM INTO` backup and restore round-trip | pass | pass | Pass | Agent | 2026-11-11 | Human review pending |
| V9 | Integrity | Orphan rows are rejected | orphan and cross-workspace rejection tests | pass | pass | Pass | Agent | 2026-11-11 | Human review pending |
| V10 | Gate | Full workspace gate | fmt, clippy, test, doc, repository checkers | pass | 1,476 passed, 8 ignored; checkers pass | Pass | Agent | 2026-11-11 | Human review pending |

## Risks and recovery

| Risk | Control | Recovery |
|---|---|---|
| Bundled SQLite fails to build | Early `cargo check` | Stop and report; the approved fallback is a JSON store, requiring a new decision. |
| Schema mistake | Versioned migration and tests | Add a numbered migration; never rewrite v1 in place. |
| Import overwrites data | Empty-target rule | Refuse and keep the target untouched. |
| Backup is inconsistent | `VACUUM INTO` | Use the backup file; do not copy a live database by hand. |
| Export depends on hash order | Ordered queries and sorted JSON keys | Fix the order in the query, not in the serializer. |

## Stop conditions

Stop and ask if:

- the bundled build fails on the pinned toolchain;
- the schema needs a record type outside the approved seven;
- import must merge into a non-empty database;
- the change would read the wall clock inside the memory;
- a new dependency beyond `rusqlite` is needed.
