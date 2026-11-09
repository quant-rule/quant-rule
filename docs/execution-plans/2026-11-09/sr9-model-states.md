# SR9 Model States — `vol_regime` and `structure_state`

- Date: 2026-11-09 (UTC)
- Status: **Approved; implemented in this work item**
- Programme: `SR9-15M-STRUCTURE`
- Parent programme: `SR9-15M-STRUCTURE`
- Claim type: model measurement; no strategy or edge claim

## 1. Intent

Expose two measured regimes to strategies: the persistent volatility regime and
the confirmed major structure direction. Both are already computed as extended
readings. The change adds them to the published discrete states so a strategy
can read them as context, with no new thresholds and no change to existing
event values.

## 2. Current state

- `vol_regime` is computed by `extended.structure` as 0 compressed, 1 normal,
  2 expanded from `ATR14 / ATR_slow` with thresholds 0.7 and 1.3. It is
  published in `readings.csv` but is not in `ModelStates`.
- `structure_state` is computed by `extended.structure` as +1 HH/HL, -1 LH/LL,
  0 mixed from consecutive confirmed major pivots. It is published in
  `readings.csv` but is not in `ModelStates`.
- The strategy parser, model spec, and bundle currently expose eight states and
  the progression states. `rvol_regime` is the precedent for a context state
  added to the same surface.
- No strategy uses these states yet. No existing event value changes.

## 3. Scope

Included:

- Add `vol_regime` and `structure_state` to `ModelStates`.
- Populate them from the existing extended readings with exact-code validation.
- Add them to `FlatStates`, JSON, text reporting, model spec, strategy parser,
  and the strategy bundle.
- Add model contracts and registry references.
- Add threshold, mapping, causality, determinism, golden, gap, zero-volume, and
  edge-case tests, plus parser and spec tests.

Excluded:

- New thresholds or indicator formulas.
- A composite structure-phase code.
- Strategy rules or backtests.
- Any change to an existing event value or event identity.

## 4. Decisions

| ID | Decision | Consequence |
|---|---|---|
| D1 | `vol_regime` is a context state, codes 0/1/2 | Strategies may filter on volatility |
| D2 | `structure_state` is a context state, codes -1/0/+1 | Strategies may filter on confirmed structure |
| D3 | Both states are pass-throughs of the existing extended readings | No new formula, no fitted parameter |
| D4 | An unexpected extended code maps to null, never a rounded value | Missing is null, not zero |
| D5 | Existing event values and identities are unchanged | No re-baseline of event records |

## 5. Work breakdown

| Step | Work | Paths |
|---|---|---|
| 1 | Add the fields and mapping | `crates/quantrule-model/src/timeframe.rs` |
| 2 | Publish them in the flat report | `crates/quantrule-model/src/report/flat.rs` |
| 3 | Add the spec fields and bounds | `crates/quantrule-model/src/spec.rs` |
| 4 | Add the strategy state fields | `crates/quantrule-backtest/src/strategy/event.rs` |
| 5 | Read them from the bundle | `crates/quantrule-shell/src/strategy/bundle.rs` |
| 6 | Render JSON and text | `crates/quantrule-shell/src/render_report/` |
| 7 | Update the spec parser | `crates/quantrule-shell/src/spec.rs` |
| 8 | Update contracts and docs | `docs/model/2026-11-09/`, `docs/config/2026-11-06/strategy-config.md`, `docs/model/2026-11-05/model-spec-v1.md` |
| 9 | Tests and verification | model, backtest, shell, repository checks |

## 6. Acceptance criteria

- `states_at(bar).vol_regime` equals the extended reading code at every bar.
- `states_at(bar).structure_state` equals the extended reading code at every
  bar.
- Both states are nullable and never replaced by zero.
- The strategy parser accepts `vol_regime` in 0..=2 and `structure_state` in
  -1..=1 and rejects out-of-range values.
- The model spec accepts both states and evaluates without error.
- The bundle reads both columns; an old bundle without them still loads.
- Existing model, backtest, and shell tests pass; no event value changes.
- Causality, determinism, golden, gap, zero-volume, and equal-high/low cases
  are covered.
- `cargo fmt`, `cargo test`, `cargo clippy`, `cargo doc`, Python tests, and
  repository checks pass.

## 7. Verification

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `cargo doc` with warnings denied
- `python3 -m unittest discover -s tools/improvement -p 'test_*.py'`
- Repository checks and `git diff --check`
- Quant Validator: confirm the states are additive and no event value changed.

## 8. Risks and recovery

| Risk | Mitigation |
|---|---|
| An existing event changes | The states read existing readings; no event computation changes |
| An unexpected extended code is rounded | `discrete_code` returns null for anything outside -1/0/1/2 |
| Old bundles fail to load | Both columns are optional in the bundle reader |
| Text report layout changes | The JSON and CSV contracts are the machine surfaces; the text table is updated deliberately |

## 9. Stop conditions

Stop if an existing event value changes, if a state cannot be mapped exactly, or
if the bundle contract cannot remain backward compatible.

## 10. Outcome

Implemented and verified. See the pull request and the verification table there.
