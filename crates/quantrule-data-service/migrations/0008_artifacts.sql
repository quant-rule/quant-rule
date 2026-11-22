-- Computed research artifacts, consumer references, and retention.
--
-- A signal artifact's bytes live in the object store; this row is the
-- content-addressed identity and the profile's view. A registered reference is
-- a consumer's promise that it still needs the artifact, so the retention pass
-- can never collect a referenced row. The pass writes its tombstone to the
-- existing catalog.retention_events in the same transaction as the delete.
CREATE TABLE IF NOT EXISTS catalog.artifacts (
    profile_id    text NOT NULL,
    artifact_kind text NOT NULL CHECK (artifact_kind IN ('signal')),
    artifact_id   text NOT NULL,
    object_key    text NOT NULL,
    sha256        text NOT NULL,
    bytes         bigint NOT NULL CHECK (bytes >= 0),
    created_at    timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (profile_id, artifact_kind, artifact_id),
    CHECK (length(artifact_id) = 71),
    CHECK (sha256 = artifact_id)
);

CREATE INDEX IF NOT EXISTS artifacts_retention_idx
    ON catalog.artifacts (profile_id, artifact_kind, created_at);

CREATE TABLE IF NOT EXISTS catalog.artifact_references (
    profile_id    text NOT NULL,
    artifact_kind text NOT NULL,
    artifact_id   text NOT NULL,
    reference_id  text NOT NULL,
    kind          text NOT NULL
        CHECK (kind IN ('run', 'stage', 'research', 'simulation', 'baseline')),
    created_at    timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (profile_id, artifact_kind, artifact_id, reference_id),
    FOREIGN KEY (profile_id, artifact_kind, artifact_id)
        REFERENCES catalog.artifacts (profile_id, artifact_kind, artifact_id)
        ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS artifact_references_artifact_idx
    ON catalog.artifact_references (profile_id, artifact_kind, artifact_id);
