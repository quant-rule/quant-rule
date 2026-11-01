# AI Transparency Log — Event Evidence 004

- Date: 2026-11-01 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: `event-evidence-004`
- Plan: [`docs/execution-plans/2026-11-01/event-evidence-004.md`](../../execution-plans/2026-11-01/event-evidence-004.md)
- Claim type: model evidence; no edge claim

## Intent and boundary

Add event-level confidence for Level, Divergence, and Signal records and run
`MODEL-2026-11-01-04` on the same 2021 partition. The model formula, strategy,
and portfolio stay frozen. The event view is evaluation only.

## Work

- Added `event_confidence.py`, which joins events at `available_bar` and grades
  Level, Divergence, and Signal directions over the horizon grid.
- Added three deterministic unit tests.
- Ran MODEL-2026-11-01-04 on six-week 2021 slices for BTCUSD, ETHUSD, and
  XAUUSD at 15m and 1h.
- Four of 300 buckets are positive, all BTCUSD level cross/test. They do not
  repeat on the other instruments.
- No model, strategy, or portfolio baseline was accepted.

## Rejected suggestions

- Rejected: grading `occurred_bar` instead of `available_bar`. The former would
  leak knowledge before it was knowable.
- Rejected: choosing an event subtype after the result and calling it an edge.

## Verification

- `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` — 10 tests pass.
- `MODEL-2026-11-01-04` run store — completed.
- Event JSON: `artifacts/events/`.

## Limitations

- Same-session review, not an independent model judgment.
- Short slices are required by the event command's full-window cap.
