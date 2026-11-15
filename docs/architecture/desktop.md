# Desktop profile

- Status: Current; v1.1 work in progress (`DIST-200`, #692)
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
passed through when the parity harness sets it.

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
  owns one forward-only SQLite baseline starting at version 1 and records it
  under its own ledger name; a changed body, an unknown version, and a database
  ahead of the binary are refused. The baseline and its ledger row commit in one
  immediate transaction.
- `model` and `backtest` own no schema and add no ledger.

## 3. Service, ledger, and prefix

| Service | Ledger | Prefix | Tables | Baseline |
|---|---|---|---|---|
| `gateway` | `gateway` | `gateway` | `gateway__profiles` | `0001_profiles.sql` |
| `data` | `catalog` | `catalog` | `catalog__datasets`, `catalog__configs`, `catalog__config_bundles` | `0001_catalog.sql` |
| `evidence` | `runs` | `runs` | `runs__runs`, `runs__stages`, `runs__artifacts`, `runs__submissions` | `0001_runs_research.sql` |
| `evidence` | `runs` | `research` | `research__hypotheses` | `0001_runs_research.sql` |
| `simulation` | `simulation` | `simulation` | `simulation__runs`, `simulation__marks` | `0001_simulation.sql` |
| runtime queue | `queue` | `queue` | `queue__messages` | pending, #694 |

Evidence owns two prefixes and one ledger and one baseline, so its history stays
forward-only. `queue` is declared and pending: #694 lands its baseline, store,
and ledger row, and the guard below fails until the inventory is updated in the
same change.

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
| `python3 tools/check_schema_prefixes.py` | Each service owns exactly its baseline, ledger, tables, and store module; no baseline uses a Postgres spelling; no crate gains an undeclared baseline; no store names another prefix; each service branches on `QR_PROFILE=desktop` with `QR_DATA_DIR` and opens `quantrule.sqlite` |
| `python3 tools/test_check_schema_prefixes.py` | The guard still rejects a wrong prefix, a JSONB or timestamptz, `CREATE SCHEMA`, a missing ledger row, and a pending ledger that lands |

The inventory lives in the checker, not in a second list, so a migration, a table
rename, or a new ledger that is not declared fails the gate. The guard scans
service sources outside their `#[cfg(test)]` modules: a test may name another
prefix to prove it is refused, and the runtime's shared `Schema` mapping may name
all six.

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
- `FilesystemStore` is implemented and tested with atomic publish, hash on read,
  and traversal refusal. The service routes still build the S3 store, so the
  embedded object store is not yet the run-time backend.
- The durable in-process queue is #694. Until it lands, a desktop start still
  needs the control-plane queue.
- The installers and the supervisor are implemented; artifacts are unsigned.
- Parity is the release gate: `tools/parity_desktop.sh` (#695) runs the same
  fixture, configuration, and image digest on both profiles and compares the
  canonical digest, every artifact, the manifest set, and the run projection. The
  harness is not in the tree yet, so no desktop/server digest parity is claimed.

There is no data migration between profiles. A desktop start uses a fresh
`QR_DATA_DIR`; the server volume is never read by it.
