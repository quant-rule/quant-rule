-- A stage lease and its durable request intent.
--
-- The run lease fences a stale worker; a stage row needs the same fence so a
-- rebuilt queue can reclaim a `running` stage whose worker died without
-- stealing a live attempt. The lease uses database time. `lease_term` is
-- monotonic per stage: a claim records a new term, a heartbeat renews the
-- expiry without changing it, and a terminal transition clears the holder and
-- expiry while the term remains as the fence.
--
-- The outbox is the transaction boundary between the stage row and the queue.
-- A stage submission and its `stage.requested` intent commit together, so a
-- crash after the commit leaves a durable notification rather than a queued
-- stage nobody sees. A terminal transition removes the pending request; the
-- stage row is authoritative.
ALTER TABLE stage.runs ADD COLUMN lease_holder text;
ALTER TABLE stage.runs ADD COLUMN lease_term bigint NOT NULL DEFAULT 0;
ALTER TABLE stage.runs ADD COLUMN lease_expires_at timestamptz;

CREATE INDEX IF NOT EXISTS stage_lease_expiry_idx
    ON stage.runs (lease_expires_at)
    WHERE status = 'running';

CREATE TABLE IF NOT EXISTS stage.outbox (
    event_id     bigserial PRIMARY KEY,
    stage_run_id text NOT NULL REFERENCES stage.runs (stage_run_id) ON DELETE CASCADE,
    routing_key  text NOT NULL CHECK (routing_key IN ('stage.requested')),
    attempts     integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    last_error   text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    published_at timestamptz,
    UNIQUE (stage_run_id, routing_key)
);

CREATE INDEX IF NOT EXISTS stage_outbox_pending_idx
    ON stage.outbox (event_id) WHERE published_at IS NULL;

-- A running row written before this migration may still have a live worker on
-- the old binary. Give it a grace lease instead of reclaiming it immediately:
-- the row is never reset while a live attempt may exist. Once the grace window
-- expires, the periodic reconcile inserts its request intent and a new worker
-- may reclaim it. A queued row has no worker, so its intent is written now.
UPDATE stage.runs
   SET lease_holder = 'pre-lease',
       lease_term = lease_term + 1,
       lease_expires_at = now() + interval '60 seconds'
 WHERE status = 'running';

INSERT INTO stage.outbox (stage_run_id, routing_key)
SELECT stage_run_id, 'stage.requested'
  FROM stage.runs
 WHERE status = 'queued'
ON CONFLICT (stage_run_id, routing_key) DO NOTHING;
