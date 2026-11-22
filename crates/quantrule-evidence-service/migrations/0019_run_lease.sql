-- A per-run lease fences a stale worker and lets a rebuilt queue recover a run
-- whose worker died.
--
-- The lease uses database time. `lease_term` is monotonic per run; a claim
-- records a new term, a heartbeat renews the expiry without changing it, and a
-- terminal transition or cancellation clears the holder and expiry while the
-- term remains as the fence. A stale worker's `computed` or `failed` write is
-- refused when its term no longer matches, so it can neither publish a
-- succeeded run nor record a partial artifact set.
ALTER TABLE runs.runs ADD COLUMN lease_holder text;
ALTER TABLE runs.runs ADD COLUMN lease_term bigint NOT NULL DEFAULT 0;
ALTER TABLE runs.runs ADD COLUMN lease_expires_at timestamptz;

CREATE INDEX IF NOT EXISTS runs_lease_expiry_idx
    ON runs.runs (lease_expires_at)
    WHERE status = 'running';
