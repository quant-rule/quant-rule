-- A config row is scoped by profile; the file bundle it points at is shared by
-- content id, so identical files are stored once.
CREATE TABLE IF NOT EXISTS catalog.config_bundles (
    config_id text PRIMARY KEY,
    files     jsonb NOT NULL
);

INSERT INTO catalog.config_bundles (config_id, files)
SELECT config_id, files FROM catalog.configs
ON CONFLICT (config_id) DO NOTHING;

ALTER TABLE catalog.configs ADD COLUMN profile_id text NOT NULL DEFAULT 'default';
ALTER TABLE catalog.configs DROP CONSTRAINT configs_pkey;
ALTER TABLE catalog.configs ADD PRIMARY KEY (profile_id, config_id);
ALTER TABLE catalog.configs DROP COLUMN files;
ALTER TABLE catalog.configs ALTER COLUMN profile_id DROP DEFAULT;
