# Four Logical Databases

- Date: 2026-11-22 (UTC)
- Status: Current for the local Compose stack, the v2 multi-host profile, and
  the v3 Patroni cluster
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
| `QR_DATABASE_URL_FILE` | the service's `<database>_app` | the long-running gateway, data, evidence, or simulation service |
| `QR_DATABASE_OWNER_URL_FILE` | the service's `<database>_owner` | only the one-shot `<service>-migrate` job |

Both variables name a mode-600 file mounted at `/run/secrets`; a plain
`QR_DATABASE_URL` or `QR_DATABASE_OWNER_URL` is a startup error. The contract is
[file-backed secrets](../2026-11-23/secret-files.md). Every URL carries
`sslmode=verify-full` and the service reads `QR_DATABASE_TLS_CA_PATH`; the
migration job uses the same CA path. `prefer`, `require` and `verify-ca` are
refused because they fall back to plaintext or skip hostname verification. The
full contract is [internal TLS](../2026-11-23/internal-tls.md).

The long-running service must have `QR_MIGRATE=0` and no owner URL. The runtime
refuses to start if either is present, and a database-owning service without its
application file fails closed. It opens only the application pool, so a
compromised service process never holds a DDL credential.

The migration job runs the same binary with `--migrate-only`. It opens the owner
URL, applies one ledger, logs the result, and exits before it creates an
application pool or binds a listener. Compose starts the jobs after
`db-bootstrap` and the long-running services after their own job; model,
strategy and backtest have no job and no database.

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

The local stack seeds every role from `QR_POSTGRES_PASSWORD`, which
`make secrets` writes to `secrets/compose.env`; it is one password for eight
roles because the local stack is a single-operator development deployment. The
v2 multi-host profile does the same. The v3 profile resolves per-role passwords:
`jobs/cluster-bootstrap.sh` creates the four databases and eight roles through
the `postgres`/HAProxy endpoint and the file-backed superuser password, reads
each owner/app password from its URL file, and runs before `db-bootstrap.sh` and
`migrate.sh`. The v3 contract is
[`patroni-cluster.md`](../2026-11-23/patroni-cluster.md). The v3 failover lab
(`tools/test_tiers.sh ha`) writes a marker through the primary, pauses the
Patroni primary with the watchdog off, verifies the marker and canonical digest
through the new primary, and records the rejoin; `--skip-database` leaves the row
`Unverified`. The v3 `archive_command`/`restore_command` use one pgBackRest
`quantrule` stanza, and the node-local `/srv/quantrule/wal-archive` volume is
removed; a live backup/restore/PITR drill is `Unverified`
([backup contract](../2026-11-24/backup-restore.md)). The service URLs are
rendered into mode-600 files, not env values. The forward-only migration
contract, the checksum pin, and the failed-job and rollback runbook are in
[`migrations.md`](../../architecture/migrations.md).

## 5. What changed locally

The `postgres` container is still one container and one pinned image. It now
holds four databases instead of one `quantrule` database with six service
schemas; the stateless model, strategy and backtest services own none.
The existing `quantrule` database is left in place as the maintenance database;
the service schemas are created in their new databases.

An existing local volume is upgraded in place: `make up` starts the one-shot
bootstrap, which creates the four databases, then the four migration jobs, then
the services. The old rows in `quantrule` are not copied. A local deployment
that must keep them needs the migration path in the release runbook, or
`make reset` for disposable data. The services do not read `quantrule`.

## 6. What changed and what remains open

| Contract | State | Consequence |
|---|---|---|
| `tools/backup.sh`, `tools/restore.sh` | Current for the local stack: one custom-format dump per database, `kind=database`, a partial-set refusal, and `--check` | A live restore drill still needs a disposable stack |
| `qr-deploy` descriptor | v2 remains readable; v3 renders per-service `_FILE` envs, per-role URL secrets, the cluster job, the Patroni/etcd/HAProxy tier, and the pgBackRest files | v3 per-role passwords are current; v2 keeps its one password |
| Existing `quantrule` rows | Not migrated | A disposable local stack is reset; a kept stack needs a data migration |
| Multi-host backup wrapper | Calls the Compose four-database tools; v3 uses one pgBackRest `quantrule` stanza | The v3 live backup/restore/PITR drill is `Unverified` |

These are recorded rather than silently widened in HA-002.

## 7. Verification

The evidence database adds `runs.outbox` and `runs.dataset_reference_releases`
in migration 18, and `lease_holder`, `lease_term`, and `lease_expires_at` in
migration 19. Migration 22 adds the same lease fence to `stage.runs` and
`stage.outbox`, so a queued or expired running stage is re-armed by evidence and
a terminal stage is never claimed. Each outbox commits with its row and is
delivered by the publisher; the release tombstone commits with a consumer delete
and is retried by the reconciler. None of these is read by another service. The
contract is
[`cross-database-consistency.md`](../../architecture/cross-database-consistency.md).

- `cargo test -p quantrule-runtime --test database_topology` checks the Compose
  `_FILE` variables, the migration jobs, the stateless services, and the
  bootstrap's databases, roles, schemas, and grants. It fails when a plain
  secret or an owner URL appears in a long-running service.
- `cargo test -p quantrule-deploy` checks the rendered service and migration env
  files, the mode-600 secret files, the role URL builder, the v3 cluster job, the
  Patroni/etcd/HAProxy configuration and check rows, the failover harness, the
  pgBackRest files, and the stateful manifest.
- `cargo test -p quantrule-deploy --test backup_tools` checks the four-database
  archive schema, the `--check` refusal of a partial set and a changed payload,
  and the Docker-free plans; `tools/test_backup.sh` is the opt-in live round trip.
- `cargo test -p quantrule-evidence-service --test outbox_conformance` checks
  the run outbox, the restart re-arm, and the release tombstone against the
  memory store, and the same body against Postgres when `QR_TEST_DATABASE_URL`
  names a disposable database; `--test stage_outbox` covers the stage lease,
  terminal exclusion, duplicate suppression, and the exact rebuild set.
- The simulation database adds migration 16: a terminal-decision constraint and
  trigger. `cargo test -p quantrule-simulation-service --test concurrency` runs
  the mark and decision race against the memory store, and the Postgres cases
  run in the same disposable-database tier.
- `tools/postgres_composed.sh` runs the bootstrap against a disposable
  PostgreSQL and checks own-database access, cross-database refusal, app DDL
  refusal, owner DDL, and app DML.
- `cargo test -p quantrule-data-service --test active_active` runs two data
  service instances over one catalog; the shared store bodies and the
  object guard are specified in
  [`data-active-active.md`](../../architecture/data-active-active.md).
- `docker compose config` validates the local topology; `qr-deploy render`
  writes the v2 env files and the four migration envs with their CA path.
- `cargo test -p quantrule-runtime` proves the `verify-full`/CA pairing and the
  migration job's CA path; `tools/pki.sh` creates the bundle.
