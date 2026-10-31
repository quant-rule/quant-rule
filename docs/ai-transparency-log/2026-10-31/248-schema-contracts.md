# AI Transparency Log — Schema Contract Truthfulness

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential implementation and verification passes
- Work item: #248
- Plan: `docs/execution-plans/2026-10-31/audit-unified-ladder.md` (W2)
- Findings: AUD-2026-10-31-02, -07, -10, -19
- Decisions: D2=A (execution v2 current, v3 target)

## Intent and boundary

The human approved the unified-ladder remediation plan and decisions D1–D12.
W2 makes the documented configuration schemas match the loaders for four
findings: the unreachable strategy v2 migration message, the execution v2/v3
contradiction, the optional `model.json.timeframes` map, and the
`missing_context` optionality.

The change is limited to the shell config parsers, their unit tests, and the
affected configuration contracts. It does not change the strategy rule, the
model formulas, execution fills, portfolio accounting, or any threshold. It does
not implement execution v3. It adds no dependency and no `unsafe` code. The
human authorized commits and PRs; the work was committed as `92417ce`, pushed to
`fix/248-schema-contracts`, and opened as PR #258. It is not merged.

## Implemented

- Checked the strategy schema before the v3 field set. A real v2 file now
  reaches the documented migration message instead of failing on
  `long_threshold`.
- Made `model.json.timeframes` optional in the v2 parser. An absent map now
  means every timeframe inherits the defaults, as the contract states.
- Kept `missing_context` required and changed the strategy contract to say so;
  the parser and `policy-and-strategy.md` already required exactly `reject`.
- Corrected the execution v3 contract: v2 is the current loader schema and v3 is
  the approved target, not a retired v2.
- Added a real v2-shape regression test, an absent-`timeframes` test, and a
  missing-`missing_context` test.

## Verification and evidence

| Check | Observed |
|---|---|
| Fail-before, schema order restored | `strategy_v2_is_rejected_with_a_migration_message` failed with `$.long_threshold: expected a known field` |
| Fail-before, required map restored | `model_v2_timeframes_map_is_optional` failed with `$.timeframes: expected a required field` |
| Restored | Both tests passed |
| Real v2 `policy check` | `quantrule.strategy/v2 is retired; use quantrule.strategy/v3 with rule "ladder-context"` |
| Model without `timeframes` | `policy ok ... strategy=ladder-context execution=5m ladder=5m,15m` |
| Missing `missing_context` | `$.no_trade.missing_context: expected a required field` |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | 45 test-result-ok suites, 0 failures |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Passed |
| `git diff --check` | Clean |
| Commit and PR | `92417ce`; PR #258; not merged |

The two code tests have fail-before evidence. The `missing_context` and
execution-v2 findings are documentation corrections: their tests and parser
behavior already matched the chosen strict rule, so a fail-before test is not
possible without changing behavior. The tests pin the chosen contract.

## Learning and rejected suggestions

- Rejected implementing execution v3 now. Decision D2=A keeps v2 current and v3
  a target; implementing v3 would be a separate schema migration.
- Rejected making `missing_context` optional with a default. The policy/strategy
  table already says exactly `reject`, and an explicit key keeps the human
  decision visible in the file.
- Rejected removing `timeframes` from the known v2 field set. The map is still
  supported; only its presence is optional.
- Rejected changing the v2 migration message to the unknown-field error. The
  documented migration path should be reachable for a real historical file.
- Kept the v3 execution document as the target contract. It is corrected to say
  the current loader reads v2, not deleted; the v3 work remains planned.
