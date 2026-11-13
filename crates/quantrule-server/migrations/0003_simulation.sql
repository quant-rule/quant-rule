-- The research plane: hindsight by design, isolated from the pipeline.
CREATE SCHEMA IF NOT EXISTS simulation;

CREATE TABLE IF NOT EXISTS simulation.runs (
    sim_id     text PRIMARY KEY,
    premise    text NOT NULL,
    bundle     text NOT NULL,
    seed       bigint NOT NULL,
    criteria   text NOT NULL,
    status     text NOT NULL CHECK (status IN ('draft', 'marking', 'reviewed', 'checked', 'decided')),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS simulation.marks (
    mark_id          text PRIMARY KEY,
    sim_id           text NOT NULL REFERENCES simulation.runs (sim_id) ON DELETE CASCADE,
    chart            text NOT NULL,
    bar              bigint NOT NULL CHECK (bar >= 0),
    marked_time_utc  timestamptz NOT NULL,
    action           text NOT NULL CHECK (action IN ('entry', 'exit')),
    quality          text NOT NULL CHECK (quality IN ('good', 'bad', 'neutral')),
    side             text NOT NULL CHECK (side IN ('long', 'short')),
    origin           text NOT NULL CHECK (origin IN ('human', 'oracle')),
    note             text NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS simulation_marks_run_idx ON simulation.marks (sim_id, chart, bar);
