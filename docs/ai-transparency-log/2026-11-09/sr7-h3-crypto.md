# AI Transparency Log — SR7 H3 Crypto Cross-Section

- Date: 2026-11-09 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: `sr7-h3-crypto`
- Plan: [`sr7-h3-crypto.md`](../../execution-plans/2026-11-09/sr7-h3-crypto.md)
- Charter: [`SR7-CHARTER.md`](../../governance/2026-11-09/SR7-CHARTER.md)
- Record: [`SR7-H3-CRYPTO.md`](../../improvement/2026-11-09/SR7-H3-CRYPTO.md)
- Status: complete; no candidate; 9 bps and 2024 not opened
- Claim type: strategy research; no edge or profitability claim

## Intent and boundary

The human approved the BTC+ETH H3 premise after SR6. The work tests whether the
SR3 H3 1h fade is a crypto-major behavior by adding ETHUSD to the BTCUSD control.
It uses the accepted model and execution/portfolio configs; it does not change
signal definitions.

## Implementation

- Wrote the charter and plan with a pooled 30-event-per-side count gate.
- Added `tools/improvement/h3_count.py` and its tests; it filters compression
  breakout code 17, maps the H3 side opposite the event, and requires stack
  alignment plus weak ADX.
- Built 10 per-year 1h model bundles (BTC and ETH, 2018–2022) and ran the count
  gate: BTC 20/23, ETH 19/19, pooled 39/42; pass.
- Added the two strategy configs, 20 base runs, 20 9 bps run configs, and the
  SR7 qualification criteria.
- Ran 20 preflights, 20 base cells, and 20 qualification records.
- Did not run 9 bps or 2024.

## Verification and evidence

| Check | Observed |
|---|---|
| Count gate | pooled 39 long / 42 short; pass |
| Preflight | 20 / 20 pass at 6 bps |
| Base cells | 20 / 20 complete |
| Qualification | 20 / 20 complete |
| H3 long pooled | 27 trades, +$2,289.36, PF 2.098, max DD 5.62%, 5/5 years |
| H3 short pooled | 36 trades, +$147.02, PF 1.067, max DD 5.64%, 1/5 years |
| BTC control | Reproduces SR3 exactly; inside 5% |
| ETH long | 9 trades, +$669.86, PF 1.870, DD 5.62% |
| ETH short | 15 trades, −$576.42, PF 0.527, DD 5.64% |
| 9 bps | not run; no side passed the base gate |
| 2024 | not opened |
| Rust gates | `cargo fmt`, `cargo test`, `cargo clippy`, `cargo doc` pass |
| Python gates | 144 improvement tests pass |
| Documentation | registry, links, changelog, structure, workflows, partitions pass |

## Learning

- The BTC control reproduces SR3 exactly; the data and strategy path are
  consistent.
- ETH long is positive but only 9 trades with a 5.62% drawdown; it is not a
  confirmation.
- ETH short is negative and breaches 5.64%; the cross-crypto premise fails on
  the short side.
- H3 long is positive in all five pooled years but remains underpowered at 27
  trades and fails the drawdown gate.
- The stack-fade family is closed as a candidate.

## Rejected or corrected

- Did not lower the 30-trade floor.
- Did not drop ETH or add a filter to rescue H3.
- Did not run 9 bps after the base gate failed.
- Did not open 2024.
- Did not promote the BTC-only control.
- Did not claim independent Quant Developer or Quant Validator review; the
  passes were sequential same-agent passes.
