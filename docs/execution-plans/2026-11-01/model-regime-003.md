# Execution Plan: Model Regime 003 — Volatility-Conditioned Axis Evidence

- Date: 2026-11-01 (UTC)
- Status: **Approved** — human approved the continuing model-evidence cycle.
- Work item: `model-regime-003`
- Prior run: [`MODEL-2026-11-01-02`](../../improvement/2026-11-01/MODEL-2026-11-01-02-multi-horizon.md)
- Protocol: [`docs/model/2026-11-01/model-evidence-protocol.md`](../../model/2026-11-01/model-evidence-protocol.md)
- AI transparency log: [`docs/ai-transparency-log/2026-11-01/model-regime-003.md`](../../ai-transparency-log/2026-11-01/model-regime-003.md)

## 1. Intent

The multi-horizon study rejected the “wrong horizon” explanation. Direction and
Momentum fail almost everywhere. This run asks whether they work inside the
model's own volatility/participation context: `context.atr_ratio`,
`context.rvol_ratio`, and `context.code`. The model formula is unchanged; the
evidence harness adds pre-registered regime buckets.

## 2. Scope

Included:

- Extend `model_confidence.py` with regime-conditioned hit rates at the primary
  horizons.
- Add a deterministic unit test for bucket assignment.
- Run `MODEL-2026-11-01-03` on the same 2021 partition with strategy and
  portfolio frozen.
- Record review and decision. No model or strategy change in this run.

Excluded:

- Changing the model formula or adding a new reading.
- Strategy or portfolio runs.
- Selecting a regime bucket post-hoc as a validated edge.

## 3. Regime buckets

| Field | Buckets |
|---|---|
| `context.atr_ratio` | `< 0.8`, `0.8..1.2`, `> 1.2` |
| `context.rvol_ratio` | `< 1.0`, `1.0..2.0`, `> 2.0` |
| `context.code` | published integer code |

## 4. Acceptance criteria

- Direction and Momentum report hit rate, n, and lower bound per regime bucket at
  the primary horizon (5 and 3 bars).
- The output is deterministic and uses no random split.
- `MODEL-2026-11-01-03` records a verdict.
- A positive bucket is a new hypothesis, not a baseline.

## 5. Verification

| Check | Command | Expected |
|---|---|---|
| Tool syntax | `python3 -m py_compile tools/improvement/*.py` | exit 0 |
| Tool test | `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` | pass |
| Rust gates | fmt, clippy, test, doc | pass |
| Run | `python3 tools/improvement/run.py ...` | `ok MODEL-2026-11-01-03` |
| Docs | link and term checks | pass |

## 6. Stop conditions

- The harness changes a model output or strategy decision: stop.
- A regime bucket is selected after the result as a validated edge: stop.
- A program defect appears: route to the Audit Workflow.
