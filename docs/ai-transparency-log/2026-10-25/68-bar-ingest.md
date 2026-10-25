# AI Transparency Log: Persist Candles, Readings, and Four-Axis Metrics (ST4)

- Work item: [#68](https://github.com/quant-rule/quant-rule/issues/68) — feat: persist candles, readings, and four-axis metrics per bar
- Plan: [`docs/execution-plans/2026-10-25/68-bar-ingest.md`](../../execution-plans/2026-10-25/68-bar-ingest.md)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Date: 2026-10-25 (UTC)

## Entry: the per-bar writer

- **Date and tool:** 2026-10-25 (UTC). opencode (DeepSeek V4.1 Flash Free).
- **Intent and boundary:** Add `store ingest`, which builds the M1/M5/M15/H1 layers in memory and writes one `dataset` row plus one wide `bar` row per Layer 1 row in one transaction. No schema change, no resolution columns, no Layer 3, no pivots, levels, events, legs, consensus, or aggregates.
- **Verification and evidence:**
  - Added `crates/quantrule-shell/src/store/ingest.rs` with `DatasetRecord`, `IngestReport`, validation, identity lookup, the dataset insert, and the 62-column prepared bar insert.
  - Added `StoreError::Conflict`, `InvalidDataset`, and `Write`.
  - Added `store ingest <csv-path> --dataset <name> --data-version <id> [--db <path>]`, its parser, and the interactive and non-interactive dispatch.
  - Reused `command::load_layers`, so the store and the research export share one layer build.
  - 13 writer unit tests pass, including a 62-column round-trip with unique values, null-versus-zero, no-op, conflict, forced rollback, cap, count mismatch, reordered rows, empty input, and the placeholder count.
  - 6 parser tests were added; `command::tests` has 36 passing tests.
  - 4 CLI tests were added: the 333-bar/4-timeframe ingest with null resolutions, the no-op rerun, the conflict, the invalid source leaving no database, and interactive/non-interactive output equality.
  - `cargo test --workspace`: 786 passed, 0 failed, 3 ignored.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
  - Help fixture and README command reference regenerated and updated.
- **Learning:**
  - `rusqlite::params!` cannot borrow temporaries such as `state_code.map(i64::from)`, so the writer builds the parameter values explicitly. The first version returned a `Vec<Value>`, which allocates once per bar; it now returns a fixed `[Value; 62]` array, removing 128,332 allocations on a 100k dataset before #74 measures it.
  - The writer validates the M1 count and the cap before opening the transaction, so a mismatch never creates a dataset row.
  - A no-op still opens `BEGIN IMMEDIATE` and commits an empty transaction; this keeps the identity check serialized against a concurrent writer. The counts reported on a no-op are read from the stored rows, not from the input.
  - `IngestReport` was briefly re-exported from the store module; nothing outside the writer used it, so the re-export was removed and the type stays private to `store::ingest`.
  - The CLI invalid-source test asserts that the database file does not exist, proving the layer build happens before `Store::open`, not merely that the process exits 1.
  - The help fixture is generated from the binary rather than hand-edited, so it cannot drift from `HELP`.

## Final verification

- Writer unit tests: 13 passed.
- Parser tests: 6 added; 36 `command::tests` pass.
- CLI store tests: 6 passed.
- `cargo test --workspace`: 786 passed, 0 failed, 3 ignored.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Status: `Verified`. Commit `7e4b173` is pushed on `feat/68-bar-ingest`, pull request [#101](https://github.com/quant-rule/quant-rule/pull/101) is open, and #68 is `Done` on the board.
