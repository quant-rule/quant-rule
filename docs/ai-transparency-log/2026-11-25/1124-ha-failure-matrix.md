# AI Transparency Log — HA Failure Matrix

- Date: 2026-11-25 (UTC)
- Work item: QA-003 [#1124](https://github.com/quant-rule/quant-rule/issues/1124)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017

## Intent and boundary

Close QA-003 by mapping every HA-01…HA-23 row to its evidence path and tier.
The lab evidence is the same-day HA-014 `Simulated HA` run; no new lab
provisioning was performed for this documentation pass.

## Work performed

- Read the 23-row `chaos-matrix.json` and the HA-014 `lab.md`/JSON evidence.
- Promoted HA-03 and HA-17 from `Unverified` to `Simulated HA` because the lab
  evidence explicitly shows `GatewayReplicaDown` and `SwarmManagerQuorumAtRisk`
  firing and resolving with the digest unchanged.
- Wrote `docs/reports/2026-11-25/ha-failure-matrix.md` with the 13 lab rows, the
  8 static alert/check rows, and the 2 real-host `Unverified` rows.
- Updated the v1.6.0 plan: 40 Done/6 Todo; QA-003 Done; V18 still pending on
  the real-host rows.

## Verification and evidence

- `docs/reports/2026-11-25/ha-014/lab.md`, `HA-01.json`, `database.json`,
  `queue.json`, `object.json`, `HA-18..21.json`.
- `docs/config/2026-11-24/chaos-matrix.json` now has 15 `Simulated HA` rows, 8
  static rows, and 2 `Unverified` real-host rows.
- The Composed tier digest `sha256:a0372f02…` matches the lab digest.
- `qr-governance check --all`: pass.

## Learning

- The matrix's `tier` is the primary evidence class; `live_evidence` records the
  strongest executed row. A static alert rule can have a live counterpart
  without changing its tier.
- HA-22 and HA-23 cannot be made a pass on a single-host lab; the report keeps
  them as release residual risk instead of relabeling the lab.

## Residual risk

- No Real HA claim. HA-22 (power loss) has no accepted-risk record; HA-23 carries
  the AZ1 MinIO/ingress accepted risk but is still `Unverified`.
- The 8 static rows were not triggered live; their evidence is the rule test and
  `qr-deploy check`.
