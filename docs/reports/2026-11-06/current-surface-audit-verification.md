# Current-Surface Audit Verification

- Date: 2026-11-06 (UTC)
- Branch: `fix/current-surface-audit-remediation`
- Audit: [`docs/audits/2026-11-06/current-surface-audit.md`](../../audits/2026-11-06/current-surface-audit.md)
- Plan: [`docs/execution-plans/2026-11-06/current-surface-audit-remediation.md`](../../execution-plans/2026-11-06/current-surface-audit-remediation.md)
- Claim type: audit remediation and verification evidence; no edge claim

## 1. Verdict

The ten current audit findings are implemented on this branch with regression
evidence. The build and contract gates are green. The findings stay `Open` in
the audit document until the branch is merged; `Fixed` is a post-merge status
and `Verified` is the independent Improvement Verifier pass.

The **per-indicator and per-event test matrix is not fully filled**. The 46-row
and 143-row matrices exist and are checked for coverage of the registry, but the
`F/C/D/G/E/A/N/P/M` cells are target boxes. Current evidence remains
family-level. This is recorded as the residual gap, not hidden behind a 37/37
claim.

## 2. Finding closure

| ID | Finding | Branch evidence | Status |
|---|---|---|---|
| AUD-2026-11-06-01 | Duplicate CSV columns | `readings_csv_headers_are_unique`; generated bundle has 97 unique headers | Implemented |
| AUD-2026-11-06-02 | Custom EMA pairs dropped | `custom_ema_periods_map_pairs_to_the_configured_roles`; custom CLI run emits 28 code-9 events | Implemented |
| AUD-2026-11-06-03 | `session_code` null outside | `outside_session_is_zero_and_inside_is_one`; session CSV 0/1; regenerated golden | Implemented |
| AUD-2026-11-06-04 | Direction provenance says vote | `a_direction_prediction_is_recorded_as_direction` | Implemented |
| AUD-2026-11-06-05 | Panel provenance missing | `panel_sources` path/hash/data-version assertion; changed panel changes provenance | Implemented |
| AUD-2026-11-06-06 | RSI equality crossing dropped | `a_cross_from_equality_is_a_threshold_crossing` | Implemented |
| AUD-2026-11-06-07 | RSI zone parameter drift | Registry parameter removed; `check_registry.py` and regenerated table pass | Implemented |
| AUD-2026-11-06-08 | 4h rejection untested | `model_v4_rejects_a_4h_override`; cross-config data-ladder test | Implemented |
| AUD-2026-11-06-09 | Docs contradictions | Tutorial, ladder, artifact, count, and checklist text corrected; link/term gates pass | Implemented |
| AUD-2026-11-06-10 | Legacy comments/gate scope | Comments rewritten; `check_legacy.py` scans 612 files including `crates/` | Implemented |

## 3. Model isolation

Two v2 specs use disjoint active event sets on the same candle source:

- `signal.rsi_threshold` only;
- `signal.ema_pair_cross` only.

`two_model_specs_with_disjoint_active_events_stay_disjoint` proves both
`events.csv` sets are non-empty, contain their declared code, and are disjoint.
This is the evidence that a model swarm can vary event selection without one
model's active set leaking into another.

## 4. Gates

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | 984 passed, 0 failed, 7 ignored |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | pass |
| `check_file_size.py` | pass; every Rust source ≤ 1,000 lines |
| `check_registry.py` | 46 indicators, 143 events, 101 parameters |
| `check_validation.py` | 189 claims; construct coverage pass |
| `check_verification_matrix.py` | 46 indicators, 143 events |
| `check_event_emitters.py` | 143 codes referenced or family-computed |
| `check_links.py` | 563 Markdown files |
| `check_terms.py` | 334 files |
| `check_legacy.py` | 612 current files, including `crates/` |
| Progression oracle | 54 tests, check pass |
| Improvement tooling | 79 tests, pass |
| Research tooling + coverage | 43 tests, pass; 15 cells |
| Spec preflight | ok; registry `2026-11-06.3` |
| `git diff --check` | pass |

## 5. Residual gaps

| Gap | Evidence | Next route |
|---|---|---|
| Per-indicator/per-event tests are target-only | `docs/model/2026-11-06/indicator-verification.md`, `event-verification.md` | Fill the 46×9 and 143×7 matrices with named tests or explicit N/A; Quant Developer + Test Engineer |
| Predictive claims are pending | `validation.json`: 100 pending, 4 no-edge, 3 pass | Research programme; no validation partition opened |
| Findings are not yet `Fixed` or `Verified` | Audit document | Merge, then Improvement Verifier |
| Legacy generic v8/v10-named strategy code remains as generic S3.x engines | `crates/quantrule-backtest/src/strategy` | Current names are generic; decide whether to delete before S3.x |
| Calendar bucket is reading-only | `session.csv`; no event | Accept as reading-only or add a spec-readable reading in a new schema |

## 6. Limits

- One sequential same-session pass; not an independent audit or Quant Validator.
- The live source path was not exercised.
- S3.x strategy/execution/portfolio/policy behavior was not implemented or
  validated.
- The audit document remains the status record; this report is evidence, not
  approval.
