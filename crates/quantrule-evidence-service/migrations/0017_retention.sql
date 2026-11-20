-- Runtime history for evidence-owned records.
--
-- A run submission is archived per profile; the run row itself is shared, so
-- archiving one profile's view never hides another's. A research record is
-- profile-scoped. The retention event is written in the same transaction as the
-- state change or delete and survives the record.
ALTER TABLE runs.submissions ADD COLUMN archived_at timestamptz;
ALTER TABLE research.improvement_runs ADD COLUMN archived_at timestamptz;

CREATE TABLE IF NOT EXISTS runs.retention_events (
    event_id   bigserial PRIMARY KEY,
    profile_id text NOT NULL,
    kind       text NOT NULL CHECK (kind IN ('run')),
    record_id  text NOT NULL,
    action     text NOT NULL CHECK (action IN ('archive', 'erase')),
    reason     text NOT NULL,
    bytes      bigint NOT NULL CHECK (bytes >= 0),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS runs_retention_events_record_idx
    ON runs.retention_events (profile_id, record_id, created_at);

CREATE TABLE IF NOT EXISTS research.retention_events (
    event_id   bigserial PRIMARY KEY,
    profile_id text NOT NULL,
    kind       text NOT NULL CHECK (kind IN ('research')),
    record_id  text NOT NULL,
    action     text NOT NULL CHECK (action IN ('archive', 'erase')),
    reason     text NOT NULL,
    bytes      bigint NOT NULL CHECK (bytes >= 0),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS research_retention_events_record_idx
    ON research.retention_events (profile_id, record_id, created_at);

CREATE TABLE IF NOT EXISTS stage.retention_events (
    event_id   bigserial PRIMARY KEY,
    profile_id text NOT NULL,
    kind       text NOT NULL CHECK (kind IN ('stage')),
    record_id  text NOT NULL,
    action     text NOT NULL CHECK (action IN ('archive', 'erase')),
    reason     text NOT NULL,
    bytes      bigint NOT NULL CHECK (bytes >= 0),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS stage_retention_events_record_idx
    ON stage.retention_events (profile_id, record_id, created_at);
