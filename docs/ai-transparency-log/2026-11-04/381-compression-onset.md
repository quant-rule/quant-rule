# AI Transparency Log — Fresh-Onset Compression Event

- Date: 2026-11-04 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Plan: [`docs/execution-plans/2026-11-04/381-compression-onset.md`](../../execution-plans/2026-11-04/381-compression-onset.md)
- Issue: [#381](https://github.com/quant-rule/quant-rule/issues/381)

## Intent and boundary

Implement a fresh-onset compression event after the 5m rolling range gate failed
the back-lag control. The event fires only at the first completed coiling window
after a known non-coiling bar, removing adjacent-window overlap. The work is a
new model event, a 5m gate, and a pre-registered 5m bracket strategy; it does
not change the rolling range event, the bracket rule, costs, or risk.

## Design decisions

- The onset is a new subtype (9) rather than a flag on the rolling event, so the
  event's definition and claim are explicit.
- The prior bar must carry a known non-coiling context code; an unknown
  predecessor is skipped rather than treated as non-coiling.
- The gate reuses the 5m next-bar touch outcome and matched non-coiling
  baseline.
- The 2021–2022 validation partition is a third use; the protocol and run
  records disclose it.

## Result

Both partitions pass.

| Partition | n | J | Hit | Matched base | Lift | Lift LCB | Rand p | Back lag | Rep |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `model_dev` | 18,324 | 12,521 | 0.5706 | 0.4760 | +0.0946 | +0.0874 | 0.0050 | +0.0365 | 3 |
| `model_validation` | 14,303 | 9,708 | 0.5373 | 0.4651 | +0.0722 | +0.0640 | 0.0050 | −0.0065 | 3 |

The fresh-onset event is an accepted model baseline (human, 2026-11-04). The
5m onset strategy proceeds under the approved protocol.

## Verification

- Onset event tests cover onset-only emission, rolling-window suppression,
  unknown-predecessor rejection, causality, determinism, and edge cases.
- The 5m gate harness selects subtype 9; the model bundles are generated with
  the onset config.
- No strategy result is claimed before baseline acceptance.

## Strategy result

The 5m onset bracket failed `strategy_dev`: BTC −37.47%, ETH −24.97%, XAU
−40.96%; pooled PF 0.654; 0/3 positive; 15/15 folds halted. The aggregate gross
P&L was −$2,286 against $9,960 of commission, so the 4 ATR target / 1 ATR stop
payoff is negative before costs. The 2024 validation was not opened; no strategy
baseline was accepted.

The next step is a barrier path diagnostic on the 5m onset event, not another
target/stop change.
