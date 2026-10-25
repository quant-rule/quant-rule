# AI Transparency Log: Persist Axis Resolutions and Layer 2 (ST6)

- Work item: [#70](https://github.com/quant-rule/quant-rule/issues/70) — feat: persist axis resolutions and layer 2
- Plan: [`docs/execution-plans/2026-10-25/70-axis-resolutions.md`](../../execution-plans/2026-10-25/70-axis-resolutions.md)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Date: 2026-10-25 (UTC)

## Entry: the axis-resolution writer

- **Date and tool:** 2026-10-25 (UTC). opencode (DeepSeek V4.1 Flash Free).
- **Intent and boundary:** Extend `store ingest` so every `bar` row carries the four axes' continuous values, ordinal buckets, and unavailable reasons from `LayerSet.layer2`. The writer copies the analytics engine's output; it does not recompute an axis value. No schema change, no Layer 3, events, legs, consensus, aggregates, or `store list/show/verify`, and no change to the command output.
- **Verification and evidence:**
  - Added `Layer2Row::from_layer1` in `crates/quantrule-analytics/src/layers/mod.rs`; `build_layers` now uses it, so the private `resolutions()` mapping remains the only derivation path.
  - Changed `IngestRows` to carry `&[Layer2Row]`; `run_store_ingest` now passes `layers.layer2` instead of `layers.layer1`.
  - Extended `bar_params` and `BAR_INSERT` to all 74 `bar` columns, appending the 12 resolution columns after `context_code`.
  - Added pre-transaction resolution validation for finite continuous values in `[-1, +1]`, ordinals in `[0, 9]`, reason presence/range, and exact equality with `ordinal_bucket(continuous)`.
  - Added analytics tests for constructor agreement, the five contract boundaries, and Progression clamping. `cargo test -p quantrule-analytics layers::tests`: 17 passed, 1 ignored.
  - Extended the writer tests to 74 columns and added boundary, null/reason, zero, progression-clamp, and invalid-resolution cases. `cargo test -p quantrule-shell store::ingest`: 19 passed.
  - Replaced the CLI all-`NULL` assertion with a resolution audit covering every axis: range, null/reason coherence, exact bucket, at least one measured value per axis, and the Progression clamp relation. `cargo test -p quantrule-shell --test cli store_ingest`: 4 passed.
  - `cargo test --workspace`: 795 passed, 0 failed, 3 ignored.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
  - `state_m1.csv` stores 333 bars across 4 timeframes, with no resolution range, reason, or bucket violations and no Progression clamp violation. The command output remains `ingested demo bars=333 timeframes=4`.
  - README updated to describe the stored resolution columns.
- **Learning:**
  - The research contract's §6.3 example said `-0.25` resolves to bucket 4. The boundary table, the bucket formula, and `ordinal_bucket` all classify `-0.25` in `[-0.40, -0.20)`, which is bucket 3. The example was corrected to bucket 3, and the plan and tests were corrected with it. This was a documentation defect, not a behavior change.
  - The plan considered range-only ordinal validation. The implementation also checks the exact bucket, which catches a mapping regression before it is stored; the writer still copies the value rather than deriving it.
  - The synthetic `sparse_row` fixture had null continuous values with no reason. The contract requires a reason whenever a value is missing, so the fixture now carries reason 1 for the three unavailable axes; a separate invalid-input case proves a null without a reason is rejected.
  - Cross-field equality between the Layer 1 raw values and the Layer 2 continuous values is not re-derived in the writer. That would duplicate the analytics mapping; the analytics constructor tests and the CLI progression audit cover the relationship instead.
  - A #68/#69-era database with the same identity remains a no-op and is not backfilled. This follows the ADR's identity rule; `store verify` in #74 is the completeness check.

## Final verification

- Analytics layer tests: 17 passed, 1 ignored.
- Writer unit tests: 19 passed.
- CLI store tests: 4 passed.
- `cargo test --workspace`: 795 passed, 0 failed, 3 ignored.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Implementation commit: `4b51caf` (`feat: persist axis resolutions and layer 2`).
- Pull request: [#103](https://github.com/quant-rule/quant-rule/pull/103).
- Status: implementation complete and locally verified; pull request open for human review.
