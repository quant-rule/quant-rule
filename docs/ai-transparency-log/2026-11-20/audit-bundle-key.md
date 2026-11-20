# AI transparency log — Model bundle key includes the image identity (AUD-29 follow-up)

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) follow-up to
  `AUD-2026-11-17-29`.
- Claim type: Behaviour Claim.
- Intent: stop a rebuilt image from colliding with a model bundle whose
  presentation bytes changed.
- Boundary: branch `fix/audit-bundle-key`, based on `main` `6953fc42`.

## What changed

- The model bundle key is now
  `model/<data>/<config>/<model>/<image>/<ladder>/bundle`; the image digest is
  `no-image` when the server has none.
- `bundle::prefix` is a named helper with a unit test proving two images have
  different keys.

## Why

The label alignment changed `summary.txt` inside the model bundle. The composed
tier rebuilt the image and computed the new bundle, found the old bundle under
the same key, and refused the mismatch:
`the stored artifact .../bundle/summary.txt does not match the computed bundle`.
The key did not include the producer image, so a content-addressed bundle could
collide across code changes.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-model-service --lib` | 7 passed, including the image-key test |
| `tools/gate.sh` | pass; 25 checker suites and the workspace tests |
| `tools/test_tiers.sh composed` after rebuild | pass; queue isolation, 8 Postgres targets, 13 canonical artifacts per run, digest `sha256:a0372f02…` |

## Quant validation

Not required: the numeric model output is unchanged; the bundle key and
artifact path are cache identity.

## Learning

- A content-addressed key must include every producer input that can change
  the bytes. The image digest is one such input.
- The canonical digest is unchanged because `summary.txt` is not in the
  canonical set; the stale bundle was a cache collision, not a numeric drift.
