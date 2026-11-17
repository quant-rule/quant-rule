-- A promoted evidence run may name the frozen model stage artifact it reused.
-- The full-run identity and every existing row keep a null value.
ALTER TABLE runs.runs ADD COLUMN model_artifact_id text;
