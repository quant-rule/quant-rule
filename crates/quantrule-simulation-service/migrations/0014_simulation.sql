-- The research plane: hindsight by design, isolated from the pipeline.
--
-- Simulation owns this schema. It has no run id, no canonical digest, no queue
-- message, and no strategy, execution, or portfolio column. `profile_id` scopes
-- every row organisationally; it is not a foreign key and not a security
-- boundary.
CREATE SCHEMA IF NOT EXISTS simulation;

CREATE TABLE IF NOT EXISTS simulation.runs (
    sim_id         text PRIMARY KEY,
    profile_id     text NOT NULL,
    premise        text NOT NULL,
    bundle         text NOT NULL,
    inputs         text NOT NULL,
    inputs_digest  text NOT NULL,
    seed           bigint NOT NULL CHECK (seed >= 0),
    criteria       text NOT NULL,
    window_spec    jsonb NOT NULL,
    oracle_spec    jsonb NOT NULL,
    check_config   jsonb NOT NULL,
    selection      jsonb,
    status         text NOT NULL CHECK (
        status IN ('draft', 'marking', 'reviewed', 'checked', 'decided')
    ),
    check_result   jsonb,
    decision       text CHECK (decision IN ('test', 'refine', 'stop')),
    decision_note  text,
    decided_at     timestamptz,
    created_at     timestamptz NOT NULL
);

CREATE INDEX IF NOT EXISTS simulation_runs_profile_idx
    ON simulation.runs (profile_id, created_at DESC, sim_id);

CREATE TABLE IF NOT EXISTS simulation.marks (
    mark_id          text PRIMARY KEY,
    sim_id           text NOT NULL REFERENCES simulation.runs (sim_id) ON DELETE CASCADE,
    profile_id       text NOT NULL,
    chart            bigint NOT NULL CHECK (chart >= 0),
    bar              bigint NOT NULL CHECK (bar >= 0),
    marked_time_utc  timestamptz NOT NULL,
    action           text NOT NULL CHECK (action IN ('entry', 'exit')),
    quality          text NOT NULL CHECK (quality IN ('good', 'bad', 'neutral')),
    side             text NOT NULL CHECK (side IN ('long', 'short')),
    origin           text NOT NULL CHECK (origin IN ('human', 'oracle')),
    note             text NOT NULL DEFAULT '',
    created_at       timestamptz NOT NULL,
    UNIQUE (sim_id, origin, chart, bar, action, side)
);

CREATE INDEX IF NOT EXISTS simulation_marks_run_idx
    ON simulation.marks (profile_id, sim_id, chart, bar, action, side);
