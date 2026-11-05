# AI Transparency Log — M2.x Model Programme (P0)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session; model DeepSeek V4.1 Flash
- Work item: #410 M2.x model programme, P0 documentation
- Plan: [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](../../execution-plans/2026-11-05/410-m2-model-programme.md)
- Status: P0 documentation written in the `docs/410-m2-model-programme`
  worktree; the commit and pull request follow this record

## 1. Intent and boundary

The human approved the M2.x programme and instructed the agent to write the
supporting documents properly, including the swarm exploratory research mode.
This pass covers P0 only:

- The plan of record for the M2.x programme.
- The naming registry for `M1.0`, `S1.1–S1.5`, `M2.1–M2.5`, and `S2.1+`.
- The retrospective Phase 1 indicator audit.
- The legacy inventory and removal boundary.
- Updates to the 405 plan, the indicator verification checklist, the research
  swarm runbook, the indicator coverage status, and the documentation index.

The agent did not change model or strategy behavior, did not add a dependency,
did not delete any legacy file, and did not run the M2 development or validation
partitions. P1–P8 remain proposed work.

## 2. What was produced

| Artifact | Kind |
|---|---|
| `docs/execution-plans/2026-11-05/410-m2-model-programme.md` | Plan of record |
| `docs/model/2026-11-05/model-families.md` | Naming registry and aliases |
| `docs/improvement/2026-11-05/INDICATOR-AUDIT.md` | Retrospective top-ten audit |
| `docs/execution-plans/2026-11-05/410-legacy-inventory.md` | Legacy keep/remove inventory |
| `docs/execution-plans/2026-11-05/405-indicator-platform.md` | Phase status and M2.x handoff |
| `docs/improvement/2026-11-05/INDICATOR-VERIFICATION.md` | Audit delivered; spec gap recorded |
| `docs/model/2026-11-05/indicator-coverage.md` | Implementation status; absorption formula reconciled |
| `docs/workflows/research-swarm.md` | Exploratory and confirmatory model modes |
| `docs/README.md` | Contract and program index |

### 2.1 Issue and board actions

After the human approved the P0 commit, the agent:

- Renamed #410 to “M2.x model swarm and validation”, re-scoped its body, and set
  it In Progress.
- Renamed #405 to “Build the indicator-complete model platform” and re-scoped
  its body.
- Closed #426 as superseded (not deleted) and #409 as delivered.
- Re-scoped #427–#430 to the M2.x phases.
- Created #454 (model spec v2), #455 (prediction evaluator), #456 (M2.x charter
  and coverage), and #457 (P0 naming/audit/legacy).
- Added native sub-issue links: #410 now contains #457, #454–#456, and
  #427–#430; #405 contains #410.
- Set #457 In Progress and #454–#456 Todo; labelled the PR item as a pull
  request and set it In Progress.

The prompts and bodies were written by the agent; the human approved the P0
commit. The issue edits are repository metadata, not code or evidence claims.

## 3. Verification and evidence

The agent ran these commands in the worktree and observed the listed results.

| Command | Observed result |
|---|---|
| `python3 tools/check_registry.py` | `registry ok: 46 indicators, 143 events, 110 parameters` |
| `cargo test -p quantrule-model --lib` | `573 passed; 0 failed; 0 ignored` |
| `cargo test -p quantrule-model --test event_validation` | `8 passed; 0 failed; 1 ignored` |
| `cargo test -p quantrule-model readings::extended::tests::vwap_is_the_volume_weighted_typical_price` | `1 passed; 0 failed` |
| `cargo fmt --check` | exit 0 |
| `python3 tools/check_links.py` | 531 Markdown files; no broken relative links |
| `python3 tools/check_terms.py` | 337 files; no banned terms |
| `git diff --check` | exit 0 |

The event-validation suite's ignored test is the large fixture run; the agent did
not claim it passed. The agent did not run the full workspace test suite in this
pass; the P0 change is documentation-only. The agent did not run `cargo clippy`
in this pass and does not claim it passed.

## 4. Findings and corrections

### 4.1 Findings from the retrospective audit

- **F-04:** `volume_oscillator` is listed as an output of `rvol` and is also a
  separate indicator (code 15). The computation is shared; ownership is
  duplicated. Recorded as Open; the recommended fix is to remove it from
  `rvol.outputs`.
- **F-05:** the coverage document's absorption formula used a high/low distance
  to a level, while the registry and implementation use the close distance to
  VWAP or POC. The agent updated the coverage document in this P0 change and
  marked the finding Fixed.
- **F-07:** the registry has 143 unique events but 148 indicator references; the
  five shared IDs must be deduplicated by the coverage checker.
- **F-02:** 37/37 rows are `tested`, 0/37 `verified`; the `Sign` column is a
  human/Validator action. The agent did not fill it.

### 4.2 Corrections made during the pass

- The first `model-families.md` draft linked the legacy inventory with a path
  relative to `docs/model`, which resolved to a missing file. The link checker
  caught it; the path was corrected to
  `../../execution-plans/2026-11-05/410-legacy-inventory.md`.
- The first plan draft put the legacy inventory at `docs/chore/`, a kind the
  documentation index does not define. The plan of record uses
  `docs/execution-plans/2026-11-05/410-legacy-inventory.md`.

## 5. Suggestions rejected or deferred

| Suggestion | Decision | Reason |
|---|---|---|
| Use bare `1.1`, `1.2` IDs | Rejected | A model and a strategy would collide in a flat namespace |
| Reuse v6–v10 for the M2 models | Rejected | Those numbers are the existing `S1.x` strategies and their history |
| Rename every historical file to `S1.x` | Rejected | Large diff, broken links, no evidence value; the registry maps aliases |
| Delete issue #426 instead of closing it | Rejected | The human wants the history record; closing preserves the link |
| Expand the audit to all 46 indicators now | Deferred | The plan named the top ten; the checklist covers the other 36 |
| Delete `prediction_pilot.py` in the legacy sweep | Deferred | P2 may reuse its forward-outcome machinery; remove only after a tested replacement |
| Sign the checklist rows on the human's behalf | Rejected | The `Sign` column is a human/Validator action; the agent cannot verify its own work |
| Mark the audit as pre-registration | Rejected | It is retrospective; the limitation is stated in the audit and the plan |

## 6. Learning

- The naming collision was real: the old v6–v10 names are strategies, and the M2
  model family needs its own prefix.
- A retrospective audit can still find contract mismatches, but it cannot
  recover the gate that the original plan intended. The limitation is recorded
  rather than hidden.
- The registry count (143 unique events) and the indicator-reference count (148)
  differ because five events are shared. A coverage checker must dedupe.
- The checklist's `tested` status is not `verified`; the `Sign` column is a
  separate human decision.
- The v1 spec is condition-only. The M2 swarm cannot run until P1 adds reading
  overrides, active events, prediction, and panel/session inputs.

## 7. Evidence boundary

This log records what the agent wrote and ran. It does not claim:

- that the M2 development swarm has run;
- that the validation partition has been opened;
- that any model or strategy has an edge;
- that the 37 checklist rows are `verified`;
- that the legacy removal has been performed;
- that the full workspace test suite or clippy passed in this pass.

The next gate is the human's authorization to commit, push, and open the P0 pull
request. P1 starts only after that review.
