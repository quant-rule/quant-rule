# Audit Remediation — M1.2 Indicator and Event Findings

- Date: 2026-11-10 (UTC)
- Status: **Approved** — human approved 2026-11-10; implementation complete; PR #579 open; findings `In progress` pending merge and verification
- Work item: `audit-model-structures-remediation`
- Audit: [`model-structures-audit.md`](../../audits/2026-11-10/model-structures-audit.md)
  (`AUDIT-2026-11-10-model-structures`, baseline `main` at `dda51d8`)
- Workflows: [Audit Workflow](../../workflows/audit-workflow.md),
  [Model Change](../../workflows/model-change.md), [Model Research](../../workflows/model-research.md)
  for the definition confirmation, [Recordkeeping](../../workflows/recordkeeping.md)
- Claim type: defect remediation and model behavior correction; no edge or
  profitability claim.
- Quant roles: Quant Researcher (definition confirmation), Quant Developer
  (model and config changes), Quant Validator (independent reproduction),
  Architecture (model identity), Test Engineer, Code Reviewer, Improvement
  Verifier.

## 1. Intent

Close all nine findings in the M1.2 audit completely: two model-semantics
defects, a missing formula/edge verification layer, a config-ownership and
validation gap, a missing Quant Validator record, and four documentation,
tooling, and code-hygiene defects.

The intended outcome is a corrected `M1.3` model contract, a registry and
verification record that match the code, a validated event set, and a documented
disposition for the SR10–SR12 runs that used the affected events. The strategy,
execution, and portfolio engines stay unchanged except where the corrected event
set is consumed.

## 2. Current state

| Item | Observed |
|---|---|
| Model | `M1.2`; registry `2026-11-10.1`; 49 indicators, 150 events, 199 claims |
| Audit findings | 5 Major, 4 Minor; all `Open` |
| `AUD-01` | Sequence events record `occurred == available == confirmation bar`; the pivot occurred 5 bars earlier |
| `AUD-02` | `level.fib_test` uses high/low endpoint proximity; the approved plan says range intersection and one event per tested level; model doc/registry say endpoint and at most one |
| `AUD-03` | Fibonacci/band have no formula or edge unit tests; the generic edge helper omits all 22 new fields |
| `AUD-04` | `fib_test_atr`/`band_atr` are fixed in the registry/model doc but accepted by spec v2 and omitted from `extended_is_valid` |
| `AUD-05` | No Quant Validator record; the M1.2 plan still says pending |
| `AUD-06` | Annotation `fib_test`/`band_test` use close-based definitions, not the model events |
| `AUD-07` | The strategy contract says only seven base event subtypes; the validator accepts registered non-diagnostic subtypes and SR10–SR12 use the new events |
| `AUD-08` | `band.rs` uses `expect` in library code |
| `AUD-09` | Event claims say “second or third” while the detector fires for any count `>= 2` |
| Consumers | SR10/SR11 use `level.fib_test`; SR12 uses `level.band_test`; all are development-only |

The audit verified the model tests, clippy, the registry, validation,
verification, event-emitter, link, and term gates on `main` at `dda51d8`. It did
not re-run the full workspace, `cargo doc`, or the 2018–2019 backtests.

## 3. Scope

### Included

- All nine audit findings, each with a regression guard or an explicit reason a
  fail-before test is impossible.
- The corrected event semantics for sequence occurrence and Fibonacci tests.
- The model identity and registry version change required by the behavior
  correction.
- Fibonacci/band formula, edge, and property tests, and the generic edge-suite
  coverage for the 22 new fields.
- Config ownership and validation for the two new tolerances.
- A Quant Validator record for the model behavior change.
- Annotation-tool parity or an explicit rename, the strategy-contract update,
  the `expect` removal, and the claim wording.
- The disposition of the SR10–SR12 runs that used the affected events.
- Changelog, AI logs, audit statuses, and the full gate.

### Excluded

- New strategy hypotheses, parameter searches, or profitability claims.
- Changes to execution, portfolio, policy, or the strategy rule.
- A general cleanup of the pre-existing `readings.extended` override surface
  beyond `fib_test_atr` and `band_atr`; that is a separate Requirements/Audit
  decision if the project wants one model-config policy for every extended
  constant.
- 2024, validation, or holdout access.
- Re-running SR10–SR12 unless the human explicitly asks for a development
  reproduction under `M1.3`.
- A refactor of the extended-reading module beyond the `expect` fix.

## 3.1 Execution note — 2026-11-10

The remediation was executed in one worktree on
`fix/audit-m12-remediation` because the model identity, event code, and golden
fixture changes share files. The per-sub-issue worktrees and PRs in §7 were not
created; the working tree groups the changes by finding and the commit/PR review
can split them. The work is committed as `5bf7152` and `d3a6b42` and opened as PR #579; the
CI `verify` and `sync` checks pass. No merge was performed. This is the only
planned-process deviation; the content and decisions follow the approved plan.

## 4. Assumptions and uncertainty

- The audit baseline is `dda51d8`; the remediation branches from the latest
  `main` at execution time. The audit document is the status record and must be
  carried into the remediation worktree before its statuses are updated.
- The audit findings are confirmed. `AUD-02` still needs a human decision on the
  intended definition; the plan recommends the approved plan's range
  intersection and one event per tested level.
- The annotation bundles contain the per-annotation `events.csv` needed for the
  parity fix. If they do not, the fallback is the explicit rename in `AUD-06`
  option C, not a false parity claim.
- The SR10–SR12 artifacts are development evidence and are not accepted
  baselines. If the model behavior change invalidates SR10/SR11, the run store
  records the invalidation; no post-hoc rescue is attempted.
- A separate agent is not guaranteed. The Quant Validator pass may be a
  sequential role pass, and the plan discloses that it is not independent.
- The model identity change is the recommended way to preserve the M1.2
  research record; if the human rejects it, the plan stops rather than silently
  changing M1.2 outputs.

## 5. Decisions requiring approval

| ID | Decision | Options | Recommendation | Consequence | Status |
|---|---|---|---|---|---|
| D1 | `level.fib_test` semantics | A. bar-range intersection and one event per tested level; B. keep endpoint proximity and correct the plan/claim; C. split into `fib_touch` and `fib_cross` | A | Event counts and fixtures change; SR10/SR11 may be invalidated; Quant Validator reproduction required | Approved |
| D2 | `fib_test_atr` / `band_atr` ownership | A. fixed: remove them from spec v2 and keep the model constants; B. configurable: mark configurable, add a config owner/default, validate the range, and document; C. keep the override and mark it configurable with a rename | A | Specs that set the two fields are rejected; the model doc/registry become consistent; no current repo spec uses them | Approved |
| D3 | Model identity and registry version | A. `M1.3` and registry `2026-11-10.2`; B. keep `M1.2` and record a repair note | A | M1.2 becomes historical; new contract, registry, README, and model-family record; existing M1.2 results keep their identity | Approved |
| D4 | Annotation-zone feature source | A. read the event batch; B. reimplement the exact event formulas; C. rename the features and document the close-based definition | A, fallback C | The zone report changes; no model behavior; the SR10–SR12 charters may cite the corrected report | Approved |
| D5 | SR10–SR12 disposition | A. record SR10/SR11 invalidation and rerun only if the human asks; B. rerun immediately on `M1.3` development; C. retire the runs without rerun | A | The invalidation log gains a tombstone; no validation/holdout access; no new edge claim | Approved |

D1, D2, and D3 are model/contract decisions. D4 and D5 are research-record
decisions. All five were approved with this plan on 2026-11-10; the table
records the approved choices. A material change to any decision stops the work
and returns to the human.

## 6. Required behavior

### 6.1 Sequence events (`AUD-01`)

- `occurred_bar_index` is the latest confirmed pivot's `occurred_at`.
- `available_bar_index` is that pivot's `available_at` (the confirmation bar).
- The event is still emitted only on the confirmation bar.
- The event count and direction are unchanged.
- A regression test resolves the pivot and asserts both fields for every
  sequence event; the fixture is regenerated only after the test is written.

### 6.2 Fibonacci test (`AUD-02`)

- A test is a bar-range intersection with `level ± fib_test_atr * ATR`, not only
  a candle high or low within tolerance.
- Every tested level emits an event; the event carries the ratio code in
  `reference_code`, the level in `reference_value`, close in `primary_value`,
  and the leg direction.
- The deduplication key remains code/source/bar/level/ratio; distinct tested
  levels on one bar are distinct events.
- The model contract, registry formula, and M1.2/M1.3 plan wording are aligned
  with the chosen definition.
- A fixture case with a bar spanning a level and closing on it emits the event;
  a bar whose range does not intersect emits none.

### 6.3 Tests (`AUD-03`)

- Unit tests for Fibonacci anchors, levels, position, nearest ratio/ATR, zero
  span, equal highs/lows, missing anchors, zero ATR, and confirmation delay.
- Unit tests for band clustering, largest-cluster selection, tie-break nearest
  close then lower center, two-level minimum, age, width, position, zero width,
  zero ATR, and equal highs/lows.
- A row availability test for 47–49 and 147–150 that asserts null before the
  required confirmed anchors and non-null at the first valid bar.
- `assert_extended_safe` and `assert_extended_empty` include all 22 new fields.
- Property tests for length, null/finite values, and append/prefix invariance.
- `verification.json`, `indicator-verification.md`, and
  `event-verification.md` cite the new tests.

### 6.4 Config ownership (`AUD-04`)

- Under D2-A, `fib_test_atr` and `band_atr` are removed from the spec-v2
  `readings.extended` allow-list and parser; the model constants remain the only
  source.
- A spec that sets either field is rejected with a clear error.
- Under D2-B, both fields are marked configurable, have a config owner/default,
  are validated to the chosen range in `extended_is_valid`, and are recorded in
  provenance.
- The registry, model contract, plan amendment, and parser agree on the chosen
  path; the `band_atr`/`cluster_atr` name is one term.

### 6.5 Verification record (`AUD-05`)

- A Quant Validator record at
  `docs/reports/2026-11-10/model-m1.3-quant-validation.md` (or the execution
  date) with the data version, config, commands, observed results, limitations,
  and verdict.
- It reproduces the sequence occurrence, Fibonacci event counts, band outputs,
  determinism, causality, and the no-future-data boundary.
- It records whether SR10/SR11 were invalidated and what replaced them.
- It is linked from the plan, the remediation PR, and the audit finding.

### 6.6 Tooling and documentation (`AUD-06`–`AUD-09`)

- The annotation zone features either come from the published event records or
  are renamed to `fib_close_proximity` / `band_close_inside` with an explicit
  definition in the report and README.
- The strategy config contract states the registered-subtype rule and the
  source/role restrictions that `validate_condition` actually enforces.
- No `expect` remains in the new library code; the band uses a null path.
- The event claims say “second or later consecutive” (or the detector is capped,
  only if a hypothesis justifies it).

## 7. Work breakdown

| # | Sub-issue | Findings | Depends on | Roles | Main paths |
|---:|---|---|---|---|---|
| 1 | Plan approval, audit status update, fix issues | all | human approval | Planner | audit doc, project board |
| 2 | Model identity `M1.3` and registry `2026-11-10.2` | D3 | 1 | Architect, Quant Developer | `crates/quantrule-model/src/lib.rs`, `docs/registry/`, `docs/model/2026-11-10/model-m1.3.md`, `docs/model/2026-11-05/model-families.md`, `docs/README.md` |
| 3 | Sequence event occurrence/availability | AUD-01 | 2 | Quant Developer, Test Engineer | `crates/quantrule-model/src/events/extended/structure.rs`, `events/extended/shared.rs`, fixtures, tests |
| 4 | Fibonacci test semantics | AUD-02, D1 | 2 | Quant Researcher, Quant Developer, Test Engineer | `crates/quantrule-model/src/events/extended/fibonacci.rs`, model contract, registry, fixtures, tests |
| 5 | Fibonacci/band formula and edge tests | AUD-03 | 3, 4 | Test Engineer, Quant Developer | `crates/quantrule-model/tests/per_row_extended_indicators.rs`, `tests/edge_cases.rs`, `src/readings/extended/tests.rs`, `verification.json`, matrices |
| 6 | Tolerance config ownership/validation | AUD-04, D2 | 2 | Quant Developer, Test Engineer | `crates/quantrule-shell/src/spec/v2.rs`, `crates/quantrule-model/src/spec/v2.rs`, registry, model contract, tests |
| 7 | Quant Validator record and SR10/SR11 invalidation | AUD-05, D5 | 3–6 | Quant Validator, Orchestrator | `docs/reports/2026-11-10/model-m1.3-quant-validation.md`, `docs/improvement/INVALIDATIONS.md` |
| 8 | Annotation-zone parity or rename | AUD-06, D4 | 4 | Quant Developer, Test Engineer | `tools/trades/annotation_zones.py`, `tools/trades/test_annotation_zones.py`, `tools/trades/README.md`, zone report |
| 9 | Strategy contract update | AUD-07 | 2 | Architect, Code Reviewer | `docs/config/2026-11-06/strategy-config.md`, strategy tests |
| 10 | Band `expect` removal | AUD-08 | 2 | Quant Developer, Test Engineer | `crates/quantrule-model/src/readings/extended/band.rs` |
| 11 | Event claim wording | AUD-09 | 2 | Quant Developer, Test Engineer | registry, generated Markdown |
| 12 | Full gate, PRs, audit statuses, AI logs | all | 2–11 | Test Engineer, Code Reviewer, Improvement Verifier, Orchestrator | repository-wide |

Ordering: 1 → 2 → 3 and 4 (4 depends on the D1 decision) → 5 → 6 → 7 → 8–11
→ 12. Sub-issues 3 and 4 both touch the event batch and the fixtures; they run
sequentially to avoid overlapping writes. Sub-issues 8–11 can run in parallel
once the behavior is frozen; they do not touch the model event logic.

Each sub-issue gets its own worktree and branch. Proposed branch names:

| Sub-issue | Proposed branch |
|---|---|
| 2 | `arch/audit-m12-model-identity` |
| 3 | `fix/audit-m12-01-sequence-occurrence` |
| 4 | `fix/audit-m12-02-fib-test` |
| 5 | `test/audit-m12-03-fib-band-coverage` |
| 6 | `fix/audit-m12-04-tolerance-config` |
| 7 | `docs/audit-m12-05-quant-validation` |
| 8 | `fix/audit-m12-06-annotation-parity` |
| 9 | `docs/audit-m12-07-strategy-contract` |
| 10 | `fix/audit-m12-08-band-expect` |
| 11 | `docs/audit-m12-09-event-claims` |

These were the planned branches. The execution note records that the work was
consolidated on `fix/audit-m12-remediation`; no issue or PR was created. The
traceability table below uses `TBD` for the PR links.

## 8. Traceability

| Finding | Severity | Sub-issue | Branch | PR | Regression guard | Status |
|---|---|---|---|---|---|---|
| AUD-2026-11-10-01 | Major | 3 | `fix/audit-m12-remediation` | #579 | sequence event occurred/available test; fixture | In progress (PR #579) |
| AUD-2026-11-10-02 | Major | 4 | `fix/audit-m12-remediation` | #579 | wide-bar test, non-intersection test, multiplicity test; fixture | In progress (PR #579) |
| AUD-2026-11-10-03 | Major | 5 | `fix/audit-m12-remediation` | #579 | formula/edge/property tests; edge helper includes new fields | In progress (PR #579) |
| AUD-2026-11-10-04 | Major | 6 | `fix/audit-m12-remediation` | #579 | spec rejection or validation tests; registry/parser agreement | In progress (PR #579) |
| AUD-2026-11-10-05 | Major | 7 | `fix/audit-m12-remediation` | #579 | Quant Validator record | In progress (PR #579) |
| AUD-2026-11-10-06 | Minor | 8 | `fix/audit-m12-remediation` | #579 | parity test or renamed definition | In progress (PR #579) |
| AUD-2026-11-10-07 | Minor | 9 | `fix/audit-m12-remediation` | #579 | contract/validator test | In progress (PR #579) |
| AUD-2026-11-10-08 | Minor | 10 | `fix/audit-m12-remediation` | #579 | no `expect`; null-path test | In progress (PR #579) |
| AUD-2026-11-10-09 | Minor | 11 | `fix/audit-m12-remediation` | #579 | registry text/formula test | In progress (PR #579) |

The execution was consolidated on `fix/audit-m12-remediation`; PR #579 carries the work. The execution PR moves each finding to `Fixed` after merge; the Improvement Verifier moves it to `Verified`.

## 9. Acceptance criteria

- `M1.3` is the current model identity and `2026-11-10.2` is the registry
  version; M1.2 remains historical and its records are unchanged.
- Sequence events carry the pivot occurrence and the confirmation availability;
  a regression test proves it.
- `level.fib_test` uses the D1 definition; range-intersecting levels emit, and
  every tested level is represented; tests and fixture prove it.
- Fibonacci and band formula, availability, tie-break, zero-ATR, zero-span,
  equal-high/low, and property tests exist; the generic edge suite includes all
  22 new fields.
- `fib_test_atr` and `band_atr` follow D2; the registry, model contract, parser,
  and validation agree; invalid values are rejected.
- A Quant Validator record exists and reproduces the corrected behavior; SR10/
  SR11 are invalidated or re-run per D5; SR12's model-version tie is recorded.
- Annotation features either match the event records or are renamed and
  documented; the report states which.
- The strategy config contract matches the validator; a test proves a registered
  extended event is accepted and a diagnostic is rejected.
- No `expect` remains in the new library code; the event claims match the
  detector.
- The full gate passes, the changelog is updated, the AI logs are present, and
  every audit finding is `Fixed` and then `Verified`.
- No 2024, validation, or holdout access; no edge, profitability, or strategy
  claim.

## 10. Verification

| Command | Expected |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets` | no warnings |
| `cargo test --workspace` | pass |
| `cargo doc` | no warnings |
| `python3 tools/check_registry.py` | pass; M1.3 and the new registry version |
| `python3 tools/check_validation.py` | pass; claims match |
| `python3 tools/check_verification.py` | pass; new evidence classes resolve |
| `python3 tools/check_verification_matrix.py` | pass; matrices regenerated |
| `python3 tools/check_event_emitters.py` | pass; 150 codes covered |
| `python3 tools/check_command_surface.py` | pass |
| `python3 tools/check_links.py` | pass |
| `python3 tools/check_terms.py` | pass |
| `python3 tools/check_changelog.py` | pass |
| `python3 tools/check_file_size.py` | pass |
| `python3 -m unittest discover -s tools/trades -p 'test_*.py'` | pass |
| `git diff --check` | pass |
| Quant Validator reproduction | sequence occurrence, Fibonacci counts, band values, determinism, causality, no future data; verdict recorded |

Additional evidence:

- Each audit finding has a fail-before/pass-after regression test, or a written
  reason a fail-before test is impossible.
- The golden fixture diff is reviewed line by line; no casual regeneration.
- The `M1.3` contract and registry are diffed against `M1.2`; every changed
  value is intentional.
- SR10/SR11 invalidation is recorded before any rerun; no validation/holdout
  access.
- The annotation report is regenerated (or renamed) and compared; no strategy
  claim is added.

## 11. Risks and recovery

| Risk | Mitigation |
|---|---|
| Fibonacci semantics change alters SR10/SR11 results | Record invalidation; rerun only as a new pre-registered development run if the human asks; never use validation/holdout |
| Fixture churn hides an unintended change | Review the full diff; keep the fixture regeneration as its own commit with the reason |
| Removing the config override breaks an existing spec | Search all specs; add a parser test; document the rejection and migration |
| Model identity change leaves stale references | `check_registry.py`, `check_links.py`, `check_terms.py`, and a full `rg` for `M1.2`/`2026-11-10.1` |
| Annotation event data is unavailable | Fall back to the rename option C with human approval; do not claim parity |
| QV finds a further defect | Stop, return the finding to remediation, and do not mark it `Fixed` |
| Sequential role passes are mistaken for independent review | Disclose the sequencing in the AI log and the QV record; do not claim independence |
| The plan expands into a general config-contract refactor | Keep the two tolerances in scope; route the broader surface to a separate Requirements/Audit item |

Recovery: each sub-issue is a separate branch and PR. A failed sub-issue is
reverted without affecting the others. The M1.2 contract, registry, and run
records remain historical. The audit document remains the status record.

## 12. Stop conditions

Stop and request human direction when:

- D1, D2, D3, D4, or D5 is rejected or materially changed.
- The Fibonacci test definition or multiplicity cannot be stated exactly.
- A fix would require a new dependency or a new condition language.
- A test would need to be weakened or a fixture regenerated without review.
- The model identity cannot be changed without breaking a published record.
- The annotation event data is unavailable and the rename option is not approved.
- The work would access 2024, validation, or holdout data.
- The work turns into a new strategy hypothesis rather than a defect fix.
- The Quant Validator cannot reproduce the corrected behavior.

## 13. Records and handoff

| Record | Path |
|---|---|
| Audit | [`docs/audits/2026-11-10/model-structures-audit.md`](../../audits/2026-11-10/model-structures-audit.md) |
| Model contract | `docs/model/2026-11-10/model-m1.3.md` (proposed) |
| Registry | `docs/registry/indicators.json`, `indicators.md`, `validation.json`, `verification.json` |
| Model family | `docs/model/2026-11-05/model-families.md` |
| Quant Validator | `docs/reports/2026-11-10/model-m1.3-quant-validation.md` (proposed) |
| Run invalidation | `docs/improvement/INVALIDATIONS.md` |
| Changelog | `CHANGELOG.md` `[Unreleased]` |
| AI logs | `docs/ai-transparency-log/2026-11-10/audit-model-structures-remediation.md` and one per sub-issue |
| Fix issues and PRs | Project board and this plan's traceability table |
| Finding statuses | The audit document |

## 14. Proposal

Recommended approval: D1-A, D2-A, D3-A, D4-A with fallback C, D5-A. This closes
all nine findings, preserves the M1.2 record under a new identity, avoids a
silent model behavior change, and keeps the scope to the audited surface.

If the human chooses D1-B, D2-B, or D3-B, the plan is revised before
implementation; the affected sub-issues and the Quant Validator record change
accordingly.
