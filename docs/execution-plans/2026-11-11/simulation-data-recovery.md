# Execution Plan — Simulation Data Recovery and Worktree Cleanup Safety

- Date: 2026-11-11 (UTC)
- Status: **Approved** — execution started 2026-11-11
- Work item: `simulation-data-recovery` (no issue number)
- Workflow: Audit Workflow (data-loss finding) + engineering remediation
- Owner: Orchestrator, Implementation, Test Engineer, Code Reviewer; Human acceptance
- Claim type: data recovery and process hardening; no model or strategy change
- Intended repo plan: `docs/execution-plans/2026-11-11/simulation-data-recovery.md`
- AI transparency log: `docs/ai-transparency-log/2026-11-11/simulation-data-recovery.md`
- Branch/worktree: `fix/simulation-data-recovery` in `quant-rule--simulation-data-recovery`
- Canonical local store: `/home/zantgo/quant-rule-data/simulation/` (override with `QUANTRULE_DATA_ROOT`)

## 1. Intent

Recover the local simulation store that was deleted when the merged PR #584 task
worktree was removed, and prevent the same loss from recurring.

The code, docs, and committed reports are safe. The lost artifacts are local-only:
the per-year M1.3 model reports, the SIM-01..SIM-10 windows, marks, charts, zoom
images, and check files. They are deterministic from the source data and the
harness, so recovery is possible. The campaign itself remains closed with no
edge; this plan does not reopen the candidate or the research.

The second goal is structural: a task worktree must never be the only home of
persistent local data. The cleanup step must check, archive, and record local
data before `git worktree remove`, and the harness must default its persistent
output to the canonical main-worktree store.

## 2. Current state

- PR #584 merged at `ff42095`; `main` is clean.
- The task worktree and the local/remote `feat/simulation-runs` branch were
  removed. `git worktree list` shows only `main`.
- The task worktree's `data/simulation/` was untracked and ignored by Git. It
  contained:
  - `_data/M1.3-2018-2022/report-2018..2022/model/` (five per-year reports),
  - `2026-11-10/SIM-01..SIM-10/` (manifests, windows, marks, charts, zooms,
    reviews, checks, decisions).
- The committed records preserve the numbers and conclusions:
  `docs/reports/2026-11-11/simulation-batch-1.md`, `simulation-batch-2.md`,
  `simulation-batch-3.md`, `simulation-synthesis.md`, and
  `sim08-bull-divergence-hypothesis.md`.
- The human marks are preserved at
  `data/improvement/2026-11-09/BINANCE-BTCUSDT/annotations-binance.csv`.
- The BTCUSDT M1 source is preserved at
  `data/sources/BTCUSDT_M1_2018-01-01_2025-12-31.csv`.
- The M1.3 report configs and SIM run/check configs are not committed. Their
  parameters are in the batch reports, the plan, and the AI transparency log;
  they can be reconstructed.
- `data/README.md` says local data is not committed. It has no canonical root
  across worktrees and no archive step.
- `AGENTS.md` section 9 requires cleanup after merge and says to verify that no
  uncommitted or unmerged work will be lost. It has no concrete check or
  archive command for ignored local data.
- The cleanup command used was `git worktree remove` plus branch deletion. It
  deleted the ignored local store with the worktree.
- Root cause: persistent local data lived only inside a removable worktree, and
  the cleanup rule had no machine-checkable archive step.

## 3. Scope

### Included

- An incident record with reproducible evidence and the root cause.
- A canonical local data root and a harness default that writes there.
- Committed recovery configs for the M1.3 reports and all ten simulation runs.
- A `tools/worktree_cleanup.py` check/archive/remove tool with tests.
- An AGENTS.md cleanup amendment and data/tool documentation.
- Regeneration of the five M1.3 reports and the ten runs into the canonical
  store, with a comparison against the committed reports.
- Changelog and AI transparency entries.

### Excluded

- No model, strategy, execution, portfolio, or analytics behavior change.
- No reopening of SIM08-BULL-DIV or the simulation campaign.
- No validation (2024) or holdout (2025) access.
- No committing of local data, PNGs, marks, or model reports.
- No new dependencies; the cleanup tool uses the Python standard library, and
  the harness keeps its existing Pillow dependency.
- No change to the partitions or the policy envelope.

## 4. Assumptions and uncertainty

- The lost store is deterministic from the source data, the M1.3 config, and the
  harness at/after `ff42095`.
- The exact run parameters can be reconstructed from the batch reports, the
  execution plan, and the AI log. The recovery configs are committed in this
  work so a future loss is not dependent on conversation history.
- The committed check numbers are the comparison target. A mismatch means the
  recovery config is wrong or the code changed; stop and reconcile, do not tune
  the result.
- The canonical store is outside the repository and every worktree:
  `/home/zantgo/quant-rule-data/simulation/`. `git worktree remove` cannot
  touch it. A user can still delete it manually; that is outside the cleanup
  rule.
- Regeneration takes roughly 30–45 minutes: five model reports, ten runs, and
  the checks. The harness can be run in batches to survive interruptions.
- The five per-year reports are about 390 MB; the check/run artifacts add more.
  Free disk space is a precondition.

## 5. Decisions

| ID | Decision | Recommendation | Status |
|---|---|---|---|
| D1 | Canonical local root | `/home/zantgo/quant-rule-data/simulation/`; `--data-root`/`QUANTRULE_DATA_ROOT` can override; a linked worktree defaults there, never its own ignored `data/` | Approved |
| D2 | Cleanup gate | New `tools/worktree_cleanup.py` with `check`, `archive`, and `remove`; AGENTS.md requires it before `git worktree remove` | Approved |
| D3 | Recovery configs | Commit the exact M1.3 and SIM-01..SIM-10 run/check configs under `tools/simulation/examples/` (no data) | Approved |
| D4 | Recovery scope | Regenerate the five M1.3 reports and all ten runs; compare to the committed reports | Approved |
| D5 | AGENTS.md change | Add the mandatory local-data check/archive step to section 9; this is a methodology change | Approved |

D1–D5 need explicit human approval. D1 and D5 are the two that change how local
data and cleanup work; without them the recovery still restores the artifacts but
the recurrence guard is incomplete.

## 6. Acceptance criteria

- The canonical store has the five per-year M1.3 reports and the ten SIM run
  directories with manifest, windows, marks, charts, zooms, review, check, and
  decision.
- The regenerated numbers match the committed reports: for example, SIM-08
  bullish long n=466, mean +0.0786%, hit 54.1%; SIM-02 long −0.0165%, short
  −0.0747%; SIM-01 filtered −0.12% versus control −0.09%. A mismatch is
  investigated, not tuned.
- The exact run/check configs are committed under `tools/simulation/examples/`
  and a `reproduce.md` command regenerates the store from the source.
- `tools/worktree_cleanup.py check` passes a clean worktree, archives a worktree
  with local data and writes a manifest, and refuses a worktree with untracked
  non-ignored files.
- A cleanup test removes a temporary merged worktree only after the archive and
  verifies the archive hash; an unmerged branch is refused.
- AGENTS.md section 9 names the check/archive step; `data/README.md` and
  `tools/simulation/README.md` document the canonical root.
- The incident and recovery are recorded; the changelog and AI log are updated.
- No model/strategy behavior change; no validation access.

## 7. Work breakdown

### Phase 0 — Incident record (0.5 day)

1. Write `docs/audits/2026-11-11/simulation-data-loss-audit.md` with:
   - finding: the task worktree's ignored local store was deleted by
     `git worktree remove`;
   - evidence: the commands, the missing paths, the preserved records, the
     cleanup rule text;
   - impact: local PNGs/marks/reports lost, committed numbers preserved;
   - options: restore from backup (none), regenerate, or accept loss;
   - recommendation: regenerate and add the cleanup guard.
2. Write the remediation plan to
   `docs/execution-plans/2026-11-11/simulation-data-recovery.md`.
3. Start the AI transparency log.

Gate: human approves the audit, the plan, D1–D5, and the AGENTS.md change.

### Phase 1 — Canonical root and committed configs (0.5–1 day)

1. Add a data-root resolver to `tools/simulation/`:
   - `--data-root` argument or `QUANTRULE_DATA_ROOT`;
   - default: `$HOME/quant-rule-data/simulation/`;
   - a path inside the current worktree is refused unless
     `--allow-worktree-data` is given; the manifest records the exception;
   - the run configs use a `{data_root}` placeholder so they are portable.
2. Commit the exact M1.3 report configs under
   `tools/simulation/examples/m13-2018-2022/`:
   - `data-YYYY.json`, `run-YYYY.json` for 2018–2022;
   - `model.json`, `policy.json`, `execution.json`, `portfolio.json`;
   - `README.md` with the per-year `report model` commands.
3. Commit the exact SIM run/check configs under
   `tools/simulation/examples/runs/`:
   - SIM-01 meta, SIM-02, SIM-03, SIM-04, SIM-05, SIM-06, SIM-07, SIM-08
     (bull/bear checks), SIM-09 (climax/absorption checks), SIM-10 blocked;
   - the paths use the data root, not absolute paths;
   - a `reproduce.md` with the ordered commands.
4. Add tests:
   - data-root resolution in a temp repo with a linked worktree;
   - config loading and refusal to write into a linked worktree's own `data/`
     without the explicit flag;
   - recovery dry run on the fixture bundle.

Gate: tests pass; the configs reproduce the fixture.

### Phase 2 — Cleanup guard (1 day)

1. Add `tools/worktree_cleanup.py`:
   - `check <worktree>`: list tracked, untracked, and ignored files;
     classify `data/` local data, `target/`, `__pycache__`, `.venv`, and
     unknown files; return non-zero if any untracked non-ignored or unknown
     file remains.
   - `archive <worktree> --dest <canonical> [--label <task>]`: copy the local
     data to `<canonical>/worktree-archive/<task>/<UTC timestamp>/`; write
     `archive-manifest.json` with source, destination, file count, bytes,
     per-file SHA-256, source commit, and the check result; do not delete the
     source.
   - `remove <worktree> --branch <branch> [--remote origin] --yes`: run
     `check`; if local data remains, require `archive`; verify the branch is
     merged; then `git worktree remove`, `git branch -d`, and
     `git push origin --delete`; refuse on any failure.
   - The tool is read-only unless `remove --yes` is given; `check` and
     `archive` never delete.
2. Tests in `tools/test_worktree_cleanup.py` or
   `tools/worktree_cleanup/`:
   - a clean temporary worktree passes;
   - a worktree with `data/local/file` is archived with a correct hash;
   - an untracked non-ignored file blocks removal;
   - an unmerged branch is refused;
   - `remove` is not called without `--yes`.
3. Amend `AGENTS.md` section 9:
   - before `git worktree remove`, run
     `python3 tools/worktree_cleanup.py check <worktree>`;
   - if local data is present, run `archive` and record the manifest in the task
     record;
   - do not remove until the check passes; if a file cannot be classified,
     preserve the worktree and report a blocker.
4. Update `data/README.md` with the canonical root and the archive path.
5. Update `tools/simulation/README.md` with the data-root default.

Gate: the cleanup tool tests pass; the AGENTS.md change is approved; a dry run
on a temporary worktree archives and removes without loss.

### Phase 3 — Recovery (0.5 day, after Phase 1)

1. Create the canonical store under `/home/zantgo/quant-rule-data/simulation/`:
   - `_data/M1.3-2018-2022/config/` (the committed examples copied/pointed to),
   - `_data/M1.3-2018-2022/report-YYYY/` (five `report model` runs),
   - `2026-11-10/SIM-01..SIM-10/` (ten runs and checks).
2. Run the five per-year reports from `main` or from the task worktree with
   `--data-root` pointing at `/home/zantgo/quant-rule-data/simulation`.
3. Run SIM-01..SIM-09 and the SIM-10 blocked record; run the checks.
4. Compare the check numbers to the committed reports:
   - SIM-08 bull n=466, mean +0.0786%, hit 54.1%, 4/6 half-years;
   - SIM-02 long −0.0165%, short −0.0747%;
   - SIM-01 filtered −0.12% versus control −0.09%;
   - SIM-03/SIM-04/SIM-05/SIM-06/SIM-07 numbers as recorded;
   - SIM-09 underpowered; SIM-10 blocked.
5. Write `docs/reports/2026-11-11/simulation-recovery.md` with the commands,
   the comparison table, the canonical paths, and the limitations.
6. If a number does not match, stop and reconcile the config; do not tune the
   rule, horizon, or filter.

Gate: the store is regenerated and the committed numbers are reproduced.

### Phase 4 — Verification and closure (0.5 day)

1. Run the simulation, cleanup-tool, trade, chart, and Rust tests; repo checks.
2. Review the diff for unrelated changes, secrets, and generated files.
3. Update the changelog (recovery + cleanup guard), the AI log, and the docs
   index.
4. Open the PR, run CI, merge, and use the new cleanup tool on this task
   (dogfood). The worktree and branches are removed only after the archive
   check passes.

## 8. Recovery config table

The exact values to commit under `tools/simulation/examples/runs/`:

| Run | Events | Window / max / min / seed | Oracle | Check |
|---|---|---|---|---|
| SIM-01 | human marks | meta label 2018–2019; 50-bar label | — | 2020–2022, 20 bars, 6 bps, EMA stack + ATR cell |
| SIM-02 | `level.fib_test` | 400 / 48 / 30 / 0 | 60, 2.0, 1.0, 1.5, 1.0, offset 2 | event 149, 20 bars, 6 bps, wick 0.30, event control |
| SIM-03 | SIM-02 marks | exit lab 2018–2019 | 60, 2.0, 1.0, partial 1.0 | event 149, 2020–2022, 60 bars, 6 bps |
| SIM-04 | `level.break` | 400 / 32 / 30 / 0 | 60, 2.0, 1.0, 1.5, 1.0, offset 1 | event 3, 20 bars, confirmation 1, close-back, opposite direction |
| SIM-05 | `signal.breakout_retest_hold` | 400 / 32 / 30 / 0 | 60, 2.0, 1.0, 1.5, 1.0, offset 1 | event 146, 20 bars, RVOL ≥ 1.0, direction only |
| SIM-06 | `signal.trend_continuation` | 400 / 32 / 30 / 0 | 60, 2.0, 1.0, 1.5, 1.0, offset 1 | event 16, 20 bars, EMA stack 1/−1, direction only |
| SIM-07 | `signal.compression_breakout` | 400 / 32 / 30 / 0 | 60, 2.0, 1.0, 1.5, 1.0, offset 1 | event 17, 20 bars, EMA stack 1/−1, direction only |
| SIM-08 | `divergence.rsi_regular_bullish/bearish` | 400 / 32 / 30 / 0 | 60, 2.0, 1.0, 1.5, 1.0, offset 1 | bull code 4 long control bars; bear code 5 short control bars; 20 bars, 6 bps |
| SIM-09 | `signal.climax_reversal`, `signal.absorption_rejection` | 400 / 32 / 30 / 0 | 60, 2.0, 1.0, 1.5, 1.0, offset 1 | codes 48/52, 20 bars, 6 bps, direction only, bar control; underpowered |
| SIM-10 | `signal.opening_range_sweep` | blocked | — | no session events; no run |

All runs use M1.3, registry `2026-11-10.2`, BTCUSDT 15m, label 2018–2019, check
2020–2022, studies `ema` + `rsi`, zoom 30 bars, theme light.

## 9. Verification

| Check | Command | Expected evidence |
|---|---|---|
| Simulation tests | `python3 -m unittest discover -s tools/simulation -p 'test_*.py'` | 45+ pass |
| Cleanup tests | `python3 -m unittest discover -s tools/worktree_cleanup -p 'test_*.py'` (or the chosen path) | pass |
| Trade/chart tests | `python3 -m unittest discover -s tools/trades ...`; `tools/charts` | 8 / 23 pass |
| Rust | `cargo test` | pass |
| Structure | `python3 tools/check_structure.py` | pass |
| Workflows | `python3 tools/check_workflows.py` | pass |
| Changelog | `python3 tools/check_changelog.py` | pass |
| Links | `python3 tools/check_links.py` | no broken links |
| Recovery | five `report model` runs + ten simulation runs + checks | store present; committed numbers reproduced |
| Archive | temp worktree with `data/local/file`; `check` fails, `archive` copies, `check` passes | manifest with sha256 |
| Refusal | temp worktree with untracked non-ignored file; `remove` refuses | non-zero exit; worktree preserved |
| No validation | read the store date ranges | 2023–2025 absent |

## 10. Risks and recovery

| Risk | Mitigation | Recovery |
|---|---|---|
| Regeneration numbers differ | Commit exact configs; compare to the reports; stop on mismatch | Reconcile the config, not the result |
| Canonical store is deleted manually | The external root is independent of the repo; archive/backup is documented | Regenerate from the committed configs |
| Cleanup tool deletes a branch or worktree by mistake | `--yes`, merged-branch check, check-before-remove, tests | Restore from the remote or re-create; the tool never force-removes |
| AGENTS.md change is not approved | The tool and docs still help; the gate is not enforced | Keep the worktree; ask |
| Disk space for 390 MB reports + charts | Check free space before recovery; clean disposable caches | Free space or use `--data-root` on a larger volume |
| Server restarts during recovery | Run in batches; checkpoint per run; rerun only the missing run | Resume from the last complete run |
| The task worktree itself loses data again | The new data-root default writes to main; the cleanup check archives anything local | Restore from the archive |

## 11. Stop conditions

Stop and report when:

- the human does not approve D1–D5 or the AGENTS.md change;
- the recovery config cannot reproduce the committed numbers;
- the cleanup tool cannot classify a file; preserve the worktree;
- a model, strategy, execution, or portfolio change would be required;
- validation or holdout data would be needed;
- disk space or the source data is unavailable;
- the cleanup would delete unmerged or uncommitted work.

## 12. Approval

This plan is `Proposed`. On approval:

1. Mark it `Approved` and commit it to
   `docs/execution-plans/2026-11-11/simulation-data-recovery.md`.
2. Create the worktree `quant-rule--simulation-data-recovery` and branch
   `fix/simulation-data-recovery`.
3. Execute Phase 0 and Phase 1; then the recovery and the cleanup guard.
4. Use the new cleanup tool on the recovery task itself before reporting
   completion.

## 13. Checklist

| # | Phase | Checklist item | Evidence | Done |
|---|---|---|---|---|
| 0.1 | 0 | Audit finding recorded | `docs/audits/2026-11-11/simulation-data-loss-audit.md` | ☑ |
| 0.2 | 0 | Plan approved and committed | `docs/execution-plans/2026-11-11/simulation-data-recovery.md` | ☑ |
| 0.3 | 0 | AI log started | `docs/ai-transparency-log/2026-11-11/simulation-data-recovery.md` | ☑ |
| 1.1 | 1 | Data-root resolver | linked worktree defaults to `/home/zantgo/quant-rule-data/simulation` | ☑ |
| 1.2 | 1 | M1.3 report configs committed | `tools/simulation/examples/m13-2018-2022/` | ☑ |
| 1.3 | 1 | Ten run/check configs committed | `tools/simulation/examples/runs/` | ☑ |
| 1.4 | 1 | Recovery commands | `reproduce.md` | ☑ |
| 1.5 | 1 | Data-root tests | temp main + linked worktree | ☑ |
| 2.1 | 2 | Cleanup `check` | clean pass, local data detected | ☑ |
| 2.2 | 2 | Cleanup `archive` | manifest with sha256 | ☑ |
| 2.3 | 2 | Cleanup `remove` | merged branch only; `--yes`; refuses unmerged | ☑ |
| 2.4 | 2 | AGENTS.md cleanup amendment | section 9 | ☑ |
| 2.5 | 2 | Data README and tool README | canonical root and archive | ☑ |
| 3.1 | 3 | Five M1.3 reports | canonical store | ☑ |
| 3.2 | 3 | SIM-01..SIM-09 runs | canonical store | ☑ |
| 3.3 | 3 | SIM-10 blocked record | canonical store | ☑ |
| 3.4 | 3 | Checks and comparison | `simulation-recovery.md` | ☑ |
| 4.1 | 4 | Tests and repo checks | table in section 9 | ☑ |
| 4.2 | 4 | Changelog and AI log | `CHANGELOG.md`, AI log | ☑ |
| 4.3 | 4 | PR, CI, merge | PR link | ☐ |
| 4.4 | 4 | Cleanup with the new tool | archive manifest; worktree/branches removed | ☐ |
