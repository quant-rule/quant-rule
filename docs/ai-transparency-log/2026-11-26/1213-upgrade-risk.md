# AI Transparency — Versioned Upgrade and Rollback Accepted Risk

- Date: 2026-11-26 (UTC)
- Work item: `REM-HA-003` ([#1213](https://github.com/quant-rule/quant-rule/issues/1213))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim evidence record; no model, strategy, or edge claim.

## 1. Intent and boundary

Record the v1.6.0 → v1.6.1 upgrade and rollback evidence available on the single
laptop, and record the v3 Swarm rolling gate as accepted risk. The Compose image
swap and rollback are measured; the three-VM lab, active-run continuity,
no-partial-publish check, and `tools/swarm.sh` rollback are not run and are not
claimed.

## 2. Implemented

- `docs/reports/2026-11-26/ha-003-upgrade-risk.md` records the Compose
  measurement, the missing v3 evidence, the accepted risk, and the release
  boundary.
- The plan records V-HA-03 as `Accepted risk`.
- No code, descriptor, image, or release note was changed.

## 3. Verification

Commands actually run:

- Compose recreate with `quantrule-server:v1.6.0`, wait healthy, `/version`
  1.6.0, `quantrule verify` v4/v5.
- Compose recreate with `quantrule-server:v1.6.1`, wait healthy, `/version`
  1.6.1, `quantrule verify` v4/v5.
- The v4 digest `a0372f02…` and v5 digest `761396c6…` were unchanged.
- `deploy/v3/lab.sh --plan` prints the v3 sequence; `virsh -c qemu:///system
  list --all` timed out, so no lab VMs exist to run it.
- `qr-governance check --all` and `tools/gate.sh --changed main` → recorded in
  the pull request.

No v3 Swarm service update, active run during the swap, partial-publish check,
signed image, or `tools/swarm.sh` invocation was performed.

## 4. Learning

- A Compose full-stack recreate is not a rolling update; it proves the image
  transition and digest stability, not service-by-service continuity.
- The lab driver container exists, but its libvirt connection is unavailable and
  the three VMs are not provisioned; the v3 gate is not runnable on this host.
- The v1.6.1 release note must distinguish the Compose measurement from the v3
  Swarm accepted risk.

## 5. Next action

Push, pull request, merge and cleanup remain gated. HA-006 updates the matrix
with this accepted risk and the other accepted risks.
