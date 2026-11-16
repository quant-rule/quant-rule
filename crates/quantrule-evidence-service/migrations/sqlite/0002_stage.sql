-- The diagnostic stage tables for the embedded profile.
--
-- This is an additive migration, not a rewrite of the `runs` baseline: a
-- desktop database that already applied version 1 keeps its runs and research
-- rows and gains the `stage` tables here. SQLite has no schemas, so `stage.runs`
-- is `stage__runs`. An instant is RFC 3339 UTC TEXT; `STRICT` keeps the column
-- types that the Postgres table declares.
--
-- The checks mirror the Postgres table: a succeeded row names its artifact, a
-- failed row names its error, and the stage kind and its input kind agree.
CREATE TABLE IF NOT EXISTS stage__runs (
    stage_run_id  TEXT PRIMARY KEY,
    kind          TEXT NOT NULL CHECK (kind IN ('model', 'strategy', 'portfolio')),
    status        TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
    owner         TEXT NOT NULL,
    config_id     TEXT NOT NULL,
    input_kind    TEXT NOT NULL CHECK (input_kind IN ('data_version', 'model_artifact', 'decisions_artifact')),
    input_digest  TEXT NOT NULL,
    data_version  TEXT NOT NULL,
    config_hash   TEXT NOT NULL,
    model_version TEXT NOT NULL,
    image_digest  TEXT NOT NULL,
    artifact_id   TEXT,
    submitted_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    started_at    TEXT,
    finished_at   TEXT,
    error         TEXT,
    attempts      INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    CHECK (status <> 'succeeded' OR artifact_id IS NOT NULL),
    CHECK (status <> 'failed' OR error IS NOT NULL),
    CHECK ((kind = 'model' AND input_kind = 'data_version') OR (kind = 'strategy' AND input_kind = 'model_artifact') OR (kind = 'portfolio' AND input_kind = 'decisions_artifact')),
    CHECK (kind <> 'model' OR data_version = input_digest)
) STRICT;

CREATE TABLE IF NOT EXISTS stage__submissions (
    profile_id   TEXT NOT NULL,
    stage_run_id TEXT NOT NULL REFERENCES stage__runs (stage_run_id) ON DELETE CASCADE,
    submitted_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (profile_id, stage_run_id)
) STRICT;

CREATE TABLE IF NOT EXISTS stage__artifacts (
    stage_run_id TEXT NOT NULL REFERENCES stage__runs (stage_run_id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    bytes        INTEGER NOT NULL CHECK (bytes >= 0),
    sha256       TEXT NOT NULL,
    PRIMARY KEY (stage_run_id, name)
) STRICT;

CREATE INDEX IF NOT EXISTS stage__status_submitted_idx
    ON stage__runs (status, submitted_at DESC);
CREATE INDEX IF NOT EXISTS stage__submissions_profile_idx
    ON stage__submissions (profile_id, submitted_at DESC);
CREATE INDEX IF NOT EXISTS stage__artifacts_sha_idx ON stage__artifacts (sha256);
