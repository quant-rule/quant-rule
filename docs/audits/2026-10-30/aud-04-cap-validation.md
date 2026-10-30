# AUD-04 — Entry Envelope Quant Validation

- Date: 2026-10-30 (UTC)
- Finding: [`AUD-2026-10-30-04`](mvp-pre-release-audit.md), fixed-notional current-equity envelope
- Baseline: merged `main` at `fe66881`
- Data version: `dukascopy-bid-solusd-m1-2025-01-01_2025-12-31`
- Config: `config/examples/solusd-2025`, `portfolio.sizing = fixed_notional`, notional `10000`, leverage `1.0`, policy max position fraction `1.0`
- Tool: OpenCode agent, DeepSeek V4.1 Flash (`deepseek-v4.1-flash`)
- Role pass: Quant Validator, sequential by the same agent that implemented the remediation. This is not an independent-agent or independent-model judgment.

## Change under validation

The runner computes an entry envelope of

```text
current equity × portfolio.leverage × policy.risk.max_position_fraction
```

and passes it to execution. If the fill notional exceeds the envelope, execution
caps the fill quantity to the envelope; a non-positive envelope rejects with
`risk_limit`. The cap is applied at the fill price, so a gap or slippage cannot
push a filled entry above the envelope.

## Hypothesis and pass criteria

For every executed entry:

```text
fill.price × fill.quantity <= equity_before_entry × leverage × max_position_fraction
```

with no allowance beyond floating-point tolerance. The cap must remain true after
a drawdown and after a gap-up fill. Equity before an entry is computed as initial
cash plus the net P&L of every trade closed before that entry; entries occur only
while flat, so this is the cash equity available to the entry.

## Commands and observed results

| Command | Observed |
|---|---|
| `target/debug/quantrule backtest --config config/examples/solusd-2025/run.json --out /tmp/opencode/aud04-run` | five bundles written in 4m35s |
| `cargo test -p quantrule-backtest --lib execution::engine_tests::an_entry_above_the_risk_limit_is_capped_at_the_fill` | passed |
| `cargo test -p quantrule-backtest --lib runner::tests::the_risk_envelope_caps_the_first_entry_at_half_equity` | passed |

Full-year portfolio: 65 entries, 65 exits, 65 trades, final equity
`8302.124215`, max drawdown `0.204439`, halted. Commission `0.001`, execution fee
`0`, funding configured as in the example.

Envelope check over every entry fill:

| Field | Observed |
|---|---:|
| Entry fills checked | 65 |
| Entries after equity fell below initial cash | 54 |
| Largest `fill notional / equity_before_entry` | `1.000000000000002` |
| Violations beyond `1e-9` tolerance | 0 |
| Largest entry | notional `10000.00` against equity `10000.00` |

The largest ratio is floating-point representation noise, not an over-envelope
fill. The gap test and half-equity runner test independently exercise the cap
when the fill price differs from the decision price.

## Interpretation

- The cap holds on the full-year example after drawdown; 54 entries occurred when
  equity was below the initial cash and none exceeded the current-equity
  envelope.
- The gap test caps a fill at the envelope when the next open jumps above the
  decision price.
- The cap is applied before the position opens; the ledger receives the capped
  fill quantity and the order record keeps its original sizing quantity. A capped
  fill is a partial fill and may be below the configured quantity step.
- No future bar is read: the envelope uses the decision-bar equity, and the fill
  uses the next bar's open.

## Limitations

- One data version, one in-sample full-year run. No out-of-sample or walk-forward
  evidence is claimed.
- The validator pass was performed sequentially by the implementing agent; no
  independent model judgment is claimed.
- The example uses a fixed notional equal to initial cash; a strict rejection
  would have been untradeable, so the audit's allowed cap option was implemented.
- No profitability claim is made. This validates a risk-control invariant only.

## Verdict

The current-equity entry envelope is reproduced and holds on the full-year
example and the targeted gap/half-equity tests. AUD-04 can move from `Fixed` to
`Verified`, with the same-agent validation limitation recorded.
