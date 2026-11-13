-- A config is reproducible only with every file it references, not just the
-- run file. Existing rows start empty and are refreshed by the next scan.
ALTER TABLE catalog.configs ADD COLUMN files jsonb NOT NULL DEFAULT '{}'::jsonb;
