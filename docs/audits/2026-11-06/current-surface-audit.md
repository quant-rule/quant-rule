# Current-Surface Audit

- Date: 2026-11-06 (UTC)
- Audit ID: `AUDIT-2026-11-06-current-surface`
- Status: Published; all findings fixed in PR [#496](https://github.com/quant-rule/quant-rule/pull/496) at `dc82043`; independent verification pending
- Baseline: `main` at `6ec84eb`
- Scope: current shell surface, M1.0 model/event/registry contract, config
  loading, report/provenance artifacts, current docs, and gates
- Limitations: one sequential audit pass; S3.x backtest/strategy/portfolio
  internals and the live source path were not audited

## Summary

| Severity | Count |
|---|---:|
| Critical | 0 |
| Major | 5 |
| Minor | 5 |
| Info | 0 |

The pasted pre-remediation audit described a different tree. On this baseline,
event identity is by `event_code`, the missing emitters exist, multi-source
events match by code, and validation claims cover every active entry. The
remaining defects are artifact ambiguity, configurable-parameter drift,
provenance gaps, and verification/doc overclaim.

## Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| AUD-2026-11-06-01 | Major | shell/report | `readings.csv` has duplicate `atr_ratio` and `rvol_ratio` headers | `model_artifacts.rs:20`; generated bundle has 97 columns with two of each | A consumer reading by name silently gets the extended ratio instead of the context ratio | A. prefix extended columns; B. prefix context columns; C. document positional access | A | unique-header test; rerun bundle | Fixed | [#496](https://github.com/quant-rule/quant-rule/pull/496) |
| AUD-2026-11-06-02 | Major | model/config | Custom `ema_pairs` are accepted but only 20/50/200 resolve, silently dropping `signal.ema_pair_cross` | `model_v4.rs:333-367`; `events/signal.rs:282-310,505-528`; custom pairs → 0 code-9 events, default pairs → 28 | A supported config silently disables a registered signal | A. resolve against configured periods; B. role selectors; C. standalone pair series | A | custom-pair event test; default unchanged | Fixed | [#496](https://github.com/quant-rule/quant-rule/pull/496) |
| AUD-2026-11-06-03 | Major | model/spec v2 | `session_code` is null outside the session, not `0` | `sessions.rs:76,103,123-124,158`; `indicators.json:1471`; session CSV outside rows are empty | Outside-session is conflated with unavailable | A. publish 0 outside; B. change the contract to null | A | session CSV 0/1 test | Fixed | [#496](https://github.com/quant-rule/quant-rule/pull/496) |
| AUD-2026-11-06-04 | Major | shell/provenance | A direction prediction is recorded as `kind: vote` and its event list is omitted | `report_run.rs:551-560`; direction run has correct `prediction.csv`, wrong provenance | A later reader cannot reconstruct the prediction kind | A. serialize the actual kind; B. normalize both shapes | A | direction provenance test | Fixed | [#496](https://github.com/quant-rule/quant-rule/pull/496) |
| AUD-2026-11-06-05 | Major | shell/reproducibility | Panel source identity is absent from provenance | `report_run.rs:561`; changed panel data changes outputs but not `provenance.json` | A panel run is not reproducible from its record | A. record path/hash/version; B. require caller version; C. disable panel | A | changed-panel provenance test | Fixed | [#496](https://github.com/quant-rule/quant-rule/pull/496) |
| AUD-2026-11-06-06 | Minor | model | RSI threshold crossing drops a cross from equality | `events/signal.rs:254-261`; `indicator-coverage.md:46`; test `signal/tests.rs:178-185` | A crossing from 30 to 31 is never emitted | A. use `previous <=`; B. change the contract | A | `[30,31]` emits; `[29,30]` does not | Fixed | [#496](https://github.com/quant-rule/quant-rule/pull/496) |
| AUD-2026-11-06-07 | Minor | registry/model | `signal.rsi_zone_change` lists `rsi_levels` but hardcodes 30/70 | `indicators.json:1872`; `events/signal.rs:439-447` | Registry metadata overstates configurability | A. remove the parameter; B. implement configurable zone | A | registry check | Fixed | [#496](https://github.com/quant-rule/quant-rule/pull/496) |
| AUD-2026-11-06-08 | Minor | tests/docs | 4h rejection is claimed but not tested | `config/tests.rs:279-295`; `model-m1.0.md:86` | The 4h ceiling can regress silently | A. add tests; B. remove the claim | A | fail-before/pass-after | Fixed | [#496](https://github.com/quant-rule/quant-rule/pull/496) |
| AUD-2026-11-06-09 | Minor | docs | Docs contradict the tutorial and the report ladder | `docs/README.md:23-24` vs `:45`; `README.md:46-48` | Readers are told the tutorial does not exist and misread the ladder | A. correct both; B. leave history only | A | link/term check; manual read | Fixed | [#496](https://github.com/quant-rule/quant-rule/pull/496) |
| AUD-2026-11-06-10 | Minor | docs/workflow | Retired axis vocabulary remains in current Rust comments; the legacy gate misses `crates/` | `events/signal.rs:160-162`; `timeframe.rs:272-274`; `kernel.rs:214`; `progression/leg.rs:687`; `check_legacy.py:23-32` | Legacy vocabulary survives and cannot be caught | A. scan source and fix comments; B. document the narrower scan | A | legacy scan over `crates/` | Fixed | [#496](https://github.com/quant-rule/quant-rule/pull/496) |

## Detail

### AUD-2026-11-06-01 — duplicate CSV columns

The context ratios are written at positions 28–29 and the extended ratios at
46/62 under the same names. `csv.DictReader` keeps the last duplicate, so
`path_validation.py` reads the extended ATR ratio when it asks for `atr_ratio`.
The accepted M1.0 gate does not currently set `atr_ratio_max`, so the accepted
baseline is not directly invalidated, but the artifact is ambiguous.

### AUD-2026-11-06-02 — custom EMA pairs

With `ema.periods=[9,21,55]` and `ema_pairs=[[9,21],[21,55],[9,55]]`, the run
exits 0 and emits no code-9 events. With the same periods and default pairs, it
emits 28. The detector matches the literal periods 20/50/200, not the configured
series.

### AUD-2026-11-06-03 — session code

The registry formula and field doc say zero outside. The implementation leaves
the series `None` outside and sets `Some(1.0)` inside. The session events use a
private `in_session` helper, so internal events are correct; the published
`session.csv` is not.

### AUD-2026-11-06-04 — direction provenance

The v2 provenance writer hardcodes `"kind":"vote"` and writes only `long` and
`short`. A direction spec produces correct `prediction.csv` values but an
incorrect `provenance.json`.

### AUD-2026-11-06-05 — panel provenance

The spec hash records the panel paths, not the panel file contents. Two runs
with materially different panel files produce different `panel.csv` and
`prediction.csv` but byte-identical provenance.

## Remediation evidence (branch)

The fixes are merged in PR [#496](https://github.com/quant-rule/quant-rule/pull/496)
at `dc82043`. The findings are `Fixed`; independent Improvement Verifier
closure is the next status.

| ID | Branch fix | Regression evidence |
|---|---|---|
| AUD-2026-11-06-01 | Extended columns are `extended_atr_ratio`/`extended_rvol_ratio` | `readings_csv_headers_are_unique` |
| AUD-2026-11-06-02 | EMA pairs resolve against the configured periods; explicit non-role pairs are rejected | `custom_ema_periods_map_pairs_to_the_configured_roles`; custom CLI run emits code 9 |
| AUD-2026-11-06-03 | `session_code` is 0 outside and 1 inside | `outside_session_is_zero_and_inside_is_one`; session CSV assertion; regenerated golden |
| AUD-2026-11-06-04 | Provenance serializes `kind: direction` and the event list | `a_direction_prediction_is_recorded_as_direction` |
| AUD-2026-11-06-05 | Provenance records panel paths, content hashes, and data versions | `panel_sources` provenance assertion; changed-panel run changes provenance |
| AUD-2026-11-06-06 | RSI threshold uses the contract's `previous <= threshold` | `a_cross_from_equality_is_a_threshold_crossing` |
| AUD-2026-11-06-07 | `rsi_levels` removed from the zone event's parameter list | registry check; regenerated `indicators.md` |
| AUD-2026-11-06-08 | 4h model override and data-ladder rejection have tests | `model_v4_rejects_a_4h_override`; cross-config test |
| AUD-2026-11-06-09 | Tutorial, report-ladder, and checklist/status text corrected | `check_links.py`, `check_terms.py`, manual read |
| AUD-2026-11-06-10 | Legacy comments removed; `check_legacy.py` scans `crates/` | `check_legacy.py` over 612 current files |

## Gate evidence

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | 984 passed, 0 failed, 7 ignored |
| `cargo doc` | pass |
| `check_registry.py` | 46 indicators, 143 events, 101 parameters |
| `check_validation.py` | 189 claims; construct coverage pass |
| `check_verification_matrix.py` | 46 indicators, 143 events |
| `check_event_emitters.py` | 143 codes referenced or family-computed |
| `check_links.py` | 563 Markdown files |
| `check_terms.py` | 334 files |
| `check_legacy.py` | 612 files, including `crates/` |
| `check_file_size.py` | pass |
| `git diff --check` | pass |
