# Execution Plan: Event Evidence 004 — Level, Divergence, and Signal Confidence

- Date: 2026-11-01 (UTC)
- Status: **Approved** — human approved the continuing model-evidence cycle.
- Work item: `event-evidence-004`
- Prior run: [`MODEL-2026-11-01-03`](../../improvement/2026-11-01/MODEL-2026-11-01-03-context.md)
- AI transparency log: [`docs/ai-transparency-log/2026-11-01/event-evidence-004.md`](../../ai-transparency-log/2026-11-01/event-evidence-004.md)

## 1. Intent

Close the remaining model-evidence gap: the harness grades Direction, Momentum,
and ordered axes, but not the published Level, Divergence, and Signal events.
This run measures whether an event's direction is followed by a forward move at
1, 3, 5, 10, and 20 bars. The model formula is unchanged.

## 2. Scope

Included:

- `tools/improvement/event_confidence.py`: run `events`, join each event's
  `available_bar` to the timeframe close series, and report hit rate by type,
  subtype, and horizon.
- A deterministic unit test for the event grouping.
- `MODEL-2026-11-01-04` on the same 2021 partition, strategy and portfolio
  frozen.
- Review and decision. No model formula change.

Excluded:

- Strategy or portfolio runs.
- Selecting an event subtype after the result as a validated edge.
- Grading events that have no direction.

## 3. Acceptance criteria

- Events are grouped by `type_label` and `subtype_label`; each group reports n
  and a 95% lower bound at every horizon.
- Forward returns use the event's `available_bar`, never `occurred_bar`.
- The output is deterministic.
- `MODEL-2026-11-01-04` records a verdict.

## 4. Verification

| Check | Command | Expected |
|---|---|---|
| Tool syntax | `python3 -m py_compile tools/improvement/*.py` | exit 0 |
| Tool test | `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` | pass |
| Rust gates | fmt, clippy, test, doc | pass |
| Run | `python3 tools/improvement/run.py ...` | `ok MODEL-2026-11-01-04` |
| Docs | link and term checks | pass |

## 5. Stop conditions

- A program defect in event availability: route to the Audit Workflow.
- A subtype is selected after the result as a validated edge: stop.
