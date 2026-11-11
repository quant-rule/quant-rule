# Local Research Memory v1

- Date: 2026-11-11 (UTC)
- Status: Current
- Schema: `quantrule.research-memory/v1`
- Export schema: `quantrule.research-memory-export/v1`
- Code: `crates/quantrule-app/src/memory/`
- Related: [`docs/execution-plans/2026-11-11/608-local-research-memory.md`](../../execution-plans/2026-11-11/608-local-research-memory.md)

## Scope

The memory is a local SQLite database owned by the application service. It
stores the researcher's workspace, dataset, configuration, run, metric, claim,
and note records. It has no account, no network path, and no engine dependency.

## Rules

| Rule | Contract |
|---|---|
| Time | Every timestamp is a caller-supplied UTC Unix second. The memory never reads the clock. |
| Identity | IDs are `INTEGER PRIMARY KEY AUTOINCREMENT`; a deleted ID is never reused. |
| Null | A metric kind is `real`, `integer`, `text`, or `null`. A null is stored as null, never as zero. |
| Integer | An integer metric is stored as decimal text, so it is never rounded through a float. |
| Order | Every query is ordered by ID; export keys are sorted by the serializer. |
| Integrity | Foreign keys are enabled. A run's config and a note's run must belong to the same workspace. |
| Migration | `PRAGMA user_version` selects an additive migration. A newer version is refused. |

## Tables

| Table | Holds | Key rule |
|---|---|---|
| `meta` | The schema identity | One row, `schema`. |
| `workspaces` | Local roots and names | `root` is unique. |
| `datasets` | Dataset name, path, data version, instrument, candle extent | Unique per workspace and path. |
| `configs` | Run name, path, config hash, schema version | Unique per workspace and path. |
| `runs` | Run key, data version, config hash, status, timestamps, summary JSON | `run_key` is unique. |
| `metrics` | Typed metric values | Unique per run and name. |
| `claims` | Claim type, statement, status | One row per claim. |
| `notes` | Workspace and optional run notes | A note may be workspace-only. |

## Export and import

`export_json` produces one document with the export schema and the seven record
arrays. The same database state produces the same bytes. `import_json` accepts
that document only into an empty memory and runs the whole import in one
transaction; a rejected record leaves the target empty.

## Backup and restore

`backup` writes a consistent SQLite file with `VACUUM INTO`; the target must not
exist. `restore` copies a backup to a new path, opens it through the same schema
check, and verifies integrity. It removes the new file if the open or check
fails. The target must not exist.

## Verification

- `cargo test -p quantrule-app memory::` covers the schema, migrations, CRUD,
  null and integer values, foreign keys, ID reuse, export determinism,
  export/import, backup/restore, and unsupported schemas.
- `cargo test -p quantrule-app service::tests` covers recording a validated run
  through the application service.
