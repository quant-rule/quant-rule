# AI Transparency Log: The Three Research Layers (AN4)

- Work item: [#79](https://github.com/quant-rule/quant-rule/issues/79) — feat: build layers 1, 2, and 3
- Plan: [`docs/execution-plans/2026-10-24/79-layers.md`](../../execution-plans/2026-10-24/79-layers.md)
- Contract: [`docs/analytics/2026-10-24/research-output.md`](../../analytics/2026-10-24/research-output.md) §5–§7
- Date: 2026-10-24 (UTC)

## Entry: the three layers

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Build Layers 1–3 from the model's per-bar output, with no store, export, or model change.
- **Verification and evidence:**
  - Added `layers::metrics` (the typed per-bar record), `layers::series` (the per-bar snapshot assembly), and `layers` (the rows, the builder, the errors, and the column constants).
  - 34 unit tests and 2 doctests pass, including the L1→L2 derivation, the L3 block and aggregate checks, causality, reordering, repeated runs, the M1 requirement, and the zero/duplicate errors.
  - `cargo test --workspace`: 713 passed, 0 failed, 2 ignored.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
  - The ignored release benchmark built 128,332 Layer 1 rows and 100,000 Layer 3 rows in 4.396 s.
- **Learning:**
  - The model's `TimeframeSnapshot` is a latest-bar record, and its `build_timeframe` is private. The analytics crate rebuilds the snapshot at any bar from the model's public series (readings, four axes, context, events, confirmed levels) instead of changing the model. The alternative — exposing a per-bar builder in the model — would have been a larger, cross-crate change than #79 needs.
  - The first sketch cloned the full event list into every snapshot. Reading `aggregate/events.rs` showed that the cross-timeframe totals use only the five-bar window counts and the latest event; the age fields live in the per-timeframe summary. The snapshot now retains the window plus the latest event, which keeps every published `EventAggregates` field correct and avoids an O(n²) clone at 100k rows.
  - The contract said `agg_ladder` and the `*_timeframe_count` fields were “configured”, but the model aggregates the snapshots present in the row. Early rows, before a higher timeframe closes, have fewer timeframes. The contract is refined to “aggregated in this row”; the model contract already defines the ladder as the snapshots passed in.
  - The column counts are contract constants, not a counted CSV header. #80 will assert the header against the same constants; the plan states that limitation rather than claiming a header check here.
  - A draft test called `LayerConfig::with_default_scale()`, which does not exist. It was corrected to `LayerConfig::default()` before the first compile; no test ever passed against the wrong API.
  - Layer 2 is derived from the Layer 1 metrics and reason codes, not from a second read of the model bars, so the “Layer 2 derives from Layer 1” requirement is literal. The resolution function is the same `AxisResolution::new` and `clamp_unit` mapping #77 tested.
  - The builder sorts its inputs and uses no unordered collection; the reorder and repeated-run tests prove the determinism requirement.

## Final verification

- `cargo test -p quantrule-analytics`: 34 passed, 0 failed, 1 ignored; 2 doctests passed.
- `cargo test --workspace`: 713 passed, 0 failed, 2 ignored.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- 100k release benchmark: 128,332 Layer 1 rows / 100,000 Layer 3 rows in 4.396 s.
- Status: `Verified`. Commit `9b6efc6` is pushed on `feat/79-layers`, pull request [#94](https://github.com/quant-rule/quant-rule/pull/94) is open, and #79 is `Done` on the board.
