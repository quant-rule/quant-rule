# AI Transparency Log: Verify the Stored Dataset and the 100k Cap

- Work item: [#74](https://github.com/quant-rule/quant-rule/issues/74) — test: verify the stored dataset and the 100k cap
- Plan: [`docs/execution-plans/2026-10-25/74-store-verify.md`](../../execution-plans/2026-10-25/74-store-verify.md)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Date: 2026-10-25 (UTC)

## Entry: read-only verification and the 100k gate

- **Date and tool:** 2026-10-25 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Add `store verify`, which rebuilds the source layers
  and compares the dataset identity, the foreign-key graph, and every stored
  table against the fresh values. Add an ignored 100k M1 integration test that
  records ingest, model, verify, query, and export timings. Verification is
  read-only; it never creates, migrates, repairs, or writes a database.
- **Implementation:**
  - Exposed the writer's expected parameter rows and a raw un-limited table
    reader; reused the query layer's cell conversion.
  - Added `store/verify.rs`: dataset identity and contract comparison, the
    `layer3_materialized` guard, `PRAGMA foreign_key_check`, event `id` removal,
    row-count checks, and exact per-column value comparison.
  - Added `store verify <csv-path> --dataset <name> --data-version <id>
    [--db <path>]` in `command/store_verify.rs`.
  - Added the `verified <name> bars=<n> tables=7` summary; a mismatch reports
    the first table, row, column, stored value, and expected value.
  - Added `tests/store_verify.rs` and the ignored `tests/store_100k.rs`.
  - Updated README, help, and the ADR command table.
- **Verification and evidence:**
  - `cargo test -p quantrule-shell --bin quantrule command::store_verify`:
    2 passed.
  - `cargo test -p quantrule-shell --test store_verify`: 8 passed. The tests
    cover success, no-op re-ingest, changed value, wrong data version,
    incomplete Layer 3, missing dataset, missing database, orphan row, and
    interactive equality.
  - `cargo test --workspace`: 871 passed, 0 failed, 4 ignored.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
  - The 100k test passed explicitly in the debug build. Observed timings:
    ingest 58.138 s, model 1.293 s, verify 75.945 s, `store list` 0.300 s,
    1,000-row `db series` 0.014 s, `db layer3` 0.008 s, export 54.097 s.
    It stored 128,330 ladder bars and 100,000 Layer 3 rows in a 229,732,352-byte
    database and wrote 9 export files.
- **Learning:**
  - The first 100k source used a start timestamp 20 seconds off the minute
    boundary; the data engine rejected it. The corrected start is minute-aligned.
    This is a useful edge case: a synthetic 100k fixture must satisfy the same
    alignment rules as a real source.
  - Verification reuses the writer's parameter order rather than re-deriving
    expected values. That keeps the writer and verifier from disagreeing about a
    column, at the cost of relying on the existing insert/DDL tests.
  - The event `id` is the only column excluded. It is an autoincrement row
    identifier, not a record field; every other event column is compared.
  - `foreign_key_check` is run before the table comparisons so an orphan is
    reported as a structural failure rather than as a confusing row mismatch.
  - A flag of `0` is an explicit incomplete error. Treating an old #68–#72
    dataset as verified would claim more than the stored evidence supports.
  - The 100k timings are debug-build observations. They are reported, not
    asserted; the test does not depend on wall-clock thresholds.
  - Role passes were sequential in one session, not independent agents. The
    human owns review and merge.

## Final verification

- Verify command parser tests: 2 passed.
- Verify binary tests: 8 passed.
- 100k integration test: 1 passed (explicit ignored run).
- `cargo test --workspace`: 871 passed, 0 failed, 4 ignored.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Pull request: [#110](https://github.com/quant-rule/quant-rule/pull/110).
- Status: implementation complete and locally verified; pull request open for
  human review.
