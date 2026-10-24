# AI Transparency Log: Research Output Contract (AN1)

- Work item: [#76](https://github.com/quant-rule/quant-rule/issues/76) — docs: define the research output contract
- Plan: [`docs/execution-plans/2026-10-24/76-research-output.md`](../../execution-plans/2026-10-24/76-research-output.md)
- Contract: [`docs/analytics/2026-10-24/research-output.md`](../../analytics/2026-10-24/research-output.md)
- Date: 2026-10-24 (UTC)

## Entry: contract definition

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Define the research dataset contract before any analytics code exists: the three layers, every column, the resolution mapping, the null and reason rules, the as-of rule, the record files, and the metadata. Documentation only; no analytics, store, or shell code in this work item.
- **Verification and evidence:**
  - Read the model contract §7–§14 and the model's published structs to take the field names and code registries from the implementation rather than inventing them.
  - Wrote `docs/analytics/2026-10-24/research-output.md` and added the analytics kind to `docs/README.md`.
  - A column-count script reported Layer 1 = 61, Layer 2 = 73, Layer 3 = 356, and 82 aggregate columns; the group counts are 7 Direction, 9 Momentum, 19 Structure, 9 Progression, 3 Context, 12 resolutions.
  - A `rustc` probe confirmed the float-format claim: `f64` `Display` emits no exponent, `-0.0` prints `-0`, and `1.2345678901234567` round-trips.
  - `cargo fmt --all -- --check`: clean. `cargo clippy --all-targets -- -D warnings`: clean. `cargo test --workspace`: 677 passed, 0 failed, 1 ignored. `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
  - Relative links resolve after the plan and the log are added.
- **Learning:**
  - The issue says "Layer 1: candle + metrics", while #68 stores candles, readings, and metrics. The contract resolves this as candle + readings + metrics (D1), because the readings are the audit trail of the metrics and are already part of the per-bar record. This is a documented interpretation, not a silent change.
  - The issue's "as-of M5/M15/H1 blocks" needed a width decision. #73 names "candle, metrics, and resolutions", so the block carries exactly those groups and the higher readings stay in Layer 2 (D2). The base row is the full Layer 2 row, which keeps Layer 3 cumulative.
  - The equal-width bucket cannot be computed by `(v + 1.0) / 0.2`: a probe of the first attempt showed `-0.8` landing in bucket 0 through floating-point error. The contract counts the nine boundaries directly, which maps `-0.8` to 1 as #77 requires (D7).
  - The manifest must not carry a wall-clock timestamp or a hash: the first is forbidden in command output, and the second would introduce a dependency the issue does not need. Row counts and byte sizes are enough (D8).
  - The model publishes no per-reading reason code. Rather than invent one, the contract keeps the axis reason codes authoritative and documents each reading's null condition (D11). Adding per-reading reasons would be a model change.
  - An earlier attempt in this session began the `quantrule-analytics` crate under this issue. It was the wrong issue: #76 is documentation, and #77 owns the crate. The draft was moved to a scratch directory, removed from this branch, and is not part of this delivery. The branch was renamed from `feat/76-research-output` to `docs/76-research-output` to match the documentation convention.

## Final verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 677 passed, 0 failed, 1 ignored.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Contract column counts: 61 / 73 / 356, 82 aggregates.
- Status: `Verified`. Commit `df3a42a` is pushed on `docs/76-research-output`, pull request [#91](https://github.com/quant-rule/quant-rule/pull/91) is open, and #76 is `Done` on the board.
