# STR-002 — Object DR Implementation Matches ADR-STR-001

- Date: 2026-11-26 (UTC)
- Status: **Verified** against [ADR-STR-001](../../adr/2026-11-26/1227-minio-object-dr.md)
  by fast tests and static review. No live archive or DR run in this work item.
- Work item: [#1228](https://github.com/quant-rule/quant-rule/issues/1228)
  (`REM-STR-002`, V-STR-02)
- Plan: [`rem-000-v1.6.1-remediation.md`](../../execution-plans/2026-11-26/rem-000-v1.6.1-remediation.md)
- Claim type: Behaviour Claim evidence record.
- Boundary: single-cluster object DR; no second site, no replication, no
  `Real HA`, no model, strategy, or edge claim.

## 1. Decision being verified

ADR-STR-001 keeps `backup.object_replication` false and uses the one-way
off-host archive plus `deploy/v3/dr.sh` as the object recovery path. It defers
MinIO bucket/site replication to a separate programme. This record checks that
the implementation and the contracts match that decision.

## 2. Implementation mapping

| Requirement | Implementation | Evidence |
|---|---|---|
| Replication is refused | `backup.object_replication` must be false; validation returns `deploy.backup.object_replication` and check rows say no second-site copy | `crates/quantrule-deploy/src/validate/backup.rs`; `src/check/backup.rs`; `src/check/minio.rs`; `object_replication_must_be_false_until_ha013_lands` |
| Descriptor and references stay false | Sample and both reference descriptors carry `object_replication: false` | `deploy/v3/deploy.example.json`; `deploy/v3/reference/single-az.json`; `deploy/v3/reference/three-az.json` |
| Archive is one-way | Source identity `qr_archive_reader` may list/get only; `mc mirror --overwrite --summary`; no `--remove`, no `--watch`, no `mc replicate`; target differs from the application bucket | `tools/object-archive.sh`; `crates/quantrule-deploy/tests/object_archive.rs` |
| Restore is drilled | `deploy/v3/dr.sh` restores a fresh scratch bucket before rows, reruns the same run/config, and fails on a digest mismatch | `deploy/v3/dr.sh`; `crates/quantrule-deploy/tests/dr_tool.rs` |
| Contract matches the decision | Backup and MinIO contracts state archive-only, no replication or second site; ADR links added | `docs/config/2026-11-24/backup-restore.md`; `docs/config/2026-11-23/minio-cluster.md`; `docs/architecture/ha.md` |
| Release boundary is recorded | The v1.6.1 note input carries the archive-only boundary; the released v1.6.0 note is not rewritten | `REM-000` REL-003 row; ADR-STR-001 |

## 3. Test evidence

Commands actually run in this worktree:

| Command | Observed |
|---|---|
| `cargo test -p quantrule-deploy` | 284 unit tests passed (4 ignored); 22 `cli`, 10 `chaos_harness`, 6 `dr_tool`, 30 `ha_harness`, 10 `minio_harness`, 7 `object_archive`, 4 `backup_tools`, and the remaining suites passed |
| `object_replication_must_be_false_until_ha013_lands` | Pass; a true value is refused and the valid descriptor stays false |
| `object_archive` suite | 7 passed; plan is Docker-free, live run needs `--confirm`, distinct target, no destructive or replication flag, restore mode |
| `dr_tool` suite | 6 passed; plan, confirmation gate, and scratch/in-place phases |
| `qr-governance check --all` | `governance check ok` |
| `tools/gate.sh --changed main` | `gate docs` (recorded in the pull request) |

The 2026-11-25 in-place lab DR is prior evidence and is `Measured` in its own
record; it was not rerun here.

## 4. Limitations and no-claim boundary

- No live archive was run; the archive target is operator-provided.
- The archive is periodic, not continuous. Its RPO is the archive interval, not
  zero; a key changed after the last run is not in the target.
- The archive does not delete, watch, or heal the target; a removed source key
  stays in the target.
- No second site, object lock, version-history claim, byte-identical remote
  replica, or `Real HA` is claimed.
- The fast suite does not prove a live S3 target, bandwidth, or restore time.

## 5. Verdict

**Pass.** The implemented object DR path matches ADR-STR-001: replication is
refused, the descriptor stays false, the archive is one-way, and the DR drill
restores objects before rows. No code change is required; V-STR-02 is `Pass`
with the live archive/DR boundary recorded.
