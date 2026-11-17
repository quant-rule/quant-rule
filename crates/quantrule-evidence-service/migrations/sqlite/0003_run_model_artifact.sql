-- A promoted evidence run may name the frozen model stage artifact it reused.
ALTER TABLE runs__runs ADD COLUMN model_artifact_id TEXT;
