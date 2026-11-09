# AI Transparency Log — SR9 Model States

- Date: 2026-11-09 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: `sr9-model-states`
- Plan: [`sr9-model-states.md`](../../execution-plans/2026-11-09/sr9-model-states.md)
- Programme: `SR9-15M-STRUCTURE`
- Status: implemented; model states published; no strategy run
- Claim type: model measurement; no strategy or edge claim

## Intent and boundary

Expose the existing `vol_regime` and `structure_state` extended readings as
strategy-readable context states. The change is additive: no event formula,
event identity, or event value changes. No strategy rule or backtest is part of
this work item.

## Implementation

- Added both states to `ModelStates` and `FlatStates`.
- Mapped the extended `vol_regime` and `structure_state` values to exact i8
  codes; an unexpected value becomes null.
- Added `SpecStateField` variants and bounds.
- Added the strategy `StateField` variants and value mapping.
- Read both columns from `readings.csv` in the strategy bundle; both are
  optional for old bundles.
- Published both states in the JSON report and the text latest table.
- Updated the model spec, strategy config, README, and changelog records.
- Added mapping, causality, determinism, gap, zero-volume, equal-high/low,
  parser, spec, and bundle coverage.

## Verification and evidence

| Check | Observed |
|---|---|
| `cargo test -p quantrule-model` | pass |
| `cargo test -p quantrule-backtest` | pass |
| `cargo test -p quantrule-shell` | pass |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo doc` with warnings denied | pass |
| Python improvement tests | pass |
| Repository checks | pass |
| Existing event values | unchanged |

## Learning

- Both states are exact discretisations of already-tested extended readings;
  the new layer is a publication and boundary change, not a new formula.
- The `readings.csv` already carried both columns, so the strategy bundle can
  read them without a CSV schema change.
- An old bundle without the columns remains readable.

## Rejected or corrected

- Did not invent new thresholds or fitted parameters.
- Did not round an unexpected code; it maps to null.
- Did not change any event computation.
- Did not call the states predictive or directional.
- Did not claim independent Quant Validator review; the verification is a
  sequential same-agent pass.
