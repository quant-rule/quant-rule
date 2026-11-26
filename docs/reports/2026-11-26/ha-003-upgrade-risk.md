# HA-003 — Versioned Upgrade and Rollback Accepted Risk

- Date: 2026-11-26 (UTC)
- Status: **Accepted risk** for the v3 Swarm rolling gate; the Compose image
  swap and rollback are `Measured`.
- Work item: [#1213](https://github.com/quant-rule/quant-rule/issues/1213)
  (`REM-HA-003`, V-HA-03)
- Plan: [`rem-000-v1.6.1-remediation.md`](../../execution-plans/2026-11-26/rem-000-v1.6.1-remediation.md)
- Claim type: Behaviour Claim evidence record; no model, strategy, or edge claim.
- Boundary: one host, Compose; no v3 Swarm rolling update, no active run during
  the swap, no signed image.

## 1. What the v3 gate requires

A v1.6.0 → v1.6.1 rolling update on the v3 Swarm profile (or the three-VM lab),
with an active run completing, no partial publish, the canonical digest
unchanged, and `tools/swarm.sh rollback` returning to v1.6.0. The image must be
resolved and verified; the lab path may continue only with `--allow-unverified`.

## 2. Compose measurement

The v1.6.1 stack was recreated on the same volumes with the v1.6.0 image and then
upgraded back to v1.6.1:

| Step | `/version` | Image digest | v4 digest | v5 digest |
|---|---|---|---|---|
| v1.6.1 | `1.6.1` | `sha256:d14ba602…` | `sha256:a0372f02…` | `sha256:761396c6…` |
| rollback to v1.6.0 | `1.6.0` | `sha256:b76b2ec7…` | `sha256:a0372f02…` | `sha256:761396c6…` |
| upgrade to v1.6.1 | `1.6.1` | `sha256:d14ba602…` | `sha256:a0372f02…` | `sha256:761396c6…` |

The stack reported healthy after each recreate, and the v4/v5 canonical digests
were unchanged. The image swap is a full-stack recreate, not a service-by-service
rolling update, and no run was in flight at the moment of the swap.

## 3. What was not run

- `deploy/v3/lab.sh` / the three-VM Swarm lab: the host's libvirt connection
  timed out and the lab VMs are not provisioned.
- `tools/swarm.sh upgrade` and `tools/swarm.sh rollback`: they need the v3
  Swarm stack and a resolved image; no such stack exists here.
- A signed candidate image: REL-002 records the signature accepted risk.
- An active run spanning the upgrade and a no-partial-publish check.

## 4. Accepted risk and release boundary

The v3 rolling upgrade and Swarm rollback are accepted risk on this laptop. The
v1.6.1 release note must say the Compose image swap and rollback are `Measured`
and the v3 Swarm rolling gate is `Accepted risk`, not `Verified`. No rolling
update, active-run continuity, or partial-publish claim is made.

## 5. What would close it

On a three-VM lab or real hosts:

1. Push/load the v1.6.1 image and resolve its digest.
2. Run `tools/swarm.sh upgrade` with a run in flight; confirm the run completes,
   no partial publish occurs, and the canonical digest matches.
3. Run `tools/swarm.sh rollback`; confirm v1.6.0 returns and the digest is
   unchanged.
4. Record the evidence and replace this accepted risk with a `Simulated HA` or
   `Real HA` row as appropriate.

## 6. Verification commands actually run

- Compose rollback: `QR_IMAGE=quantrule-server:v1.6.0` recreate, wait healthy,
  `/version` 1.6.0, `verify` v4/v5.
- Compose upgrade: `QR_IMAGE=quantrule-server:v1.6.1` recreate, wait healthy,
  `/version` 1.6.1, `verify` v4/v5.
- `docker inspect` image IDs; `curl /version`; `quantrule verify`.
- `deploy/v3/lab.sh --plan`; `virsh -c qemu:///system list --all` → connection
  timeout (no lab VMs).
- `qr-governance check --all` and `tools/gate.sh --changed main` → recorded in
  the pull request.

## 7. Verdict

**Accepted risk.** The Compose image swap and rollback preserve the v4/v5
digests and the version contract. The v3 Swarm rolling update, active-run
continuity, no-partial-publish check, and `tools/swarm.sh rollback` were not run
and are not claimed.
