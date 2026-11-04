# Research Governance and Directional Program Closeout

- Date: 2026-11-04 (UTC)
- Status: **Approved** (human, 2026-11-04, "proceed with the full implementation")
- Issue: [#386](https://github.com/quant-rule/quant-rule/issues/386) and
  [#387](https://github.com/quant-rule/quant-rule/issues/387)
- Branch: `docs/386-research-governance`
- Claim type: documentation and research-governance change; no model, strategy,
  or edge claim

## 1. Intent

Close the failed directional event-family program, preserve its evidence, and
write the research mandate and program rules that govern the next research
program. The closeout must not invalidate the accepted model baselines or start
a new strategy attempt.

## 2. Current state

- Five 15m event families failed the model-development gate:
  `MODEL-2026-11-04-13..17`.
- No model validation, strategy run, or strategy baseline exists for them.
- The five run records are local under each research worktree's
  `data/improvement/2026-11-04/`.
- The five candidate docs and their plans/AI logs are on five unmerged local
  branches.
- AGENTS.md and the improvement workflow do not yet define a program charter,
  research budget, or the no-auto-search rule.

## 3. Scope

### Included

- Preserve the five run records in the main checkout's local
  `data/improvement/2026-11-04/`.
- Commit the five candidate docs, plans, and AI logs on the governance branch.
- Add `DIRECTIONAL-EVENT-PROGRAM-CLOSEOUT.md`.
- Add `docs/governance/2026-11-04/research-mandate.md` and
  `program-charter-template.md`.
- Add the research goal/evidence standard and program rules to AGENTS.md.
- Add the program-charter protocol to `docs/workflows/improvement-workflow.md`.
- Update `docs/improvement/baselines.md` and `index.md`.
- Remove the five research worktrees and branches after the evidence is
  preserved and the governance branch is pushed.

### Excluded

- No model or strategy code change.
- No new model gate or strategy run.
- No validation access.
- No baseline acceptance.
- No research campaign.

## 4. Assumptions and uncertainty

- The five run records are complete and reproducible; the candidate docs record
  their hashes.
- The five branches are local and unmerged; copying their docs into the
  governance branch preserves the evidence before cleanup.
- The human approved the research mandate and the program rules in the current
  instruction.

## 5. Decisions

| Decision | Consequence |
|---|---|
| Stop the directional program | No validation/strategy for the five families; no baseline |
| Preserve run records locally | The committed docs reference local evidence; the records survive worktree cleanup |
| Copy candidate docs/plans/AI logs | The process record is committed before the five branches are removed |
| AGENTS.md gets the normative rules | The workflow doc gets the detailed protocol |
| No auto-search | A batch is a finite pre-registered list; the human promotes |

## 6. Acceptance criteria

- The five run records exist in the main checkout's
  `data/improvement/2026-11-04/`.
- The five candidate docs, plans, and AI logs are committed on the governance
  branch.
- The closeout doc records the five results and the program stop.
- AGENTS.md states the research goal, evidence standard, program/hypothesis
  distinction, exploration/confirmation boundary, correction, stop rule, and
  no-auto-search rule.
- The workflow doc contains the program-charter protocol.
- `baselines.md` records no strategy baseline and the stopped program.
- `index.md` records the five model runs as no-edge.
- `tools/check_terms.py` passes.
- The five research worktrees and branches are removed only after the governance
  branch is pushed and the run records are preserved.

## 7. Work breakdown

| Step | Work | Paths |
|---|---|---|
| 1 | Preserve the five run records | `data/improvement/2026-11-04/MODEL-*` |
| 2 | Copy candidate docs/plans/AI logs from the five branches | `docs/improvement`, `docs/execution-plans`, `docs/ai-transparency-log` |
| 3 | Write the closeout | `docs/improvement/2026-11-04/DIRECTIONAL-EVENT-PROGRAM-CLOSEOUT.md` |
| 4 | Write the mandate and charter template | `docs/governance/2026-11-04/` |
| 5 | Update AGENTS.md and the workflow | `AGENTS.md`, `docs/workflows/improvement-workflow.md` |
| 6 | Update the baseline and run registries | `docs/improvement/baselines.md`, `index.md` |
| 7 | Verify, commit, push, open PR | branch `docs/386-research-governance` |
| 8 | Merge and clean the five worktrees/branches | after the PR |

## 8. Verification

| Check | Command | Expected |
|---|---|---|
| Run records | `ls data/improvement/2026-11-04/MODEL-2026-11-04-1{3,4,5,6,7}` | all five exist |
| Candidate docs | `ls docs/improvement/2026-11-04/*-CANDIDATE.md` | five files |
| Terms | `python3 tools/check_terms.py` | no banned terms |
| Diff | `git diff --check` | no whitespace errors |
| Markdown links | inspect changed docs | links resolve |

Limitations: the run records are local ignored data, not committed. The five
candidate docs are same-session evidence, not an independent Quant Validator
record. No strategy tier is opened.

## 9. Risks and recovery

- **Evidence loss during cleanup.** Preserve the run records and copy the docs
  before removing the worktrees. Recovery: the local run records and the
  committed docs remain.
- **Governance docs conflicting with AGENTS.md.** The new section is concise;
  the detailed protocol lives in the workflow doc.
- **Accidental baseline acceptance.** The closeout says no strategy baseline and
  no validation access.
- **Registry inconsistency.** `baselines.md` and `index.md` are updated in the
  same change; the local run store remains the working copy.

## 10. Stop conditions

- A run record is missing or corrupt.
- The five branches do not contain the expected candidate docs.
- A shared file conflict.
- A request to open validation or run a strategy.
- A request to commit, push, or merge without the current authorization.
