# AI Transparency Log — Simulation Data Recovery

- Date: 2026-11-11 (UTC)
- Work item: `simulation-data-recovery`
- Plan: [`docs/execution-plans/2026-11-11/simulation-data-recovery.md`](../../execution-plans/2026-11-11/simulation-data-recovery.md)
- Audit: [`docs/audits/2026-11-11/simulation-data-loss-audit.md`](../../audits/2026-11-11/simulation-data-loss-audit.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash), remediation session
- Claim type: data recovery and process hardening; no model or strategy change

## Entry 1 — Plan, audit, and canonical root

### Intent and boundary

- Record the loss of the local simulation store after PR #584 and implement the
  recovery plus the cleanup guard.
- No model, strategy, execution, portfolio, or analytics change. No validation or
  holdout access.

### Work performed

- Wrote the audit finding and the approved recovery plan.
- Chose the external local root `/home/zantgo/quant-rule-data/simulation/`,
  outside the repository and every worktree, with `QUANTRULE_DATA_ROOT` and
  `--data-root` overrides.
- The plan commits only the small run/config examples; the reports, marks, PNGs,
  and archives remain local.

### Verification

- The audit records the exact cleanup commands and the missing paths.
- The committed reports preserve the campaign numbers.
- The plan is committed as Approved.

### Learning

- The previous cleanup treated committed records as sufficient and did not
  preserve the local store. The new rule makes the local root external and the
  archive step explicit.
- A task worktree must not be the only home of persistent local data.

## Entry 2 — Implementation and recovery

### Intent and boundary

- Implement the external data root, the exact recovery configs, the cleanup
  guard, and the recovery. No model, strategy, execution, or portfolio change.

### Work performed

- Added `tools/simulation/data_root.py` and wired `--data-root`,
  `QUANTRULE_DATA_ROOT`, `{data_root}`, and the worktree-write guard into
  `run.py`, `check.py`, `meta.py`, and `exit_lab.py`.
- Committed the M1.3 per-year configs and the SIM-01..SIM-10 configs under
  `tools/simulation/examples/`.
- Added `tools/simulation/reproduce.py` and `tools/worktree_cleanup.py` with
  tests.
- Amended `AGENTS.md` section 9 to require the check/archive step.
- Regenerated the five M1.3 reports and the ten run directories under
  `/home/zantgo/quant-rule-data/simulation/`.
- Ran the checks; the committed numbers are reproduced, including SIM-08 bull
  n=466, +0.0786%, hit 54.1%, and SIM-02 long −0.0165%, short −0.0747%.

### Verification

- `python3 -m unittest discover -s tools/simulation -p 'test_*.py'`: 51 tests
  pass.
- `python3 tools/test_worktree_cleanup.py`: 5 tests pass, including archive,
  blocker, unmerged refusal, and merged removal.
- The external store has five reports, ten run directories, 461 PNGs, and the
  nine checks.
- The recovery report is `docs/reports/2026-11-11/simulation-recovery.md`.

### Learning and corrections

- The first recovery command timed out at 30 minutes after the SIM-07 run and
  the first five checks; the remaining checks were run individually. The
  external store persisted, which is the point of the change.
- The SIM-08 bear check was initially regenerated with an all-bars control; it
  was changed back to the event control to match the committed run.
- The local `decision.md` files are regenerated skeletons; the committed batch
  reports remain the authoritative decisions.

## Entry 3 — Improvement Verifier pass

### Intent and boundary

- Independently check the merged recovery and cleanup guard against
  `AUD-2026-11-11-01`; read-only except for the audit status and the
  verification record.
- This is a sequential same-agent pass, not an independent agent or independent
  model judgment; disclosed in the record. The human approved the pass.

### Verification

- Baseline `main` at `9e466fa` (PR #586); diff 46 files, +2,062/−25; no
  `crates/` change and no model/strategy/execution/portfolio behavior change.
- Store: five reports, ten runs, 461 PNGs, nine checks; SIM-08 bull n=466,
  +0.0786%, hit 54.1%; SIM-02 long −0.0165%, short −0.0747%; SIM-01 −0.1170%
  versus −0.0923%; SIM-03 101/3,328; SIM-09 underpowered; SIM-10 blocked.
- Tests: 51 simulation, 6 cleanup, 8 trade, 23 chart; `cargo test`; repo checks.
- Guard: archive/refusal/remove tests pass; AGENTS.md section 9 requires the
  step.
- Residual: nested `crates/*/target/` is a false-positive blocker; it was
  removed manually during dogfooding; no data loss. Follow-up recommended.

### Learning

- The guard needs a nested `target/` rule; the data-loss closure itself is
  verified. The residual is tooling friction, not a recurrence.
