# Simulation Data Loss Audit

- Date: 2026-11-11 (UTC)
- Status: Current; read-only finding; recovery complete, `Verified`
- Scope: the local simulation store and the merged-task cleanup
- Baseline: `main` at `ff42095` (PR #584 merged)
- Claim type: operational defect audit; no model or strategy claim

## Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| AUD-2026-11-11-01 | Major | Local data / cleanup | The merged task worktree's ignored `data/simulation/` store was deleted by `git worktree remove`; the cleanup rule had no concrete local-data check or archive step | `git worktree remove /home/zantgo/Proyectos/quant-rule--simulation-runs`; `git worktree list` now shows only `main`; `data/simulation/` in main has only `README.md`; the task store held the five M1.3 reports and the SIM-01..SIM-10 artifacts | The M1.3 reports, marks, windows, PNGs, and check files are lost; the committed reports preserve the numbers; no code, docs, or model/strategy result is lost | A. Regenerate from the source and committed configs; B. restore from a backup (none exists); C. accept the loss | A + a cleanup guard that archives local data before removal | The regenerated store reproduces the committed check numbers; a temporary worktree with local data is archived and removed only after the manifest is written | Verified | PR #584; plan [`simulation-data-recovery.md`](../../execution-plans/2026-11-11/simulation-data-recovery.md); synthesis [`simulation-synthesis.md`](../../reports/2026-11-11/simulation-synthesis.md) |

## Detail

The simulation store is local by design: `data/README.md` and
`data/simulation/README.md` state that candles, marks, PNGs, and model reports
are not committed. The task worktree was the only copy.

The cleanup sequence was:

```text
git worktree remove /home/zantgo/Proyectos/quant-rule--simulation-runs
git branch -d feat/simulation-runs
git push origin --delete feat/simulation-runs
```

`git worktree remove` removes the whole directory, including ignored local data.
The cleanup rule said to verify that no uncommitted or unmerged work would be
lost, but it provided no machine-checkable command for ignored `data/` content.
The agent treated the committed reports as sufficient and did not archive the
store.

## Root cause

1. Persistent local data lived only inside a removable task worktree.
2. The cleanup rule had no local-data classification or archive step.
3. The archive destination was not defined.

## Impact

- Lost: the five per-year M1.3 reports, SIM-01..SIM-10 windows, marks, context
  and zoom PNGs, check files, and local decision records.
- Preserved: the committed batch reports, the campaign synthesis, the retired
  hypothesis note, the source candles, and the human annotation CSV.
- The campaign remains closed with no edge; no model, strategy, execution, or
  portfolio result is affected.

## Remediation

1. Regenerate the store into `/home/zantgo/quant-rule-data/simulation/`, outside
   the repository and every worktree.
2. Commit the exact M1.3 and SIM run/check configs so recovery does not depend on
   conversation history.
3. Add `tools/worktree_cleanup.py` with `check`, `archive`, and `remove`; require
   it before `git worktree remove`.
4. Amend AGENTS.md section 9 to require the check/archive step and a recorded
   archive manifest.

## Verification
- Verifier: sequential same-agent Improvement Verifier pass; not an independent
  agent or independent model judgment. The human approved this pass.
- Date: 2026-11-11 (UTC).
- Baseline: `main` at `9e466fa` (PR #586). The diff is 46 files, +2,062/−25;
  no `crates/` file, no model, strategy, execution, or portfolio behavior change.
- Recovered store: `/home/zantgo/quant-rule-data/simulation/` has five M1.3
  reports, ten SIM run directories, 461 PNGs, and nine checks. The key numbers
  match the committed reports:
  - SIM-08 bull: n=466, mean +0.0786%, hit 54.1%; all-bars control n=4,998,
    mean −0.0276%.
  - SIM-02: long n=1,902, mean −0.0165%; short n=1,692, mean −0.0747%.
  - SIM-01: filtered −0.1170% (n=1,292) versus control −0.0923% (n=4,999).
  - SIM-03: 101 label and 3,328 check entries; SIM-09 underpowered; SIM-10
    blocked.
- Tests: 51 simulation tests, 6 cleanup-guard tests, 8 trade-tool, 23 chart-tool,
  and `cargo test` all pass. `check_structure`, `check_workflows`,
  `check_changelog`, `check_command_surface`, `check_links`, and
  `check_registry` pass.
- Guard: `tools/worktree_cleanup.py` archives `data/` with a SHA-256 manifest,
  refuses unclassified or unmerged worktrees, and was used to remove the
  remediation task worktree. `AGENTS.md` section 9 names the check/archive step.
- Residual: the guard treated `crates/quantrule-shell/target/` as a blocker
  during dogfooding because a nested `target/` directory is not in the
  disposable set. It was a build artifact, was removed manually, and did not
  cause data loss. A follow-up should classify nested `target/` as disposable.
- The external store is local to this machine; a second local copy or backup is
  a separate operational decision.

Status: `Verified`. The finding is closed; the residual is a tooling follow-up,
not a data-loss recurrence.
