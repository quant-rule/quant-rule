-- Simulation decision and mark concurrency.
--
-- The repository serializes every mark mutation and the terminal decision on
-- one `simulation.runs` row with `SELECT ... FOR UPDATE`. These constraints make
-- the decision terminal even if a later code path or an operator bypasses that
-- lock. A mark identity is already unique in `simulation.marks`.
ALTER TABLE simulation.runs
    ADD CONSTRAINT simulation_runs_decision_complete CHECK (
        (status = 'decided' AND decision IS NOT NULL AND decided_at IS NOT NULL)
        OR (status <> 'decided' AND decision IS NULL AND decided_at IS NULL)
    );

CREATE OR REPLACE FUNCTION simulation.runs_decision_is_terminal() RETURNS trigger AS $$
BEGIN
    IF OLD.decision IS NOT NULL AND (
        NEW.decision IS DISTINCT FROM OLD.decision
        OR NEW.decided_at IS DISTINCT FROM OLD.decided_at
        OR NEW.status <> 'decided'
    ) THEN
        RAISE EXCEPTION 'the simulation decision is terminal'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS simulation_runs_decision_terminal ON simulation.runs;
CREATE TRIGGER simulation_runs_decision_terminal
    BEFORE UPDATE ON simulation.runs
    FOR EACH ROW EXECUTE FUNCTION simulation.runs_decision_is_terminal();
