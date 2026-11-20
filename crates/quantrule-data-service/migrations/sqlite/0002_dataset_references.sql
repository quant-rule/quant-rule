-- Runtime history for datasets, in the SQLite baseline history.
--
-- The dataset row carries `archived_at`; a registered reference blocks erase;
-- the retention event is the surviving audit record. SQLite has no schema, so
-- `catalog.dataset_references` is `catalog__dataset_references`.
ALTER TABLE catalog__datasets ADD COLUMN archived_at TEXT;

CREATE TABLE IF NOT EXISTS catalog__dataset_references (
    profile_id   TEXT NOT NULL,
    data_version TEXT NOT NULL,
    reference_id TEXT NOT NULL,
    kind         TEXT NOT NULL CHECK (kind IN ('run', 'stage', 'research', 'simulation', 'baseline')),
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (profile_id, data_version, reference_id),
    FOREIGN KEY (profile_id, data_version)
        REFERENCES catalog__datasets (profile_id, data_version) ON DELETE RESTRICT
) STRICT;

CREATE INDEX IF NOT EXISTS catalog__dataset_references_version_idx
    ON catalog__dataset_references (profile_id, data_version);

CREATE TABLE IF NOT EXISTS catalog__retention_events (
    event_id   INTEGER PRIMARY KEY,
    profile_id TEXT NOT NULL,
    kind       TEXT NOT NULL,
    record_id  TEXT NOT NULL,
    action     TEXT NOT NULL CHECK (action IN ('archive', 'erase')),
    reason     TEXT NOT NULL,
    bytes      INTEGER NOT NULL CHECK (bytes >= 0),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
) STRICT;

CREATE INDEX IF NOT EXISTS catalog__retention_events_record_idx
    ON catalog__retention_events (profile_id, kind, record_id, created_at);
