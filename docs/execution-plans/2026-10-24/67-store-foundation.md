# Execution Plan: The Store Foundation and Migrations (ST3)

- Issue: [#67](https://github.com/quant-rule/quant-rule/issues/67) — feat: add the SQLite store foundation and migrations
- Branch: `feat/67-store-foundation`
- Pull request: [#100](https://github.com/quant-rule/quant-rule/pull/100)
- Status: Verified
- Date: 2026-10-24 (UTC)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/67-store-foundation.md`](../../ai-transparency-log/2026-10-24/67-store-foundation.md)

## 1. Intent

Create the store foundation: the `rusqlite` dependency, the shell-owned store module, schema version 1, the migration and PRAGMA rules, the error types, and the `store init` command. The later store issues add writers and queries on this foundation.

## 2. Current state

- `main` is at `0eef84f`; the store ADR (#65) is merged.
- The shell has no SQLite dependency and no store module.
- The research contract defines the column names and widths the schema must mirror.

## 3. Scope

Included:

- `rusqlite` with the `bundled` feature in the shell.
- `crates/quantrule-shell/src/store/`: the connection, the v1 DDL, the migration, the PRAGMAs, the errors, and the schema-version check.
- `store init [--db <path>]`, the help text, the README, and the help fixture.
- Tests: fresh schema, widths, idempotent reopen, newer schema, failed migration rollback, corrupt file, foreign keys, the 100k CHECK, and the CLI command.

Excluded:

- The writers and queries (#68–#75).
- Backtest tables.
- Any change to the data, model, or analytics crates.

## 4. Assumptions and uncertainty

- The schema DDL is generated from the analytics column registry and checked into the store module; the widths are asserted in a test.
- The in-memory constructor is test-only until the writers need it; the production path is `Store::open`.
- A corrupt file may fail on the first PRAGMA or on `PRAGMA user_version`; both are mapped to `Corrupt`.
- WAL applies to file databases; in-memory databases keep the default journal mode.
- `integrity_check` is test-only until `store verify` (#74) uses it.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Driver | `rusqlite` bundled; system SQLite | Bundled | No system dependency; the ADR approved it |
| D2 | Location | A crate; a shell module | Shell module | The shell owns I/O; the engines stay pure |
| D3 | DDL source | Hand-written; generated from the registry | Generated once, checked in | The names and types cannot drift from the contract |
| D4 | Migration | One statement at a time; one transaction | One transaction | A failed migration rolls back completely |
| D5 | Version | `user_version`; a table | `user_version` | One pragma, no extra table |
| D6 | Corrupt detection | Only on query; pragma + query | Both | A garbage file fails on the first pragma |
| D7 | In-memory | Production API; test-only | Test-only | No production caller yet; no dead API |
| D8 | `store init` default | Required path; default | Default `data/quantrule.db` | The common case is one command; `--db` overrides |

## 6. Acceptance criteria

- A fresh database has the v1 schema and `user_version = 1`.
- Reopening a file database is idempotent.
- A newer schema is rejected with the found and supported versions.
- A failed migration rolls back and leaves the version and tables unchanged.
- A corrupt file is rejected with a clear error.
- `foreign_keys` is on; the 100k CHECK rejects 100,001.
- The widths are 74 (`bar`), 285 (`layer3`), and 357 (`layer3_full`).
- `store init` creates the database, prints the path and version, and is idempotent.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.

## 7. Work breakdown

1. Add the dependency and the store module.
2. Generate the v1 DDL from the research column registry.
3. Add the store tests and the CLI tests.
4. Update the help text, the README, and the fixture.
5. Run the checks, write the plan and the log, commit, push, open the PR, move #67 to Done, merge, and clean up.
6. Continue with #68.

## 8. Verification

| Check | Result |
|---|---|
| Store unit tests | 8 passed |
| CLI store tests | 2 passed |
| `cargo test --workspace` | 763 passed, 0 failed, 3 ignored |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | clean |
| Schema widths | bar 74, layer3 285, layer3_full 357 |

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| The DDL drifts from the contract | It is generated from the registry and the widths are asserted |
| The bundled SQLite increases the build | The ADR accepted `bundled`; the C compiler is present |
| A corrupt file fails at an unexpected point | The test covers the pragma path; the version path maps the same way |
| WAL leaves sidecar files | The tests remove their directory; the sidecars are normal SQLite behavior |
| The schema is large | The store is a wide table by design; #74 measures the 100k ingest |

## 10. Stop conditions

- The bundled build fails without a C compiler.
- A schema statement needs a column the contract does not name.
- The migration cannot roll back on a failed statement.
- A check fails and the fix is outside this scope.

## 11. Delivery

| Item | Value |
|---|---|
| Commit | `ef1c68f` — `feat: add the SQLite store foundation` |
| Branch | `feat/67-store-foundation` |
| Pull request | [#100](https://github.com/quant-rule/quant-rule/pull/100) |
| Board | #67 moved to `Done` when the PR is open |

Merge, release, tag, and deployment remain with the human.
