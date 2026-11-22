-- Object-key lookups for the active/active guards.
--
-- An object key is shared by profiles and is not unique: content addressing is
-- what identifies the bytes. The erase guard rechecks whether any dataset row
-- still names the object, and artifact collection rechecks sharing, so both
-- lookups are indexed.
CREATE INDEX IF NOT EXISTS datasets_object_key_idx
    ON catalog.datasets (object_key);

CREATE INDEX IF NOT EXISTS artifacts_object_key_idx
    ON catalog.artifacts (object_key);
