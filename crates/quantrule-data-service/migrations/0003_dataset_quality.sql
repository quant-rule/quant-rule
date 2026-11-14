-- A tolerant ingest records what it skipped beside the accepted bytes.
ALTER TABLE catalog.datasets ADD COLUMN quality jsonb;
