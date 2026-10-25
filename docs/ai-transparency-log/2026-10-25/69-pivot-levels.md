# AI Transparency Log: Persist Pivots and Confirmed Levels (ST5)

- Work item: [#69](https://github.com/quant-rule/quant-rule/issues/69) — feat: persist pivots and confirmed levels
- Plan: [`docs/execution-plans/2026-10-25/69-pivot-levels.md`](../../execution-plans/2026-10-25/69-pivot-levels.md)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Date: 2026-10-25 (UTC)

## Entry: the pivot and level writer

- **Date and tool:** 2026-10-25 (UTC). opencode (DeepSeek V4.1 Flash Free).
- **Intent and boundary:** Extend `store ingest` so the existing transaction also writes every pivot candidate, every confirmed pivot, and every confirmed level. The records are the model's own output; no pivot or level is recomputed in the shell. No schema change, no resolutions, no events, legs, consensus, aggregates, or Layer 3, and no change to the command output.
- **Verification and evidence:**
  - Added `crates/quantrule-analytics/src/layers/records.rs` with `PivotRow`, `LevelRow`, and builders from the layer build's `PivotSeries` and `ConfirmedLevelRegistry`.
  - Added `LayerSet::pivots` and `LayerSet::levels`; both are built before the Layer 3 M1-first swap, so the records keep the configured timeframe order.
  - Extended the writer with `IngestRows`, pre-transaction validation, and eight-parameter `pivot` and `level` inserts inside the existing `BEGIN IMMEDIATE` transaction.
  - Added two analytics tests: model agreement with candidate/confirmed availability and ordering, and per-timeframe records. `cargo test -p quantrule-analytics layers::tests`: 16 passed, 1 ignored.
  - Moved the writer tests to `crates/quantrule-shell/src/store/ingest/tests.rs` and added round-trip, availability, invalid-code, duplicate, and no-op preservation cases. `cargo test -p quantrule-shell store::ingest`: 16 passed.
  - Added CLI record-integrity assertions to the ingest, no-op, and conflict tests. `cargo test -p quantrule-shell --test cli store_ingest`: 4 passed.
  - `cargo test --workspace`: 791 passed, 0 failed, 3 ignored.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
  - `state_m1.csv` stores 354 pivots (94 confirmed, 260 candidates) and 94 levels (M1 68, M5 21, M15 5; H1 none), with zero availability violations and contiguous `1001..` reference codes per timeframe. The command output remains `ingested demo bars=333 timeframes=4`.
  - README updated to describe the stored pivots and confirmed levels.
- **Learning:**
  - The `pivot` primary key is per occurrence, so a candidate that later confirms is stored once with `confirmed_code = 1` and the confirmation bar as its availability bar. Storing two rows would require a schema change that this issue excludes.
  - A candidate is available on its occurrence bar; a confirmed pivot is available at or after it. The analytics test checks both against the model, and the CLI integrity query rejects a confirmed pivot whose availability is not strictly later.
  - The writer validates record keys, codes, prices, bar ranges, and availability before opening the transaction, so an invalid record never creates a dataset row. The rollback test covers a failure that occurs inside the transaction and verifies all four tables stay empty.
  - `PivotKind` has no code method, so the analytics record builder maps `High` to 1 and `Low` to 2. The model's `PivotScale::code()` is reused.
  - `confirmed[index][bar]` triggered `clippy::needless-range-loop`; the lookup now uses `.get(bar)` and `and_then`, which also avoids a potential panic if the vectors ever disagree in length.
  - The writer test module grew past the practical file-size limit, so it moved to `store/ingest/tests.rs`; `ingest.rs` is now 593 lines.
  - No completeness guard was added for a #68-era database that has bars but no pivots. The ADR defines the identity no-op by identity only, and `store verify` in #74 is the completeness check.
  - The execution plan's duplicate criterion says "fails the transaction"; the implementation rejects duplicate pivot and level keys in validation, before the transaction. This follows decision D5 and leaves nothing written, which is the observable outcome the criterion requires.

## Final verification

- Analytics layer tests: 16 passed, 1 ignored.
- Writer unit tests: 16 passed.
- CLI store tests: 4 passed.
- `cargo test --workspace`: 791 passed, 0 failed, 3 ignored.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Status: `Verified`. Commit `d60bfe4` is pushed on `feat/69-pivot-levels`, pull request [#102](https://github.com/quant-rule/quant-rule/pull/102) is open, and #69 is `Done` on the board.
