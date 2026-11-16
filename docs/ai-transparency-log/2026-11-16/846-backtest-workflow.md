# AI Transparency Log — Backtest Workflow

- Date: 2026-11-16 (UTC)
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #846, `846-backtest-workflow`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)

## Intent and boundary

Define the coupled evidence workflow from a frozen configuration and a declared
window through submission, observation, artifact verification, validation,
recording, and reproduction. The AI wrote the workflow, its index and selection
entries, the changelog entry, and this log. It did not change the run path, the
worker, the canonical digest, or any engine.

## Verification and evidence

- The workflow is marked `Current` for the existing `run submit` path; the model
  artifact reuse flag is marked as landing in #855.
- It states the evidence rule: only a coupled run produces a canonical digest;
  stage artifacts never do; strategy and portfolio are recomputed.
- It is indexed in the workflow index and the status index.
- The checks are recorded in the pull request.

## Learning

- A backtest is not an improvement run. It answers what a frozen configuration
  did; the Improvement Workflow decides whether a change is better.
- The model artifact is the one reusable input. Strategy and portfolio cannot be
  reused because the coupled run supplies the position execution actually holds.
- A failed run is not partial evidence. The workflow says so explicitly so a
  retry cannot be mistaken for a result.
