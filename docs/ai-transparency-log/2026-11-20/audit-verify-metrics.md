# AI transparency log — Canonical verifier includes metrics (AUD-22 follow-up)

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) verification
  follow-up for `AUD-2026-11-17-22`.
- Claim type: Behaviour Claim.
- Intent: make `tools/verify.sh` recompute the same canonical digest as the
  evidence service after `metrics.json` entered the claim boundary.
- Boundary: branch `fix/audit-verify-metrics`, based on `main` `cae63157`.

## What changed

- `canonical_digest` and `canonical_artifacts` include `metrics.json` at any
  depth (`portfolio/metrics.json`).
- The verifier self-test fixture adds a metrics artifact and its expected
  digest is updated to `773aea9e...`.

## Verification observed

| Command | Result |
|---|---|
| `bash tools/verify.sh --self-test` | pass, digest `773aea9e…` |
| `tools/test_tiers.sh composed` on a clean stack | pass: 13 canonical artifacts on both runs, digest `a0372f02…`, gateway healthy |
| `tools/verify.sh --self-test` after the constant update | pass |

## Why the composed first run failed

The first composed attempt hit a stale MinIO bundle from a previous stack; a
clean `make reset` removed it. The second attempt then exposed the verifier's
old canonical filter: the service included `metrics.json`, the shell verifier
did not, so the recorded and recomputed digests differed. The third run passed
after this change.

## Quant validation

Not required: the verifier and canonical set changed; no model or strategy
value changed. The digest change was already validated in P2.8a.

## Learning

- The canonical-set change must be applied to every consumer, not only the
  service. The composed tier caught the shell verifier.
- A stale object store can make a correct bundle look corrupt; the composed
  tier contract assumes a clean stack.
