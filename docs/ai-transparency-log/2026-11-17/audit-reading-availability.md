# AI transparency log — Reading availability and session VWAP registration (P2.6, AUD-50/51)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 3
  (Execution) for `AUD-2026-11-17-50` and `AUD-2026-11-17-51`.
- Claim type: Behaviour Claim.
- Intent: stop ATR from gating readings whose formulas do not use ATR, and
  register the `session_vwap` reading that `session.csv` publishes and
  `level.session_vwap_test` consumes.
- Boundary: branch `fix/audit-reading-availability`, based on current `main`
  `7ae3a984`. No push, PR, merge, or board change. The Quant Validator pass is
  sequential by the same agent, not independent.

## What changed

- `extended.rs` now gates only `vwap_distance_atr` on ATR. `vwap_band_upper`,
  `vwap_band_lower`, and `vwap_slope` are computed from VWAP/MAD alone and
  publish when ATR is inactive.
- `structure.rs` computes `range_expansion = (high - low) / mean_range` without
  requiring ATR; the mean must still be positive.
- `docs/registry/indicators.json` adds `session_vwap` to the `session`
  indicator's outputs, formula, inputs, and availability, and the generated
  `indicators.md` is regenerated.
- `session_row_contract` now compares `session_vwap` against the session golden
  fixture, and `london_session_and_opening_range` asserts the first session's
  VWAP value, so the registry construct evidence covers the output.

## Verification observed

| Command | Result |
|---|---|
| `python3 tools/check_registry.py` | 49 indicators, 150 events, 100 parameters |
| `python3 tools/check_validation.py` | 199 claims for 49 indicators and 150 active events |
| `python3 tools/check_verification.py` | 49 indicators, 150 events; row/family/tool coverage complete |
| `cargo test -p quantrule-model --lib non_atr_vwap_outputs` | 1 passed, 0 failed |
| `cargo test -p quantrule-model --test per_row_session_indicators session_row_contract` | 1 passed, 0 failed |
| `cargo test -p quantrule-model --lib london_session_and_opening_range` | 1 passed, 0 failed |
| `cargo test -p quantrule-model --lib --tests` | all suites pass |
| `cargo test -p quantrule-app --lib artifact::model_bundle::artifacts` | 4 passed, 0 failed |
| `cargo fmt --all -- --check` | pass |
| `tools/gate.sh` | pass: 2,322 passed, 0 failed, 16 ignored; 25 checkers |

## Quant validation (sequential, not independent)

- **Expected invariant:** a reading is null only when its own formula is
  unavailable (`docs/registry/README.md`). A reading that appears in an
  artifact and feeds an event is listed in the registry (`AUD-2026-11-17-51`).
- **Observed before:** the VWAP loop `continue`d when ATR was absent, so bands
  and slope were null without ATR; `range_expansion` required
  `atr_fast > 0.0`; the `session` indicator listed only `session_code` and
  `session_age`.
- **Observed after:** with `atr_active = false`, the test at bar 4 shows
  `vwap_distance_atr = None` while the bands, slope, and range expansion are
  `Some`; the registry lists `session_vwap`, and the row contract compares it
  bit-for-bit against the golden fixture.
- **Default output:** ATR is active by default, so the model and event goldens
  are unchanged; no fixture regeneration is needed.
- **Interaction with P2.1:** P2.1 makes a missing session volume null the
  session VWAP. The registry availability added here states the necessary
  `sum(V,session) > 0` condition; the merged behavior is the stricter P2.1
  rule. No accepted baseline uses a missing-volume session.
- **Residual risk:** a run with ATR disabled now publishes VWAP bands, slope,
  and range expansion it previously omitted. That is the registered
  availability; a consumer that treated the old null as meaningful will see a
  value.
- **Independence:** the Quant Researcher/Developer/Validator roles were
  performed sequentially by the same agent. This is not an independent
  validation and is not claimed as one.

## Learning

- The ATR guard was shared by outputs with different dependencies; splitting the
  guard is the correct fix, not removing ATR from the distance.
- The registry gap was invisible to the registry checker because the checker
  validates shape and codes, not the artifact header. The row contract is the
  test that ties the published column to the registered output.
- `session_vwap` is session-anchored, so its availability is a session-level
  condition rather than a per-window condition.
