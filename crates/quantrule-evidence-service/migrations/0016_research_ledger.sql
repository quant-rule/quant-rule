-- The pre-registered improvement record and its human-owned registries.
-- The manifest is immutable; the pipeline link and decision are recorded once.
CREATE TABLE IF NOT EXISTS research.improvement_runs (
    profile_id        text NOT NULL,
    improvement_id    text NOT NULL,
    manifest          jsonb NOT NULL CHECK (jsonb_typeof(manifest) = 'object'),
    pipeline_run_id   text,
    canonical_digest  text,
    state             text NOT NULL CHECK (state IN ('created', 'submitted', 'decided')),
    decision          text CHECK (decision IN ('accepted', 'rejected', 'no-edge', 'invalidated', 'new-hypothesis')),
    decision_note     text,
    created_at        timestamptz NOT NULL,
    submitted_at      timestamptz,
    decided_at        timestamptz,
    PRIMARY KEY (profile_id, improvement_id),
    CHECK (state = 'decided' OR decision IS NULL),
    CHECK (state <> 'decided' OR (decision IS NOT NULL
                                  AND decision_note IS NOT NULL
                                  AND decided_at IS NOT NULL)),
    CHECK (state = 'created' OR (pipeline_run_id IS NOT NULL
                                 AND submitted_at IS NOT NULL)),
    CHECK (decision <> 'accepted' OR canonical_digest IS NOT NULL),
    CHECK (state <> 'created' OR (pipeline_run_id IS NULL
                                  AND canonical_digest IS NULL
                                  AND submitted_at IS NULL))
);

CREATE INDEX IF NOT EXISTS improvement_runs_profile_created_idx
    ON research.improvement_runs (profile_id, created_at DESC);
CREATE INDEX IF NOT EXISTS improvement_runs_profile_state_idx
    ON research.improvement_runs (profile_id, state, created_at DESC);

CREATE OR REPLACE FUNCTION research.reject_manifest_change() RETURNS trigger AS $$
BEGIN
    IF NEW.manifest IS DISTINCT FROM OLD.manifest
       OR NEW.profile_id IS DISTINCT FROM OLD.profile_id
       OR NEW.improvement_id IS DISTINCT FROM OLD.improvement_id
       OR NEW.created_at IS DISTINCT FROM OLD.created_at THEN
        RAISE EXCEPTION 'a research manifest is immutable';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS improvement_runs_manifest_immutable ON research.improvement_runs;
CREATE TRIGGER improvement_runs_manifest_immutable
    BEFORE UPDATE ON research.improvement_runs
    FOR EACH ROW EXECUTE FUNCTION research.reject_manifest_change();

CREATE OR REPLACE FUNCTION research.reject_link_change() RETURNS trigger AS $$
BEGIN
    IF OLD.pipeline_run_id IS NOT NULL
       AND (NEW.pipeline_run_id IS DISTINCT FROM OLD.pipeline_run_id
            OR NEW.submitted_at IS DISTINCT FROM OLD.submitted_at) THEN
        RAISE EXCEPTION 'a research pipeline link is recorded once';
    END IF;
    IF OLD.canonical_digest IS NOT NULL
       AND NEW.canonical_digest IS DISTINCT FROM OLD.canonical_digest THEN
        RAISE EXCEPTION 'a research canonical digest is recorded once';
    END IF;
    IF OLD.decision IS NOT NULL
       AND (NEW.decision IS DISTINCT FROM OLD.decision
            OR NEW.decision_note IS DISTINCT FROM OLD.decision_note
            OR NEW.decided_at IS DISTINCT FROM OLD.decided_at) THEN
        RAISE EXCEPTION 'a research decision is recorded once';
    END IF;
    IF OLD.state = 'decided' AND NEW.state <> 'decided' THEN
        RAISE EXCEPTION 'a research decision cannot be reopened';
    END IF;
    IF OLD.state = 'created' AND NEW.state NOT IN ('created', 'submitted') THEN
        RAISE EXCEPTION 'a research run must be submitted before a decision';
    END IF;
    IF OLD.state = 'submitted' AND NEW.state NOT IN ('submitted', 'decided') THEN
        RAISE EXCEPTION 'a research submission cannot move backwards';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS improvement_runs_link_once ON research.improvement_runs;
CREATE TRIGGER improvement_runs_link_once
    BEFORE UPDATE ON research.improvement_runs
    FOR EACH ROW EXECUTE FUNCTION research.reject_link_change();

CREATE TABLE IF NOT EXISTS research.baselines (
    profile_id    text NOT NULL,
    baseline_id   text NOT NULL,
    tier          text NOT NULL CHECK (tier IN ('model', 'strategy', 'portfolio')),
    run_id        text,
    improvement_id text,
    accepted_by   text NOT NULL,
    accepted_on   date NOT NULL,
    scope         text NOT NULL,
    supersedes    text,
    created_at    timestamptz NOT NULL,
    PRIMARY KEY (profile_id, baseline_id),
    CHECK (run_id IS NOT NULL OR improvement_id IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS baselines_profile_tier_idx
    ON research.baselines (profile_id, tier, accepted_on DESC, baseline_id);

CREATE TABLE IF NOT EXISTS research.invalidations (
    profile_id      text NOT NULL,
    invalidation_id text NOT NULL,
    date            date NOT NULL,
    "commit"        text NOT NULL,
    removed         text NOT NULL,
    reason          text NOT NULL,
    superseded_by   text,
    created_at      timestamptz NOT NULL,
    PRIMARY KEY (profile_id, invalidation_id)
);

CREATE INDEX IF NOT EXISTS invalidations_profile_date_idx
    ON research.invalidations (profile_id, date DESC, invalidation_id);

CREATE OR REPLACE FUNCTION research.reject_append_only() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'an append-only research row is immutable';
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS baselines_append_only ON research.baselines;
CREATE TRIGGER baselines_append_only
    BEFORE UPDATE OR DELETE ON research.baselines
    FOR EACH ROW EXECUTE FUNCTION research.reject_append_only();

DROP TRIGGER IF EXISTS invalidations_append_only ON research.invalidations;
CREATE TRIGGER invalidations_append_only
    BEFORE UPDATE OR DELETE ON research.invalidations
    FOR EACH ROW EXECUTE FUNCTION research.reject_append_only();

-- Historical accepted model baselines from docs/improvement/baselines.md.
INSERT INTO research.baselines
    (profile_id, baseline_id, tier, run_id, improvement_id, accepted_by,
     accepted_on, scope, supersedes, created_at)
VALUES
    ('default', 'MODEL-2026-11-04-04..05', 'model', 'MODEL-2026-11-04-04..05', NULL,
     'Human', '2026-11-04',
     '1m, 4 ATR over 20 bars, BTCUSD/ETHUSD/XAUUSD; dev and validation pass',
     NULL, '2026-11-04T00:00:00Z'),
    ('default', 'MODEL-2026-11-04-06..07', 'model', 'MODEL-2026-11-04-06..07', NULL,
     'Human', '2026-11-04',
     '1m, 5-bar compression range, next-bar boundary touch; dev and validation pass; second use of model_validation disclosed',
     NULL, '2026-11-04T00:00:00Z'),
    ('default', 'MODEL-2026-11-04-10..11', 'model', 'MODEL-2026-11-04-10..11', NULL,
     'Human', '2026-11-04',
     '5m fresh-onset range; dev and validation pass; third use of model_validation disclosed',
     NULL, '2026-11-04T00:00:00Z')
ON CONFLICT (profile_id, baseline_id) DO NOTHING;

-- Historical tombstones from docs/improvement/INVALIDATIONS.md. The removed
-- identity is the tombstone id; no new id is invented.
INSERT INTO research.invalidations
    (profile_id, invalidation_id, date, "commit", removed, reason, superseded_by, created_at)
VALUES
    ('default', 'docs/experiments/2026-10-30/EXP-2026-10-30-01, EXP-2026-10-30-02',
     '2026-11-01', 'worktree', 'docs/experiments/2026-10-30/EXP-2026-10-30-01, EXP-2026-10-30-02',
     'Removed; the workflow, run identity, and evidence contract changed',
     'MODEL-2026-11-01-01', '2026-11-01T00:00:00Z'),
    ('default', 'docs/experiments/2026-11-01/EXP-2026-11-01-03, EXP-2026-11-01-04, EXP-2026-11-01-05, EXP-2026-11-01-06',
     '2026-11-01', 'worktree', 'docs/experiments/2026-11-01/EXP-2026-11-01-03, EXP-2026-11-01-04, EXP-2026-11-01-05, EXP-2026-11-01-06',
     'Removed; the v4/v5 level_lost behavior changed and the run contract replaced the experiment contract',
     'MODEL-2026-11-01-01', '2026-11-01T00:00:00Z'),
    ('default', 'docs/audits/2026-10-30/*, docs/audits/2026-10-31/*',
     '2026-11-01', 'worktree', 'docs/audits/2026-10-30/*, docs/audits/2026-10-31/*',
     'Removed; their closure no longer describes the workflow-reset head',
     'docs/audits/2026-11-01/workflow-reset-audit.md', '2026-11-01T00:00:00Z'),
    ('default', 'data/labs, data/reports, data/research',
     '2026-11-01', 'worktree', 'data/labs, data/reports, data/research',
     'Removed; replaced by data/improvement',
     'data/improvement/2026-11-01/MODEL-2026-11-01-01', '2026-11-01T00:00:00Z'),
    ('default', 'data/improvement/2026-11-01/MODEL-2026-11-01-01..04',
     '2026-11-03', 'worktree', 'data/improvement/2026-11-01/MODEL-2026-11-01-01..04',
     'Invalidated by the model v3 flat contract; the axis-based measurements are not comparable with the v3 states and events',
     'MODEL-2026-11-03-01..02', '2026-11-03T00:00:00Z'),
    ('default', 'data/improvement/2026-11-01/STRATEGY-2026-11-01-01..15',
     '2026-11-03', 'worktree', 'data/improvement/2026-11-01/STRATEGY-2026-11-01-01..15',
     'Invalidated by the model v3 contract and the 10% policy; the strategies used the retired axes and a 20% drawdown envelope',
     'MODEL-2026-11-03-01..02', '2026-11-03T00:00:00Z'),
    ('default', 'docs/improvement/2026-11-10/SR10.md',
     '2026-11-10', 'worktree', 'docs/improvement/2026-11-10/SR10.md',
     'Invalidated by M1.3: the SR10 entries used the M1.2 level.fib_test endpoint event set; M1.3 corrects the event to a bar-range intersection with one event per tested level',
     NULL, '2026-11-10T00:00:00Z'),
    ('default', 'docs/improvement/2026-11-10/SR11.md',
     '2026-11-10', 'worktree', 'docs/improvement/2026-11-10/SR11.md',
     'Invalidated by M1.3: the SR11 entries used the M1.2 level.fib_test endpoint event set; M1.3 corrects the event to a bar-range intersection with one event per tested level',
     NULL, '2026-11-10T00:00:00Z')
ON CONFLICT (profile_id, invalidation_id) DO NOTHING;
