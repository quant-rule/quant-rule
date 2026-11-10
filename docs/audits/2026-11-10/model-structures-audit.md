# Model Structures Audit — M1.2 Indicators and Events

- Date: 2026-11-10 (UTC)
- Audit ID: `AUDIT-2026-11-10-model-structures`
- Status: Published; all findings `In progress` under the approved remediation plan
- Baseline: `main` at `dda51d8` (2026-11-10)
- Scope: the `M1.2` additions — `pivot_sequence` (47), `fibonacci` (48),
  `horizontal_band` (49), and events 147–150 — plus their registry entries,
  validation claims, verification evidence, model contract, config surface,
  golden fixtures, and the tools and strategy configs that consume them.
- Workflow: [`audit-workflow.md`](../../workflows/audit-workflow.md)
- Execution note: the remediation is implemented on `fix/audit-m12-remediation`
  and the full gate passes; no commit, push, or pull request exists yet, so the
  findings remain `In progress` until the PR merges.
- Claim type: program audit; no edge, profitability, or strategy claim.

## Summary

| Severity | Count |
|---|---:|
| Critical | 0 |
| Major | 5 |
| Minor | 4 |
| Info | 0 |

The new model code is deterministic and causal in the tested paths; the model
unit/integration tests, clippy, and the registry, validation, verification, and
event-emitter gates pass. The audit did not find a crash, a NaN leak, or a
future-data read. It did find two model-semantics defects, a substantial
verification gap, a config-ownership/validation gap, and a missing mandatory
Quant Validator record. The new events are used by the SR10–SR12 development
configs, so the semantics findings may change those research results if the
event definitions are corrected.

The audit is read-only. No code, test, registry, config, issue, or pull request
was changed. AUD-01, AUD-03, AUD-04, AUD-05, AUD-08, and AUD-09 are confirmed by
code, fixture, or repository inspection. AUD-02 is a confirmed divergence between
the approved plan, the model contract, and the implementation; the intended
semantics still need a human decision. AUD-06 and AUD-07 are confirmed
documentation/tooling drift, not unverified hypotheses.

## Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| AUD-2026-11-10-01 | Major | model/events | The two pivot-sequence events record the confirmation bar as `occurred_bar_index`; the pivot occurred five bars earlier | `events/extended/structure.rs:21-60`; `events/extended/shared.rs:50-51`; fixture event 147 at bar 59 vs pivot occurrence 54 | Every sequence event is misdated by the major-pivot lookback; consumers of occurrence time are wrong | A. use pivot occurred/available; B. keep completion time; C. add a completion field | A | New test asserts `occurred = pivot.occurred_at`, `available = bar`; fixture regenerated | In progress | [plan](../../execution-plans/2026-11-10/audit-model-structures-remediation.md) |
| AUD-2026-11-10-02 | Major | model/events | `level.fib_test` tests only candle high/low proximity, not bar-range intersection; a bar that trades through and closes on a level can emit no event | `events/extended/fibonacci.rs:42-45`; plan `model-structures.md:131`; model doc `model-m1.2.md:59-62`; fixture bar 33 | The event undercounts tests; the approved plan and the model contract disagree; SR10/SR11 entries may be undercounted | A. range intersection; B. keep endpoint and fix plan/claim; C. split touch/cross | A | Wide-bar case emits; non-intersecting case does not; nearest/multiplicity rule tested | In progress | [plan](../../execution-plans/2026-11-10/audit-model-structures-remediation.md) |
| AUD-2026-11-10-03 | Major | model/tests | Fibonacci and band formula/edge verification is missing; the row tests are regression locks and the generic edge suite omits all 22 new fields | `per_row_extended_indicators.rs:103-145`; `edge_cases.rs:34-180`; `readings/extended/tests.rs` has no Fibonacci/band test; model doc `model-m1.2.md:101` | Wrong anchors, tie-breaks, null rules, or zero-ATR behavior could ship silently; the plan's edge acceptance criteria are unmet | A. add unit/edge tests; B. document fixture sufficiency; C. A + property tests | C | New unit/row tests fail before the fix; verification matrix updated | In progress | [plan](../../execution-plans/2026-11-10/audit-model-structures-remediation.md) |
| AUD-2026-11-10-04 | Major | model/config | `fib_test_atr` and `band_atr` are declared fixed but accepted by spec v2, and both are omitted from `extended_is_valid` | `shell/src/spec/v2.rs:296-297,338-339`; `model/src/spec/v2.rs:779-811`; registry `indicators.md:323-324` | A run can silently change or disable the new event families; negative/huge tolerances pass validation | A. remove from parser; B. make configurable and validate; C. keep override, rename, document | A or B (human) | Invalid values rejected; valid value recorded in provenance; registry and parser agree | In progress | [plan](../../execution-plans/2026-11-10/audit-model-structures-remediation.md) |
| AUD-2026-11-10-05 | Major | process/evidence | No Quant Validator record exists for the M1.2 model behavior change | Plan `model-structures.md:308`; `model-change.md:103,119`; `audit-workflow.md:114-115`; only `docs/roles/.../quant-validator-manual.md` exists | The change was merged without the required independent reproduction; audit green criteria cannot be met | A. produce the record; B. human exception; C. treat the AI log as the record | A | Dated validator record with commands, results, limitations, verdict | In progress | [plan](../../execution-plans/2026-11-10/audit-model-structures-remediation.md) |
| AUD-2026-11-10-06 | Minor | tooling/analysis | The annotation-zone `fib_test` and `band_test` features do not implement the model event definitions | `tools/trades/annotation_zones.py:144-159`; `events/extended/fibonacci.rs:42-45`; `events/extended/band.rs:32` | The zone report's fib/band rates are not event rates; conclusions may be artifacts | A. use the event batch; B. exact parity implementation; C. rename and document | A or C | Parity test against the event batch, or renamed/documented definitions | In progress | [plan](../../execution-plans/2026-11-10/audit-model-structures-remediation.md) |
| AUD-2026-11-10-07 | Minor | docs/strategy | The event-composite contract says extended events are not part of the language, but the validator accepts registered non-diagnostic subtypes and SR10–SR12 use them | `docs/config/2026-11-06/strategy-config.md:121-124`; `backtest/src/strategy/event.rs:343-379`; `config/sr10/...`; SR6 log `sr6-vwap.md:28` | The public strategy contract is stale; readers cannot tell whether the new events are usable | A. update the contract; B. restore a whitelist and rerun; C. publish an allow-list | A | Contract matches the validator; registered extended event accepted, diagnostic rejected | In progress | [plan](../../execution-plans/2026-11-10/audit-model-structures-remediation.md) |
| AUD-2026-11-10-08 | Minor | model/code style | `band.rs` uses `expect` in library code | `readings/extended/band.rs:74,123`; `AGENTS.md` §3 | A future refactor that empties a cluster or band panics instead of publishing null | A. replace with null path; B. narrow allow; C. leave and document | A | No `expect` in the new library files; clippy/test pass | In progress | [plan](../../execution-plans/2026-11-10/audit-model-structures-remediation.md) |
| AUD-2026-11-10-09 | Minor | docs/registry | The event claims say "second or third consecutive" but the detector fires for any count `>= 2`, including 4+ | `docs/registry/indicators.md:211-212`; `events/extended/structure.rs:27-28,45-46`; fixture event 147 with count 3 | The claim understates the event population; a reader expects a cap that does not exist | A. claim "second or later"; B. cap at 3; C. document the range | A | Registry text matches the formula; no code change | In progress | [plan](../../execution-plans/2026-11-10/audit-model-structures-remediation.md) |

## Detail

### AUD-2026-11-10-01 — sequence events use the confirmation bar as occurrence

- **Expected invariant:** an event record carries the bar on which the event
  occurred and the first bar on which it became knowable; a confirmed pivot is
  knowable only at `occurred_at + major lookback` and must never expose its
  price before that bar (`AGENTS.md` §2–3; `readings/pivots.rs:79-95`).
- **Observed:** `structure_sequence_events` passes `bar` to `push`, and `push`
  writes it to both `occurred_bar_index` and `available_bar_index`
  (`events/extended/structure.rs:21-60`; `events/extended/shared.rs:44-56`).
  In the checked-in fixture, event 147 is recorded at bar 59 with
  `occurred=59, available=59`, while the confirming major high occurred at bar
  54 (`major_pivot_occurred_at=54`). All six sequence events in the fixture have
  `occurred == available`. The RSI divergence detector, by contrast, records
  `current.occurred_at` and `current.available_at`
  (`events/divergence.rs:323-330`).
- **Root cause:** the sequence detector used the shared `push` helper, which
  has no pivot argument and writes one bar for both fields; the event was
  treated as a bar-local signal instead of a confirmed-pivot measurement.
- **Impact:** reports, annotation contexts, and any consumer that uses
  occurrence time see the sequence event five bars late. Strategy decisions keyed
  on `available_bar_index` are unaffected, which is why the existing tests pass.
- **Affected contracts/docs:** `AGENTS.md` §2 (availability, not occurrence) and
  §3 (event records); `docs/model/2026-11-10/model-m1.2.md` §2; registry events
  147–148.
- **Dependencies:** model event record shape; the golden fixture.
- **Effort:** S.
- **Options:**
  - A. Set `occurred_bar_index = pivot.occurred_at` and
    `available_bar_index = bar`; keep the detector's emission bar unchanged.
  - B. Keep both fields at the confirmation bar and document that sequence
    events deliberately use completion time as occurrence.
  - C. Add a separate `completion_bar_index` and preserve both pivot
    occurrence and completion.
- **Recommendation:** A. It matches the divergence precedent and the event
  record contract without changing when the event becomes available.
- **Verification:** a new test resolves the confirming pivot and asserts
  `occurred == pivot.occurred_at` and `available == pivot.available_at` for each
  sequence event; the fixture diff is reviewed; the event batch remains in
  availability order; `cargo test -p quantrule-model` passes.
- **Links:** — (the Planner assigns the fix issue).

### AUD-2026-11-10-02 — Fibonacci test excludes levels inside the bar range

- **Expected invariant:** `level.fib_test` is a test of a published retracement
  level; the approved plan defines a test as a bar range intersecting any level
  within `fib_test_atr * ATR`, with one event per tested level
  (`docs/execution-plans/2026-11-10/model-structures.md:131`).
- **Observed:** the detector computes
  `min(|high - level|, |low - level|) <= tolerance`
  (`events/extended/fibonacci.rs:42-45`). A bar whose range contains the level
  but whose high and low are farther than the tolerance emits nothing. The
  checked-in fixture has such a bar: bar 33 has open/high/low/close
  `99.59/99.72/99.31/99.44`, ATR `0.5619`, the `0.236` level at `99.4416`, and
  `fib_nearest_atr=0.0028`; the close is effectively on the level, yet no code
  149 event exists at bar 33. A scan of the 240 bars with a Fibonacci leg finds
  19 bars with an in-range level (expanded by `0.2 * ATR`) and no event. At bar
  144 two levels are in range; the model contract emits at most one, while the
  plan says one per tested level.
- **Root cause:** the model contract (`docs/model/2026-11-10/model-m1.2.md:59-62`)
  was written to match the endpoint implementation, but the approved plan says
  range intersection and one event per tested level. The implementation and the
  contract agree with each other and disagree with the plan and the event's
  stated claim.
- **Impact:** the event set is undercounted for bars that trade through a level.
  The `SR10`/`SR11` development entries use `level.fib_test`; their signal
  counts and results can change if the definition is corrected. This is a
  silent signal loss for the intended "test" semantics.
- **Affected contracts/docs:** plan §5.2 and §7; model doc §3 and §7; registry
  event 149 and indicator 48; the SR10/SR11 charters/configs.
- **Dependencies:** a human decision on the intended event definition; the
  golden fixture; Quant Validator reproduction if the behavior changes.
- **Effort:** S–M.
- **Options:**
  - A. Define a test as a bar-range intersection with `level ± tolerance`,
    emit the nearest tested level (or one per tested level), update the model
    contract and registry, and regenerate the fixture.
  - B. Keep the endpoint rule, change the plan and the event claim to "candle
    high or low within tolerance", and add a test proving that this is the
    intended semantics.
  - C. Split the definition into `level.fib_touch` (endpoint proximity) and
    `level.fib_cross` (range intersection), with separate claims and codes.
- **Recommendation:** A is the simplest correction and matches the plan. The
  Planner must confirm the intended semantics with the human before the change,
  because it is a model behavior change and may invalidate SR10/SR11 results.
- **Verification:** a synthetic wide bar spanning a level and closing on it
  emits the event; a bar whose range does not intersect the band emits none; the
  nearest-level and multiplicity rules are tested; the fixture diff is reviewed
  line by line; the Quant Validator reproduces the new event counts.
- **Links:** —.

### AUD-2026-11-10-03 — Fibonacci and band formula/edge verification is missing

- **Expected invariant:** every new indicator and event has a causality test, a
  determinism test, a golden fixture, and edge-case coverage: empty series,
  single candle, short series, alignment, gap, zero volume, equal highs/lows,
  and tie-breaking (`AGENTS.md` §4; `docs/workflows/model-change.md:105-119`).
  The plan additionally requires equal highs/lows, missing anchors, insufficient
  pivots, zero ATR, confirmation delays, and deterministic band tie-breaking
  (`docs/execution-plans/2026-11-10/model-structures.md:211-217`).
- **Observed:** `assert_extended_indicator` compares every column to the
  checked-in fixture, recomputes for determinism, checks a prefix for causality,
  and checks only the *lengths* of the empty/single/short series
  (`tests/per_row_extended_indicators.rs:103-145`). It never asserts that
  Fibonacci or band outputs are null before the anchors, that a zero ATR nulls
  the band, that equal highs/lows break the pivot sequence, that the band
  tie-break chooses the documented cluster, or that a two-level minimum is
  enforced. `tests/edge_cases.rs` is the generic edge suite, but
  `assert_extended_safe` and `assert_extended_empty` enumerate the pre-M1.2
  fields and omit all 22 new columns (`tests/edge_cases.rs:34-180`). There is no
  unit test for Fibonacci or band in
  `crates/quantrule-model/src/readings/extended/tests.rs`; the only new unit
  tests there cover the `sequence_count`/`structure_code` helpers
  (`crates/quantrule-model/src/readings/extended/structure.rs:286-320`). The
  model contract nevertheless says "Equal highs/lows and missing ATR are covered
  by unit tests" (`docs/model/2026-11-10/model-m1.2.md:101`).
- **Root cause:** the M1.2 test work reused the existing row-contract helper and
  the generated fixtures; the edge-suite field list and the unit-test module
  were not extended.
- **Impact:** a wrong anchor, tie-break, or null rule would be locked in by the
  fixture and pass all gates. The verification matrix marks `edge` as row-level
  for the new indicators, but the row test only checks lengths; the coverage
  claim is stronger than the test.
- **Affected contracts/docs:** plan §7; model doc §6–§7; registry
  `verification.json` for 47–49 and 147–150; `indicator-verification.md` and
  `event-verification.md`.
- **Dependencies:** the golden fixtures; the event semantics decision in
  AUD-02.
- **Effort:** M.
- **Options:**
  - A. Add unit tests for the Fibonacci anchors/levels/position/nearest, band
    clustering/tie-break/min-touches/age/width/position, zero ATR, zero span,
    equal highs/lows, null-before-anchors, and a row availability test; extend
    `assert_extended_safe`/`empty` to every new field.
  - B. Document why the fixture plus the row test is sufficient and remove the
    "unit tests" claim (not acceptable against the plan's acceptance criteria).
  - C. Add A plus property tests for length, null/finite values, and
    append/prefix invariance across arbitrary candle sequences.
- **Recommendation:** C. It closes the plan criteria and gives the Quant
  Validator something independent to reproduce.
- **Verification:** the new tests fail on a deliberately wrong formula or
  tie-break and pass on the implementation; `cargo test -p quantrule-model`
  passes; `check_verification.py` and the generated matrices cite the new tests;
  the fixture diff is reviewed.
- **Links:** —.

### AUD-2026-11-10-04 — fixed tolerances are accepted and unvalidated

- **Expected invariant:** a parameter marked `configurable: false` is a fixed
  formula constant and is not accepted by a run config; a configurable parameter
  is accepted by a config parser and recorded in provenance
  (`docs/registry/README.md:95-102`). The model's spec validation rejects an
  invalid extended config (`crates/quantrule-model/src/spec/v2.rs:779-812`).
- **Observed:** the registry and model contract mark `model.fibonacci.test_atr`
  and `model.band.cluster_atr` as fixed (`docs/registry/indicators.md:323-324`;
  `docs/model/2026-11-10/model-m1.2.md:85-92`), while the approved plan calls
  them configurable (`docs/execution-plans/2026-11-10/model-structures.md:133,156`).
  The shell spec parser accepts `fib_test_atr` and `band_atr`
  (`crates/quantrule-shell/src/spec/v2.rs:296-297,338-339`), and the model's
  `extended_is_valid` checks every extended field through `gap_atr` but not the
  two new tolerances (`crates/quantrule-model/src/spec/v2.rs:779-811`). A
  negative `fib_test_atr` therefore passes validation and makes every
  `touch <= tolerance` comparison false, silently disabling the event; a very
  large value makes it fire almost everywhere.
- **Root cause:** the M1.2 change added the fields to `ExtendedConfig`, the spec
  allow-list, and the event config plumbing, but did not settle the registry
  ownership or add the validation branch.
- **Impact:** a run config can silently change or disable the new event families
  while the registry says it cannot; the contract's ranges (`min: 0`, `max: 2`)
  are not enforced. The value is in provenance, so the run is reproducible, but
  the config contract is wrong.
- **Affected contracts/docs:** registry parameters; model doc §5; plan §5.2–5.3;
  spec v2; provenance. The naming also drifts: the registry parameter is
  `model.band.cluster_atr`, while the code/spec field is `band_atr`.
- **Dependencies:** a human decision on fixed vs configurable ownership.
- **Effort:** S.
- **Options:**
  - A. Keep them fixed: remove `fib_test_atr` and `band_atr` from the spec
    allow-list and parser, and use the model constants.
  - B. Make them configurable: mark `configurable: true`, give them a config
    owner/default, validate `0 <= value <= 2` (or the chosen range) in
    `extended_is_valid`, document the fields, and add parser tests.
  - C. Keep the spec override, mark the registry configurable, and rename
    `band_atr`/`cluster_atr` to one term everywhere.
- **Recommendation:** the human must settle ownership. If the plan's
  "configurable" is intended, B; if the model doc's "fixed" is intended, A.
  Either way, add a validation test for negative and out-of-range values.
- **Verification:** a spec with `-1` or `100` is rejected (or clamped as
  documented); a valid value appears in provenance; `check_registry.py` and the
  parser test agree.
- **Links:** —.

### AUD-2026-11-10-05 — no Quant Validator record

- **Expected invariant:** a model behavior change has a Quant Validator record
  before merge (`docs/workflows/model-change.md:103,119`; `AGENTS.md` §10), and
  the audit green criteria require it (`docs/workflows/audit-workflow.md:114-115`).
- **Observed:** no validation record exists for M1.2. The approved plan's own
  progress section still lists "Quant Validator record" as pending
  (`docs/execution-plans/2026-11-10/model-structures.md:308`). The AI log records
  implementation and gate evidence but not an independent validator pass
  (`docs/ai-transparency-log/2026-11-10/model-structures.md:43-74`). A repository
  search finds only the role manual
  (`docs/roles/2026-10-29/quant-validator-manual.md`).
- **Root cause:** the plan was merged after the implementation PR, but the
  validator step was not completed or recorded; the plan was not updated.
- **Impact:** the change was merged without the required independent
  reproduction; the audit cannot be green; SR10–SR12 results rest on a model
  change that has not been validated. The AI log's single-agent pass is not an
  independent validator record.
- **Affected contracts/docs:** plan §11; model-change workflow §7–§8; audit
  green criteria; the M1.2 model family record.
- **Dependencies:** the semantics fixes in AUD-01/AUD-02 should be settled
  first, so the validator reproduces the corrected behavior.
- **Effort:** S–M.
- **Options:**
  - A. Produce a dated Quant Validator record after the semantics fixes, with
    commands, observed results, limitations, and a verdict.
  - B. Record a human-approved exception with residual-risk acceptance (not
    recommended for a model behavior change).
  - C. Treat the AI log's sequential pass as the record (not independent; does
    not satisfy the role rule).
- **Recommendation:** A.
- **Verification:** the record is linked from the plan and the remediation PR;
  it names the data version, config, commands, and limitations; the human
  accepts the residual risk.
- **Links:** —.

### AUD-2026-11-10-06 — annotation features differ from the events

- **Expected invariant:** a tool feature named after a model event measures the
  same definition, or the difference is explicit (`AGENTS.md` §3, one term per
  concept; `docs/workflows/model-change.md` handoff).
- **Observed:** `tools/trades/annotation_zones.py:144-147` defines `fib_test` as
  "any zone bar whose `fib_nearest_atr <= tolerance`"; that reading is the
  close distance to the nearest level, not the event's high/low proximity or
  range intersection. `tools/trades/annotation_zones.py:150-159` defines
  `band_test` as "close inside `[band_lower, band_upper]`"; the model event is a
  bar-range intersection with the band expanded by `band_atr * ATR`
  (`events/extended/band.rs:30-44`). The unit test locks the tool's definitions
  (`tools/trades/test_annotation_zones.py:92-96`).
- **Root cause:** the analysis tool reuses the reading columns and a simple
  close-based definition instead of reading the published event batch.
- **Impact:** the 2018–2019 zone report's "Fibonacci test" and "band test" rates
  are not event rates; the conclusion that horizontal bands are "rarely tested"
  may be an artifact of the close-inside rule. The report is descriptive, not a
  strategy claim, so this is Minor, but it can misdirect the next hypothesis.
- **Affected contracts/docs:** annotation zone report and AI log; the SR10–SR12
  charters that cite the zone analysis.
- **Dependencies:** none; the event definitions in AUD-02 should be settled first
  if the tool is made parity-exact.
- **Effort:** S–M.
- **Options:**
  - A. Compute the zone features from the published event batch (preferred).
  - B. Reimplement the exact event formulas and add a parity test against the
    model fixture.
  - C. Rename the features to `fib_close_proximity` and `band_close_inside`,
    document the difference, and keep the descriptive analysis.
- **Recommendation:** A if the report informs strategy hypotheses; C if the
  descriptive close-based view is intentional.
- **Verification:** a parity test shows the tool feature equals event presence
  for every fixture bar, or the report/README states the different definition.
- **Links:** —.

### AUD-2026-11-10-07 — event-composite contract is stale after SR6

- **Expected invariant:** the strategy config contract describes what the
  parser and engine accept (`AGENTS.md` §3 command/config contract).
- **Observed:** `docs/config/2026-11-06/strategy-config.md:121-124` says the
  event-composite engine allows only the seven base event subtypes and that
  extended and session subtypes are not part of the bounded language. The
  validator accepts every registered non-diagnostic subtype
  (`crates/quantrule-backtest/src/strategy/event.rs:343-379`), and the SR6 log
  records the deliberate widening from the hardcoded `1..=7`
  (`docs/ai-transparency-log/2026-11-09/sr6-vwap.md:28-31`). `config/sr10` uses
  `level.fib_test` (source 56, subtype 1) and `config/sr12` uses
  `level.band_test` (source 57).
- **Root cause:** the SR6 validator change was not accompanied by a contract
  update.
- **Impact:** the public strategy contract is wrong; a reader cannot tell
  whether the new extended events are usable, and an auditor cannot tell whether
  the SR10–SR12 configs are within the language.
- **Affected contracts/docs:** strategy config doc; SR6 log; SR10–SR12 configs.
- **Dependencies:** none.
- **Effort:** S.
- **Options:**
  - A. Update the contract to state the registered-subtype rule and the
    source/role restrictions.
  - B. Restore a whitelist and rerun SR10–SR12 (not recommended; it would
    invalidate the research).
  - C. Publish an explicit extended-event allow-list in the contract.
- **Recommendation:** A.
- **Verification:** the contract matches `validate_condition`; a test proves a
  registered extended event is accepted and a diagnostic is rejected; the SR6
  link is recorded.
- **Links:** —.

### AUD-2026-11-10-08 — `expect` in the band library code

- **Expected invariant:** `unwrap` and `expect` are not allowed in library code
  (`AGENTS.md` §3).
- **Observed:** `crates/quantrule-model/src/readings/extended/band.rs:74` uses
  `cluster.last().expect("cluster is non-empty")`; line 123 uses
  `.max().expect("band is non-empty")`.
- **Root cause:** the invariants are true by construction, so the author used
  `expect` as a local assertion.
- **Impact:** a future refactor that creates an empty cluster or band would
  panic instead of publishing null; the rule is explicit, and the panic path is
  in the model engine.
- **Affected contracts/docs:** `AGENTS.md` §3; the band module docs.
- **Dependencies:** none.
- **Effort:** XS.
- **Options:**
  - A. Replace with `let Some(last) = cluster.last() else { continue; }` and
    `let Some(youngest) = ... else { continue; }`, publishing null instead of
    panicking.
  - B. Add a narrow `#[allow]` with a comment (not recommended; the rule says no
    `expect`).
  - C. Leave as is and document the invariant (not sufficient).
- **Recommendation:** A.
- **Verification:** no `expect` in the new library files; a single-level active
  set still produces null; clippy/test pass.
- **Links:** —.

### AUD-2026-11-10-09 — event claims understate the count

- **Expected invariant:** a registry claim states the event's actual condition
  (`docs/registry/README.md:120-136`).
- **Observed:** events 147–148 claim "A second or third consecutive confirmed
  lower high/low...", while the detector emits for `lower_high_count >= 2` and
  `lower_low_count >= 2` without an upper bound
  (`events/extended/structure.rs:27-28,45-46`). The fixture contains a count-3
  event (event 147 at bar 242).
- **Root cause:** the claim was written for the common two/three-pivot case and
  the detector's lower bound was copied without the wording.
- **Impact:** the claim understates the event population; a reader expects a cap
  that does not exist. It is a documentation defect, not a behavior defect.
- **Affected contracts/docs:** registry `indicators.json`/`indicators.md`;
  `validation.json` event claims.
- **Dependencies:** none.
- **Effort:** XS.
- **Options:**
  - A. Change the claims to "second or later consecutive..." and keep the
    formula.
  - B. Cap the event at count 3 (model behavior change; not recommended
    without a hypothesis).
  - C. Leave the claim and document the count range in the model doc.
- **Recommendation:** A.
- **Verification:** the registry text matches the formula; `check_registry.py`
  passes; no code change.
- **Links:** —.

## Verification performed

| Command | Observed |
|---|---|
| `cargo test -p quantrule-model` | pass; the model unit, integration, and doctest suites completed with 0 failures |
| `cargo clippy -p quantrule-model --all-targets` | no warnings |
| `python3 tools/check_registry.py` | `49 indicators, 150 events, 106 parameters` |
| `python3 tools/check_validation.py` | `199 claims for 49 indicators and 150 active events` |
| `python3 tools/check_verification.py` | pass |
| `python3 tools/check_verification_matrix.py` | pass |
| `python3 tools/check_event_emitters.py` | `150 registered codes referenced or family-computed` |
| `python3 tools/check_links.py` | `checked 761 Markdown files; no broken relative links` |
| `python3 tools/check_terms.py` | `checked 479 file(s); no banned terms` |
| fixture scan for sequence events | event 147 at bar 59 has `occurred=59`; the confirming major pivot occurred at bar 54 |
| fixture scan for Fibonacci in-range bars | 19 of 240 leg bars have an in-range level and no code-149 event; bar 33 closes 0.0016 from the 0.236 level with no event |
| `grep -rn "fibonacci\|horizontal_band"` in the reading tests and `edge_cases.rs` | no unit test for either indicator; the generic edge helper omits the new fields |

## Limitations and unexamined areas

- One sequential audit pass by one agent. The audit is not an independent
  review and does not claim to be one.
- The full workspace test suite, `cargo doc`, and the 2018–2019 backtest
  reproduction were not re-run in this audit; the M1.2 AI log claims them, but
  this audit did not verify those claims. The link and term checks were run and
  passed on this worktree.
- The strategy-position excursion and giveback-exit changes in the same PR were
  not audited; the scope is the new indicators and events.
- The 2024 partition was not accessed, and no run outcome was evaluated.
- The annotation contexts under `data/improvement/` are not part of this
  worktree; the tool findings are based on the committed source and the report,
  not on a re-run of the annotation export.
- The severity of AUD-02 assumes the approved plan's range-intersection
  definition is the intended one. If the human confirms the endpoint definition,
  the finding becomes a contract/claim correction rather than a model behavior
  change.

## Recommended order

1. Decide AUD-02's intended definition and AUD-04's config ownership (human).
2. Fix AUD-01 and AUD-02 under the Model Change workflow, with the Quant
   Researcher, Quant Developer, and Quant Validator; regenerate the fixtures
   deliberately.
3. Add the AUD-03 tests and the AUD-04 validation; update the verification
   matrix.
4. Produce the AUD-05 Quant Validator record on the corrected baseline.
5. Fix the documentation/tooling findings AUD-06–AUD-09.
