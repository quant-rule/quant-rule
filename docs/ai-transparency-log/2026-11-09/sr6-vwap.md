# AI Transparency Log — SR6 VWAP Rejection

- Date: 2026-11-09 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: `sr6-vwap`
- Plan: [`sr6-vwap.md`](../../execution-plans/2026-11-09/sr6-vwap.md)
- Charter: [`SR6-CHARTER.md`](../../governance/2026-11-09/SR6-CHARTER.md)
- Record: [`SR6-VWAP.md`](../../improvement/2026-11-09/SR6-VWAP.md)
- Status: complete; no candidate; 9 bps and 2024 not opened
- Claim type: strategy research; no edge or profitability claim

## Intent and boundary

The human approved the VWAP-rejection premise after SR5. The work tests a 15m
`signal.vwap_rejection` with a 1h EMA-stack context on BTCUSD. It uses the
existing accepted model and execution/portfolio configs; it does not change
signal definitions.

## Implementation

- Wrote the charter and plan with a 30-event-per-side count gate.
- Added `tools/improvement/vwap_rejection_count.py` and its unit tests; the tool
  filters on the unique event code 28 and excludes `signal.vwap_reclaim` (27).
- Ran the count gate on the five recorded 15m/1h model bundles: 4,822 long /
  4,281 short A1 events.
- Added the six strategy configs, 30 base runs, 10 V1 9 bps runs, and the SR6
  qualification criteria.
- Fixed the event-composite validator: it hardcoded subtypes `1..=7`, which
  excluded the registered `signal.vwap_rejection` (14). The validator now
  accepts registered signal subtypes and still rejects diagnostics, with
  regression tests.
- Ran 30 preflights, 30 base cells, and 30 qualification records.
- Did not run the 9 bps sensitivity or 2024.

## Verification and evidence

| Check | Observed |
|---|---|
| Count gate | 4,822 long / 4,281 short; pass |
| Preflight | 30 / 30 pass at 6 bps |
| Base cells | 30 / 30 complete |
| Qualification | 30 / 30 complete |
| V1 long | 229 trades, −$242.14, PF 0.966, max DD 6.37% |
| V1 short | 154 trades, −$719.73, PF 0.853, max DD 5.61% |
| Halts | every V1/V2/V3 cell halted in every year |
| 9 bps | not run; V1 failed the base gate |
| 2024 | not opened |
| Parser tests | subtype 14 accepted; diagnostic subtype 21 rejected |
| Rust gates | `cargo fmt`, `cargo test`, `cargo clippy`, `cargo doc` pass |
| Python gates | 141 improvement tests pass |
| Documentation | registry, links, changelog, structure, workflows, partitions pass |

## Learning

- The VWAP-rejection event is frequent, but the 1 ATR / 3 ATR design has no
  cost-aware follow-through at 15m.
- Every variant halts every year; the bar-close 5% halt overshoots to 5.26–6.85%.
- The 1h stack does not improve the event-only control; V1 and V3 are both
  negative on the long side, and V1 short is negative with V3 short worse.
- The 1h ADX control is the least bad variant but still fails PF >1.10 and
  breaches drawdown.
- The parser limit `1..=7` was a real blocker for registered signal subtypes;
  the registry lookup already provides the exact identity and role check.

## Rejected or corrected

- Did not tune the event, filters, stop, target, or timeframe after the result.
- Did not run 9 bps after V1 failed the base gate.
- Did not open 2024.
- Did not promote V2 or V3.
- Did not weaken the 5% drawdown gate.
- Did not claim independent Quant Developer or Quant Validator review; the
  passes were sequential same-agent passes.
