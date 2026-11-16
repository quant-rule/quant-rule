# AI Transparency Log — End-to-End Verification Workflow

- Date: 2026-11-16 (UTC)
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #847, `847-end-to-end-verification`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)

## Intent and boundary

Define the clean-stack verification workflow from `make up` and health through
upload, config validation, a coupled run, artifact verification, and a canonical
digest comparison. The AI wrote the workflow, its index and selection entries,
the changelog entry, and this log. It did not deploy, run the stack, or change a
service.

## Verification and evidence

- The workflow is marked `Current`; it uses the existing `run submit`,
  `run status`, `run fetch`, and `verify` commands and the existing
  `make verify` fixture.
- It states that verification is operational proof, not evidence, not a
  baseline, and not a qualification verdict.
- It is indexed in the workflow index and the status index.
- The checks are recorded in the pull request.

## Learning

- A digest match proves the deployment reproduces the reference result; it does
  not prove an edge. The workflow separates the two claims.
- A different image digest is a different execution. The verification record
  names the image digest rather than treating a mismatch as a silent failure.
- The release gate is separate from the verification gate. The verifier records
  the result; the human authorizes the tag.
