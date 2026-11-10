# M1.3 Quant Validation — Sequence and Fibonacci Corrections

- Date: 2026-11-10 (UTC)
- Status: Sequential Quant Validator pass; not an independent review; PR #579 open
- Model: `M1.3`
- Registry: `2026-11-10.2`
- Baseline: `dda51d8` plus `5bf7152` and `d3a6b42`; PR #579
- Audit: [`model-structures-audit.md`](../../audits/2026-11-10/model-structures-audit.md)
  (`AUDIT-2026-11-10-model-structures`)
- Plan: [`audit-model-structures-remediation.md`](../../execution-plans/2026-11-10/audit-model-structures-remediation.md)
- Claim type: validation of a model behavior correction; no edge, profitability,
  or strategy claim.

## 1. Scope

Validate the `M1.3` corrections for:

- `AUD-2026-11-10-01`: sequence-event occurrence and availability;
- `AUD-2026-11-10-02`: `level.fib_test` range intersection and multiplicity;
- `AUD-2026-11-10-03`: the new formula, availability, edge, and property tests;
- `AUD-2026-11-10-04`: the fixed `fib_test_atr` / `band_atr` config behavior.

The band event definition is unchanged, so `AUD-2026-11-10-06` is a tooling
rename, not a model behavior change. `AUD-2026-11-10-05` is this record;
`AUD-2026-11-10-07`–`09` are documentation, contract, and hygiene fixes with no
model value change.

## 2. Inputs and configuration

| Item | Value |
|---|---|
| Candle fixture | `crates/quantrule-model/tests/fixtures/readings_golden.csv`, 260 bars, 300 s |
| Model config | `ExtendedConfig::DEFAULT`; `ReadingsConfig::DEFAULT` |
| Event config | `ExtendedEventConfig::from_secs(3600)` in the unit tests; `300` in the model fixture |
| Data partition | repository fixture only; no 2024, validation, or holdout access |
| Seed/randomness | none |
| Code state | `fix/audit-m12-remediation` at `d3a6b42`; PR #579 |

## 3. Observed results

### 3.1 Sequence occurrence and availability

Fixture scan of `extended_events_golden.csv` before and after the fix:

| Event | Before occurred / available | After occurred / available |
|---|---|---|
| 147 | 59 / 59 | 54 / 59 |
| 148 | 83 / 83 | 78 / 83 |
| 147 | 175 / 175 | 170 / 175 |
| 147 | 211 / 211 | 206 / 211 |
| 148 | 228 / 228 | 223 / 228 |
| 147 | 242 / 242 | 237 / 242 |

`event_validation::sequence_events_record_the_pivot_occurrence_and_availability`
passes and resolves the confirming major pivot for each event.

### 3.2 Fibonacci events

| Measure | Before | After |
|---|---:|---:|
| Code 149 events in the fixture | 164 | 433 |
| Code 150 events in the fixture | 65 | 65 |

The bar-33 case (close `99.44`, `0.236` level `99.4416`) now emits code 149 with
`reference_code=236` and `reference_value=99.4416`. The unit tests
`a_fibonacci_level_inside_the_bar_range_is_tested`,
`every_tested_fibonacci_level_emits_an_event`, and
`a_fibonacci_level_outside_the_bar_range_is_not_tested` pass. The band event
count is unchanged, which is the expected result of a Fibonacci-only correction.

### 3.3 Readings and availability

The extended readings fixture is unchanged; the row contracts pass. The
availability tests pass for all three indicators:

- `pivot_sequence_availability_starts_at_two_confirmed_pivots`
- `fibonacci_availability_starts_at_both_confirmed_anchors`
- `horizontal_band_availability_starts_at_two_confirmed_levels`

The Fibonacci and band formula tests pass, including the zero-ATR band case,
the Fibonacci missing-ATR distance case, the zero-span Fibonacci leg, the band
largest/nearest/lower-center tie-break, the two-level minimum, and equal-level
zero-width clusters.

### 3.4 Config behavior

Spec v2 rejects `readings.extended.fib_test_atr` and
`readings.extended.band_atr`; the two new parser tests pass. The model's
`extended_is_valid` also checks both values are in `0.0..=2.0` for programmatic
callers, and `extended_fixed_tolerances_are_range_checked` passes.

### 3.5 Determinism and causality

The existing `event_validation` determinism, prefix-causality, availability, and
one-occurrence tests pass on the new fixture. The model tests use no clock,
randomness, I/O, or unordered iteration.

## 4. Gates

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets` | no warnings |
| `cargo test --workspace` | pass |
| `cargo doc --workspace --no-deps` | no warnings |
| `python3 tools/check_registry.py` | pass; 49 indicators, 150 events, 106 parameters |
| `python3 tools/check_validation.py` | pass; 199 claims |
| `python3 tools/check_verification.py` | pass |
| `python3 tools/check_verification_matrix.py` | pass |
| `python3 tools/check_event_emitters.py` | pass |
| `python3 tools/check_command_surface.py` | pass |
| `python3 tools/check_links.py` | pass |
| `python3 tools/check_terms.py` | pass |
| `python3 tools/check_changelog.py` | pass |
| `python3 tools/check_file_size.py` | pass |
| `python3 -m unittest discover -s tools/trades -p 'test_*.py'` | pass |
| `git diff --check` | pass |

## 5. Run dispositions

- `SR10` used the M1.2 endpoint `level.fib_test`; it is invalidated by the
  M1.3 correction and recorded in `docs/improvement/INVALIDATIONS.md`.
- `SR11` used the same event; it is invalidated and recorded in the same log.
- `SR12` used `level.band_test`, which is unchanged; it is retained as M1.2
  development evidence with a dated note. No rerun was performed.
- No validation or holdout partition was opened.

## 6. Limitations

- This is a sequential Quant Validator pass by the same agent that implemented
  the correction. It is not an independent agent or an independent judgment.
- The full 2018–2019 backtest and annotation-zone reports were not re-run; the
  fixture and unit/integration gates are the validation input. No strategy
  result is claimed.
- The validation was run on the PR branch at `d3a6b42`. The merge commit must
  be added before the Improvement Verifier closes the findings.
- The 2024 partition was not accessed.

## 7. Verdict

The corrected `M1.3` behavior is reproducible on the fixed fixture: sequence
events carry the pivot occurrence and confirmation availability, Fibonacci tests
detect levels inside the bar range and emit one event per tested level, band
events are unchanged, and the fixed tolerances are rejected by the spec surface.
The evidence supports marking `AUD-2026-11-10-01`–`04` `Fixed` after the
remediation PR merges; it does not support a predictive or edge claim. The
findings remain subject to the Improvement Verifier and human acceptance.

## 8. Links

- Audit: [`model-structures-audit.md`](../../audits/2026-11-10/model-structures-audit.md)
- Plan: [`audit-model-structures-remediation.md`](../../execution-plans/2026-11-10/audit-model-structures-remediation.md)
- Model contract: [`model-m1.3.md`](../../model/2026-11-10/model-m1.3.md)
- Invalidations: [`INVALIDATIONS.md`](../../improvement/INVALIDATIONS.md)
- AI log: [`audit-model-structures-remediation.md`](../../ai-transparency-log/2026-11-10/audit-model-structures-remediation.md)
