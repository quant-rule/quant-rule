# AI Transparency Log: The Store Foundation and Migrations (ST3)

- Work item: [#67](https://github.com/quant-rule/quant-rule/issues/67) — feat: add the SQLite store foundation and migrations
- Plan: [`docs/execution-plans/2026-10-24/67-store-foundation.md`](../../execution-plans/2026-10-24/67-store-foundation.md)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Date: 2026-10-24 (UTC)

## Entry: the foundation

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Add the SQLite dependency, the shell-owned store module, schema version 1, the migration, the PRAGMAs, the errors, and `store init`. No writers or queries.
- **Verification and evidence:**
  - Added `rusqlite` with `bundled` to the shell.
  - Added `crates/quantrule-shell/src/store/` with the connection, the generated v1 DDL, the migration, the version check, and the error type.
  - Added `store init [--db <path>]` and the help/README entries.
  - 8 store unit tests and 2 CLI tests pass.
  - `cargo test --workspace`: 763 passed, 0 failed, 3 ignored.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
  - The schema widths are asserted: `bar` 74 columns, `layer3` 285, `layer3_full` 357.
- **Learning:**
  - The DDL was generated from the analytics column registry instead of typed by hand. The `bar` table has 74 columns (3 keys + 71), `layer3` has 285 (2 keys + 201 block + 82 aggregate), and the view has 357 (the `dataset_id` plus the 356 research columns). The test pins those widths.
  - The first corrupt-file test failed because `configure` runs before `read_version`, so the garbage file failed at the PRAGMA, not the version query. The error mapping now checks both stages for "not a database"/"malformed".
  - The table-order assertion initially put `leg` before `layer3`; SQLite sorts `layer3` first because `a` precedes `e`. The expected list was corrected; the store's schema was fine.
  - `open_in_memory` and `integrity_check` have no production caller yet. They are `#[cfg(test)]` rather than `#[allow(dead_code)]`; #68 and #74 will lift that when the writers and verify need them.
  - The ADR's consequences said the `layer3` table had 301 columns. The actual count is 283 data columns plus 2 keys = 285; the ADR is corrected in this PR. The 301 came from adding the two keys twice in the earlier count.
  - The bundled SQLite compiled without a system library, as the ADR assumed. The version is 0.32, pinned in the shell manifest.
  - The 100k CHECK is enforced by SQLite as well as by the full-series path; a direct insert of 100,001 is rejected in the test.

## Final verification

- Store unit tests: 8 passed.
- CLI store tests: 2 passed.
- `cargo test --workspace`: 763 passed, 0 failed, 3 ignored.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Status: `Verified`. Commit, push, pull request, and board transition follow the delivery step.
