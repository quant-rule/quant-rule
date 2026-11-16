# Desktop profile

- Status: Current; released as v1.0.0 (prepared as v1.1.0; renumbered 2026-11-15); current in v1.2.0 (`DIST-200`, #692)
- Part of: [`README.md`](README.md)
- Decision: [`ADR-2026-11-12-12`](../adr/2026-11-12/12-desktop-distribution.md)
- Claim type: Behaviour Claim

The desktop profile runs the same six services and the same contracts on
embedded backends: SQLite, a filesystem content-addressed store, and a durable
in-process queue. It is a deployment shape, not a second architecture. The server
profile is the default and remains the oracle.

## 1. Environment contract

| Variable | Default | Meaning |
|---|---|---|
| `QR_PROFILE` | `server` | `server` or `desktop`; any other value is refused |
| `QR_DATA_DIR` | none | Required when the profile is desktop; a blank value is refused |
| `QR_BIND` | `0.0.0.0:8080` | Loopback port chosen by the supervisor in the package |
| `QR_STATIC_DIR` | none | Bundled client, set by the supervisor |
| `QR_CONFIG_ROOT` | `config` | Bundled example configs, set by the supervisor |

`QR_PROFILE` absent or `server` selects Postgres, S3, and RabbitMQ with the
existing defaults. A service branches once in its binary; no route, use case,
engine, or client reads the profile. An invalid value is a startup error, never a
silent fallback.

The packaged supervisor removes `QR_BEARER_TOKEN`, `QR_DATABASE_URL`,
`QR_QUEUE_URL`, and the object-store variables from a desktop child, so an
embedded start cannot borrow a server database or broker. `QR_IMAGE_DIGEST` is
passed through when the parity harness sets it; otherwise the supervisor
exports the SHA-256 of the six installed sidecars as the package identity.

## 2. One SQLite file

SQLite has no schemas, so a service's Postgres relation `schema.table` is the
table `schema__table`. The four database-owning services share one file, each
opening it from its own process:

```text
QR_DATA_DIR/
  quantrule.sqlite          the one database, WAL mode
  supervisor/ports.json     diagnostic only; no client reads it
```

- The file is `quantrule.sqlite`, from one runtime constant. A second name is a
  defect, because the ledger and the tables would live in two files.
- Each connection enables foreign keys, a five-second busy timeout, full
  synchronous commits, and WAL. A filesystem that refuses WAL is refused at
  startup rather than run in rollback mode.
- One shared `schema_migrations` table carries a `ledger` column. Each service
  owns one forward-only SQLite ledger starting at version 1 and records every
  baseline under its own ledger name; a changed body, an unknown version, and a
  database ahead of the binary are refused. The baseline and its ledger row
  commit in one immediate transaction. A new table family is an additive
  migration, not a rewrite of an applied baseline. The queue creates its two
  tables outside that ledger, which is why its adapter owns the DDL.
- `model` and `backtest` own no schema and add no ledger.

## 3. Service, ledger, and prefix

| Service | Ledger | Prefix | Tables | Baseline |
|---|---|---|---|---|
| `gateway` | `gateway` | `gateway` | `gateway__profiles` | `0001_profiles.sql` |
| `data` | `catalog` | `catalog` | `catalog__datasets`, `catalog__configs`, `catalog__config_bundles` | `0001_catalog.sql` |
| `evidence` | `runs` | `runs` | `runs__runs`, `runs__stages`, `runs__artifacts`, `runs__submissions` | `0001_runs_research.sql` |
| `evidence` | `runs` | `research` | `research__hypotheses` | `0001_runs_research.sql` |
| `evidence` | `runs` | `stage` | `stage__runs`, `stage__submissions`, `stage__artifacts` | `0002_stage.sql` |
| `simulation` | `simulation` | `simulation` | `simulation__runs`, `simulation__marks` | `0001_simulation.sql` |
| runtime queue | none | `queue` | `queue__dead_letter`, `queue__messages` | adapter-owned DDL |

Evidence owns three prefixes and one ledger, so its history stays forward-only.
The run and research tables share the version-1 baseline; the stage tables are an
additive version-2 migration, so an existing desktop database keeps its rows. The
runtime queue has no service ledger and no binary of its own:
`InProcessQueue` owns two idempotent `CREATE TABLE` statements in
`src/adapters/inprocess.rs`, because the queue is runtime infrastructure rather
than a service schema. The guard records that exception and still checks its
store and both tables.

## 4. Storage rules

| Postgres | SQLite spelling |
|---|---|
| schema | `prefix__` table name |
| `jsonb` | `TEXT` checked by `json_valid` |
| `timestamptz` | RFC 3339 UTC `TEXT`, seconds precision |
| `boolean` | `INTEGER` 0 or 1, checked |
| `FOR UPDATE` | `BEGIN IMMEDIATE`, then re-read the row inside the transaction |

A baseline never contains `CREATE SCHEMA`, `jsonb`, `timestamptz`, or
`FOR UPDATE`: SQLite would accept the words as names and fail to mean them. A
rule that must hold even for a writer that bypasses the repository is a trigger.
A JSON document is never a BLOB and a timestamp is never an epoch, because both
would change stored bytes and therefore parity.

## 5. Guard and inventory

| Command | Proves |
|---|---|
| `python3 tools/check_schema_prefixes.py` | Each service owns exactly its baseline, ledger, tables, and store module; the runtime queue owns its two tables and store; no baseline uses a Postgres spelling; no crate gains an undeclared baseline; no store names another prefix; each service branches on `QR_PROFILE=desktop` with `QR_DATA_DIR` and opens `quantrule.sqlite` |
| `python3 tools/test_check_schema_prefixes.py` | The guard still rejects a wrong prefix, a JSONB or timestamptz, `CREATE SCHEMA`, `FOR UPDATE`, a missing ledger row, and a store-owned table that drifts |

The inventory lives in the checker, not in a second list, so a migration, a table
rename, or a new ledger that is not declared fails the gate. The guard scans
service sources outside their `#[cfg(test)]` modules: a test may name another
prefix to prove it is refused, and the runtime's shared `Schema` mapping may name
all seven.

## 6. Start and stop

The packaged shell spawns one supervisor, `qr-desktop`, and holds its standard
input open. The supervisor allocates loopback ports only — the gateway from
8080, the four HTTP services from 18081, the worker none — starts the six
services with `QR_PROFILE=desktop` and an absolute `QR_DATA_DIR`, waits for
health, restarts a crashed child within a bounded budget, and stops every child
when its input closes. A supervisor crash closes the socket, so no orphan
outlives it. `--profile external` starts nothing and points the shell at a
gateway that is already running.

## 7. Current state and parity

- SQLite is current for `gateway`, `data`, `evidence`, and `simulation`; the
  server profile is unchanged and still the default.
- `FilesystemStore`, `InProcessQueue`, and the desktop selection are wired in
  every service that uses them. `quantrule-runtime::backends` selects the
  object store and the queue from `QR_PROFILE`; the data, model, evidence,
  simulation, and backtest binaries call it once. The queue is the one on the
  shared SQLite file, so evidence's publish is a row the backtest worker claims
  from another process.
- The installers and the supervisor are implemented; artifacts are unsigned.
- Parity is the release gate: `tools/parity_desktop.sh` runs the same fixture,
  configuration, and image digest on both profiles and compares the canonical
  digest, every artifact, the manifest set, and the run projection. The desktop
  fixture and the server comparison agreed on 2026-11-15; the harness is
  release-tier because the server oracle needs Docker or a running gateway.

There is no data migration between profiles. A desktop start uses a fresh
`QR_DATA_DIR`; the server volume is never read by it.
