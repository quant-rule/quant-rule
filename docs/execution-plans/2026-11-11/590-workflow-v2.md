# Development Workflow v2

- Status: Verified
- Date: 2026-11-11 (UTC)
- Issue: #590
- Claim type: Behaviour Claim
- Owner: Planner / Implementer / Reviewer
- Workflow: [Execution Plan](../../workflows/execution-plan.md)

## Intent

Make the four-stage board, the four claim types, the start and close dates, the
review gate, and the execution-plan verification table clear and enforceable
before product or course work begins.

## Current state

The board has `Todo`, `In Progress`, `In Review`, and `Done`, plus `Start date`
and `Closed date`. It did not have a `Claim Type` field. The lifecycle text in
`AGENTS.md` still described three stages and set `Done` when the pull request
opened. The project-sync script reset a leaf to `Todo` from its open state and
could overwrite `In Review`.

## Scope

Included:

- Add the `Claim Type` field with the four approved options.
- Update `AGENTS.md` lifecycle, review gate, dates, and verification table.
- Add the Execution Plan workflow and template.
- Update issue and pull-request templates.
- Update project-sync automation and CI checks.
- Pilot the workflow on this issue.

Excluded:

- Product code.
- The Tauri Platform implementation.
- Changes to model, strategy, execution, or portfolio behavior.

## Decisions

| ID | Decision | Consequence |
|---|---|---|
| D1 | Four claim types: Problem, Behaviour, Outcome, ADR | Every issue has one claim boundary. |
| D2 | `In Review` is the human gate | `Done` follows acceptance, merge, and cleanup. |
| D3 | `Start date` and `Closed date` are agent-owned with automation backfill | The board records the real work interval. |
| D4 | Every plan ends with a verification table | The reviewer has a row-by-row checklist. |

## Work breakdown

| Phase | Work | Output |
|---|---|---|
| 1A | Board field and automation | Claim Type field; four-stage-safe project-sync |
| 1B | Documents and templates | AGENTS, workflow, template, issue/PR templates |
| 1C | Checkers and pilot | `check_claims.py`, `check_execution_plans.py`, CI, pilot |

## Verification

| Check | Command | Expected |
|---|---|---|
| Claim types | `python3 tools/check_claims.py` | pass |
| Execution plans | `python3 tools/check_execution_plans.py` | pass |
| Workflows | `python3 tools/check_workflows.py` | pass |
| Links | `python3 tools/check_links.py` | pass |
| Terms | `python3 tools/check_terms.py` | pass |
| Legacy | `python3 tools/check_legacy.py` | pass |
| Project sync syntax | `bash -n .github/scripts/project-sync.sh` | pass |

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Claim Type field | Four options exist on the board | `gh project field-list 1 --owner quant-rule` | Field and options present | Field exists with four options | Pass | Agent | 2026-11-11 | |
| V2 | Lifecycle documents | Four stages and dates are normative | `python3 tools/check_workflows.py` | pass | 17 workflows, 8 templates | Pass | Agent | 2026-11-11 | |
| V3 | Templates | Every issue/PR template names the four types | `python3 tools/check_claims.py` | pass | pass | Pass | Agent | 2026-11-11 | |
| V4 | Execution plan rule | New plans require the verification table | `python3 tools/check_execution_plans.py` | pass | pass | Pass | Agent | 2026-11-11 | |
| V5 | Automation | `In Review` is preserved; dates and claim type are read | `bash -n .github/scripts/project-sync.sh`; issue #590 sync | script runs; fields preserved | syntax pass; #590 In Progress and start date preserved | Pass | Agent | 2026-11-11 | |
| V6 | Pilot | Issue #590 passes Todo -> In Progress -> In Review -> Done | GitHub Project | lifecycle works | PR #592 merged; issue #590 Done | Pass | Human | 2026-11-11 | |

## Risks and recovery

| Risk | Control | Recovery |
|---|---|---|
| Automation resets `In Review` | Preserve leaf status in project-sync | Fix script before pilot |
| Missing claim type | Template and `Missing claim type` view | Keep the issue in `Todo` |
| Plan table becomes theatre | Reviewer fills observed status | Return failed rows |

## Stop conditions

- A workflow change requires a new claim type outside the four.
- The project-sync script cannot preserve `In Review`.
- The verification table cannot be checked by a reviewer.

## Related documents

- [`AGENTS.md`](../../../AGENTS.md)
- [`docs/workflows/execution-plan.md`](../../workflows/execution-plan.md)
- [`docs/workflows/recordkeeping.md`](../../workflows/recordkeeping.md)
