# AI Transparency Log — Restore Ownership and Fresh-Bootstrap Target

- Date: 2026-11-25 (UTC)
- Work item: QA-004 [#1125](https://github.com/quant-rule/quant-rule/issues/1125); found during the MIG-001 cutover drill
- Tool: OpenCode, DeepSeek V4.1 Flash 1017

## Intent and boundary

Fix the Compose restore path found by a live cutover drill: a restored database
kept the migration ledger owned by the restore superuser and dropped the app
role grants, and a fresh `db-bootstrap` target left empty schemas that
`pg_restore` refused to replace. The boundary is the Compose backup/restore
pair; the v3 pgBackRest/PITR path is unchanged.

## Work performed

- Ran a live backup of the Compose stack with a queued run, restored it into a
  fresh `quantrule-cutover` project, and observed two failures: `schema
  "gateway" already exists` on a fresh bootstrap, and `permission denied for
  table schema_migrations` when the migration job started after the restore.
- Removed `--no-owner --no-privileges` from `pg_dump` and `pg_restore`, and
  added `--clean --if-exists` to `pg_restore`. The dumps now carry the fixed
  owner role and ACLs, and an empty schema from `db-bootstrap` is replaced.
- Extended `tools/test_backup.sh` to bootstrap a fresh four-database target
  before the restore, so the empty-schema path is covered. Replaced the stale
  `catalog.profiles` table with the current stable table list.
- Updated the backup/restore contract section 10.

## Verification and evidence

- `tools/test_backup.sh` with the local `secrets/compose.env`: passed;
  `backup round trip ok: 14 rows, 6 objects, sha256:1b88…`.
- The test's fresh target ran `db-bootstrap`, restored with `--force`, and the
  four migration jobs exited 0; all seven services became healthy.
- The cutover drill itself is recorded separately for QA-004/MIG-001.

## Learning

- A restore test that wipes the volumes does not cover a freshly bootstrapped
  stack; `db-bootstrap` creates schemas before rows, and the restore guard
  counted tables, not schemas.
- `pg_dump --no-owner --no-privileges` makes the restored objects belong to the
  restore user and removes the app grants. The fixed `qr_*` role names are the
  product contract, so keeping ownership and ACLs is the correct path.

## Residual risk

- The restore contract requires the target to carry the fixed roles from
  `db-bootstrap`; restoring into a cluster with different role names is not a
  supported path. The v3 DR path remains the operator drill.
