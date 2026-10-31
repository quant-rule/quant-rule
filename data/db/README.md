# Databases

SQLite stores written by the `store` commands.

- Default: `data/db/quantrule.db`; override with `--db <path>`.
- Created by `store init`, which applies the schema migrations in order.
- A store is local state, never a source of truth: delete it and re-ingest to
  rebuild it from the source candles.
- Reopening is idempotent. A schema newer than the binary is rejected; an older
  one is migrated in place.
- Query commands open read-only and never create or migrate a database.
