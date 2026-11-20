-- Runtime history for simulation runs, in the SQLite history.
--
-- A run is archived before it can be erased. Checked and decided runs refuse
-- erase; marks follow their run row. The retention event is the surviving audit
-- record.
ALTER TABLE simulation__runs ADD COLUMN archived_at TEXT;

CREATE TABLE IF NOT EXISTS simulation__retention_events (
    event_id   INTEGER PRIMARY KEY,
    profile_id TEXT NOT NULL,
    kind       TEXT NOT NULL CHECK (kind IN ('simulation')),
    record_id  TEXT NOT NULL,
    action     TEXT NOT NULL CHECK (action IN ('archive', 'erase')),
    reason     TEXT NOT NULL,
    bytes      INTEGER NOT NULL CHECK (bytes >= 0),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX IF NOT EXISTS simulation__retention_events_record_idx
    ON simulation__retention_events (profile_id, record_id, created_at);
