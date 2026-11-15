-- The simulation research plane's SQLite baseline for the embedded profile.
--
-- This is the final logical shape, not a translation of the Postgres history:
-- a desktop database starts at v1.1 and has no earlier rows to migrate. SQLite
-- has no schemas, so `simulation.runs` is `simulation__runs` and the marks
-- relation is `simulation__marks`. A JSON document is `TEXT` and an instant is
-- RFC 3339 UTC `TEXT`, the SQLite spellings of `jsonb` and `timestamptz`.
--
-- The constraints carry the rules the repository also enforces. A mark is
-- unique by its identity `(sim_id, origin, chart, bar, action, side)`, so a
-- repeated write is one row rather than two. A decision and its time exist
-- exactly when the run is decided, so a half-written decision cannot be
-- recorded. The triggers close marking once a decision lands: SQLite has no
-- `FOR UPDATE`, so the repository takes an immediate transaction and re-reads
-- the run row, and these triggers refuse a direct writer that tries to bypass
-- it. A decided run is final: its status, decision, note, and time never change
-- again, while its check result may still be refreshed.
CREATE TABLE IF NOT EXISTS simulation__runs (
    sim_id         TEXT PRIMARY KEY,
    profile_id     TEXT NOT NULL,
    premise        TEXT NOT NULL,
    bundle         TEXT NOT NULL,
    inputs         TEXT NOT NULL,
    inputs_digest  TEXT NOT NULL,
    seed           INTEGER NOT NULL CHECK (seed >= 0),
    criteria       TEXT NOT NULL,
    window_spec    TEXT NOT NULL,
    oracle_spec    TEXT NOT NULL,
    check_config   TEXT NOT NULL,
    selection      TEXT,
    status         TEXT NOT NULL CHECK (
        status IN ('draft', 'marking', 'reviewed', 'checked', 'decided')
    ),
    check_result   TEXT,
    decision       TEXT CHECK (decision IN ('test', 'refine', 'stop')),
    decision_note  TEXT,
    decided_at     TEXT,
    created_at     TEXT NOT NULL,
    CHECK ((status = 'decided') = (decision IS NOT NULL)),
    CHECK ((status = 'decided') = (decision_note IS NOT NULL)),
    CHECK ((status = 'decided') = (decided_at IS NOT NULL))
) STRICT;

CREATE INDEX IF NOT EXISTS simulation__runs_profile_idx
    ON simulation__runs (profile_id, created_at DESC, sim_id);

CREATE TABLE IF NOT EXISTS simulation__marks (
    mark_id          TEXT PRIMARY KEY,
    sim_id           TEXT NOT NULL REFERENCES simulation__runs (sim_id) ON DELETE CASCADE,
    profile_id       TEXT NOT NULL,
    chart            INTEGER NOT NULL CHECK (chart >= 0),
    bar              INTEGER NOT NULL CHECK (bar >= 0),
    marked_time_utc  TEXT NOT NULL,
    action           TEXT NOT NULL CHECK (action IN ('entry', 'exit')),
    quality          TEXT NOT NULL CHECK (quality IN ('good', 'bad', 'neutral')),
    side             TEXT NOT NULL CHECK (side IN ('long', 'short')),
    origin           TEXT NOT NULL CHECK (origin IN ('human', 'oracle')),
    note             TEXT NOT NULL DEFAULT '',
    created_at       TEXT NOT NULL,
    UNIQUE (sim_id, origin, chart, bar, action, side)
) STRICT;

CREATE INDEX IF NOT EXISTS simulation__marks_run_idx
    ON simulation__marks (profile_id, sim_id, chart, bar, action, side);

CREATE TRIGGER IF NOT EXISTS simulation__marks_closed_after_decision_insert
BEFORE INSERT ON simulation__marks
WHEN (SELECT status FROM simulation__runs WHERE sim_id = NEW.sim_id) = 'decided'
BEGIN
    SELECT RAISE(ABORT, 'simulation: marking is closed after a decision');
END;

CREATE TRIGGER IF NOT EXISTS simulation__marks_closed_after_decision_update
BEFORE UPDATE ON simulation__marks
WHEN (SELECT status FROM simulation__runs WHERE sim_id = OLD.sim_id) = 'decided'
BEGIN
    SELECT RAISE(ABORT, 'simulation: marking is closed after a decision');
END;

CREATE TRIGGER IF NOT EXISTS simulation__marks_closed_after_decision_delete
BEFORE DELETE ON simulation__marks
WHEN (SELECT status FROM simulation__runs WHERE sim_id = OLD.sim_id) = 'decided'
BEGIN
    SELECT RAISE(ABORT, 'simulation: marking is closed after a decision');
END;

CREATE TRIGGER IF NOT EXISTS simulation__runs_are_final
BEFORE UPDATE ON simulation__runs
WHEN OLD.status = 'decided'
 AND (
     NEW.status IS NOT OLD.status
     OR NEW.decision IS NOT OLD.decision
     OR NEW.decision_note IS NOT OLD.decision_note
     OR NEW.decided_at IS NOT OLD.decided_at
 )
BEGIN
    SELECT RAISE(ABORT, 'simulation: a decided run is final');
END;
