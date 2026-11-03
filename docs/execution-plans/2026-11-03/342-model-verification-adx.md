# Execution Plan: Model Verification, v3 Config, ADX/DI, and Predictive Validation

- Date: 2026-11-03 (UTC)
- Status: **Approved**
- Epic: [#342](https://github.com/quant-rule/quant-rule/issues/342)
- Child: [#346](https://github.com/quant-rule/quant-rule/issues/346)
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-03/342-model-verification-adx.md`](../../ai-transparency-log/2026-11-03/342-model-verification-adx.md)
- Claim type: model correctness and validation evidence; no strategy or edge claim.

## 1. Intent

Prove that every active indicator and event is calculation-correct before claiming
that it predicts anything. Separate numerical correctness, construct validity, and
predictive validity. Add the ADX/DI family and model v3 configuration only after
the verification matrix exists. Stop before strategy or portfolio work.

## 2. Current state

- Model v3 flat report and MACD are implemented; the v3 config parser and axis
  removal are pending.
- The registry has eight indicators and fourteen events. ADX/DI is absent.
- The claim harness hardcodes twelve event names and five state columns.
- The model gate returned no edge: 48 development cells were grade C and one
  validation cell reached B without development replication.
- The board has three open issues, one stale in-progress item, seven merged PRs
  with `State = Open`, and one issue without `Item type`.

## 3. Scope

Included:

- Board reconciliation and issue disposition.
- Registry validation metadata and a registry-driven verification matrix.
- Known-answer, synthetic-regime, causality, determinism, availability, and
  edge-case tests for every active indicator and event.
- Model v3 config for readings, active flags, per-timeframe overrides, and ADX
  parameters, with effective-config provenance.
- ADX/DI readings, states, event, registry entries, report/CSV fields, and tests.
- Registry-driven claim validation with path outcomes, matched baselines,
  negative controls, and multiple-testing correction.
- A model verification report and a no-edge or model-baseline decision.

Excluded:

- Strategy v8 and every strategy or portfolio change.
- Axis removal from the model output or strategy input.
- Threshold changes after a validation result.
- New data sources or partitions.

## 4. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| #349 strategy v8 | Discard for now | Reopen only after a model baseline passes |
| #346 | Model-only v3 config | Axis removal moves out of this program |
| #342 | Repurpose as the verification epic | Close when its children are done |
| ADX role | Context, trend strength | ADX never emits a standalone directional claim |
| DMI role | Signal, direction and DI cross | DI supplies direction to the ADX regime |
| Validation | Fast loop plus one-shot slow loop | Correctness can be retested; validation cannot be looped |
| Axis removal | Out of scope | Current strategy keeps its input; no strategy work |

## 5. Work breakdown

| Phase | Work | Paths |
|---|---|---|
| 0 | Board reconciliation | GitHub project and issues |
| 1 | Registry validation metadata and checker | `docs/registry/`, `tools/check_registry.py` |
| 2 | Fast verification matrix | `crates/quantrule-model/src/`, `crates/quantrule-model/tests/` |
| 3 | Model v3 config | `crates/quantrule-model/src/input.rs`, `crates/quantrule-shell/src/config/model.rs` |
| 4 | ADX/DI | `crates/quantrule-model/src/readings/`, report, shell artifacts |
| 5 | Predictive harness | `tools/improvement/claim_validation.py` |
| 6 | Model run and report | `docs/model/`, `docs/improvement/`, `data/improvement/` |

## 6. Verification

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo doc --workspace --no-deps`
- `python3 tools/check_registry.py`
- `python3 tools/check_links.py`
- `python3 tools/check_terms.py`
- `python3 -m unittest discover -s tools/improvement`

## 7. Acceptance criteria

- Every active indicator and event has a registry validation spec.
- Every active indicator and event passes the fast loop or has a recorded defect.
- ADX/DI is registered, computed, reported, and covered by synthetic-regime and
  edge-case tests.
- The v3 config parses, records the effective config, and preserves current
  defaults.
- The claim harness enumerates the registry and grades every active event.
- The model decision is recorded as no edge or a model baseline candidate.
- The board has no stale status, missing `Item type`, or unexplained open item.

## 8. Risks and stop conditions

- A calculation defect invalidates affected runs; record it and re-run after the
  fix.
- No predictive edge is a valid result; do not tune thresholds.
- Axis removal requires strategy work and is out of scope.
- Any new dependency or partition change stops the work for approval.

## 9. Execution evidence (2026-11-03)

- Fast loop: `cargo test --workspace` passes; `cargo clippy --all-targets -- -D
  warnings` passes; `cargo fmt --all -- --check` passes; `cargo doc --workspace
  --no-deps` passes.
- Registry: 10 indicators, 16 events, 55 parameters; `check_validation.py`
  reports 25 claims for every active indicator and event.
- ADX/DI: implemented with `adx`, `di_plus`, `di_minus`, `adx_regime`,
  `adx_slope`, and `di_state`; `signal.di_cross` and
  `signal.trend_continuation` are registry events.
- Claim harness: enumerates active registry events and keys Signal events by
  source; grades `di_state`.
- Pilot: BTCUSD 2018-01, 5m/15m/1h, ADX threshold 25 and 20; both `no_edge`.
  Recorded in `docs/model/2026-11-03/adx-verification.md`.

Remaining gap: the v3 parser covers the scalar readings, active flags, ADX
parameters, and per-timeframe overrides. The `events` and `progression`
parameter blocks are still fixed by their detectors and are rejected by the v3
parser rather than silently ignored. The full three-instrument `model_dev`
matrix and the one-shot `model_validation` run are also not executed in this
change. The pilot is not a baseline and does not authorise a strategy run.
