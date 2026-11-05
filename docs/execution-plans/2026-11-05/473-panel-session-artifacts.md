# P5 — Panel and Session Artifacts, Formula Reconciliation

- Date: 2026-11-05 (UTC)
- Issue: [#473](https://github.com/quant-rule/quant-rule/issues/473)
- Status: **Implemented** 2026-11-05; PR pending. Approved by the human
  instruction "proceed" on 2026-11-05
- Claim type: model-output publication and formula reconciliation; one event
  behavior change
- Parent: [`467-clean-current-architecture.md`](467-clean-current-architecture.md)
- Predecessor: [`472-config-cleanup.md`](472-config-cleanup.md)
- AI transparency log: [`docs/ai-transparency-log/2026-11-05/473-panel-session-artifacts.md`](../../ai-transparency-log/2026-11-05/473-panel-session-artifacts.md)

## 1. Intent

Publish the panel and session readings that the v2 spec already computes, expose
`calendar_bucket`, and make the named indicator formulas agree with the
machine-checked registry. The only intended event behavior change is the missing
bearish mirror of `signal.vwap_reclaim`.

## 2. Current state

- `PanelReadings` and `SessionReadings` are computed inside `SpecRun::compute` but
  are discarded; only their events survive.
- The existing golden fixtures `panel_readings_golden.csv` and
  `session_readings_golden.csv` prove the readings, but no shell artifact exposes
  them.
- `signal.vwap_reclaim` emits only the bullish reclaim; the registry formula says
  `cross_up(C,VWAP) and count(C < VWAP, 6) >= 6; mirror cross_down`.
- The indicator-coverage table drifts from the registry on breadth, gap fade,
  level strength, and the lead-lag signal value. The registry is the named
  contract.

## 3. Scope

Included:

- Store `SessionReadings` and `PanelReadings` on `SpecRun` and expose them.
- Add `session.csv` and `panel.csv` to the `report model` spec bundle when the
  spec declares a session or panel.
- Publish `calendar_bucket` in `session.csv`.
- Add the bearish VWAP reclaim mirror.
- Correct the indicator-coverage table to the registry for breadth, gap fade,
  level strength, absorption, and the lead-lag signal value.
- Add formula tests for breadth, gap fade, level strength, absorption, and the
  VWAP reclaim mirror.
- Record the Quant Researcher, Quant Developer, and Quant Validator passes.

Excluded:

- Changing the panelled instrument set, panel windows, or session DST rules.
- Changing breadth to use VWAP; the registry formula is return-based.
- Changing level-strength behavior; the implementation is retained and the table
  is corrected.
- P6–P8.

## 4. Decisions

| ID | Decision | Status |
|---|---|---|
| P5-1 | The registry (`docs/registry/indicators.json`) is the named formula contract; the coverage table is corrected to it | Approved for this plan |
| P5-2 | `panel.csv` and `session.csv` are spec artifacts, present only when declared | Approved for this plan |
| P5-3 | `calendar_bucket` is published, not marked out of scope | Approved for this plan |
| P5-4 | Add the bearish VWAP reclaim mirror; no other event behavior changes | Approved for this plan |
| P5-5 | Retain the current level-strength reading (confirmed major levels within the 0.2 ATR band) and correct the table | Approved for this plan |

## 5. Work breakdown

1. **Model data.** Add the readings to `SpecRun` and accessors.
2. **Artifacts.** Write `panel.csv` and `session.csv` from the spec readings.
3. **VWAP mirror.** Add the bearish reclaim branch and its event test.
4. **Formula tests.** Add explicit arithmetic tests for breadth, gap fade, level
   strength, absorption, and the reclaim mirror.
5. **Documentation.** Update the indicator-coverage table, the v2 spec artifact
   table, the README bundle table, and the history/plan records.
6. **Verification.** Run Rust, registry, validation, link, term, and legacy
   checks.

## 6. Acceptance criteria

- A v2 spec with a panel writes `panel.csv`; a v2 spec without a panel does not.
- A v2 spec with a session writes `session.csv`; a v2 spec without a session does
  not.
- `session.csv` carries `calendar_bucket`.
- A downward VWAP reclaim emits `signal.vwap_reclaim` with direction `-1`.
- Formula tests prove the registry formulas for breadth, gap fade, level
  strength, and absorption.
- The indicator-coverage table no longer contradicts the registry.
- No other model, strategy, or command behavior changes.
- `cargo test`, clippy, fmt, registry, validation, links, terms, and legacy checks
  pass.

## 7. Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
python3 tools/check_legacy.py
python3 tools/check_registry.py
python3 tools/check_validation.py
python3 tools/check_links.py
python3 tools/check_terms.py
```

## 8. Risks and recovery

| Risk | Mitigation |
|---|---|
| Bearish reclaim changes existing event goldens | Regenerate only the affected extended-event fixture after the QD review |
| Panel/session artifacts are non-deterministic | They are projections of already deterministic readings; test repeated runs |
| Formula reconciliation accidentally changes behavior | Only the reclaim mirror is a code behavior change; the rest are table corrections |
| Panel CSV order depends on a map | Iterate timestamps, then the sorted instrument map; test byte equality |
| Session.csv grows with every bar | It is the evaluation series, not the full ladder; the spec controls its size |

Recovery: every path exists at `50549dd`.

## 9. Stop conditions

- A formula cannot be reconciled without changing a claim the human has not
  approved.
- The panel/session readings are not deterministic.
- The bearish reclaim requires a new event code or registry identity.
- A golden fixture changes beyond the intended reclaim rows.

## 10. Execution record

- Stored `SessionReadings` and `PanelReadings` on `SpecRun` and exposed them.
- Added `panel.csv` and `session.csv` to the v2 spec bundle, gated on the spec
  declaring the panel or session.
- `session.csv` publishes `calendar_bucket` for every evaluation bar.
- Added the bearish VWAP reclaim mirror. The event keeps the existing
  `signal.vwap_reclaim` code and uses direction `-1`.
- Corrected the indicator-coverage table to the registry for breadth, gap fade,
  level strength, absorption, and the lead-lag signal value. Clarified the
  registry level-strength formula and added the named-contract note.
- Added formula tests for breadth, gap, absorption direction, and both VWAP
  reclaim directions; added writer tests for both CSV artifacts and an
  integration test for the spec v2 bundle.

### Quant Developer record

The QD pass added the bearish VWAP reclaim mirror and the two artifact writers.
The extended-event fixture content diff is exactly four added
`signal.vwap_reclaim` rows with direction `-1`; no event row was removed or
changed. The fixture's leading row-index column shifts for later rows. The panel and session
readings were already golden-tested; the pass only exposes them. No other model
formula changed.

### Quant Validator record

The QV pass was performed sequentially by the same agent, not by an independent
agent or model judgment. It checked:

- `cargo test`: 982 passed, 0 failed, 7 ignored.
- The extended-event fixture content: four added bearish reclaim rows, no event
  content removed or changed; the leading row-index column shifts.
- `panel.csv` and `session.csv` are present only for a v2 spec that declares the
  panel/session, and the session CSV carries `calendar_bucket`.
- The formula table and registry now agree for the reconciled rows.
- The specification and shell outputs are deterministic.

Residual risk: the validator reproduced the working tree before the commit; the
PR and CI run are the post-commit reproduction.

### Verification

| Check | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| `cargo test` | 982 passed, 0 failed, 7 ignored across 28 suites |
| `python3 tools/check_legacy.py` | 175 current files; no retired labels |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |
| `python3 tools/check_validation.py` | 188 claims |
| `python3 tools/check_links.py` | 544 Markdown files; no broken relative links |
| `python3 tools/check_terms.py` | 326 files; no banned terms |
| Spec v2 bundle | `panel.csv` and `session.csv` present |
| Spec v1 bundle | no panel/session artifacts |
| Extended-event fixture | four added bearish reclaim rows; no event content removed or changed |
