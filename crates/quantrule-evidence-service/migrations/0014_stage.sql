-- Diagnostic stage runs, their profile bindings, and their artifact files.
--
-- A stage run is content-addressed and shared like a run, but it lives in its
-- own `stage` schema: no stage row is visible to the run store, no run row is
-- visible to the stage store, and `artifact_id` is a stage bundle digest, never
-- a canonical run digest. A succeeded row must name its artifact; a failed row
-- must carry an error. The kind/input check keeps a strategy stage from reading
-- a dataset or a portfolio stage from reading a model artifact.
CREATE SCHEMA IF NOT EXISTS stage;

CREATE TABLE IF NOT EXISTS stage.runs (
    stage_run_id  text PRIMARY KEY,
    kind          text NOT NULL CHECK (kind IN ('model', 'strategy', 'portfolio')),
    status        text NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
    owner         text NOT NULL,
    config_id     text NOT NULL,
    input_kind    text NOT NULL CHECK (input_kind IN ('data_version', 'model_artifact', 'decisions_artifact')),
    input_digest  text NOT NULL,
    data_version  text NOT NULL,
    config_hash   text NOT NULL,
    model_version text NOT NULL,
    image_digest  text NOT NULL,
    artifact_id   text,
    submitted_at  timestamptz NOT NULL DEFAULT now(),
    started_at    timestamptz,
    finished_at   timestamptz,
    error         text,
    attempts      integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    CHECK (status <> 'succeeded' OR artifact_id IS NOT NULL),
    CHECK (status <> 'failed' OR error IS NOT NULL),
    CHECK ((kind = 'model' AND input_kind = 'data_version') OR (kind = 'strategy' AND input_kind = 'model_artifact') OR (kind = 'portfolio' AND input_kind = 'decisions_artifact')),
    CHECK (kind <> 'model' OR data_version = input_digest)
);

CREATE TABLE IF NOT EXISTS stage.submissions (
    profile_id   text NOT NULL,
    stage_run_id text NOT NULL REFERENCES stage.runs (stage_run_id) ON DELETE CASCADE,
    submitted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (profile_id, stage_run_id)
);

CREATE TABLE IF NOT EXISTS stage.artifacts (
    stage_run_id text NOT NULL REFERENCES stage.runs (stage_run_id) ON DELETE CASCADE,
    name         text NOT NULL,
    bytes        bigint NOT NULL CHECK (bytes >= 0),
    sha256       text NOT NULL,
    PRIMARY KEY (stage_run_id, name)
);

CREATE INDEX IF NOT EXISTS stage_runs_status_submitted_idx
    ON stage.runs (status, submitted_at DESC);
CREATE INDEX IF NOT EXISTS stage_submissions_profile_idx
    ON stage.submissions (profile_id, submitted_at DESC);
CREATE INDEX IF NOT EXISTS stage_artifacts_sha_idx ON stage.artifacts (sha256);
