# AI Transparency Log — SR5 Absorption Rejection

- Date: 2026-11-09 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: `sr5-absorption`
- Plan: [`sr5-absorption.md`](../../execution-plans/2026-11-09/sr5-absorption.md)
- Charter: [`SR5-CHARTER.md`](../../governance/2026-11-09/SR5-CHARTER.md)
- Record: [`SR5-ABSORPTION.md`](../../improvement/2026-11-09/SR5-ABSORPTION.md)
- Status: complete; underpowered at the count gate; no backtest; 2024 not opened
- Claim type: strategy research; no edge or profitability claim

## Intent and boundary

The human approved the absorption-rejection premise after SR4. The work tests a
15m `signal.absorption_rejection` with a 1h EMA-stack context on BTCUSD. The
programme starts with a read-only count gate; if the event is too rare, it stops
before any backtest.

## Implementation

- Wrote the charter and plan with a 30-event-per-side count gate.
- Added `tools/improvement/absorption_count.py` and its unit tests. The tool
  joins each 15m event to the latest closed 1h bar using the strategy's
  close-time rule.
- Built five per-year 15m/1h model bundles for 2018–2022.
- Ran the count gate and recorded `count-gate/count.json`.
- Did not build backtest cells, run preflight, run 9 bps, or open 2024.
- Wrote `results.json`, `review.md`, `decision.md`, and the committed summary.

## Verification and evidence

| Check | Observed |
|---|---|
| A1 count | 14 long, 12 short |
| A2 count | 7 long, 8 short |
| A3 event-only count | 30 long, 32 short |
| Count gate | fail; threshold 30 per side |
| A1 by year | 2018 0/0; 2019 11/4; 2020 2/2; 2021 0/0; 2022 1/6 |
| Model bundles | 5 / 5 per-year 15m/1h bundles |
| Count tool tests | 3 pass |
| Backtest | not run; count gate failed |
| 9 bps | not run |
| 2024 | not opened |
| Rust gates | `cargo fmt`, `cargo test`, `cargo clippy`, `cargo doc` pass |
| Python gates | 135 improvement tests pass |
| Documentation | registry, links, changelog, structure, workflows, partitions pass |

## Learning

- The absorption-rejection event is too rare when filtered by the 1h stack:
  14 long / 12 short over five years.
- The event-only control is only at the threshold (30/32), and it is clustered:
  2019 carries most of the A1 events; 2018 and 2021 have none.
- The 1h ADX control is smaller still (7/8).
- The count gate avoided spending 30 backtest cells on an underpowered event.

## Rejected or corrected

- Did not lower the 30-per-side threshold.
- Did not promote the event-only control.
- Did not add or remove a filter to rescue A1.
- Did not run the backtest or 9 bps after the gate failed.
- Did not open 2024.
- Did not claim independent Quant Developer or Quant Validator review; the
  passes were sequential same-agent passes.
