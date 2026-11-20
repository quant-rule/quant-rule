-- Runtime history for evidence-owned records, in the SQLite history.
--
-- A run submission is archived per profile; the run row is shared. A research
-- record is profile-scoped. The retention event is the surviving audit record.
ALTER TABLE runs__submissions ADD COLUMN archived_at TEXT;
ALTER TABLE research__improvement_runs ADD COLUMN archived_at TEXT;

CREATE TABLE IF NOT EXISTS runs__retention_events (
    event_id   INTEGER PRIMARY KEY,
    profile_id TEXT NOT NULL,
    kind       TEXT NOT NULL CHECK (kind IN ('run')),
    record_id  TEXT NOT NULL,
    action     TEXT NOT NULL CHECK (action IN ('archive', 'erase')),
    reason     TEXT NOT NULL,
    bytes      INTEGER NOT NULL CHECK (bytes >= 0),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX IF NOT EXISTS runs__retention_events_record_idx
    ON runs__retention_events (profile_id, record_id, created_at);

CREATE TABLE IF NOT EXISTS research__retention_events (
    event_id   INTEGER PRIMARY KEY,
    profile_id TEXT NOT NULL,
    kind       TEXT NOT NULL CHECK (kind IN ('research')),
    record_id  TEXT NOT NULL,
    action     TEXT NOT NULL CHECK (action IN ('archive', 'erase')),
    reason     TEXT NOT NULL,
    bytes      INTEGER NOT NULL CHECK (bytes >= 0),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX IF NOT EXISTS research__retention_events_record_idx
    ON research__retention_events (profile_id, record_id, created_at);

CREATE TABLE IF NOT EXISTS stage__retention_events (
    event_id   INTEGER PRIMARY KEY,
    profile_id TEXT NOT NULL,
    kind       TEXT NOT NULL CHECK (kind IN ('stage')),
    record_id  TEXT NOT NULL,
    action     TEXT NOT NULL CHECK (action IN ('archive', 'erase')),
    reason     TEXT NOT NULL,
    bytes      INTEGER NOT NULL CHECK (bytes >= 0),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX IF NOT EXISTS stage__retention_events_record_idx
    ON stage__retention_events (profile_id, record_id, created_at);
