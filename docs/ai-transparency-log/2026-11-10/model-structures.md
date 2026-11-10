# AI Transparency Log — Model Structure Extensions

- Date: 2026-11-10 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash
- Work item: [`model-structures`](../../execution-plans/2026-11-10/model-structures.md)
- Claim type: model implementation; no strategy or edge claim.

## Intent and boundary

Add the missing structural measurements requested after the 2018–2019
annotation review, and make them available to the annotation analysis. This entry
records the model layer only. No strategy rule was changed.

## Implemented

- `pivot_sequence` (indicator 47): lower-high, lower-low, higher-high, and
  higher-low counts from confirmed major pivots, with `signal.lower_high_sequence`
  and `signal.lower_low_sequence` (codes 147–148, source 55).
- `fibonacci` (indicator 48): causal retracement levels for the last confirmed
  major leg, fixed 0.236/0.382/0.5/0.618/0.786 ratios, position, nearest ratio,
  nearest ATR distance, and `level.fib_test` (code 149, source 56).
- `horizontal_band` (indicator 49): deterministic clustering of confirmed major
  levels, band edges, center, touches, age, width, position, and `level.band_test`
  (code 150, source 57).
- Registry `2026-11-10.1`: 49 indicators, 150 events, 106 parameters; sources
  55–57; model identity `M1.2`.
- `ExtendedReadings`, `ExtendedConfig`, model events, model bundle readings CSV,
  provenance, spec-v2 extended parsing, and annotation exports carry the new
  columns (121 readings columns).
- Golden readings and event fixtures regenerated deliberately; the diff was
  reviewed for the new rows.
- Row-level indicator and event contracts, availability, causality,
  determinism, uniqueness, and edge tests.
- Model contract `docs/model/2026-11-10/model-m1.2.md`; registry, validation,
  verification matrices, and model-family documentation updated.
- `StrategyPosition` excursion fields: entry price, entry ATR, unrealized ATR,
  MFE ATR, MAE ATR, and `giveback_atr()`.
- Execution-engine excursion tracking and a causal giveback exit in the
  event-composite strategy, with an arming threshold and a `giveback` exit
  reason. The strategy still receives no cash, equity, P&L, fill, order, or
  quantity.

## Verification

- `cargo test -p quantrule-model` — pass.
- `python3 tools/check_registry.py` — 49 indicators, 150 events, 106 parameters.
- `python3 tools/check_validation.py` — 199 claims for 49 indicators and 150
  active events.
- `python3 tools/check_verification.py` — pass.
- `python3 tools/check_verification_matrix.py` — 49 indicator rows and 150 event
  rows.
- `python3 tools/check_event_emitters.py` — 150 codes covered.
- 2018 backtest regenerated with the new model: 11 trades, 121 readings columns.
- 2019 backtest regenerated with the new model: 23 trades, 121 readings columns.
- 2018 annotation contexts re-exported: 595 contexts, 121 columns, including
  `lower_high_count`, `fib_618`, and `band_upper`.
- 2019 annotation contexts re-exported: 1,025 contexts, 121 columns, including
  the same new fields.
- Existing 2018 and 2019 trades are unchanged: 11/11 and 23/23, with identical
  entry/exit times, prices, P&L, and exit reasons. The new measurements did not
  alter the recorded strategy behavior.
- `cargo test --workspace` — pass; `cargo clippy --all-targets` — no warnings;
  `cargo fmt --all -- --check` — pass.
- Repository checks — all pass, including registry, validation, verification
  matrix, event emitters, research coverage, links, terms, and legacy scan.

## Not claimed

- No model edge or predictive validation result. All new signal claims are
  `predictive_status: pending`.
- No strategy behavior change, no trailing stop, and no position-excursion
  contract yet.
- No 2024 access.
- No claim that the new events are profitable.
