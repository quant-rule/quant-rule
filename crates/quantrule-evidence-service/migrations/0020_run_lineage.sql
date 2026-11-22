-- Research artifact lineage for a run.
--
-- A run cites its model artifact, strategy pack, signal artifact, portfolio
-- policy and execution model by id and hash. Evidence keeps that set in its own
-- database; it never reads the catalog schema and no cross-database transaction
-- is involved. The header row makes "recorded with no references" distinct from
-- "not recorded", and the references are immutable once recorded: a repeat with
-- the same set is idempotent, a different set is refused.
CREATE TABLE IF NOT EXISTS runs.run_lineage (
    run_id      text PRIMARY KEY REFERENCES runs.runs (run_id) ON DELETE CASCADE,
    recorded_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS runs.run_lineage_references (
    run_id         text NOT NULL
        REFERENCES runs.run_lineage (run_id) ON DELETE CASCADE,
    reference_kind text NOT NULL
        CHECK (reference_kind IN ('model_artifact', 'strategy_pack', 'signal_artifact',
                                  'portfolio_policy', 'execution_model')),
    reference_id   text NOT NULL,
    reference_hash text NOT NULL,
    pack           jsonb,
    PRIMARY KEY (run_id, reference_kind),
    CHECK (length(reference_id) = 71),
    CHECK (length(reference_hash) = 71)
);
