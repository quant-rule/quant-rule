# AI Transparency Log — Level-Fade Rule (v7)

- Date: 2026-11-01 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: `level-fade-v7`
- Plan: [`docs/execution-plans/2026-11-01/level-fade-v7.md`](../../execution-plans/2026-11-01/level-fade-v7.md)
- Claim type: strategy implementation and evidence; no edge claim

## Intent and boundary

Implement a new rule family that fades a failed level test, then pre-register and
run 2024 validation. The model and portfolio stay frozen. No threshold changes
after the result.

## Work

- Added `StrategyMode::LevelFade`, the v7 parser, policy/explain output, and
  unit tests proving the fade ignores the directional context.
- Added v7 configs for BTCUSD, ETHUSD, and XAUUSD on 15m/1h.
- Ran 2024 validation: BTCUSD −15.23%, ETHUSD −18.97%, XAUUSD −4.73%.
- The validation gate failed, so 2025 OOS and portfolio runs were not executed.
- No strategy baseline is accepted.

## Rejected suggestions

- Rejected: another breakout/continuation threshold variant.
- Rejected: using Direction/Momentum as a trend gate for the fade.
- Rejected: calling a single cell an edge.

## Verification

- `cargo test --workspace` — pass.
- `STRATEGY-003-comparison.md` records the validation.
- Gates: fmt, clippy, doc, links, terms pass.

## Limitations

- Same-session review, not an independent Quant Validator record.
- One OOS year; 2023 excluded.
