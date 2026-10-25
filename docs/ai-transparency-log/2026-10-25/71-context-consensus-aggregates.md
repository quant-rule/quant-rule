# AI Transparency Log: Persist Context, Consensus, and Cross-Timeframe Aggregates (ST7)

- Work item: [#71](https://github.com/quant-rule/quant-rule/issues/71) — feat: persist context, consensus, and cross-timeframe aggregates
- Plan: [`docs/execution-plans/2026-10-25/71-context-consensus-aggregates.md`](../../execution-plans/2026-10-25/71-context-consensus-aggregates.md)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Date: 2026-10-25 (UTC)

## Entry: the consensus and aggregate writer

- **Date and tool:** 2026-10-25 (UTC). opencode (DeepSeek V4.1 Flash Free).
- **Intent and boundary:** Extend `store ingest` so a dataset also carries one
  `consensus` row per closed bar per timeframe and one `layer3` row per M1 bar
  with the 82 cross-timeframe aggregate columns. The writer copies the model's
  and analytics engine's values; it does not recompute a count, a resolution,
  or an aggregate. No schema change, no higher-timeframe block columns, no
  event or leg records, no `store list/show/verify`, and no change to the
  command output.
- **Implementation:**
  - Added `TimeframeConsensus::from_bars` in
    `crates/quantrule-model/src/state.rs`; `TimeframeConsensus::compute` now
    delegates to it. This keeps one consensus rule for the latest snapshot and
    for every closed bar.
  - Added `ConsensusRow` in
    `crates/quantrule-analytics/src/layers/consensus.rs` and
    `LayerSet.consensus`. `TimeframeSeries::consensus_at` calls the model
    constructor; `build_layers` emits one row per closed bar in Layer 1 order.
  - Added `IngestRows.consensus` and `IngestRows.layer3`, plus
    `CONSENSUS_INSERT` and `LAYER3_AGGREGATE_INSERT`. The Layer 3 insert names
    only `dataset_id`, `bar_index`, and the 82 `agg_*` columns, so every block
    column stays `NULL` and `layer3_materialized` stays `0`.
  - Encoded list fields as `;`-separated text with no spaces; an empty list is
    `''`, never `NULL`.
  - Added pre-transaction validation for consensus coverage, duplicate and
    missing keys, count range and sum, weakest-axis code, Layer 3 coverage,
    M1 base, duplicate and missing bars, ladder shape, per-axis list
    membership and order, and finite aggregate floats.
  - Split the writer into `store/ingest/sql.rs` and `store/ingest/validate.rs`
    so no source file exceeds 1,000 lines.
- **Verification and evidence:**
  - `cargo test -p quantrule-model state::tests`: 7 passed, 0 failed.
  - `cargo test -p quantrule-analytics layers::tests`: 18 passed, 1 ignored,
    0 failed.
  - `cargo test -p quantrule-shell --bin quantrule store::ingest`: 25 passed,
    0 failed. The 6 new tests cover the insert/DDL column order, the consensus
    and 82-column round-trip, list encoding, null/zero preservation, invalid
    consensus, and invalid Layer 3.
  - `cargo test -p quantrule-shell --test store_aggregates`: 1 passed,
    0 failed. Over `state_m1.csv`, the database holds 333 `bar` rows, 333
    `consensus` rows, and 260 `layer3` rows; every consensus count matches the
    stored axes; every weakest code names an axis with a percentile; every
    `layer3` row joins one M1 base; the ladder and per-axis lists are sorted
    and contain only known timeframes; all block columns are `NULL`;
    `layer3_materialized = 0`; and a second ingest prints `unchanged demo
    bars=333 timeframes=4`.
  - `cargo test --workspace`: 804 passed, 0 failed, 3 ignored.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
  - README updated to describe the consensus row, the aggregate row, the list
    encoding, and the partial Layer 3 state.
- **Learning:**
  - The approved decision was to expose a per-bar model constructor rather
    than duplicate the consensus rule in the shell. The alternative — having
    the writer recompute the counts from the stored axes — was rejected
    because it would put a model rule in the shell and make the writer a
    second source of truth.
  - The approved decision was to leave `layer3_materialized = 0` until #73.
    Setting it to `1` now was rejected because the block columns are still
    null; #74's `store verify` uses the flag to detect an incomplete dataset.
  - The plan said `tests.rs` would change only at the `IngestRows` call sites.
    Adding shared consensus and Layer 3 fixtures pushed that file to 1,238
    lines, so the helpers moved to `store/ingest/fixtures.rs`. This is a
    test-organization deviation, not a behavior change; the plan evidence
    records the final line counts.
  - The old rollback test used a duplicate primary key to force a SQLite
    failure. Pre-transaction validation now rejects a duplicate consensus or
    Layer 3 key before the transaction, so the test was corrected to use a
    `BEFORE INSERT ON level` trigger. The test still proves that a SQLite
    failure rolls back the dataset, `bar`, `consensus`, `layer3`, `pivot`, and
    `level` rows.
  - The synthetic writer fixtures build shape-valid consensus and Layer 3 rows
    from each Layer 2 bar. The tests assert on the stored rows, so the fixture
    values are not treated as evidence that the model produces those values;
    the binary test over `state_m1.csv` is the cross-check against the model
    and analytics output.
  - A #68–#70 database with the same identity remains a no-op and is not
    backfilled. This follows the approved assumption; `store verify` in #74 is
    the completeness check.

## Final verification

- Model consensus tests: 7 passed.
- Analytics layer tests: 18 passed, 1 ignored.
- Writer unit tests: 25 passed.
- Binary aggregate audit: 1 passed.
- `cargo test --workspace`: 804 passed, 0 failed, 3 ignored.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Pull request: opened after this commit.
- Status: implementation complete and locally verified; pull request open for
  human review.
