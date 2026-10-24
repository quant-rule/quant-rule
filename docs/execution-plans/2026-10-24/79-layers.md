# Execution Plan: The Three Research Layers (AN4)

- Issue: [#79](https://github.com/quant-rule/quant-rule/issues/79) — feat: build layers 1, 2, and 3
- Branch: `feat/79-layers`
- Pull request: [#94](https://github.com/quant-rule/quant-rule/pull/94)
- Status: Verified
- Date: 2026-10-24 (UTC)
- Contract: [`docs/analytics/2026-10-24/research-output.md`](../../analytics/2026-10-24/research-output.md) §5–§7
- Transparency log: [`docs/ai-transparency-log/2026-10-24/79-layers.md`](../../ai-transparency-log/2026-10-24/79-layers.md)

## 1. Intent

Build the three cumulative research layers from the model's per-bar output: Layer 1 (candle, readings, metrics), Layer 2 (Layer 1 plus the four axis resolutions), and Layer 3 (the M1 base plus the latest closed higher-timeframe blocks and the cross-timeframe aggregates). The layers are the input to the export (#80) and the store materialization (#73).

## 2. Current state

- `main` is at `4a1cd62`; the analytics crate has `resolution` (#77) and `alignment` (#78).
- The model publishes every per-bar series publicly (`get(bar)`), and it publishes `CrossTimeframeAggregates::compute(&[TimeframeSnapshot])`.
- The model's `TimeframeSnapshot` is a latest-bar record; it carries the full event list and the four latest swings but no per-bar builder.
- No layer types or builder exist.

## 3. Scope

Included:

- `layers::metrics`: the typed per-bar record (`ReadingsSnapshot`, five metric groups, `Metrics`).
- `layers::series`: the private per-timeframe model series and the per-bar `TimeframeSnapshot` assembly.
- `layers`: `Layer1Row`, `Layer2Row`, `HigherBlock`, `Layer3Row`, `LayerSet`, `LayerConfig`, `LayerError`, `build_layers`, and the contract column constants.
- Tests for row and column counts, the L1→L2 derivation, the L3 blocks and aggregates, causality, determinism, the M1 requirement, edge cases, and the errors.
- An ignored 100k release benchmark.
- A clarification of `agg_ladder` and the `*_timeframe_count` fields in the research contract: they name the timeframes aggregated in the row, not the configured ladder, because a higher timeframe with no closed bar yet is absent.

Excluded:

- CSV writing, the manifest, and the data dictionary (#80).
- The store tables and their ingest (#65–#75).
- Events, legs, levels, and consensus exports (#72, #80).
- Any model change; the builder uses the model's public API only.

## 4. Assumptions and uncertainty

- The layer builder consumes in-memory candles per timeframe. The store will feed it the same accepted series; the builder does not read SQLite, because the analytics crate is pure.
- Layer 3 requires an M1 base. A dataset without M1 has no Layer 3 rows.
- The caller supplies the higher-timeframe candles. Deriving them from M1 is the data engine's job and happens before the builder.
- The model's event aggregate reads only events in the five-bar window plus the latest event for its per-timeframe summary. Bounding the snapshot's event list to those records keeps every published `EventAggregates` field correct without cloning the full history.
- The 100k cap is a store/export rule; the builder itself is unbounded and the benchmark reports its cost at the cap.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Per-bar record shape | Name-keyed map; typed structs | Typed groups | The export needs fixed fields; a map hides a typo and allocates per row |
| D2 | Layer 2 derivation | Re-read the model bars; derive from Layer 1 | Derive from Layer 1 | The issue requires the derivation, and the resolution is a function of the published values and reasons |
| D3 | Layer 3 base | Any fastest timeframe; M1 only | M1 only | The contract defines Layer 3 for an M1 base; another base has no defined block layout |
| D4 | Block content | Full Layer 2 row; candle + metrics + resolutions | Candle + metrics + resolutions | Matches #73 and the contract; the higher readings remain in Layer 2 |
| D5 | Snapshot events | Full history; five-bar window plus the latest event | Window plus the latest event | Every published cross-timeframe field stays correct; the full history would be cloned 100k times |
| D6 | Missing higher block | A block with null fields; an absent block | Absent in memory, nulls in the export | The export knows the configured ladder; the in-memory row stays typed |
| D7 | Column counts | Count the CSV header; contract constants | Constants, asserted | The builder has no CSV; #80 asserts the header against the same constants |
| D8 | Input order | Require sorted; sort | Sort by timeframe | Reordered inputs must produce identical layers |
| D9 | Invalid input | Skip silently; `Result` | `Result` with `ZeroTimeframe` and `DuplicateTimeframe` | A silently skipped timeframe would change the ladder without a signal |
| D10 | `agg_ladder` wording | Leave “configured”; refine | Refine to “aggregated in this row” | The model aggregates the snapshots present at the row; early rows have fewer timeframes |

## 6. Acceptance criteria

- Layer 1 and Layer 2 have one row per bar per timeframe, in timeframe-then-bar order; Layer 3 has one row per M1 bar.
- Layer 2 is exactly Layer 1 plus the four axis resolutions, each derived from its own axis.
- Layer 3 uses the latest closed higher bar, with the contract's staleness, and every block closes at or before the M1 close.
- Appending later M1 bars changes no earlier Layer 3 row; reordered and repeated builds are identical.
- The contract column widths are asserted: 61, 73, 67 per block, 82 aggregates, 356 for three higher timeframes.
- The 100k release benchmark completes and is reported.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.

## 7. Work breakdown

1. Add `layers::metrics` with the typed record and the model conversion.
2. Add `layers::series` with the per-bar snapshot assembly and the bounded event list.
3. Add `layers` with the public rows, the builder, the errors, and the column constants.
4. Add the tests and the ignored 100k benchmark.
5. Refine the `agg_ladder` and `*_timeframe_count` wording in the contract.
6. Run the checks and the release benchmark, write the plan and the log, commit, push, open the PR, move #79 to Done, merge, and clean up.
7. Continue with #80 on its own branch.

## 8. Verification

| Check | Result |
|---|---|
| `cargo test -p quantrule-analytics` | 34 passed, 0 failed, 1 ignored; 2 doctests passed |
| `cargo test --workspace` | 713 passed, 0 failed, 2 ignored |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | clean |
| 100k release benchmark | 128,332 Layer 1 rows and 100,000 Layer 3 rows in **4.396 s** |
| Alignment in Layer 3 | 09:03 → M5 bar 1, 240 s; 09:04 → M5 bar 2, 0 s; H1 08:00, 300 s |

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| The per-bar snapshot diverges from the model's latest snapshot | `layer_one_matches_the_model_snapshot_at_the_latest_bar` compares the last row to `MarketState::compute` |
| The bounded event list drops a published field | The event aggregate reads the five-bar window and the latest event; both are retained, and the event tests in #82 verify the export |
| The 100k build is too slow | The release benchmark reports 4.396 s; the state is rebuilt only for the as-of bar, and the full event history is never cloned |
| The contract's “configured” wording is read literally | The contract is refined in this PR and the refinement is recorded in the plan and the log |
| Memory at 100k | Layer 1 and Layer 2 duplicate the rows; the benchmark completes within the expected range, and the store will persist rather than hold both |

## 10. Stop conditions

- A layer needs a value the model does not publish.
- A block would need a bar that closes after the M1 bar's close.
- The builder needs to read the store or the clock.
- A check fails and the fix is outside this scope.

## 11. Delivery

| Item | Value |
|---|---|
| Commit | `9b6efc6` — `feat: build the three research layers` |
| Branch | `feat/79-layers` |
| Pull request | [#94](https://github.com/quant-rule/quant-rule/pull/94) |
| Board | #79 moved to `Done` when the PR is open |

Merge, release, tag, and deployment remain with the human.
