-- The evidence service's SQLite baseline for the embedded profile.
--
-- This is the final logical shape, not a translation of the Postgres history:
-- a desktop database starts at v1.1 and has no earlier rows to migrate. SQLite
-- has no schemas, so `runs.runs` is `runs__runs` and `research.hypotheses` is
-- `research__hypotheses`. Evidence owns both prefixes and one ledger, named
-- `runs`, under the runtime's shared `schema_migrations` table.
--
-- An instant is RFC 3339 UTC TEXT, the SQLite spelling of `timestamptz`; a
-- JSON document is TEXT guarded by `json_valid`, the SQLite spelling of
-- `jsonb`. A sealed hypothesis is immutable in both dialects: SQLite spells
-- the Postgres PL/pgSQL trigger as two `BEFORE` triggers, one per event because
-- SQLite allows a single event per trigger, and each aborts. Evidence still
-- cannot rewrite or erase what it recorded.
CREATE TABLE IF NOT EXISTS runs__runs (
    run_id        TEXT PRIMARY KEY,
    config_id     TEXT NOT NULL,
    data_version  TEXT NOT NULL,
    owner         TEXT NOT NULL,
    status        TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
    submitted_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    started_at    TEXT,
    finished_at   TEXT,
    digest        TEXT,
    error         TEXT,
    config_hash   TEXT,
    model_version TEXT,
    image_digest  TEXT,
    attempts      INTEGER NOT NULL DEFAULT 0,
    CHECK (status <> 'succeeded' OR digest IS NOT NULL)
) STRICT;

CREATE TABLE IF NOT EXISTS runs__stages (
    run_id     TEXT NOT NULL REFERENCES runs__runs (run_id) ON DELETE CASCADE,
    name       TEXT NOT NULL CHECK (name IN ('data', 'model', 'backtest', 'evidence')),
    state      TEXT NOT NULL CHECK (state IN ('pending', 'running', 'done', 'failed', 'skipped')),
    elapsed_ms INTEGER,
    PRIMARY KEY (run_id, name)
) STRICT;

CREATE TABLE IF NOT EXISTS runs__artifacts (
    run_id TEXT NOT NULL REFERENCES runs__runs (run_id) ON DELETE CASCADE,
    name   TEXT NOT NULL,
    bytes  INTEGER NOT NULL CHECK (bytes >= 0),
    sha256 TEXT NOT NULL,
    PRIMARY KEY (run_id, name)
) STRICT;

CREATE INDEX IF NOT EXISTS runs__status_submitted_idx
    ON runs__runs (status, submitted_at DESC);

CREATE TABLE IF NOT EXISTS runs__submissions (
    profile_id   TEXT NOT NULL,
    run_id       TEXT NOT NULL REFERENCES runs__runs (run_id) ON DELETE CASCADE,
    submitted_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (profile_id, run_id)
) STRICT;

CREATE TABLE IF NOT EXISTS research__hypotheses (
    fingerprint TEXT NOT NULL,
    premise     TEXT NOT NULL,
    rule        TEXT NOT NULL,
    dataset     TEXT NOT NULL,
    costs       TEXT NOT NULL CHECK (json_valid(costs)),
    split       TEXT NOT NULL CHECK (json_valid(split)),
    criteria    TEXT NOT NULL,
    sealed_at   TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    body        TEXT NOT NULL CHECK (json_valid(body)),
    profile_id  TEXT NOT NULL,
    PRIMARY KEY (profile_id, fingerprint)
) STRICT;

CREATE TRIGGER IF NOT EXISTS research__hypotheses_immutable_update
BEFORE UPDATE ON research__hypotheses
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'a sealed hypothesis is immutable');
END;

CREATE TRIGGER IF NOT EXISTS research__hypotheses_immutable_delete
BEFORE DELETE ON research__hypotheses
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'a sealed hypothesis is immutable');
END;
