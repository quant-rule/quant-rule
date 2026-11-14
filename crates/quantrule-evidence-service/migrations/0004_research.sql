-- A sealed hypothesis is the record of what was tested. The trigger refuses
-- both an edit and a delete, so the evidence cannot be rewritten.
CREATE SCHEMA IF NOT EXISTS research;

CREATE TABLE IF NOT EXISTS research.hypotheses (
    fingerprint text PRIMARY KEY,
    premise     text NOT NULL,
    rule        text NOT NULL,
    dataset     text NOT NULL,
    costs       jsonb NOT NULL,
    split       jsonb NOT NULL,
    criteria    text NOT NULL,
    sealed_at   timestamptz NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE OR REPLACE FUNCTION research.reject_change() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'a sealed hypothesis is immutable';
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS hypotheses_immutable ON research.hypotheses;
CREATE TRIGGER hypotheses_immutable
    BEFORE UPDATE OR DELETE ON research.hypotheses
    FOR EACH ROW EXECUTE FUNCTION research.reject_change();
