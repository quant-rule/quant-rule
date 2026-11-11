# AI Transparency Log — Development Workflow v2

- Date: 2026-11-11 (UTC)
- Work item: `590-workflow-v2`
- Issue: [#590](https://github.com/quant-rule/quant-rule/issues/590)
- Plan: [`docs/execution-plans/2026-11-11/590-workflow-v2.md`](../../execution-plans/2026-11-11/590-workflow-v2.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash)
- Branch: `chore/590-workflow-v2`
- Claim type: Behaviour Claim

## Intent and boundary

Make the four-stage board, the four approved claim types, the start and close
dates, the human review gate, and the execution-plan verification table
normative and checkable. No product code and no model, strategy, execution, or
portfolio behavior changed.

## Work performed

- Added the `Claim Type` project field with the four approved options.
- Updated `AGENTS.md` with the claim-type section, four-stage lifecycle, date
  rules, review gate, board field ownership, and the verification-table rule.
- Added `docs/workflows/execution-plan.md` and its template.
- Updated the workflow index, recordkeeping, requirements/design handoff, issue
  templates, pull-request template, and the README development checks.
- Updated `project-sync.sh` to preserve leaf `In Progress` and `In Review`, set
  and backfill dates, read `Claim Type`, and compute four-stage container status.
- Added `tools/check_execution_plans.py`, `tools/check_claims.py`, and their
  tests; wired them into CI.
- Added the Phase 1 execution plan and this log.

## Verification

- `python3 tools/check_claims.py`: pass.
- `python3 tools/check_execution_plans.py`: pass.
- `python3 tools/check_workflows.py`: pass.
- `python3 tools/check_links.py`: pass.
- `python3 tools/check_terms.py`: pass.
- `python3 tools/check_legacy.py`: pass.
- `python3 tools/check_file_size.py`: pass.
- `python3 tools/check_structure.py`: pass.
- `bash -n .github/scripts/project-sync.sh`: pass.
- `python3 -m unittest discover -s tools -p 'test_check_execution_plans.py'`: pass.
- `python3 -m unittest discover -s tools -p 'test_check_claims.py'`: pass.

## Learning

- The project-sync script previously reset an open leaf to `Todo`, so `In Review`
  could not survive an event. It now preserves the agent-set leaf status.
- The existing `simulation-data-recovery.md` plan predates the verification-table
  rule and is explicitly exempt; it is not rewritten.
- The board field is `Closed date`; the workflow prose calls it the close date.
  The field name is unchanged to keep the automation compatible.
