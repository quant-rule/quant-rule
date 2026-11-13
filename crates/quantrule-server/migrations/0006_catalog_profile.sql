-- A profile scopes the catalog row; the object it points at is shared.
ALTER TABLE catalog.datasets ADD COLUMN profile_id text NOT NULL DEFAULT 'default';
ALTER TABLE catalog.datasets DROP CONSTRAINT datasets_pkey;
ALTER TABLE catalog.datasets ADD PRIMARY KEY (profile_id, data_version);
ALTER TABLE catalog.datasets ALTER COLUMN profile_id DROP DEFAULT;
