# Execution Plan: Persist Axis Resolutions and Layer 2 (ST6)

- Issue: [#70](https://github.com/quant-rule/quant-rule/issues/70) — feat: persist axis resolutions and layer 2
- Branch: `feat/70-axis-resolutions`
- Pull request: [#103](https://github.com/quant-rule/quant-rule/pull/103)
- Status: Verified
- Date: 2026-10-25 (UTC)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-25/70-axis-resolutions.md`](../../ai-transparency-log/2026-10-25/70-axis-resolutions.md)

## 1. Intent

Extend `store ingest` so every stored ladder bar also carries the four axes'
continuous and ordinal resolutions and their unavailable reasons. The stored
values are the analytics engine's own Layer 2 output, not a shell
recomputation, and the resulting `bar` table is queryable as Layer 2: Layer 1
plus the 12 resolution columns.

## 2. Current state

- `main` is at `1a0a4d2`; #69 is merged.
- Schema v1 already defines all 74 `bar` columns, including the 12 resolution
  columns after `context_code`. No table or migration is needed.
- `quantrule-analytics` already publishes `LayerSet.layer2`, one `Layer2Row`
  per Layer 1 row, carrying the Layer 1 fields plus `AxisResolutions`.
- The resolution mapping lives in `resolution.rs`: continuous values are in
  `[-1, +1]`, ordinals are equal-width buckets in `[0, 9]`, and Progression is
  clamped from raw `progress` while Layer 1 keeps the raw value.
- `store ingest` currently passes `layers.layer1` to `IngestRows`; `BAR_INSERT`
  binds 62 columns and leaves the 12 resolution columns `NULL`.
- The writer tests assert those 12 columns are `NULL`, and the CLI test asserts
  the same on the `state_m1.csv` ingest.

## 3. Scope

Included:

- A public `Layer2Row::from_layer1` constructor in `quantrule-analytics::layers`
  that applies the existing private `resolutions()` mapping. `build_layers`
  uses it, so there is one derivation path.
- `IngestRows` carries `&[Layer2Row]` instead of `&[Layer1Row]`.
- `bar_params` binds all 74 columns and `BAR_INSERT` names and binds them.
- Pre-transaction resolution validation: finite continuous value in
  `[-1, +1]`; ordinal in `[0, 9]`; ordinal present exactly when the continuous
  value is present; reason present exactly when the continuous value is absent;
  reason in `1..=6`; ordinal equals `ordinal_bucket(continuous)`.
- `run_store_ingest` passes `layers.layer2`.
- Analytics, writer, and CLI tests for the constructor, the exact boundary
  mapping, progression clamping, null/reason rules, invalid input, round-trip,
  no-op, and rollback.
- README and module-doc updates describing the now-populated resolution
  columns.
- This plan and the transparency log.

Excluded:

- Layer 3, events, legs, consensus, and aggregates (#71–#73).
- `store list`, `store show`, and `store verify` (#74–#75).
- Schema changes or migrations; schema v1 is complete.
- Any change to the data or model crates.
- Backfilling databases written by #68/#69.

## 4. Assumptions and uncertainty

- `LayerSet.layer2` has the same row set and row order as `LayerSet.layer1`.
  The existing `layer_two_derives_from_layer_one` test already covers this;
  the writer relies on it rather than zipping two independently built slices.
- The model publishes an unavailable reason whenever an axis publishes no
  value, as the research contract §6.3 requires. If real data violates that
  rule, the ingest fails rather than storing an unexplained null.
- The resolution columns are a projection of the model output, not a new
  measurement. The store validates and copies them; it does not recompute an
  axis value.
- The command output stays `ingested <name> bars=<n> timeframes=<k>` or
  `unchanged <name> bars=<n> timeframes=<k>`. Resolution counts belong to
  `store show`/`store verify`, not to this command.
- An existing dataset with the same identity remains a no-op even if it was
  written before resolutions were populated. The ADR defines the no-op by
  identity; #74's `store verify` is the completeness check.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Resolution source | Recompute in the shell; use `LayerSet.layer2` | Use `layer2` | One computation path, no drift |
| D2 | Writer input | Parallel bar/resolution slices; `&[Layer2Row]` | `&[Layer2Row]` | One row carries both halves; a zip cannot misalign |
| D3 | Constructor | Duplicate the mapping in the shell; expose `from_layer1` | `from_layer1` | Keeps the private mapping the only derivation |
| D4 | Validation | Rely on SQLite; validate before the transaction | Validate | A malformed resolution never creates a dataset |
| D5 | Ordinal check | Range only; range plus exact bucket | Range plus bucket | Catches a mapping regression before it is stored |
| D6 | Output | Add resolution counts; keep the #68 line | Keep | Existing command contract stays stable |
| D7 | Existing rows | Backfill on re-ingest; leave untouched | Leave untouched | Same-identity no-op is the ADR contract; #74 verifies |

## 6. Acceptance criteria

- Every `bar` row stores the 12 resolution columns from `LayerSet.layer2`.
- The continuous value is `NULL` exactly when the axis is unavailable, with
  its reason code; a measured zero is stored as `0`, bucket 5, reason `NULL`.
- The ordinal is in `[0, 9]` and equals the contract bucket of the stored
  continuous value.
- Boundary mapping is exact in stored rows: `-1.0 -> 0`, `-0.8 -> 1`,
  `0.0 -> 5`, `+0.8 -> 9`, `+1.0 -> 9`.
- Progression stores raw `progress` in Layer 1 and the clamped value in
  `progression_continuous`; `1.5` resolves to `1.0`/bucket 9 and `-0.25`
  resolves to `-0.25`/bucket 3.
- No cross-axis score or composite resolution is computed or stored.
- A non-finite, out-of-range, mis-bucketed, or null/reason-inconsistent
  resolution is rejected before the transaction; no dataset or bar row is
  written.
- The 333-bar/4-timeframe ingest output is unchanged, and the same-identity
  re-ingest is still a no-op.
- `fmt`, `clippy -D warnings`, `cargo test --workspace`, and `doc -D warnings`
  are clean.

## 7. Work breakdown

1. Add `Layer2Row::from_layer1` in `layers/mod.rs` with rustdoc; use it in
   `build_layers`; extend the analytics tests to prove the constructor matches
   the built Layer 2 rows and preserves the boundary and clamp values.
2. Change `IngestRows` to hold `&[Layer2Row]`; update bar counting, M1 count
   validation, and timeframe distinctness to read `row.layer1`.
3. Add per-axis resolution validation to `validate`, using
   `ordinal_bucket` and the contract's reason range; reject before the
   transaction opens.
4. Extend `bar_params` to 74 values and `BAR_INSERT` to 74 columns/placeholders,
   appending the 12 resolution columns after `context_code`.
5. Pass `layers.layer2` from `run_store_ingest`.
6. Update writer tests: 74-column `COLUMNS`, a full round-trip with boundary
   values, null/reason and zero cases, invalid-resolution cases, and the
   existing no-op/rollback cases. Update the CLI test to assert stored
   resolutions cover every ladder bar, satisfy the range/null/bucket rules, and
   keep the Progression clamp, replacing the all-`NULL` assertion.
7. Update the README and the `store/ingest.rs` module doc; run the checks,
   record evidence, write the transparency log, mark this plan `Verified`,
   commit, push, open the PR, and move #70 to Done.

## 8. Verification

| Check | Expected evidence |
|---|---|
| Analytics tests | `from_layer1` matches `layer2`; boundaries and progression clamp exact |
| Writer unit tests | 74-column round-trip; boundary values; Progression clamp; null/reason rules; invalid resolutions rejected; rollback |
| CLI test | Every `state_m1.csv` bar has coherent resolutions; at least one value per axis; no clamp violation |
| SQL audit | Zero range, reason, or bucket violations in the ingested fixture |
| `cargo test --workspace` | All tests pass; no test is skipped or weakened |
| `cargo fmt --all -- --check` | Clean |
| `cargo clippy --all-targets -- -D warnings` | Clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean |

## 9. Risks and recovery

- A malformed resolution could be stored if validation misses a case. The
  range, null/reason, and exact-bucket checks plus the round-trip and CLI tests
  cover the contract's failure modes.
- Changing `IngestRows` is a compile-time break for every caller. Only the
  shell crate uses it, and the call site is updated in the same change.
- A 74-parameter insert can drift from its column list. The placeholder-count
  test, the 74-column round-trip, and the schema-width test catch that.
- Existing #68/#69 databases are not backfilled. This is documented and is
  detected by `store verify` in #74.
- No schema change is made, so reverting the commit leaves existing databases
  readable; no migration or rollback script is required.

## 10. Stop conditions

Stop and ask the human if:

- the resolution contract requires a schema change or a new column;
- `LayerSet.layer2` cannot be aligned with `layer1` without reordering;
- real data produces a null continuous value with no reason, which would mean
  the model or the contract needs a decision rather than a writer workaround;
- the command output must expose resolution counts; or
- any check fails in a way that would require weakening a test.

## 11. Verification evidence

Observed on 2026-10-25 (UTC) in the task worktree:

| Check | Result |
|---|---|
| `cargo test -p quantrule-analytics layers::tests` | 17 passed, 0 failed, 1 ignored |
| `cargo test -p quantrule-shell store::ingest` | 19 passed, 0 failed |
| `cargo test -p quantrule-shell --test cli store_ingest` | 4 passed, 0 failed |
| `cargo test --workspace` | 795 passed, 0 failed, 3 ignored |
| `cargo fmt --all -- --check` | Clean |
| `cargo clippy --all-targets -- -D warnings` | Clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean |

The `state_m1.csv` ingest stores 333 bars across 4 timeframes and still prints
`ingested demo bars=333 timeframes=4`. The CLI audit finds no range, null/reason,
or bucket violations on any stored resolution, at least one measured value per
axis, and no Progression clamp violation.

The research contract's §6.3 example said `-0.25` resolves to bucket 4. The
boundary table and `ordinal_bucket` both classify it as bucket 3; the example,
this plan, and the tests were corrected to bucket 3. No schema change was
needed.
