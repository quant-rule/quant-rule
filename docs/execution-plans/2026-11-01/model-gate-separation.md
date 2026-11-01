# Execution Plan: Model-Gate Separation

- Date: 2026-11-01 (UTC)
- Status: **Approved** — human approved the model-gate separation and continuation.
- Work item: `model-gate-separation`
- Prior evidence: [`MODEL-2026-11-01-04`](../../improvement/2026-11-01/MODEL-2026-11-01-04-events.md)
- AI transparency log: [`docs/ai-transparency-log/2026-11-01/model-gate-separation.md`](../../ai-transparency-log/2026-11-01/model-gate-separation.md)

## 1. Intent

The first four model runs tested whether each axis or event is a standalone
predictor. None is stable across instruments. That is a strategy-edge question,
not a model-correctness question. This change separates the two gates:

- **Model correctness baseline:** formulas, causality, determinism, availability,
  coverage, and descriptive review. A model can be correct without being a
  standalone predictor.
- **Strategy edge gate:** the full cost-inclusive OOS criteria. The strategy run
  is where predictability is judged.

The signal-usability grades remain: A/B/C describe how safe an axis is as a
standalone signal; C axes may be used only as explicit strategy hypotheses,
with the limitation recorded. D still blocks.

## 2. Scope

Included:

- Update the model evidence protocol and Improvement Workflow contract.
- Record the model correctness baseline with the four evidence runs and their C
  grades.
- Run the first controlled strategy comparison v3/v4/v5/v6 on 2024 validation.
- Update the baseline registry, index, audit follow-up, and AI log.

Excluded:

- Changing the model formula.
- Accepting a strategy baseline without OOS green criteria.
- Claiming an edge from a C axis.

## 3. Acceptance criteria

- The model baseline row names the evidence runs and the C-axis limitation.
- The strategy run freezes the model and portfolio and compares four candidates.
- No strategy is called green without the pre-registered criteria.
- Gates and docs pass.

## 4. Verification

| Check | Command | Expected |
|---|---|---|
| Docs | link and term checks | pass |
| Tool tests | `python3 -m unittest discover -s tools/improvement` | pass |
| Strategy matrix | four candidate runs | completed; comparison table |
| Gates | fmt, clippy, test, doc | pass |

## 5. Stop conditions

- A C axis is presented as a validated edge: stop.
- A strategy passes in-sample but not OOS: not green.
- A program defect appears: route to Audit Workflow.
