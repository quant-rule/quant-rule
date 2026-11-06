# Indicator and Event Verification Report

- Date: 2026-11-06 (UTC)
- Branch: `test/indicator-event-verification`
- Plan: [`docs/execution-plans/2026-11-06/indicator-event-verification.md`](../../execution-plans/2026-11-06/indicator-event-verification.md)
- Claim type: verification evidence map; no model behavior change and no edge claim

## 1. Result

`docs/registry/verification.json` now maps every active indicator and event to
the eleven evidence classes. The matrix documents are generated from it.

| Scope | Cells | Meaning |
|---|---:|---|
| `row` | 67 | The test names the row's indicator or event |
| `family` | 1376 | The test exercises the family that contains the row |
| `tool` | 332 | A repository gate covers every row |
| `n/a` | 304 | The class does not apply |

The base EMA/RSI/ATR contracts and the existing MACD/ADX/DMI/context/progression tests are row-level; the large `family` count is the honest result: the existing suites exercise
families, not one bespoke test per row. The matrix marks these cells `◐` and the
status `partial (family)`, not `verified`.

## 2. What is row-level

- Event emitter coverage is row-level through `tools/check_event_emitters.py`:
  all 143 registered codes are directly referenced or family-computed.
- Registry math is row-level through `check_registry.py` and
  `check_validation.py`.
- 33 formula/construct cells are row-level because the test name contains the
  row identifier; the rest use a family construct test and are marked `◐`.

## 3. Checker

`tools/check_verification.py` rejects:

- a missing indicator or event;
- a missing evidence class;
- an unknown scope;
- a missing Rust test function;
- a `row` scope whose test does not name the row;
- an unknown tool gate;
- an `n/a` class that carries tests.

`tools/build_verification.py --write` regenerates the manifest and matrices.

## 4. Remaining work

The family-level cells need either a row-specific test or an explicit decision
to accept family-level coverage for that class. The matrix is now truthful about
the difference; it no longer presents family checks as per-row proof.

Predictive claims remain mostly `pending`; no edge is claimed and
`model_validation` was not opened.
