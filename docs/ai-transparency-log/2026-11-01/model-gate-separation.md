# AI Transparency Log — Model-Gate Separation

- Date: 2026-11-01 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: `model-gate-separation`
- Plan: [`docs/execution-plans/2026-11-01/model-gate-separation.md`](../../execution-plans/2026-11-01/model-gate-separation.md)
- Claim type: workflow contract and strategy evidence; no edge claim

## Intent and boundary

Separate the model-correctness baseline from the strategy-edge gate. Record the
model correctness baseline with the four evidence runs and their C-axis
limitations, then run the first controlled strategy comparison. No model formula
change and no automatic strategy acceptance.

## Work

- Updated the model evidence protocol and Improvement Workflow gate.
- Recorded the model correctness baseline `MODEL-2026-11-01-01..04` with the
  C-axis limitation.
- Ran six strategy runs: v3/v4/v5/v6 on BTCUSD and v6 on ETHUSD/XAUUSD.
- No candidate met the green criteria. The only positive cell was BTCUSD v6
  +2.62% with 5 trades; ETHUSD and XAUUSD v6 were negative.
- No strategy baseline was accepted.

## Rejected suggestions

- Rejected: calling a C axis a validated edge.
- Rejected: accepting a strategy baseline without OOS green criteria.
- Rejected: changing the model formula to make the baseline pass.

## Verification

- `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` — 10 pass.
- Strategy comparison: `docs/improvement/2026-11-01/STRATEGY-001-comparison.md`.
- Gates: fmt, clippy, test, doc, links, terms pass.

## Limitations

- Same-session review, not an independent Quant Validator record.
- The strategy comparison is a validation sample, not a production claim.
