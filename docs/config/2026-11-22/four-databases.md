# Four Logical Databases

- Date: 2026-11-22 (UTC)
- Status: Current for the local Compose stack and the v2 multi-host profile
- Issue: [#1100](https://github.com/quant-rule/quant-rule/issues/1100)
- Claim type: Behaviour Claim
- Related: [HA architecture](../../architecture/ha.md), [deployment](../../architecture/deployment.md)

One PostgreSQL cluster holds four logical databases. Each database has one
service owner, one owner role, and one application role. The application role
has DML only; the owner role applies migrations and owns every object. No role
is granted connect on another service's database.

## 1. Ownership

| Database | Service | Schemas | Migration ledger |
|---|---|---|---|
| `qr_gateway` | gateway | `gateway` | `gateway.schema_migrations` |
| `qr_catalog` | data | `catalog` | `catalog.schema_migrations` |
| `qr_evidence` | evidence | `runs`, `research`, `stage` | `runs.schema_migrations` |
| `qr_simulation` | simulation | `simulation` | `simulation.schema_migrations` |

Model, strategy, and backtest have no database. They read catalog references
through the data service, artifacts through the object store, and run records
through evidence.

The role names are fixed:

| Database | Owner role | Application role |
|---|---|---|
| `qr_gateway` | `qr_gateway_owner` | `qr_gateway_app` |
| `qr_catalog` | `qr_catalog_owner` | `qr_catalog_app` |
| `qr_evidence` | `qr_evidence_owner` | `qr_evidence_app` |
| `qr_simulation` | `qr_simulation_owner` | `qr_simulation_app` |

## 2. Connection contract

| Variable | Role | Process |
|---|---|---|
| `QR_DATABASE_URL` | the service's `<database>_app` | the long-running gateway, data, evidence, or simulation service |
| `QR_DATABASE_OWNER_URL` | the service's `<database>_owner` | only the one-shot `<service>-migrate` job |

The long-running service must have `QR_MIGRATE=0` and no owner URL. The runtime
refuses to start if either is present. It opens only the application pool, so a
compromised service process never holds a DDL credential.

The migration job runs the same binary with `--migrate-only`. It opens the owner
URL, applies one ledger, logs the result, and exits before it creates an
application pool or binds a listener. Compose starts the jobs after
`db-bootstrap` and the long-running services after their own job; model and
backtest have no job and no database.

## 3. Grants

The bootstrap grants each application role:

- `CONNECT` on its own database only;
- `USAGE` on its owner schemas;
- `SELECT`, `INSERT`, `UPDATE`, `DELETE` on all tables;
- `USAGE`, `SELECT` on sequences (required by `bigserial` inserts);
- `EXECUTE` on functions (required by the evidence immutability trigger).

Default privileges apply the same table, sequence, and function grants to
objects the owner creates after the bootstrap. The bootstrap also revokes
`PUBLIC` connect and `TEMP` on each database and `PUBLIC` use of each owner
schema. No role is a member of another role.

## 4. Bootstrap and migration jobs

[`deploy/postgres/bootstrap.sh`](../../../deploy/postgres/bootstrap.sh) is the
idempotent bootstrap. It creates the roles and databases, converges the
database and schema owners, applies the grants, and rotates each role password
to the current secret. A second run on the same cluster succeeds.

The local Compose stack runs it as the one-shot `db-bootstrap` service after
Postgres is healthy. It then runs `gateway-migrate`, `data-migrate`,
`evidence-migrate`, and `simulation-migrate`; each waits for the bootstrap and
exits before its long-running service starts. The multi-host infrastructure
role runs the bootstrap, the core and gateway roles run the same one-shot
migration jobs, and the long-running services wait for the job in their own
Compose file. The script is also safe to run by hand against an existing
cluster:

```bash
docker compose run --rm db-bootstrap
```

The local stack seeds every role from `QR_POSTGRES_PASSWORD`; it is one password
for eight roles because the local stack is a single-operator development
deployment. The v2 multi-host profile does the same, so per-role passwords
remain a v3 (`quantrule.deploy/v3`) concern. The forward-only migration
contract, the checksum pin, and the failed-job and rollback runbook are in
[`migrations.md`](../../architecture/migrations.md).

## 5. What changed locally

The `postgres` container is still one container and one pinned image. It now
holds four databases instead of one `quantrule` database with six schemas.
The existing `quantrule` database is left in place as the maintenance database;
the service schemas are created in their new databases.

An existing local volume is upgraded in place: `make up` starts the one-shot
bootstrap, which creates the four databases, then the four migration jobs, then
the services. The old rows in `quantrule` are not copied. A local deployment
that must keep them needs the migration path in the release runbook, or
`make reset` for disposable data. The services do not read `quantrule`.

## 6. What remains stale

| Contract | State | Consequence |
|---|---|---|
| `tools/backup.sh`, `tools/restore.sh` | Dump and restore one `quantrule` database | They do not yet archive the four databases; a restore drill must be re-run after the backup change |
| `qr-deploy` descriptor | v2 remains readable and renders per-service URLs plus migration envs | Per-role password files need v3 (HA-012) |
| Existing `quantrule` rows | Not migrated | A disposable local stack is reset; a kept stack needs a data migration |
| Multi-host backup wrapper | Calls the single-database tools | Same limitation |

These are recorded rather than silently widened in HA-002.

## 7. Verification

The evidence database adds `runs.outbox` and `runs.dataset_reference_releases`
in migration 18, and `lease_holder`, `lease_term`, and `lease_expires_at` in
migration 19. The outbox commits with a run write and is delivered by the
publisher; the release tombstone commits with a consumer delete and is retried
by the reconciler; the lease fences a stale worker. None of these is read by
another service. The contract is
[`cross-database-consistency.md`](../../architecture/cross-database-consistency.md).

- `cargo test -p quantrule-runtime --test database_topology` checks the Compose
  URLs, the migration jobs, the stateless services, and the bootstrap's
  databases, roles, schemas, and grants. It fails when an owner URL appears in a
  long-running service.
- `cargo test -p quantrule-deploy` checks the rendered service and migration env
  files and the role URL builder.
- `cargo test -p quantrule-evidence-service --test outbox_conformance` checks
  the notification outbox, the restart re-arm, and the release tombstone against
  the memory store, and the same body against Postgres when
  `QR_TEST_DATABASE_URL` names a disposable database.
- `tools/postgres_composed.sh` runs the bootstrap against a disposable
  PostgreSQL and checks own-database access, cross-database refusal, app DDL
  refusal, owner DDL, and app DML.
- `docker compose config` validates the local topology; `qr-deploy render`
  writes the v2 env files and the four migration envs.
