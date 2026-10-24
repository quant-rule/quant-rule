# Execution Plan: The SQLite Store and Research Schema (ST1)

- Issue: [#65](https://github.com/quant-rule/quant-rule/issues/65) — arch: define the SQLite per-bar store and research schema
- Branch: `arch/65-store-adr`
- Pull request: pending
- Status: Verified
- Date: 2026-10-24 (UTC)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/65-store-adr.md`](../../ai-transparency-log/2026-10-24/65-store-adr.md)

## 1. Intent

Decide the store architecture and the schema before the implementation sub-issues (#67–#75) write code. The store must persist one dataset per run, commit it atomically, answer queries, and prove the stored values match the in-memory build.

## 2. Current state

- `main` is at `0eef84f`; the model (#17–#30) and the research engine (#76–#82) are complete.
- The shell owns all I/O; the analytics, data, and model crates are pure.
- There is no SQLite dependency and no store module.
- The research export is the deterministic file artifact; it is not queryable and does not record run completion.

## 3. Scope

Included:

- The ADR: the dependency, the module location, schema version 1, the tables, indexes, PRAGMAs, migrations, identity, transactions, the command surface, determinism, consequences, and the verification plan.
- This plan and the transparency log.

Excluded:

- Store code (the implementation sub-issues).
- Backtest run persistence (#10–#12).
- Any change to the model, data, analytics, or shell behavior.

## 4. Assumptions and uncertainty

- The dataset ladder is M1/M5/M15/H1, and Layer 3 requires M1.
- The 100k cap applies to M1; the derived rows are bounded by it.
- `rusqlite` with `bundled` is available and approved; the exact version is pinned when #67 lands.
- The store is a shell module; the crate list and dependency direction do not change.
- SQLite file bytes are not the reproducible artifact; the CSV export remains the byte-identical artifact, and `store verify` compares logical rows.
- The record tables store the model's retained leg ledger, not every leg ever created; the model's bound is 1024 per scale.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Engine | CSV/JSON; DuckDB; KV; SQLite | SQLite with `rusqlite` bundled | One file, transactions, SQL, no server, no system library |
| D2 | Location | A new crate; a shell module | Shell module | The shell owns I/O; the engines stay pure |
| D3 | Per-bar shape | Long metric rows; one wide row | One wide row | The model publishes a per-bar record; a 100k dataset stays 128,332 rows |
| D4 | Layer 1/2 copies | Three tables; one table + projections | One `bar` table | Layer 1 and Layer 2 are projections of the same record |
| D5 | Layer 3 | A full 356-column copy; blocks + view | Blocks + `layer3_full` view | The base is not duplicated 100k times |
| D6 | Schema version | One migration per sub-issue; one v1 | One complete v1 | The target is known; sub-issues add writers, not migrations |
| D7 | Identity | Overwrite on re-ingest; no-op or error | Same identity no-op; different identity error | A silent overwrite destroys evidence |
| D8 | Transaction | Per table; per dataset | One per dataset | No partial dataset is visible |
| D9 | 100k cap | Application only; CHECK + application | Both | The database cannot hold an over-cap dataset even if a writer is wrong |
| D10 | Determinism claim | DB bytes; logical rows | Logical rows | SQLite file layout is not a stable artifact; the CSV is |

## 6. Acceptance criteria

- The ADR names every table, its key, its columns, and its indexes.
- The schema's column names match the research contract.
- The migration, PRAGMA, identity, transaction, and recovery rules are stated.
- The command surface covers init, ingest, list, show, and verify, and every table is reachable.
- The dependency and module location are approved; no crate/layer change is needed.
- The 100k cap, the layer storage split, and the determinism boundary are stated.
- The verification plan maps the remaining sub-issues to observable checks.

## 7. Work breakdown

1. Write the ADR.
2. Write the plan and the log.
3. Verify the links and the schema against the research contract.
4. Run the checks, commit, push, open the PR, move #65 to Done, merge, and clean up.
5. Continue with #67 (store foundation) on its own branch.

## 8. Verification

| Check | Result |
|---|---|
| ADR relative links | resolve |
| Schema column groups | match the research contract's Layer 2 (61 + 12) and Layer 3 (67 × 3 + 82) |
| `cargo fmt --all -- --check` | clean (no code change) |
| `cargo clippy --all-targets -- -D warnings` | clean (no code change) |
| `cargo test --workspace` | 753 passed, 0 failed, 3 ignored (baseline) |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | clean (baseline) |

The ADR is a design document. The implementation checks are listed in its §12 and are performed by #67–#75; no runtime claim is made here.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| The wide tables are slower than expected | #74 measures the 100k ingest and reports it; the wide shape is the decision, not an accident |
| SQLite's file bytes are mistaken for a reproducible artifact | The ADR states that the CSV export is the byte-identical artifact and the store is compared logically |
| A later sub-issue needs a schema change | Migrations append columns; no rename or reorder; the version is bumped |
| The dependency grows the binary | `bundled` is a deliberate trade for no system SQLite; the ADR records it |
| The layer3 view is slow | The view is a primary-key join on 100k rows; #75 measures a query |

## 10. Stop conditions

- A required column has no model source.
- The shell boundary cannot own the store without a new crate.
- The 100k cap cannot be enforced in SQLite and in the ingest.
- A schema decision conflicts with the research contract.

## 11. Delivery

| Item | Value |
|---|---|
| Commit | pending |
| Branch | `arch/65-store-adr` |
| Pull request | pending |
| Board | #65 moved to `Done` when the PR is open |

Merge, release, tag, and deployment remain with the human.
