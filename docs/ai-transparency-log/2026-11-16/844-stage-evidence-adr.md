# AI Transparency Log — Stage Execution and Evidence Boundary ADR

- Date: 2026-11-16 (UTC)
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #844, `844-stage-evidence-adr`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)

## Intent and boundary

Record the architecture decision for diagnostic stage execution and the evidence
boundary. The AI drafted the ADR, the architecture pointers, the changelog
entry, and this log. It did not change engine code, configs, schemas, or tests.

## Verification and evidence

- The ADR is under the 110-line cap and its links resolve.
- `http-api.md` §8 and `data-plane.md` §8 now carry a bounded stage-run exception
  pointing to the ADR; no other prohibition changed.
- The changelog entry is under `[Unreleased] / Added`.
- The checks are recorded in the pull request; the ADR is not marked verified by
  this log.

## Learning

- The standalone strategy stage cannot be evidence because it simulates its own
  position; the coupled run supplies the execution position. The ADR records
  this as the reason strategy and portfolio stages are recomputed.
- The model artifact is the one safe reuse: the worker already consumes a
  content-addressed model bundle, so `run submit --model-artifact` can preserve
  byte-identical model evidence while recomputing the rest.
