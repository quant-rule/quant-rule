# Execution Plan: Persist Context, Consensus, and Cross-Timeframe Aggregates (ST7)

- Issue: [#71](https://github.com/quant-rule/quant-rule/issues/71) — feat: persist context, consensus, and cross-timeframe aggregates
- Branch: `feat/71-context-consensus-aggregates`
- Pull request: [#104](https://github.com/quant-rule/quant-rule/pull/104)
- Status: Verified
- Date: 2026-10-25 (UTC)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-25/71-context-consensus-aggregates.md`](../../ai-transparency-log/2026-10-25/71-context-consensus-aggregates.md)

## 1. Intent

Extend `store ingest` so a dataset also carries the per-bar cross-axis
consensus and the 82 cross-timeframe aggregate fields. The consensus is a count,
not a score: missing axes are excluded, and Progression never votes. The
aggregates are the model's own per-M1-bar output; their list fields name the
source timeframes, and no aggregate combines two axes into one number.

Context (`atr_ratio`, `rvol_ratio`, `context_code`) is already stored in `bar`
by #68 and needs no new writer.

## 2. Current state

- `main` is at `dbbe5b4`; #70 is merged.
- Schema v1 already defines `consensus` and the 285-column `layer3` table,
  including the 82 `agg_*` columns. No table or migration is needed.
- `bar` already stores the three context columns; #68 writes them from the
  model's `ContextBar`.
- `LayerSet.layer3` already builds one `Layer3Row` per M1 bar, with the as-of
  `HigherBlock`s and `CrossTimeframeAggregates`. The analytics tests cover its
  causality and the 82-column contract width.
- The model's `TimeframeConsensus::compute` computes the latest-bar consensus
  from a `TimeframeSnapshot`. There is no per-bar constructor.
- `IngestRows` carries bars, pivots, and levels only; `consensus` and `layer3`
  are empty after an ingest, and `layer3_materialized` stays `0`.
- The research contract defines `consensus.csv` per closed bar per timeframe
  and the Layer 3 aggregate columns per M1 bar. The export writes the aggregate
  columns in `layer3.csv` but does not write `consensus.csv`; the store is the
  first persistence path for both.

## 3. Scope

Included:

- A public `TimeframeConsensus::from_bars` constructor in `quantrule-model`,
  taking the timeframe and the three voting bars; `compute` delegates to it.
- A public `ConsensusRow` in `quantrule-analytics::layers`, one per closed bar
  per timeframe, carrying `bar_index` and the model consensus fields.
- `LayerSet.consensus`, built alongside Layer 1 and Layer 2.
- `layer3` aggregate rows: one row per M1 bar with `dataset_id`, `bar_index`,
  and the 82 aggregate columns. The higher-timeframe block columns stay `NULL`;
  `layer3_materialized` stays `0`.
- `IngestRows` carries `&[ConsensusRow]` and `&[Layer3Row]`.
- Pre-transaction validation of consensus keys, counts, weakest-axis codes,
  Layer 3 bar ranges, duplicate or missing keys, ladder shape, list membership,
  and finite aggregate floats.
- Insertion of both tables in the existing `BEGIN IMMEDIATE` transaction.
- List encoding as `;`-separated text with no spaces; an empty per-axis list is
  an empty string, never `NULL`.
- Analytics, model, writer, and CLI tests; README and module-doc updates; this
  plan and the transparency log.

Excluded:

- The three higher-timeframe block columns and `layer3_materialized = 1`
  (#73).
- Event and leg records (#72); confirmed levels are already stored by #69. The
  event *aggregate counts* in the 82 columns are written, but no event record
  row is created.
- `store verify`, the 100k benchmark, and `store list/show` (#74–#75).
- Schema changes or migrations; schema v1 is complete.
- Any change to the data crate or to indicator, axis, event, or aggregation
  logic.
- A cross-axis score or a composite of two axes.
- Backfilling databases written by #68–#70.

## 4. Assumptions and uncertainty

- `LayerSet.layer3` has exactly one row per M1 bar and its aggregates are
  causal. The existing `layer_three_is_causal` and `every_block_is_the_stored_bar`
  tests cover the block side; the aggregate side is covered by the model's
  aggregation tests and the export tests.
- `LayerSet.consensus` will have exactly one row per Layer 1 row. Every closed
  bar has the three voting bars, even when their values are `None`; the counts
  then exclude the missing axes.
- The model's consensus rule is the contract's: Direction, Momentum, and
  Structure only; a value above `+0.1` is bullish, below `-0.1` is bearish, and
  the inclusive `±0.1` band is neutral; a missing axis is excluded from all
  three counts; the weakest axis is the lowest valid size percentile, with the
  lower axis code breaking a tie.
- SQLite stores the contract's list fields as `;`-separated text. The export
  uses the same encoding, so a stored list and an exported list are comparable.
- Existing #68–#70 databases with the same identity remain a no-op and are not
  backfilled. `layer3_materialized = 0` marks them incomplete; #74's
  `store verify` is the completeness check.
- The command output stays `ingested <name> bars=<n> timeframes=<k>` or
  `unchanged <name> bars=<n> timeframes=<k>`. Record counts belong to
  `store show`/`store verify`, not to this command.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Consensus derivation | Duplicate the rule in the shell; expose a model per-bar constructor | Model `from_bars` | One rule; the shell copies |
| D2 | Aggregate placement | Defer all `layer3` work to #73; write the 82 aggregate columns now | Write aggregates now | #71's scope says persist aggregates; #73 adds the blocks |
| D3 | Empty list | `NULL`; empty string | Empty string | The contract says a list field is never null; matches the export |
| D4 | Layer 3 row set | Only rows with a closed higher block; every M1 bar | Every M1 bar | The base is always present; a missing block is null columns, not a missing row |
| D5 | Consensus validation | Recompute counts from `bar` in the writer; validate shape only | Shape only | Cross-field equality belongs to analytics and CLI tests, as with Layer 1/Layer 2 |
| D6 | Layer 3 flag | Set `layer3_materialized = 1`; leave it `0` | Leave `0` | The table has no blocks yet; #73 completes it |

## 6. Acceptance criteria

- The three context columns are written for every `bar` row exactly as #68
  writes them: the model's value or `NULL`, never zero. No regression.
- `consensus` has one row per closed bar per timeframe. The three counts are in
  `0..=3`, their sum is at most three, and a missing axis is not counted as
  neutral. `weakest_directional_axis_code` is `1..=3` or `NULL`, and a tie is
  broken toward the lower code.
- `layer3` has one row per M1 bar with all 82 aggregate columns written. A
  missing measurement is `NULL`, never zero. `agg_ladder` is non-empty and
  names the source timeframes fastest first; every per-axis `timeframes` list
  is a subset of the ladder and names its sources. List fields are never
  `NULL`; an empty list is an empty string. Until #73, a higher timeframe named
  by the ladder has `NULL` block columns; `layer3_materialized = 0` marks the
  row partial.
- The structure histogram and both progression stage histograms are counts, and
  no aggregate averages two axes.
- Every `layer3` block column is `NULL` and `layer3_materialized` is `0`.
- A same-identity re-ingest is still a no-op, and the command output is
  unchanged.
- A duplicate consensus key, a missing consensus row, an out-of-range count, an
  invalid weakest code, a duplicate Layer 3 bar, a missing Layer 3 bar, an
  out-of-range Layer 3 bar, an empty ladder, a non-member timeframe, an
  unsorted or duplicate list, or a non-finite aggregate float is rejected
  before the transaction; no dataset, bar, consensus, or Layer 3 row is
  written.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --workspace`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
  are clean.

## 7. Work breakdown

1. **Model consensus constructor.** Add `TimeframeConsensus::from_bars` in
   `crates/quantrule-model/src/state.rs`; make `compute` delegate to it. Add a
   test that `compute` equals `from_bars` over the same bars; the existing
   count, inclusive-boundary, missing-axis, and lowest-code tie-break tests
   continue to cover the rule. Affected: `state.rs`, `state/tests.rs`.
2. **Analytics consensus rows.** Add `ConsensusRow` and its builder in a new
   `crates/quantrule-analytics/src/layers/consensus.rs`; add
   `LayerSet.consensus`; build one row per closed bar in `build_layers`. Add
   tests for row count, bar alignment, and agreement with
   `TimeframeConsensus::compute` at the latest bar. Affected:
   `layers/mod.rs`, `layers/consensus.rs`, `layers/series.rs`,
   `layers/tests.rs`, `summary.rs` (the test-only `LayerSet` literal).
3. **Store validation.** Extend `IngestRows` with `consensus` and `layer3`;
   validate consensus keys and codes, Layer 3 bar ranges and duplicate or
   missing keys, the ladder and per-axis list membership, the list encoding,
   and finite aggregate floats before the transaction opens. Affected:
   `store/ingest.rs`.
4. **Store inserts.** Add `CONSENSUS_INSERT` and `LAYER3_AGGREGATE_INSERT`; bind
   the consensus fields and the 82 aggregate values, encoding lists as
   `;`-separated text. Keep the block columns out of the insert so they default
   to `NULL`; keep `layer3_materialized` at `0`. Affected: `store/ingest.rs`.
5. **Call site and docs.** Pass `layers.consensus` and `layers.layer3` from
   `run_store_ingest`; update the `store/ingest.rs` module doc and the README's
   `store ingest` description. Affected: `command.rs`, `README.md`.
6. **Tests.** Add the consensus and aggregate writer tests in a new
   `crates/quantrule-shell/src/store/ingest/aggregate_tests.rs` module, so the
   existing 1,066-line `tests.rs` does not grow; its only change is the
   `IngestRows` signature. Cover round-trips, list encoding, nulls, duplicate
   and missing keys, invalid counts, invalid ladders, rollback, and an
   assertion that the aggregate insert's column list matches the DDL's `agg_*`
   order. Add a new `crates/quantrule-shell/tests/store_aggregates.rs`
   integration test, so the 1,243-line `cli.rs` does not grow: ingest
   `state_m1.csv`, then assert consensus counts against the stored axis values,
   weakest-axis validity, one Layer 3 row per M1 bar, aggregate ladder
   membership, non-null list fields, and unchanged command output. Affected:
   `store/ingest.rs`, `store/ingest/tests.rs`,
   `store/ingest/aggregate_tests.rs`, `tests/store_aggregates.rs`.
7. **Evidence and handoff.** Run the checks, record evidence here and in the
   transparency log, mark this plan `Verified`, commit, push, open the PR, and
   move #71 to `Done` on the board.

## 8. Verification

| Check | Expected evidence |
|---|---|
| Model consensus tests | Counts, inclusive boundary, missing axes excluded, lower-code tie-break |
| Analytics layer tests | One consensus row per Layer 1 row; bar alignment; latest-bar agreement |
| Writer unit tests | Consensus and 82-column aggregate round-trip; list encoding; nulls; invalid input rejected; rollback |
| `store_aggregates` test | `state_m1.csv` has 333 consensus rows and 260 Layer 3 rows; counts match the stored axes; on an M1-only ladder the Direction and Momentum means equal the base values; output unchanged |
| SQL audit | Zero consensus count, weakest-code, ladder, list, or block-null violations |
| `cargo test --workspace` | All tests pass; no test is skipped or weakened |
| `cargo fmt --all -- --check` | Clean |
| `cargo clippy --all-targets -- -D warnings` | Clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean |

## 9. Risks and recovery

- `layer3` is partial until #73: aggregates are present and the blocks are
  `NULL`. `layer3_materialized = 0` records that fact, and #74's `store verify`
  detects it. No existing database is silently rewritten.
- The 82 aggregate columns can drift from the DDL or from the export order.
  The schema-width test, a full round-trip, and a column-by-column CLI audit
  catch a drift.
- The per-bar consensus can drift from the model's latest-bar consensus. The
  `from_bars` refactor keeps one rule, and the analytics test compares the two
  at the latest bar.
- A list could be stored out of order or with a duplicate. Validation checks
  that each list is strictly ascending and that every value is a known
  timeframe, and checks that `agg_ladder` is non-empty before the transaction.
- The `IngestRows` signature changes are compile-time breaks. Only the shell
  crate uses it; the call site and tests are updated in the same change.
- No schema change is made, so reverting the commit leaves existing databases
  readable; no migration or rollback script is required.

## 10. Stop conditions

Stop and ask the human if:

- the consensus contract requires a different rule or a schema change;
- the aggregate columns cannot be written without the higher blocks, for
  example because a `NOT NULL` constraint is discovered;
- the list encoding needs a representation other than `;`-separated text;
- the 100k cap or the Layer 3 row count cannot be satisfied without changing
  the schema; or
- any check fails in a way that would require weakening a test.

## 11. Verification evidence

Verified on 2026-10-25 (UTC) in the task worktree.

| Check | Command | Observed result |
|---|---|---|
| Model consensus | `cargo test -p quantrule-model state::tests` | 7 passed, 0 failed |
| Analytics layers | `cargo test -p quantrule-analytics layers::tests` | 18 passed, 1 ignored, 0 failed |
| Writer unit tests | `cargo test -p quantrule-shell --bin quantrule store::ingest` | 25 passed, 0 failed |
| Binary aggregate audit | `cargo test -p quantrule-shell --test store_aggregates` | 1 passed, 0 failed |
| Workspace | `cargo test --workspace` | 804 passed, 0 failed, 3 ignored |
| Format | `cargo fmt --all -- --check` | Clean |
| Lints | `cargo clippy --all-targets -- -D warnings` | Clean |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean |

Observed behavior:

- `state_m1.csv` stores 333 `bar` rows across four timeframes, 333
  `consensus` rows, and 260 `layer3` rows.
- Every `consensus` count equals the stored Direction/Momentum/Structure values
  under the contract thresholds; every weakest code names an axis with a
  percentile; every count is in `0..=3` and sums to at most three.
- Every `layer3` row joins one M1 base; `agg_ladder` starts at 60 and is
  strictly ascending; every per-axis list is a strictly ascending subset of
  the ladder; empty lists are `''`; all block columns are `NULL`;
  `layer3_materialized = 0`.
- On M1-only rows, `agg_direction_mean` equals the stored Direction value and
  `agg_momentum_force_mean` equals the stored force.
- A second ingest prints `unchanged demo bars=333 timeframes=4` and does not
  duplicate rows.

Implementation notes and deviations:

- `tests.rs` reached 1,238 lines when the shared consensus and Layer 3
  fixtures were added. The shared helpers moved to
  `store/ingest/fixtures.rs`; `tests.rs` is now 764 lines, `fixtures.rs` 490,
  and the writer modules are `ingest.rs` 306, `sql.rs` 339, and `validate.rs`
  515. This is a test-organization change only; no behavior changed.
- The rollback test now forces a SQLite failure with a `BEFORE INSERT ON level`
  trigger. Duplicate consensus and Layer 3 keys are rejected by
  pre-transaction validation, so they no longer reach SQLite.
- No schema change and no migration were made. Existing #68–#70 databases with
  the same identity remain a no-op; `layer3_materialized = 0` records their
  incompleteness for #74.
