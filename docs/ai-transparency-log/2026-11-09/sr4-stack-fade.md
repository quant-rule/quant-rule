# AI Transparency Log — SR4 Stack-Aligned 1h Fade

- Date: 2026-11-09 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: `sr4-stack-fade`
- Plan: [`sr4-stack-fade.md`](../../execution-plans/2026-11-09/sr4-stack-fade.md)
- Charter: [`SR4-CHARTER.md`](../../governance/2026-11-09/SR4-CHARTER.md)
- Record: [`SR4-STACK-FADE.md`](../../improvement/2026-11-09/SR4-STACK-FADE.md)
- Status: complete; no candidate; 2024 not opened
- Claim type: strategy research; no edge or profitability claim

## Intent and boundary

The human approved proceeding with the next programme after SR3: a larger-sample
test of the stack-aligned 1h fade on BTCUSD, EURUSD, and XAUUSD. The work tests a
pre-registered 18-variant matrix, not a search. It does not change the strategy
engine, the model, the execution/portfolio configs, or the 2024 partition.

## Implementation

- Wrote the programme charter and execution plan.
- Added six strategy configs, 90 data/run configs, 30 S1 9 bps run configs, three
  1h policies, and the SR4 qualification criteria.
- Ran 90 feasibility preflights at 6 bps.
- Ran 90 base cells and 90 qualification records.
- Did not run the 9 bps sensitivity because S1 failed the base gate.
- Wrote `results.json`, `review.md`, `decision.md`, and the committed summary.
- Updated the registry, baselines, README, changelog, and this log.

## Verification and evidence

| Check | Observed |
|---|---|
| Preflight | 90 / 90 pass at 6 bps |
| Base cells | 90 / 90 complete |
| Qualification | 90 / 90 complete |
| S1 long | 113 trades, +$1,150.76, PF 1.34, 4/5 years, 1/3 instruments |
| S1 short | 111 trades, +$411.97, PF 1.16, 3/5 years, 1/3 instruments |
| EURUSD | negative on every variant and side |
| XAUUSD | negative on every variant and side |
| S1 drawdown | BTC long 5.19% (2018), 5.0035% (2021); short 5.11% (2019) |
| 9 bps | not run; S1 failed the base gate |
| 2024 | not opened |
| Reproducibility | S1 long BTCUSD and EURUSD 2018 byte-identical |
| Rust gates | `cargo fmt`, `cargo test`, `cargo clippy`, `cargo doc` pass |
| Python gates | 131 improvement tests pass |
| Documentation | registry, links, changelog, structure, workflows, partitions pass |

## Learning

- The larger sample did not confirm the premise; it falsified the
  multi-instrument claim. The stack-aligned fade is BTC-specific in this sample.
- EURUSD and XAUUSD are negative on every variant and side; adding the RVOL or
  ADX filter does not create a multi-instrument edge.
- The BTC side still has a 2019 regime loss and a 5% drawdown overshoot.
- The pooled positive result is not evidence of a robust edge; it is BTC
  dominance.

## Rejected or corrected

- Did not run 9 bps after the base gate failed.
- Did not add or remove a filter to rescue S1.
- Did not drop a failing instrument from the pooled result.
- Did not relax the 5% drawdown ceiling.
- Did not open 2024.
- Did not present the pooled positive result as a candidate.
- Did not claim independent Quant Developer or Quant Validator review; the
  passes were sequential same-agent passes.
