# Schema Migrations

- Status: Current; accepted 2026-11-22 as HA-003
- Part of: [`README.md`](README.md)
- Issue: [#1101](https://github.com/quant-rule/quant-rule/issues/1101)
- Claim type: Behaviour Claim
- Related: [ADR-HA](../adr/2026-11-22/1082-ha-deployment-profile.md), [four databases](../config/2026-11-22/four-databases.md), [deployment](deployment.md)

Four logical databases hold one ledger each. A ledger moves forward only; a
rollback changes the image, never the schema. The rule is enforced by the
runner, the checksum manifest, and the fast tests, not by convention alone.

## 1. Contract

| Rule | Meaning |
|---|---|
| Forward-only | There is no down migration. A failed or superseded change is corrected by a new migration; an applied body is never edited. |
| One owner | Only the service's `<database>_owner` role applies its ledger. The long-running service holds `<database>_app` with DML only. |
| One ledger per database | `gateway`, `catalog`, `runs`, and `simulation` each record version, name, checksum, and apply time. A migration names only its own schema. |
| Checksum-pinned | [`deploy/postgres/migration-ledger.json`](../../deploy/postgres/migration-ledger.json) pins every version and SQL checksum. Editing an applied body fails the fast gate and is refused by the runner. |
| Strict order | Versions are unique and strictly increasing in the manifest, the source list, and the ledger. A duplicate or reorder is refused. |
| One transaction per migration | The SQL and its ledger row commit together. A failure rolls back that migration only; earlier versions stay applied. |
| Idempotent job | A re-run applies only pending versions. The session advisory lock serializes concurrent attempts on one ledger; a second migrator finds the work recorded. |
| Expand/contract | Additive change first, code switch second, destructive change in a later release. |
| No destructive switch | No `DROP`, rewrite, or narrowing statement in the same release as the code that stops reading the old shape. |
| No cross-database work | No migration reads or writes another service's database, and no cross-database transaction exists. |
| Transaction-safe SQL | A migration must run inside a transaction: no `CREATE INDEX CONCURRENTLY`, `VACUUM`, or other statement the runner's transaction cannot carry. |
| Rollback compatibility | The previous image must tolerate the expanded schema for the whole rollback window. Keep it loaded until the fixture and canonical digest pass. |

## 2. Ledgers and jobs

| Database | Service | Ledger | Job |
|---|---|---|---|
| `qr_gateway` | gateway | `gateway.schema_migrations` | `gateway-migrate` |
| `qr_catalog` | data | `catalog.schema_migrations` | `data-migrate` |
| `qr_evidence` | evidence | `runs.schema_migrations` | `evidence-migrate` |
| `qr_simulation` | simulation | `simulation.schema_migrations` | `simulation-migrate` |

Each job runs the service binary with `--migrate-only`, opens only
`QR_DATABASE_OWNER_URL`, applies one ledger, logs the result, and exits before
it creates an application pool or binds HTTP. Model and backtest own no database
and have no job. The Compose contract is in
[`four-databases.md`](../config/2026-11-22/four-databases.md) section 2.

## 3. Release sequence

1. **Expand.** Add the new column, table, or index as an additive migration. New
   columns are nullable or have a default. Backfill in bounded batches in a
   separate migration. Code in this release keeps reading the old shape.
2. **Switch.** Deploy code that stops reading the old shape. This release
   contains no destructive statement. The previous image still tolerates the
   expanded schema and remains the rollback target.
3. **Contract.** In a later release, after the switch release has passed its
   fixture and canonical-digest check and the previous image is no longer a
   rollback target, a migration may drop or narrow the old shape.

A destructive statement proposed in the switch release is rejected in review.
The rollback window ends at the contract migration; after it, rolling back the
image is not supported and recovery is a PITR restore.

### Job order

`db-bootstrap` completes first. Then each database's migration job completes,
and only then does the service that owns that database start. There is no
cross-database order because no job reads another database.

| Profile | Order |
|---|---|
| Local Compose | `db-bootstrap` → four jobs (one per database, so they run concurrently; a retried job on one ledger is serialized by the advisory lock) → each service waits for its own job |
| Multi-host | infrastructure `db-bootstrap` → core `data-migrate`, `evidence-migrate`, `simulation-migrate` and their services → compute → gateway `gateway-migrate` → gateway |

The local stack lists `gateway-migrate`, `data-migrate`, `evidence-migrate`, and
`simulation-migrate` in that order for readability; the dependency graph, not
the listing, is the contract.

## 4. Rollback compatibility

| Migration | Previous image | Action |
|---|---|---|
| New nullable column, new table, new index, wider type | Reads and writes still work | Roll back the image; leave the schema expanded |
| Rename or drop of a shape the previous image reads | Broken | Not allowed until the rollback window closes |
| Destructive contract migration | Broken | Closed window; restore PITR if the change must be undone |

The old image may not run its own migration job after a newer ledger is applied:
the runner refuses a database ahead of the binary. Rollback therefore starts the
previous long-running service without the job. The service opens the expanded
schema read/write with its application role; the job is a forward step, not a
reverse one.

## 5. Operator runbook

Keep the old gateway, old image, old rendered environment, and old volumes until
the new stack passes the fixture and the canonical digest. Do not remove or
overwrite them while a rollback is still possible.

### A migration failed

1. Stop the rollout. Do not switch DNS or the public ingress; the old stack keeps
   serving. A failed job is recorded by Compose as a failed one-shot service, and
   the dependent service does not start.
2. Read the job log and exit code:
   `docker compose ps -a` and `docker compose logs <service>-migrate`. On the
   multi-host profile, run the same commands against that role's Compose file.
3. Query the ledger with the owner or app role:
   `SELECT version, name, checksum, applied_at FROM <ledger>.schema_migrations ORDER BY version;`
   The failed version is absent; earlier versions are present.
4. Do not edit the failed SQL or the ledger row. Correct forward with a new
   migration version, or fix the environment and re-run the same job. The runner
   resumes at the first pending version.
5. If the expanded schema is still compatible with the previous image, roll
   back the image. If it is not, restore PITR and record the data-loss window.

### A migration job stopped

1. A job that is killed, OOMs, or exits non-zero leaves its service unstarted
   because the Compose dependency requires successful completion.
2. Inspect `docker compose ps -a` for the exit code and read the log. A job that
   is merely slow (for example, an index build) is left to finish; do not kill a
   transaction midway.
3. Re-run the job. It is idempotent and continues from the ledger. Never start
   the service by hand to bypass it, and never set `QR_MIGRATE=1`; the runtime
   refuses both.
4. If the job is blocked on the advisory lock, the holder is the other migration
   session. A dead holder releases the session lock; an idle live holder is found
   in `pg_locks`/`pg_stat_activity` and terminated by the operator. Never disable
   the lock.

### Roll back to the previous image digest

1. Keep the previous image (`docker image inspect`) on every host, the previous
   descriptor, and the previous rendered environment.
2. On the multi-host profile stop the gateway, then compute, then core; on the
   local stack stop the application services. Leave infrastructure and volumes
   running.
3. Restore the previous descriptor tag and digest (multi-host) or `QR_IMAGE`
   (local), then re-render. The schema is not reverted.
4. Start the previous services without the one-shot migration job (`docker
   compose ... up -d --no-deps <services>`; add `--no-build` on the local
   stack). The old binary refuses a database ahead, so its job is expected to
   fail and is not part of a rollback.
5. Run the fixture and compare the canonical digest with the recorded pre-upgrade
   digest. Only after both pass may the new image and stack be retired. If the
   digest differs, stop; do not overwrite the old evidence.

## 6. Verification

- `cargo test -p quantrule-runtime --test migration_contract` checks the four
  ledgers, the pinned checksums, strict order, and the `--migrate-only` entry
  point for every database-owning service.
- `cargo test -p quantrule-runtime --test database_topology` checks the Compose
  job split, the owner/app URLs, and the bootstrap dependency.
- `qr-governance check --all` and `tools/gate.sh` run both in the fast gate.
