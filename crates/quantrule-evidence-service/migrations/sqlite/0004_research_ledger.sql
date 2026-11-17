-- The pre-registered improvement record and its human-owned registries on the
-- embedded profile. The manifest is immutable; the link and decision are
-- recorded once. The removed identity is the tombstone id.
CREATE TABLE IF NOT EXISTS research__improvement_runs (
    profile_id       TEXT NOT NULL,
    improvement_id   TEXT NOT NULL,
    manifest         TEXT NOT NULL CHECK (json_valid(manifest) AND json_type(manifest) = 'object'),
    pipeline_run_id  TEXT,
    canonical_digest TEXT,
    state            TEXT NOT NULL CHECK (state IN ('created', 'submitted', 'decided')),
    decision         TEXT CHECK (decision IN ('accepted', 'rejected', 'no-edge', 'invalidated', 'new-hypothesis')),
    decision_note    TEXT,
    created_at       TEXT NOT NULL,
    submitted_at     TEXT,
    decided_at       TEXT,
    PRIMARY KEY (profile_id, improvement_id),
    CHECK (state = 'decided' OR decision IS NULL),
    CHECK (state <> 'decided' OR (decision IS NOT NULL
                                  AND decision_note IS NOT NULL
                                  AND decided_at IS NOT NULL)),
    CHECK (state = 'created' OR (pipeline_run_id IS NOT NULL
                                 AND canonical_digest IS NOT NULL
                                 AND submitted_at IS NOT NULL)),
    CHECK (state <> 'created' OR (pipeline_run_id IS NULL
                                  AND canonical_digest IS NULL
                                  AND submitted_at IS NULL))
) STRICT;

CREATE INDEX IF NOT EXISTS research__improvement_runs_profile_created_idx
    ON research__improvement_runs (profile_id, created_at DESC);
CREATE INDEX IF NOT EXISTS research__improvement_runs_profile_state_idx
    ON research__improvement_runs (profile_id, state, created_at DESC);

CREATE TRIGGER IF NOT EXISTS research__improvement_runs_manifest_immutable
BEFORE UPDATE ON research__improvement_runs
FOR EACH ROW
WHEN NEW.manifest <> OLD.manifest
  OR NEW.profile_id <> OLD.profile_id
  OR NEW.improvement_id <> OLD.improvement_id
  OR NEW.created_at <> OLD.created_at
BEGIN
    SELECT RAISE(ABORT, 'a research manifest is immutable');
END;

CREATE TRIGGER IF NOT EXISTS research__improvement_runs_link_once
BEFORE UPDATE ON research__improvement_runs
FOR EACH ROW
WHEN (OLD.pipeline_run_id IS NOT NULL
      AND (NEW.pipeline_run_id IS NOT OLD.pipeline_run_id
           OR NEW.canonical_digest IS NOT OLD.canonical_digest
           OR NEW.submitted_at IS NOT OLD.submitted_at))
  OR (OLD.decision IS NOT NULL
      AND (NEW.decision IS NOT OLD.decision
           OR NEW.decision_note IS NOT OLD.decision_note
           OR NEW.decided_at IS NOT OLD.decided_at))
  OR (OLD.state = 'decided' AND NEW.state <> 'decided')
  OR (OLD.state = 'submitted' AND NEW.state NOT IN ('submitted', 'decided'))
BEGIN
    SELECT RAISE(ABORT, 'a research link or decision is recorded once');
END;

CREATE TABLE IF NOT EXISTS research__baselines (
    profile_id    TEXT NOT NULL,
    baseline_id   TEXT NOT NULL,
    tier          TEXT NOT NULL CHECK (tier IN ('model', 'strategy', 'portfolio')),
    run_id        TEXT,
    improvement_id TEXT,
    accepted_by   TEXT NOT NULL,
    accepted_on   TEXT NOT NULL,
    scope         TEXT NOT NULL,
    supersedes    TEXT,
    created_at    TEXT NOT NULL,
    PRIMARY KEY (profile_id, baseline_id),
    CHECK (run_id IS NOT NULL OR improvement_id IS NOT NULL)
) STRICT;

CREATE INDEX IF NOT EXISTS research__baselines_profile_tier_idx
    ON research__baselines (profile_id, tier, accepted_on DESC, baseline_id);

CREATE TABLE IF NOT EXISTS research__invalidations (
    profile_id      TEXT NOT NULL,
    invalidation_id TEXT NOT NULL,
    date            TEXT NOT NULL,
    "commit"        TEXT NOT NULL,
    removed         TEXT NOT NULL,
    reason          TEXT NOT NULL,
    superseded_by   TEXT,
    created_at      TEXT NOT NULL,
    PRIMARY KEY (profile_id, invalidation_id)
) STRICT;

CREATE INDEX IF NOT EXISTS research__invalidations_profile_date_idx
    ON research__invalidations (profile_id, date DESC, invalidation_id);

CREATE TRIGGER IF NOT EXISTS research__baselines_append_only_update
BEFORE UPDATE ON research__baselines
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'an append-only research row is immutable');
END;

CREATE TRIGGER IF NOT EXISTS research__baselines_append_only_delete
BEFORE DELETE ON research__baselines
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'an append-only research row is immutable');
END;

CREATE TRIGGER IF NOT EXISTS research__invalidations_append_only_update
BEFORE UPDATE ON research__invalidations
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'an append-only research row is immutable');
END;

CREATE TRIGGER IF NOT EXISTS research__invalidations_append_only_delete
BEFORE DELETE ON research__invalidations
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'an append-only research row is immutable');
END;

INSERT OR IGNORE INTO research__baselines
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
     NULL, '2026-11-04T00:00:00Z');

INSERT OR IGNORE INTO research__invalidations
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
     NULL, '2026-11-10T00:00:00Z');
