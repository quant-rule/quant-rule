# Execution Plan: Model Evidence 002 — Multi-Horizon Axis Study

- Date: 2026-11-01 (UTC)
- Status: **Approved** — human approved the model-improvement run.
- Work item: `model-evidence-002`
- Prior run: [`MODEL-2026-11-01-01`](../../improvement/2026-11-01/MODEL-2026-11-01-01-model-baseline.md)
- Protocol: [`docs/model/2026-11-01/model-evidence-protocol.md`](../../model/2026-11-01/model-evidence-protocol.md)
- AI transparency log: [`docs/ai-transparency-log/2026-11-01/model-evidence-002.md`](../../ai-transparency-log/2026-11-01/model-evidence-002.md)

## 1. Intent

The first model run graded Direction and Momentum at one fixed horizon (5 and
3 bars) and found both below 0.50 in five of six 2021 cells. That result cannot
distinguish a bad axis from a bad horizon. This run tests the pre-registered
horizon grid `1, 3, 5, 10, 20` bars and adds ordered-axis monotonicity for
Structure and Progression. The model formula is unchanged; only the evidence
harness is extended.

## 2. Scope

Included:

- `tools/improvement/model_confidence.py` multi-horizon hit rates and ordered
  bucket monotonicity.
- A deterministic unit test for the new evidence math.
- `MODEL-2026-11-01-02` on the same 2021 partition, with strategy and portfolio
  frozen.
- A run review and decision; no model formula change in this run.

Excluded:

- Changing the model formula, axes, events, or strategy thresholds.
- Strategy or portfolio runs.
- Post-hoc selection of the best horizon as a validated edge.

## 3. Acceptance criteria

- The harness reports Direction and Momentum hit rates at every pre-registered
  horizon, with n, coverage, and a 95% lower bound.
- Structure and Progression report ordered buckets at every horizon plus a
  rank-monotonicity value.
- The output is deterministic for the same source and timeframe.
- `MODEL-2026-11-01-02` runs through the store and records a verdict.
- No model change is made in this run; a positive horizon is a new hypothesis,
  not a validated edge.

## 4. Verification

| Check | Command | Expected |
|---|---|---|
| Tool syntax | `python3 -m py_compile tools/improvement/*.py` | exit 0 |
| Tool test | `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` | pass |
| Rust gates | fmt, clippy, test, doc | pass |
| Run | `python3 tools/improvement/run.py ...` | `ok MODEL-2026-11-01-02` |
| Determinism | repeat the confidence cell | identical JSON |
| Docs | link and term checks | pass |

## 5. Stop conditions

- The harness changes a model output or a strategy decision: stop.
- A horizon is selected after seeing the result as a validated edge: stop.
- The run finds a program defect: route to the Audit Workflow.
