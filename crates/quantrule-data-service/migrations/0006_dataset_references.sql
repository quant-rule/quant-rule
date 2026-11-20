-- Runtime history for datasets.
--
-- A dataset is archived before it can be erased. A registered reference is a
-- consumer's promise that it still needs the bytes; the row is refused while
-- any reference remains. The retention event is written in the same transaction
-- as the state change or delete and survives the dataset row.
ALTER TABLE catalog.datasets ADD COLUMN archived_at timestamptz;

CREATE TABLE IF NOT EXISTS catalog.dataset_references (
    profile_id   text NOT NULL,
    data_version text NOT NULL,
    reference_id text NOT NULL,
    kind         text NOT NULL CHECK (kind IN ('run', 'stage', 'research', 'simulation', 'baseline')),
    created_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (profile_id, data_version, reference_id),
    FOREIGN KEY (profile_id, data_version)
        REFERENCES catalog.datasets (profile_id, data_version) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS dataset_references_version_idx
    ON catalog.dataset_references (profile_id, data_version);

CREATE TABLE IF NOT EXISTS catalog.retention_events (
    event_id   bigserial PRIMARY KEY,
    profile_id text NOT NULL,
    kind       text NOT NULL,
    record_id  text NOT NULL,
    action     text NOT NULL CHECK (action IN ('archive', 'erase')),
    reason     text NOT NULL,
    bytes      bigint NOT NULL CHECK (bytes >= 0),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS retention_events_record_idx
    ON catalog.retention_events (profile_id, kind, record_id, created_at);
