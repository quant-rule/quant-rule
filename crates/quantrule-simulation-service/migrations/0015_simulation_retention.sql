-- Runtime history for simulation runs.
--
-- A run is archived before it can be erased. Checked and decided runs refuse
-- erase; marks are removed with the run row. The retention event is written in
-- the same transaction as the archive or delete and survives the run.
ALTER TABLE simulation.runs ADD COLUMN archived_at timestamptz;

CREATE TABLE IF NOT EXISTS simulation.retention_events (
    event_id   bigserial PRIMARY KEY,
    profile_id text NOT NULL,
    kind       text NOT NULL CHECK (kind IN ('simulation')),
    record_id  text NOT NULL,
    action     text NOT NULL CHECK (action IN ('archive', 'erase')),
    reason     text NOT NULL,
    bytes      bigint NOT NULL CHECK (bytes >= 0),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS simulation_retention_events_record_idx
    ON simulation.retention_events (profile_id, record_id, created_at);
