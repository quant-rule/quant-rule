# AI Transparency Log — Remove `strategy_id`

- Date: 2026-11-06 (UTC)
- Plan: [`docs/execution-plans/2026-11-06/remove-strategy-id-hook.md`](../../execution-plans/2026-11-06/remove-strategy-id-hook.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash), single session
- Intent and boundary: remove the unused current `strategy_id` run field; no model, strategy, or portfolio behavior change.

## 1. Implemented work

- Removed `strategy_id` from `RunConfig`, `RunFile`, loading, and identity validation.
- Removed it from the run-file allowed-field list.
- Added `run_rejects_the_removed_strategy_id`, which expects an unknown-field error at `$.strategy_id`.
- Updated the policy and model-family identity documentation: current prefixes are `M`, `POL`, `E`, `P`; `S` returns with S3.x.

## 2. Verification

Observed on 2026-11-06:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace --no-fail-fast` | pass; 0 failed; new rejection test passes |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | pass |
| `policy check --config config/examples/btcusd-2024/run.json` | `policy ok` |
| Python gates and `git diff --check` | pass |

## 3. Learning and corrections

The field was a forward hook with no current consumer. Removing it makes the current schema match the current surface; the S3.x strategy file reintroduces it with a real parser and test.
