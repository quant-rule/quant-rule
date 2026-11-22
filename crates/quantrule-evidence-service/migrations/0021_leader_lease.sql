-- One evidence finalizer at a time.
--
-- The table is a singleton. Acquisition and renewal use database time, and the
-- term increments on every acquisition, never on a renewal. The term is the
-- fencing token a finalisation checks; a stale holder's term no longer matches
-- the live row and its write is refused.
CREATE TABLE IF NOT EXISTS runs.leader_lease (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    holder text NOT NULL DEFAULT '',
    term bigint NOT NULL DEFAULT 0,
    expires_at timestamptz NOT NULL DEFAULT '-infinity',
    updated_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO runs.leader_lease (singleton)
VALUES (true)
ON CONFLICT (singleton) DO NOTHING;
