-- Runs, their stages, and their artifacts. A run is written once and finalized
-- only after every artifact hash verifies.
CREATE SCHEMA IF NOT EXISTS runs;

CREATE TABLE IF NOT EXISTS runs.runs (
    run_id       text PRIMARY KEY,
    config_id    text NOT NULL,
    data_version text NOT NULL,
    owner        text NOT NULL,
    status       text NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
    submitted_at timestamptz NOT NULL DEFAULT now(),
    started_at   timestamptz,
    finished_at  timestamptz,
    digest       text,
    error        text,
    CHECK (status <> 'succeeded' OR digest IS NOT NULL)
);

CREATE TABLE IF NOT EXISTS runs.stages (
    run_id     text NOT NULL REFERENCES runs.runs (run_id) ON DELETE CASCADE,
    name       text NOT NULL CHECK (name IN ('data', 'model', 'backtest', 'evidence')),
    state      text NOT NULL CHECK (state IN ('pending', 'running', 'done', 'failed', 'skipped')),
    elapsed_ms bigint,
    PRIMARY KEY (run_id, name)
);

CREATE TABLE IF NOT EXISTS runs.artifacts (
    run_id text NOT NULL REFERENCES runs.runs (run_id) ON DELETE CASCADE,
    name   text NOT NULL,
    bytes  bigint NOT NULL CHECK (bytes >= 0),
    sha256 text NOT NULL,
    PRIMARY KEY (run_id, name)
);

CREATE INDEX IF NOT EXISTS runs_status_submitted_idx ON runs.runs (status, submitted_at DESC);
