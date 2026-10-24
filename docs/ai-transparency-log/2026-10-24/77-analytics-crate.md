# AI Transparency Log: The Analytics Crate and Axis Resolution (AN2)

- Work item: [#77](https://github.com/quant-rule/quant-rule/issues/77) — feat: add the analytics crate and axis resolution
- Plan: [`docs/execution-plans/2026-10-24/77-analytics-crate.md`](../../execution-plans/2026-10-24/77-analytics-crate.md)
- Contract: [`docs/analytics/2026-10-24/research-output.md`](../../analytics/2026-10-24/research-output.md) §6
- Date: 2026-10-24 (UTC)

## Entry: crate and resolution

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Add the pure analytics crate and the per-axis resolution. No as-of alignment, layers, export, store, or shell code.
- **Verification and evidence:**
  - Added `crates/quantrule-analytics` with `AxisResolution`, `AxisResolutions`, `ordinal_bucket`, and `clamp_unit`.
  - 10 unit tests pass: the contract boundaries, every reachable bucket, clamping, null-with-reason, measured-without-reason, the four-axis wiring, and progression's clamp.
  - `cargo test --workspace`: 687 passed, 0 failed, 1 ignored.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
  - The crate list in `AGENTS.md` now includes `crates/quantrule-analytics` and points at the research contract.
- **Learning:**
  - The first bucket implementation divided by `0.2`. A probe showed `-0.8` landing in bucket 0 because `(-0.8 + 1.0) / 0.2` is `0.9999999999999998`. Counting the nine boundary constants maps `-0.8` to 1 exactly and removes the floating-point edge (D1).
  - `-0.0 >= 0.0` is true, so negative zero resolves to bucket 5 without a special case. The contract still normalizes `-0` to `0` when writing, so the text and the data agree.
  - The constructor deliberately does not clamp a published axis value: clamping would hide an out-of-range model value, while `from_bars` clamps only Progression's raw `progress`, which the contract allows outside `[-1, +1]` (D2).
  - The model sets `unavailable_reason` on every missing axis value, so `from_bars` needs no `Result` and the null/reason pair can be validated once at the export (D3).
  - The crate depends only on `quantrule-model`; adding `quantrule-data` now would be an unused dependency until the layer builder needs candles (D5).
  - Part of this module was drafted earlier under #76, before the issue boundary was checked. It was set aside, the division bug was found, and the module was reworked here under #77. The #76 delivery contained no code.

## Final verification

- `cargo test -p quantrule-analytics`: 10 passed, 0 failed.
- `cargo test --workspace`: 687 passed, 0 failed, 1 ignored.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Status: `Verified`. Commit, push, pull request, and board transition follow the delivery step.
