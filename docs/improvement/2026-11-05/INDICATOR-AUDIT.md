# Indicator Feature Audit — Top Ten Areas

- Date: 2026-11-05 (UTC)
- Status: **Retrospective, read-only**; closes the missing Phase 1 artifact
- Closeout: 2026-11-05; F-02 fixed by the human signature; F-10 remains triaged as accepted scope
- Scope: VWAP, volume profile, RVOL, volume climax, absorption, OBV, CMF, MFI,
  ATR term, and session
- Basis: registry `2026-11-05.15`, checklist `INDICATOR-VERIFICATION.md`, model
  implementation, and the checked-in tests
- Claim type: implementation and contract audit; no edge claim
- Plan: [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](../../execution-plans/2026-11-05/410-m2-model-programme.md)

## 1. Why this audit is retrospective

Phase 1 of the indicator platform plan called for a read-only feature audit of
the top ten indicators **before** the full primitive backlog was committed. It
was not delivered. The implementation proceeded, the registry reached 46
indicators and 143 events, and the checklist reached 37/37 `tested`.

This document closes the missing artifact, but it cannot recover the
pre-registration property. It reads the implemented feature set and records what
is there, what is inconsistent, and what remains open. It is evidence about the
implementation, not evidence that the features were screened before they were
built.

## 2. Method and evidence

The audit is read-only. It does not modify code, tests, issues, or pull
requests. It compares four sources:

1. `docs/registry/indicators.json` — the indicator, event, role, claim, formula,
   parameter, availability, and test declarations.
2. `docs/improvement/2026-11-05/INDICATOR-VERIFICATION.md` — the build and test
   checklists, including the open `Sign` column.
3. `crates/quantrule-model/src/readings/extended.rs`,
   `crates/quantrule-model/src/events/extended.rs`,
   `crates/quantrule-model/src/sessions.rs`, and
   `crates/quantrule-model/src/events/session.rs` — the implementation.
4. The checked-in tests and fixtures listed in the checklist and in
   `crates/quantrule-model/tests/`.

### 2.1 Evidence commands and observed results

| Command | Observed result |
|---|---|
| `python3 tools/check_registry.py` | `registry ok: 46 indicators, 143 events, 110 parameters` |
| `cargo test -p quantrule-model --lib` | `573 passed; 0 failed; 0 ignored` |
| `cargo test -p quantrule-model --test event_validation` | `8 passed; 0 failed; 1 ignored` |
| `cargo test -p quantrule-model readings::extended::tests::vwap_is_the_volume_weighted_typical_price` | `1 passed; 0 failed` |

The event-validation suite's ignored test is the large fixture run; the
smaller cases pass. The 573 library tests include the formula, causality,
determinism, golden, and edge tests for the extended readings.

### 2.2 Counts

| Item | Count | Note |
|---|---:|---|
| Indicators | 46 | Registry `indicators` |
| Unique events | 143 | Registry `events` |
| Event references from indicators | 148 | Five events are shared by two indicators |
| Parameters | 110 | Registry `parameters` |
| Checklist rows `tested` | 37/37 | Table A and Table B checks are ticked |
| Checklist rows `verified` | 37/37 | Human signed rows 1–37 on 2026-11-05; row 37 is reading-only |

The five shared event IDs are `divergence.rsi_regular_bullish`,
`divergence.rsi_regular_bearish`, `divergence.rsi_hidden_bullish`,
`divergence.rsi_hidden_bearish`, and `signal.structure_change`. A coverage
checker must count unique IDs, not indicator references.

## 3. Feature records

| Indicator | Code | Area | Role | Events L/D/S | Reading outputs | Registry claim |
|---|---:|---|---|---|---|---|
| `vwap` | 11 | 1 Volume-weighted price | context | 3/4/3 | `vwap`, distance, slope, bands | A reclaim or rejection at VWAP reaches the ATR target before the stop more often than a matched barrier base |
| `volume_profile` | 12 | 2 Volume by price | context | 5/2/4 | POC, VAH, VAL, distance, position, skew, concentration | A value-area breakout or POC cross continues more often than a matched base |
| `rvol` | 4 | 3 Participation | context | 0/2/3 | RV5, RV20, RV50, ratio, oscillator | Fast/slow relative volume describes participation; expansion or contraction is context |
| `volume_climax` | 13 | 3 Participation | signal | 0/2/2 | flag, direction, strength | A high-volume wide-range bar marks a climax; close location decides reversal or continuation |
| `absorption` | 14 | 3 Participation | signal | 1/0/2 | flag, direction, strength | High volume with a small range at VWAP or POC marks absorption; the next bar's break or rejection is directional |
| `obv` | 16 | 4 Volume-price flow | context | 0/4/2 | OBV, MA, slope | A price/OBV divergence marks accumulation or distribution |
| `cmf` | 17 | 4 Volume-price flow | context | 0/4/2 | CMF, slope | A price/CMF divergence marks accumulation or distribution |
| `mfi` | 18 | 4 Volume-price flow | signal | 0/4/2 | MFI, zone | A price/MFI divergence marks accumulation or distribution |
| `atr_term` | 20 | 5 Volatility structure | context | 0/2/3 | ATR ratio, slope, regime | A price/ATR-ratio divergence marks absorption or fading volatility |
| `session` | 42 | 10 Calendar and session | context | 1/0/2 | session code, age, session VWAP | The session code and age describe the local session |

### 3.1 VWAP

- Reading: `TP=(H+L+C)/3`; `VWAP=sum(TP*V,n)/sum(V,n)`; distance in ATR; slope;
  `MAD=mean(abs(TP-VWAP),n)`; bands `VWAP ± 2*MAD`.
- Events: level test/cross/break; four divergences; reclaim, rejection, slope.
- Roles: the level and slope events are `context`; the divergences and the
  reclaim/rejection events are `signal`. The registry claim is directional.
- Tests: `vwap_is_the_volume_weighted_typical_price`,
  `vwap_is_null_without_volume`, `vwap_cross_is_a_level_event`, and the
  causality/determinism/golden/edge suites.
- Observation: the formula and implementation agree. `signal.vwap_slope` is
  `context` with `claim: none`; it is a descriptor, not a vote.

### 3.2 Volume profile

- Reading: 96-bar, 24-bin profile; POC, value area, distance, position, skew, and
  concentration.
- Events: five level events; two POC divergences; value-area breakout, POC cross,
  skew, and concentration.
- Roles: seven `context`, four `signal`.
- Tests: `profile_poc_is_the_highest_volume_bin` and the profile golden suite.
- Observation: `signal.profile_concentration` uses a 0.5 threshold on
  `max(bin)/sum(V)`. With 24 bins the threshold is reachable only in a very
  concentrated profile; the event may be sparse. This is a measurement question
  for the development swarm, not a code defect.

### 3.3 RVOL (extended)

- Reading: `RV5`, `RV20`, `RV50`, fast/slow ratio, and a volume oscillator.
- Events: two divergences; expansion, contraction, and ratio.
- Roles: four `signal`, one `context`.
- Tests: `rvol_reads_the_volume_ratio` and the extended golden suite.
- Observation: the reading publishes `volume_oscillator`, and indicator 15
  (`volume_oscillator`) also owns that output. The computation is shared, but
  the registry ownership is duplicated. See F-04.

### 3.4 Volume climax

- Reading: `CL = RV5 >= 3 && range/ATR >= 1.5`; direction and strength.
- Events: two divergences; reversal and continuation.
- Roles: all four `signal`.
- Tests: `climax_flags_a_high_volume_wide_bar` and the extended golden suite.
- Observation: the reversal event closes back inside the prior range; the
  continuation event closes at an extreme. The two are mutually exclusive by
  close location, which is the intended precedence.

### 3.5 Absorption

- Reading: `AB = RV5 >= 2 && range/ATR <= 0.6 && min(|C-VWAP|,|C-POC|) <= 0.25 ATR`;
  strength `RV5/(range/ATR)`.
- Events: one level test; break and rejection.
- Roles: one `context`, two `signal`.
- Tests: `absorption_flags_a_small_range_near_vwap` and the extended golden
  suite.
- Observation: the implementation matches the registry formula (close distance
  to VWAP or POC). The coverage document states a different formula (high/low
  distance to a level). See F-05.

### 3.6 OBV

- Reading: cumulative signed volume, MA, slope.
- Events: four divergences; cross and slope.
- Roles: five `signal`, one `context`.
- Tests: `obv_accumulates_signed_volume` and the flow golden suite.
- Observation: no level events, by design. The divergence requires a confirmed
  pivot, so availability is delayed to the pivot confirmation bar.

### 3.7 CMF

- Reading: money-flow volume, CMF, slope.
- Events: four divergences; zero cross and threshold.
- Roles: five `signal`, one `context`.
- Tests: `cmf_is_one_when_every_close_is_the_high` and the flow golden suite.
- Observation: the zero-denominator case (high equals low) is null, not zero;
  the reading guard is present.

### 3.8 MFI

- Reading: typical-price money flow; MFI and zone.
- Events: four divergences; threshold and zone.
- Roles: five `signal`, one `context`.
- Tests: `mfi_is_one_hundred_when_every_bar_rises` and the flow golden suite.
- Observation: the threshold event fires on 20/50/80 crossings; the zone event is
  a descriptor. The two are not a double vote: they have different definitions.

### 3.9 ATR term

- Reading: `ATR14/ATR100`, slope, and a three-state regime.
- Events: two divergences; compression, expansion, and regime.
- Roles: two `signal`, three `context`.
- Tests: `atr_term_is_available_after_the_slow_window` and the volatility golden
  suite.
- Observation: the regime event is a state change, not a direction. It belongs in
  conditions, not in a directional vote.

### 3.10 Session

- Reading: DST-aware session code, age, session VWAP.
- Events: session VWAP test; session open and close.
- Roles: all three `context`.
- Tests: `london_session_and_opening_range`,
  `overnight_and_intraday_returns_read_the_session`, and the session golden
  suite.
- Observation: `signal.session_close` occurred on the last session bar but is
  available only on the next bar. That is the correct availability delay.

## 4. Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| F-01 | Major | Process | Phase 1 was not pre-registered; this audit is retrospective | 405 plan phase 1 “Not delivered”; this document | The feature screen did not gate the backlog | (a) Accept retrospective with limitation; (b) re-audit before M2; (c) waive | (a) | This document; 405 plan updated | Fixed | 410 plan, 405 plan |
| F-02 | Major | Verification | 37/37 rows are `tested`, 0 `verified`; `Sign` open | Checklist Table B | No row is formally verified | (a) Sign now; (b) defer to P7; (c) waive | (a) | `Sign` column | Fixed | Checklist |
| F-03 | Major | Spec | v1 cannot configure readings, active events, prediction, or panel/session | v1 contract; v2 closes the reading/active/prediction/panel gap | The M2 swarm cannot run from a spec | (a) v2; (b) extend v1 in place; (c) separate config | (a) | v2 parser and report tests | Fixed | [model-spec-v2.md](../../model/2026-11-05/model-spec-v2.md) |
| F-04 | Medium | Registry | `volume_oscillator` is owned by both `rvol` and indicator 15 | `rvol.outputs` includes `volume_oscillator`; indicator 15 owns it; `participation_series` computes it | Ambiguous ownership; coverage or vote code could count it twice | (a) Remove from `rvol.outputs`; (b) merge into `rvol`; (c) leave | (a) | `check_registry.py`; registry diff | Fixed | Registry |
| F-05 | Medium | Documentation | Absorption formula differs between coverage doc and registry/implementation | Coverage Area 3: `min(abs(H-L_level),abs(L-L_level))`; registry: `min(|C-VWAP|,|C-POC|)`; code uses close distance | A reader could implement the wrong formula | (a) Update coverage doc; (b) change code; (c) mark both | (a) | Doc diff; registry; code review | Fixed | Coverage, registry |
| F-06 | Medium | Spec | Panel and session readings are not wired into the spec run | Checklist rows 28–36 now `Wiring` and `Spec` ☑ | The ten panel/session indicators are not exercised by a spec | (a) v2 panel/session; (b) separate panel run; (c) leave out | (a) | v2 panel/session tests | Fixed | [model-spec-v2.md](../../model/2026-11-05/model-spec-v2.md) |
| F-07 | Low | Coverage | 148 indicator event references for 143 unique events | Registry count; five shared IDs | A naive coverage check overcounts | (a) Dedupe in the checker; (b) list each event once; (c) leave | (a) | Coverage checker test | Fixed | P3 |
| F-08 | Low | Prediction | Context events with `claim: none` can be misread as directional votes | Registry roles: 71 context, 67 signal, 5 diagnostic; v2 rejects a non-signal vote | A vote that counts every event would double count descriptors | (a) Vote counts only `signal` events with a direction; context enters conditions; (b) count all events | (a) | v2 role-rejection test | Fixed | [model-spec-v2.md](../../model/2026-11-05/model-spec-v2.md) |
| F-09 | Low | Registry | `signal.profile_concentration` at 0.5 may be sparse | Formula `max(bin)/sum(V) >= 0.5` over 24 bins | The event may be inert in the development run | (a) Measure occurrence; (b) change threshold; (c) retire | (a) | P5 occurrence report | Fixed | Registry |
| F-10 | Info | Scope | This audit covers 10 of 46 indicators | Scope table | The other 36 rest on the checklist and golden fixtures | (a) Expand audit; (b) accept scope | (b) | Checklist | Triaged | 405 plan |

## 5. Finding details

### F-01 — The audit is retrospective

**Evidence.** The indicator plan lists Phase 1 as “Read-only feature audit of
the top 10” and the checklist records “Not delivered.” The implementation and
the 37-row checklist were completed before this document existed.

**Impact.** The original intent was to screen candidate features before the
backlog was committed. That gate was missed. This audit can still find
inconsistencies, but it cannot claim that the ten features passed a
pre-implementation screen.

**Options.** (a) Accept the retrospective audit with the limitation stated here;
(b) require a new, independent pre-implementation audit before the M2 runs;
(c) waive Phase 1. Option (a) is recommended because the implementation already
exists and the audit still exercises the evidence. Option (b) would repeat work
without changing the built code. Option (c) would leave the artifact missing.

**Recommended.** (a), with the limitation recorded in the 405 plan and the M2
plan.

**Verification.** This document exists; the 405 plan and the 410 plan link it;
the limitation is stated in both.

**Status.** Fixed (artifact delivered); the pre-registration property is not
recoverable and is recorded as a limitation.

### F-02 — The `Sign` column is open

**Evidence.** Table B of `INDICATOR-VERIFICATION.md` has all `F/C/D/G/E/A/N/P/M`
cells ticked and all `Sign` cells open. The status definition says `verified`
requires the human or Quant Validator signature.

**Impact.** The registry and tests are `tested`, but no row is `verified`. The
M2 programme can use `tested` evidence, but the final baseline and closeout
should not claim `verified` until the signatures exist.

**Options.** (a) Sign the rows now, before P1; (b) defer the signatures to P7
closeout; (c) waive the column. Option (a) is recommended because the M2
programme will build on this evidence. Option (b) is acceptable if the human
wants to review the M2 evidence together. Option (c) would make the status
definition meaningless.

**Recommended.** (a) or (b), decided by the human; not (c).

**Verification.** The `Sign` column is populated for all 37 rows; the status
becomes `verified`.

**Status.** Fixed (P7): the human signed rows 1–37 on 2026-11-05; the agent
recorded the signature. The checklist is 37/37 `verified`.

### F-03 — The v1 spec cannot configure the M2 model

**Evidence.** The v1 contract says “The v1 model run uses the default readings
and event settings.” It has no `readings`, `active_events`, `prediction`,
`panel`, or `session` field. The v2 contract and parser now provide them; v1 is
unchanged.

**Impact.** The M2 swarm needs per-cell reading windows, event activation, and a
prediction. Without v2 it cannot run as a model programme.

**Options.** (a) Add `quantrule.model-spec/v2`; (b) extend v1 in place; (c) keep
the spec for conditions and add a second config file. Option (a) is recommended:
it preserves v1 compatibility and gives the new fields one contract. Option (b)
would break a published schema. Option (c) creates two sources of truth.

**Recommended.** (a).

**Verification.** v2 parser and report tests: parse, validate, override, active
event, vote, panel/session, causality, determinism, byte-identical rerun, edge
cases.

**Status.** Fixed. See [`model-spec-v2.md`](../../model/2026-11-05/model-spec-v2.md).

### F-04 — `volume_oscillator` has two owners

**Evidence.** `docs/registry/indicators.json` lists `volume_oscillator` in the
`rvol` outputs and also has a separate indicator with `id: volume_oscillator`
and code 15. `crates/quantrule-model/src/readings/extended.rs` computes it once
in `participation_series`.

**Impact.** The computation is shared and correct, but the registry says two
indicators own the same output. A coverage checker or a vote builder that keys
on outputs can count it twice or attribute it to the wrong indicator.

**Options.** (a) Remove `volume_oscillator` from `rvol.outputs` and keep it under
indicator 15; (b) merge indicator 15 into `rvol`; (c) leave the duplicate.
Option (a) is recommended: the reading is shared, but ownership is single.
Option (b) would remove a registered indicator and change the area map. Option
(c) leaves an ambiguity.

**Recommended.** (a).

**Verification.** `python3 tools/check_registry.py` passes; the registry diff
shows one owner; the event `signal.volume_oscillator_cross` still resolves to
code 15.

**Status.** Fixed (P7): `volume_oscillator` was removed from `rvol.outputs` and
remains owned by indicator 15. `check_registry.py` reports 46 indicators, 143
events, and 110 parameters.

### F-05 — Absorption formula mismatch

**Evidence.** `docs/model/2026-11-05/indicator-coverage.md` Area 3 says
`min(abs(H-L_level),abs(L-L_level))<=0.25ATR`. The registry says
`min(|C-VWAP|,|C-POC|) <= 0.25 ATR`. The implementation in
`participation_series` uses the close distance to the nearest of VWAP and POC.

**Impact.** The coverage document is the readable contract a researcher uses to
understand the feature. A mismatch can produce a different expectation and a
wrong review of the event.

**Options.** (a) Update the coverage document to the registry formula; (b)
change the implementation to the coverage formula; (c) keep both and label the
coverage formula as an alternative. Option (a) is recommended: the registry and
code agree, and the change is documentation-only. Option (b) changes model
behavior and requires a Quant Validator and new goldens. Option (c) leaves two
definitions.

**Recommended.** (a).

**Verification.** The coverage document matches the registry; the
`absorption_flags_a_small_range_near_vwap` test still passes.

**Status.** Fixed (documentation-only; the registry and implementation already
agreed). The coverage document was updated in the same P0 change.

### F-06 — Panel and session are not wired into the spec

**Evidence.** The v2 spec run computes session events for the evaluated
instrument and panel events for the declared panel, merges them into the event
view, and publishes the active projection. Checklist rows 28–36 are now
`Wiring` and `Spec` ☑; row 37 (`calendar_bucket`) is reading-only.

**Impact.** Ten indicators (relative strength, lead-lag, breadth, co-movement,
dispersion, session, opening range, overnight/intraday, session range, calendar
bucket) are not exercised by a model spec.

**Options.** (a) Add optional `panel` and `session` to v2; (b) run a separate
panel bundle and join it outside the spec; (c) exclude them from M2. Option (a)
is recommended: the coverage matrix needs them, and the v2 contract already
reserves the fields. Option (b) creates a second run surface. Option (c) would
leave 10 indicators untried, violating the coverage rule.

**Recommended.** (a).

**Verification.** v2 panel/session tests; a spec run with a declared panel and a
London session publishes the panel and session events for the evaluated
instrument, and conditions read them.

**Status.** Fixed. See [`model-spec-v2.md`](../../model/2026-11-05/model-spec-v2.md).

### F-07 — Event references overcount unique events

**Evidence.** The registry has 143 unique events and 148 indicator references.
The five shared IDs are the RSI divergences and `signal.structure_change`.

**Impact.** A coverage checker that iterates indicator references and counts
each reference would report 148 and could miss the fact that the unique set is
143. It could also treat a shared event as two votes.

**Options.** (a) Dedupe by event ID in the coverage checker and in the vote
builder; (b) list each event once in the registry and remove it from the second
indicator; (c) leave it. Option (a) is recommended: shared ownership is real,
and the event is one occurrence. Option (b) would hide which indicators can
produce the event. Option (c) leaves a known trap.

**Recommended.** (a).

**Verification.** A coverage test asserts `unique(events) == 143` and every
primary prediction names each event once.

**Status.** Fixed (P3): `tools/research/coverage.py` and its test dedupe by event
ID; the committed matrix reports 46 indicators and 143 events.

### F-08 — Context events are not votes

**Evidence.** The registry has 71 `context`, 67 `signal`, and 5 `diagnostic`
events. Several context events have `claim: none` (`signal.vwap_slope`,
`signal.profile_skew`, `signal.profile_concentration`, `signal.atr_compression`,
`signal.atr_expansion`, `signal.atr_regime`). The role rule says a context event
cannot be the sole entry.

**Impact.** A naive `kind: vote` that counts every active event would treat
descriptors as directional evidence and inflate the vote.

**Options.** (a) The vote counts only `signal` events with a direction, and
context events enter conditions; (b) count every active event; (c) require the
spec author to mark each event's weight. Option (a) is recommended: it follows
the registry role and keeps the vote explainable. Option (b) violates the role
contract. Option (c) turns the spec into a weighting language and invites
overfitting.

**Recommended.** (a).

**Verification.** The v2 parser rejects a context or diagnostic event in a vote
list; a context event remains usable in a condition.

**Status.** Fixed. See [`model-spec-v2.md`](../../model/2026-11-05/model-spec-v2.md).

### F-09 — Profile concentration may be sparse

**Evidence.** `signal.profile_concentration` fires on
`cross_up(max(bin)/sum(V), 0.5)` over a 24-bin profile. The formula is in the
registry. No occurrence count has been published yet.

**Impact.** If the threshold is unreachable in normal 1h data, the event is
inert and the coverage claim is nominal only.

**Options.** (a) Measure its occurrence in the development swarm; (b) change the
threshold; (c) retire the event. Option (a) is recommended: the measurement is
cheap and does not change the contract. Option (b) is a contract change that
requires evidence and approval. Option (c) is premature.

**Recommended.** (a).

**Verification.** The P5 cell report includes an occurrence count for every
event, including this one. A zero count is recorded, not hidden.

**Status.** Fixed (P7): a deterministic reproduction of `EXPL-02` on `model_dev`
recorded 13,541 `signal.profile_concentration` events across the four
instruments and three folds; the reproduced metrics match the preserved P5
result. The event is not inert.

### F-10 — Audit scope is ten of forty-six

**Evidence.** This document covers the ten features named in the Phase 1 plan.
The remaining 36 indicators are covered by the registry, the checklist, and the
golden fixtures, not by this audit.

**Impact.** The audit cannot claim to have reviewed every formula. A reader
should not treat it as a full-programme audit.

**Options.** (a) Expand the audit to all 46; (b) accept the ten-feature scope and
rely on the checklist for the rest. Option (b) is recommended for this artifact:
the plan named the top ten, and the checklist is the programme-wide control.
Option (a) is a separate, larger audit.

**Recommended.** (b).

**Verification.** The scope is stated in the header and in the conclusion.

**Status.** Triaged.

## 6. Verification

| Check | Command | Expected |
|---|---|---|
| Registry integrity | `python3 tools/check_registry.py` | `registry ok: 46 indicators, 143 events, 110 parameters` |
| Model library tests | `cargo test -p quantrule-model --lib` | 573 passed |
| Event validation | `cargo test -p quantrule-model --test event_validation` | 8 passed, 1 ignored |
| VWAP formula | `cargo test -p quantrule-model readings::extended::tests::vwap_is_the_volume_weighted_typical_price` | 1 passed |
| Checklist state | `INDICATOR-VERIFICATION.md` tables A and B | 37/37 tested; 0/37 signed |

These commands were run on 2026-11-05 in the `docs/410-m2-model-programme`
worktree. The results are recorded here as observed, not projected.

## 7. Conclusion

The top ten features are implemented, registered, and tested. The audit found
one ownership inconsistency (F-04), one documentation mismatch now fixed (F-05),
two contract gaps now closed by v2 (F-03, F-06), one coverage trap (F-07), one
prediction-role rule now fixed (F-08), one sparse-event question (F-09), and the
two process gaps that were already known (F-01, F-02).

None of the findings is a Critical defect. F-01 is fixed as an artifact but its
pre-registration limitation stands. F-02 remains a human/Validator action. F-03,
F-05, F-06, and F-08 are fixed in P1. F-04, F-07, and F-09 are routed to P3/P5.
