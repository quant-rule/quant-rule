# AI Transparency Log — Cutover, Restore and Object-Copy Drill

- Date: 2026-11-25 (UTC)
- Work items: QA-004 [#1125](https://github.com/quant-rule/quant-rule/issues/1125), MIG-001 [#1132](https://github.com/quant-rule/quant-rule/issues/1132), MIG-002 [#1133](https://github.com/quant-rule/quant-rule/issues/1133), MIG-003 [#1134](https://github.com/quant-rule/quant-rule/issues/1134)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017

## Intent and boundary

Run one live cutover drill on the laptop Compose stack: back up the four
databases and the object bucket, restore into a fresh project, switch the
endpoint, verify the canonical digest, exercise a queued run through the empty
broker, verify every object hash, and roll back to the original stack. The v3
Swarm/PITR lab is a separate tier and is cited from `dr.json`, not re-run here.

## Work performed

- Captured the baseline: two succeeded runs, digest `sha256:a0372f02…`, 70
  objects/28,790,609 bytes, 38 tables/1,276 rows.
- Took the clean archive (`7615f537…`) and a second archive (`b70d19c7…`) that
  included a queued run submitted while `backtest` was stopped.
- Restored into a fresh `quantrule-cutover` project. The first attempt found
  the empty-schema conflict; the second found the owner/ACL loss. Fixed both in
  [#1177](https://github.com/quant-rule/quant-rule/pull/1177) and re-ran the
  restore with the fixed tool.
- Started the cutover stack; all four migration jobs exited 0 and all seven
  services became healthy. The queued run completed once.
- Verified the pre-existing run's digest and all 29 artifact bytes, the queued
  run's digest, all table owners and app grants, and a 92-object copy with a
  matching SHA-256 manifest and zero orphans.
- Rolled back to the original stack, verified the digest and the three runs,
  then removed the scratch project and volumes.

## Verification and evidence

- `docs/reports/2026-11-25/cutover-restore-drill.md` and its `cutover-restore/`
  files.
- `tools/restore.sh <archive> --check`: pass before the restore.
- 29/29 artifacts matched the run record; queued run `sha256:07728…` digest
  `sha256:a0372f02…`.
- Object copy: 92/92 SHA-256 matches, manifest `be7d9205…`, 45/45 artifact
  objects referenced, 0 orphans.
- Rollback: original stack healthy, digest unchanged.

## Learning

- A restore test that wipes volumes does not cover the fresh `db-bootstrap`
  target; the empty schemas and the owner/ACL loss only appear on a realistic
  fresh stack.
- A queued run is the right vehicle for the queue-rebuild claim: the startup
  reconciliation re-armed it from its row, so the empty broker lost nothing.
- A single drill can close the dependent QA-004 and MIG-001…003 leaves, but the
  report must keep the four claims separate and cite the same evidence.

## Residual risk

- The Compose drill is single-host; v3 PITR/Patroni remains `Simulated HA`.
- The endpoint switch used the same published port, not a DNS record swap.
- The restore requires the fixed `qr_*` roles from `db-bootstrap`.
- No release tag was created.
