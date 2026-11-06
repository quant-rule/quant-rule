# Indicator and Event Verification Report

- Date: 2026-11-06 (UTC)
- Branch: `test/model-base-golden-precedence`
- Plan: [`docs/execution-plans/2026-11-06/indicator-event-verification.md`](../../execution-plans/2026-11-06/indicator-event-verification.md)
- Claim type: verification evidence map; no signal-generation change, one debug-build underflow fixed, and no edge claim

## 1. Result

`docs/registry/verification.json` now maps every active indicator and event to
the eleven evidence classes. The matrix documents are generated from it.

| Scope | Cells | Meaning |
|---|---:|---|
| `row` | 1091 | The test names the row's indicator or event |
| `family` | 329 | The test exercises the family that contains the row |
| `tool` | 332 | A repository gate covers every row |
| `n/a` | 327 | The class does not apply |

All 46 indicators are row-level for formula, causality, determinism, golden, and
edge. All 143 events are row-level for formula, causality, determinism, golden,
availability, and dedupe. The 3 base level events are also row-level for
precedence. The remaining `family` cells are event edge (143) and `spec` (186),
explicitly accepted. The matrix marks these cells `◐` and the status
`partial (family)`, not `verified`.

## 2. What is row-level

- Event emitter coverage is row-level through `tools/check_event_emitters.py`:
  all 143 registered codes are directly referenced or family-computed.
- Registry math is row-level through `check_registry.py` and
  `check_validation.py`.
- All 189 formula/construct cells are row-level. The 16 base events carry
  row-level golden, causality, determinism, availability, and dedupe evidence
  from `per_row_base_events.rs`. The 80
  extended and session events carry row-level golden, causality, determinism,
  availability, and dedupe evidence from `per_row_extended_session_events.rs`.
  The compression, VWAP divergence, profile concentration, climax, absorption,
  flow/panel divergence, candle/pattern/gap, panel cross, and session rows carry
  row-level evidence from `per_row_event_compression.rs` and the extended,
  panel, and session `row_tests` modules. All 143 events now carry row-level
  formula, causality, determinism, golden, availability, and dedupe evidence.
  The 3 base level events carry row-level precedence evidence. The remaining
  23 extended/session level events carry no pivot or EMA reference code, so
  precedence is `n/a` for them rather than a false family claim.
  All 46 indicators carry row-level formula, causality, determinism, golden, and
  edge evidence from `per_row_base.rs`, `per_row_extended_indicators.rs`,
  `per_row_panel_indicators.rs`, `per_row_session_indicators.rs`, and
  `per_row_base_remaining_indicators.rs`. The extended, session, and base-flow
  fixtures are regression locks, not independent
  oracles; the tests name that limitation in the test module.

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

## 4. Closure

Model verification is closed. The 329 remaining family cells are explicitly
accepted: event edge 143, event `spec` 143, and indicator `spec` 43. The closure
record is
[`model-verification-closure.md`](../../reports/2026-11-06/model-verification-closure.md).

Predictive claims remain mostly `pending`; no edge is claimed and
`model_validation` was not opened. Strategy, execution, portfolio, and backtest
work is the separate S3.x phase.
