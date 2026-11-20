# AI transparency log — v1.4.2 release recovery

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Release & Operations recovery for `v1.4.1`; issue
  [#989](https://github.com/quant-rule/quant-rule/issues/989).
- Claim type: Behaviour Claim.
- Intent: publish a patch release after the `v1.4.1` desktop package smoke failed
  on a stale canonical digest, without moving the published `v1.4.1` tag.
- Boundary: branch `fix/989-v142-recovery`, based on `main` `51c95392`; no HA
  work and no command or screen contract change.

## What changed

- `apps/desktop-shell/tests/smoke-package-parity-linux.sh` and
  `smoke-package-parity.ps1` default to the current canonical digest
  `sha256:a0372f02…`; the README records the same digest.
- Version `1.4.2` across the workspace, web, desktop, and supervisor manifests;
  the model-object oracle is regenerated for the version string.
- `CHANGELOG.md`, the `v1.4.2` release note, the withdrawn `v1.4.1` note, the
  `v1.4.0` addendum, the release map, README, and current-version references.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-model-service --test model` | 7 passed; oracle `sha256:35bb3120…`, 66,022 bytes |
| `apps/desktop-shell` `bun test` | 35 passed, 0 failed |
| Packaged Linux smoke | pass; `QuantRule_1.4.2_amd64.AppImage` reproduced `sha256:a0372f02…`; seeded mismatch failed as required |
| `tools/test_tiers.sh composed` | 8 targets passed; product 1.4.2; digest `sha256:a0372f02…` |
| `tools/gate.sh` | `workspace gate passed` |
| `make up` | `QuantRule v1.4.2 ready → http://localhost:8080` |
| Live archive smoke on 1.4.2 | `archived_at` RFC3339; active list hides; `include_archived` shows; erase removes |

## Why v1.4.1 was not re-run

The `v1.4.1` tag is published and its workflow checks out the tag, so a re-run
would execute the same stale digest constant. The project rule forbids moving,
deleting, or reusing a published tag. The correction is a new patch version.

## Learning

- The PR gate does not build the desktop package; only the release workflow runs
  the packaged smoke. A stale digest constant therefore surfaced after tagging.
- The digest is recorded in three places — both smoke scripts and the README —
  and the release workflow's parity step also carries it. A future change should
  derive it from one checked-in oracle.
- The package smoke observes the same `a0372f02…` digest as the composed tier,
  which confirms the desktop and server profiles still agree.
