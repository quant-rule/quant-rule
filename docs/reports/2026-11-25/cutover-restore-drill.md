# Cutover, Restore and Object-Copy Drill — Live Laptop Run

- Date: 2026-11-25 (UTC)
- Status: Current; one live Compose drill on the operator's laptop
- Issues: [#1125](https://github.com/quant-rule/quant-rule/issues/1125) (QA-004), [#1132](https://github.com/quant-rule/quant-rule/issues/1132) (MIG-001), [#1133](https://github.com/quant-rule/quant-rule/issues/1133) (MIG-002), [#1134](https://github.com/quant-rule/quant-rule/issues/1134) (MIG-003)
- Claim type: Behaviour Claim
- Related: [v1.6.0 plan](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md), [HA-014 lab](../2026-11-25/ha-014/lab.md), [backup contract](../../config/2026-11-24/backup-restore.md), [Compose tier](composed-tier.md)
- Fix found by this drill: [#1177](https://github.com/quant-rule/quant-rule/pull/1177)

## 1. Environment

| Part | Value |
|---|---|
| Host | The operator's laptop; Docker 29.10.0 |
| Source stack | `quantrule`, seven services and four databases on one Compose host |
| Target stack | `quantrule-cutover`, same image and secrets, fresh volumes |
| Image | `sha256:6750564c4080cb0a1115e5af28f7e5afd35bb40c79221fddac268e6995f71645` |
| Gateway | `http://localhost:8080` |
| Archive directory | `/tmp/qr-mig-drill/backup` |

## 2. Baseline

| Item | Value |
|---|---|
| Succeeded runs | 2; `sha256:d122…`, `sha256:dbb8…` |
| Canonical digest | `sha256:a0372f02db7bc65460906eba3242943da8c56f26c46fde470726cee856bc4cea` |
| Artifacts | 29 per run; 13 canonical; 3,560,384 canonical bytes per run |
| Objects | 70 objects, 28,790,609 bytes |
| Tables | 38 across the four databases; 1,276 rows |
| Migration ledgers | `gateway` 1, `catalog` 9, `evidence` 15, `simulation` 3 rows |

## 3. Backup and the queued run

The first archive is the clean two-run baseline:

```text
archive: /tmp/qr-mig-drill/backup/quantrule-2026-11-25T18-45-12Z.tar.gz
sha256: 7615f537ff8a0712306a80a0ff1a19a67dfa271b8460c0a4f583c1be0dac6588
databases: 4 dumps
objects: 70 files, 28790609 bytes
```

The queued-run test stopped only `backtest`, submitted a new config
`cfg_4cf606a3fb52` and run `sha256:07728c6d…`, and observed `status: queued`
and outbox event 5. The second archive then captured the queued run and its
outbox row:

```text
archive: /tmp/qr-mig-drill/backup/quantrule-2026-11-25T18-46-32Z.tar.gz
sha256: b70d19c78016639067501ab10855209537a952fcaf93508c2cc6902d609ed8ee
databases: 4 dumps
objects: 70 files, 28790609 bytes
```

`tools/restore.sh <archive> --check` verified the sidecar, entry list, manifest,
four-database set, and every payload hash without touching a stack.

## 4. Restore and cutover

The first restore into a freshly bootstrapped `quantrule-cutover` failed twice,
and both failures are real product defects:

| Failure | Observed | Fix |
|---|---|---|
| Empty schemas from `db-bootstrap` | `schema "gateway" already exists` | `pg_restore --clean --if-exists` |
| Owner and ACLs stripped | `permission denied for table schema_migrations` when the migration job started | keep owner/ACL in `pg_dump` and `pg_restore` |

[#1177](https://github.com/quant-rule/quant-rule/pull/1177) fixes both and adds
the fresh-bootstrap target to `tools/test_backup.sh`. After the fix, the same
archive restored with:

```text
restoring 70 objects (28790609 bytes)
restoring the four databases
databases: 4 dumps restored
objects: 70 files, 28790609 bytes restored
```

The cutover stack then started with every migration job exit 0 and all seven
services healthy. The queued run was re-armed from its row and completed after
startup.

## 5. Verification

| Check | Observed |
|---|---|
| Pre-existing run digest before/after | `sha256:a0372f02…` both; [before](cutover-restore/run-digest-before.json), [after](cutover-restore/run-detail-after.json) |
| Artifact bytes | 29/29 fetched through the gateway and matched the recorded SHA-256; 8,437,745 bytes |
| Queued run | `sha256:07728c6d…` succeeded after the restore with digest `sha256:a0372f02…`; [digest](cutover-restore/queued-run-digest.json) |
| Four-database rows | 1,276 → 1,314; the delta is the queued config (+1 config, +1 bundle), its dataset reference (+1), and its run (+1 run, +4 stages, +1 submission, +29 artifacts) |
| Table ownership | all 38 tables owned by the correct `qr_*_owner`; no table remained owned by the restore user |
| App grants | `qr_*_app` has SELECT/INSERT/UPDATE/DELETE on representative tables in all four databases; the four migration jobs exit 0 |
| Object copy | 92/92 objects copied to a scratch bucket; key/size/etag match; 92/92 SHA-256 match; both manifests `be7d9205…` |
| Orphans | 45 artifact objects, 45 referenced artifact hashes, 0 orphan artifact objects |
| Rollback | original `quantrule` stack healthy on the same port; digest `sha256:a0372f02…`; 3 succeeded runs; scratch project and volumes removed |

The object copy covered the `datasets/`, `model/`, `runs/`, `artifacts/`, and
`simulation/bundles/` prefixes. This profile has no packs or signals, so those
prefixes are absent rather than skipped.

## 6. What this proves

- A coordinated four-database and object backup restores into a fresh stack and
  reproduces the canonical digest; every artifact byte was fetched and checked.
- A queued run whose consumer was down survives the backup, the empty broker,
  and the restore; the startup reconciliation rebuilds the queue from the run
  rows and the run completes once.
- A failed cutover returns to the previous stack without data loss; the old
  volumes remain authoritative and the digest is unchanged after rollback.
- Object identity is preserved by copy: every object has the same key, size,
  etag, and SHA-256 in the scratch copy, and the artifact set has no orphan.
- The four migration ledgers survive the restore and the owner role can apply
  the next ledger.

## 7. Boundary

- This is the single-host Compose profile, not the v3 Swarm/Patroni/PITR lab.
  The lab PITR evidence remains [`dr.json`](../2026-11-25/ha-014/dr.json):
  RPO 2 s, restore 7 s, startup 8 s, total 219 s, digest match.
- The switch used the same published endpoint; a DNS record/TTL swap is
  operator tooling and was not exercised here.
- The restore requires the fixed `qr_*` roles from `db-bootstrap`; a cluster
  with different role names is not a supported target.
- No release tag was created.

## 8. Evidence

| File | Content |
|---|---|
| [`cutover-restore/baseline-runs.json`](cutover-restore/baseline-runs.json) | Two pre-drill run summaries |
| [`cutover-restore/run-detail-before.json`](cutover-restore/run-detail-before.json), [`run-detail-after.json`](cutover-restore/run-detail-after.json) | Run detail and artifact hashes before/after |
| [`cutover-restore/counts-before.tsv`](cutover-restore/counts-before.tsv), [`counts-after.tsv`](cutover-restore/counts-after.tsv) | 38-table row counts |
| [`cutover-restore/objects-before.jsonl`](cutover-restore/objects-before.jsonl), [`objects-current.jsonl`](cutover-restore/objects-current.jsonl) | 70-object baseline and 92-object post-run listing |
| [`cutover-restore/ownership.txt`](cutover-restore/ownership.txt) | Owner and app-grant checks |
| [`cutover-restore/artifact-verification.tsv`](cutover-restore/artifact-verification.tsv) | 29/29 artifact verification summary |
| [`cutover-restore/object-copy-hashes.log`](cutover-restore/object-copy-hashes.log), [`object-copy-summary.tsv`](cutover-restore/object-copy-summary.tsv) | Object copy hash and orphan summary |
