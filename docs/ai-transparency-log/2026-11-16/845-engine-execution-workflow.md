# AI Transparency Log — Engine Execution Workflow

- Date: 2026-11-16 (UTC)
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #845, `845-engine-execution-workflow`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)

## Intent and boundary

Define the diagnostic execution workflow for the data, model, strategy, and
portfolio engines, with four stage runbooks and the evidence rule. The AI wrote
the workflow, the index rows, the AGENTS.md selection entry, the changelog
entry, and this log. It did not implement a command, route, schema, or engine
change.

## Verification and evidence

- The workflow is marked `Target` because the stage commands land in #850–#854;
  it is a contract, not a claim that the commands already exist.
- The workflow is indexed in `docs/workflows/README.md` and `docs/README.md`.
- The document names the evidence rule, model artifact reuse, and the
  strategy/portfolio recompute rule.
- The checks are recorded in the pull request.

## Learning

- The data engine does not need a `data run`: upload plus inspection already
  gives the dataset identity, quality, and ladder. A separate command would
  duplicate the catalog.
- Execution is not a separate stage in the trader workflow. It is the fill half
  of the portfolio stage because fills and accounting are one boundary.
- The workflow is `Target` until the commands exist; marking it `Current` now
  would claim a capability the tree does not have.
