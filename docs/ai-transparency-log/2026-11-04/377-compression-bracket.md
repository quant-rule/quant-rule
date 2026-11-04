# AI Transparency Log — 1m Compression Bracket

- Date: 2026-11-04 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Plan: [`docs/execution-plans/2026-11-04/377-compression-bracket.md`](../../execution-plans/2026-11-04/377-compression-bracket.md)
- Issue: [#377](https://github.com/quant-rule/quant-rule/issues/377)

## Intent and boundary

Implement the human-approved true bracket: a neutral pre-breakout
`signal.compression_range` event, two-sided stop-entry orders with OCO, and a
1m bracket strategy with a 1 ATR stop, 4 ATR target, and 20-bar time stop. The
accepted model baseline and the policy are frozen. The work is model + execution
+ strategy; it does not change the accepted breakout event or its thresholds.

## Design decisions

- The range event has its own configuration (`enabled`, `coiling_bars`) rather
  than reusing the breakout event's thresholds. The event publishes `range_high`
  in `primary_value` and `range_low` in `reference_value`; it is neutral.
- The strategy snapshot now carries event values, so a strategy reads published
  numbers instead of recomputing a range.
- The bracket is a new `Decision::EnterBracket` / `OrderIntent::EnterBracket`
  path, not a market entry with a synthetic side.
- A bracket is valid for the next bar only. If both boundaries are touched and
  no gap resolves the side, the bar is ambiguous and neither order fills.
- The first fill is bracketed by the existing ATR stop / reward-to-risk target
  resolution. The time stop remains a strategy exit.

## Pre-registration

The range-event protocol, the strategy rule, the costs, the folds, and the green
criteria are recorded before the first run. No threshold is selected from a
backtest. `model_validation` 2021–2022 is reused for a different event and
outcome; the second use is disclosed in the protocol and will be disclosed in
the decision.

## Model gate result

The `signal.compression_range` event was implemented and gated on 1m.

| Partition | n | J | Hit | Matched base | Lift | Lift LCB | Rand p | Back lag | Rep |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `model_dev` | 346,120 | 60,605 | 0.6474 | 0.5442 | +0.1032 | +0.1011 | 0.0050 | +0.0531 | 3 |
| `model_validation` | 247,989 | 43,937 | 0.5802 | 0.4984 | +0.0818 | +0.0795 | 0.0050 | +0.0291 | 3 |

Both partitions pass. `model_validation` 2021–2022 is a second use of the
partition already used for the accepted 4 ATR breakout baseline; the disclosure
is in the protocol and both run records.

The event is an accepted model baseline (human, 2026-11-04). The strategy
tier proceeds under the approved bracket protocol.

## Verification

- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test
  --workspace`, and `cargo doc --workspace --no-deps` pass.
- The range-event tests cover causality, determinism, empty/short/equal-boundary
  cases; the execution tests cover stop-entry, gap, OCO, ambiguity, expiry,
  zero volume, non-contiguity, risk cap, and determinism.
- The model gate ran on the 15 model bundles (9 dev, 6 validation); the harness
  uses only 1m rows and keeps memory bounded.
- The 1m bracket strategy ran the 15-fold `strategy_dev` matrix after the human
  accepted the range baseline. Every fold halted at the 10% drawdown limit; the
  result is no edge and the 2024 validation was not opened.
- No portfolio result is claimed.

## Strategy result

The 1m bracket failed `strategy_dev`: BTC −41.23%, ETH −26.03%, XAU −41.18%;
pooled PF 0.592; 0/3 positive; 15/15 folds halted. Aggregate gross P&L was
+$2,887 against $15,753 of commission, so the 4 bps cost was about 5.5× the gross
edge. The 2024 validation partition was not opened; no strategy baseline was
accepted. The accepted range model baseline is unchanged.

The next structural hypothesis is the same bracket rule on 5m, where 1 ATR is
larger relative to the fixed commission. It needs a new 5m range gate and a new
pre-registration; it is not a threshold change to the 1m rule.
