-- Run notifications and dataset-reference release tombstones.
--
-- A run row and the notification it needs commit in one transaction. A crash
-- after the commit therefore leaves a durable notification, not a lost run, and
-- a retried submission finds the same content-addressed run. The publisher
-- delivers at least once; the worker and evidence treat a repeated message as a
-- lookup, so a duplicate delivery cannot create a second run record.
--
-- A release tombstone is written in the same transaction as the consumer delete
-- and survives it. The reconciler retries the data-service release until it is
-- confirmed, which makes the erase idempotent without a cross-database
-- transaction or join.
CREATE TABLE IF NOT EXISTS runs.outbox (
    event_id     bigserial PRIMARY KEY,
    run_id       text NOT NULL REFERENCES runs.runs (run_id) ON DELETE CASCADE,
    routing_key  text NOT NULL CHECK (routing_key IN ('run.requested', 'run.computed')),
    attempts     integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    last_error   text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    published_at timestamptz,
    UNIQUE (run_id, routing_key)
);

CREATE INDEX IF NOT EXISTS runs_outbox_pending_idx
    ON runs.outbox (event_id) WHERE published_at IS NULL;

CREATE TABLE IF NOT EXISTS runs.dataset_reference_releases (
    profile_id   text NOT NULL,
    data_version text NOT NULL,
    reference_id text NOT NULL,
    kind         text NOT NULL
        CHECK (kind IN ('run', 'stage', 'research', 'simulation', 'baseline')),
    created_at   timestamptz NOT NULL DEFAULT now(),
    released_at  timestamptz,
    attempts     integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    last_error   text,
    PRIMARY KEY (profile_id, data_version, reference_id)
);

CREATE INDEX IF NOT EXISTS runs_reference_releases_pending_idx
    ON runs.dataset_reference_releases (created_at, profile_id, data_version,
                                        reference_id)
    WHERE released_at IS NULL;
