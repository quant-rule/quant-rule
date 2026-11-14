-- A run is content-addressed and shared. A profile's submission is its own row,
-- so two profiles can point at one run without duplicating the run record.
CREATE TABLE IF NOT EXISTS runs.submissions (
    profile_id   text NOT NULL,
    run_id       text NOT NULL REFERENCES runs.runs (run_id) ON DELETE CASCADE,
    submitted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (profile_id, run_id)
);

ALTER TABLE runs.runs ADD COLUMN config_hash text;
ALTER TABLE runs.runs ADD COLUMN model_version text;
ALTER TABLE runs.runs ADD COLUMN image_digest text;

INSERT INTO runs.submissions (profile_id, run_id, submitted_at)
SELECT 'default', run_id, submitted_at FROM runs.runs
ON CONFLICT DO NOTHING;
