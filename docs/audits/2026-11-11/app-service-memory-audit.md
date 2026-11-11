# Application Service and Research Memory Audit — v0.9.0

- Date: 2026-11-11 (UTC)
- Audit ID: `AUDIT-2026-11-11-app-service-memory`
- Status: Published; findings in `Open`
- Baseline: `main` at `8c4f2ca` (2026-11-11, PR #624 merged)
- Scope: the v0.9.0 application service (`crates/quantrule-app`), the local
  research memory (`crates/quantrule-app/src/memory`), the desktop command and
  memory boundary (`apps/desktop`), the records that describe them, and the test
  gate used to verify them.
- Workflow: [`audit-workflow.md`](../../workflows/audit-workflow.md)
- Claim type: program audit; no model, strategy, edge, or profitability claim.
- Human decision (2026-11-11): the audit is accepted and the desktop UI is a
  first-class surface. The GUI and the shell are one program with feature parity
  and one shared source; neither duplicates the other. The shell is primarily
  for agents. This resolves AUD-10's surface question; the remaining parity gap
  is recorded in the [native-window audit](native-tauri-window-audit.md).

## Summary

| Severity | Count |
|---|---:|
| Critical | 0 |
| Major | 3 |
| Minor | 6 |
| Info | 0 |

The application service and research memory are deterministic in the tested
paths and the v0.9.0 gate passes on a refreshed build. The audit found no model
or strategy behavior change, no signal-path defect, and no non-deterministic
memory output. It did find three data-integrity/reproducibility defects and six
smaller defects:

1. the memory adopts a SQLite database with `user_version = 0` and no
   conflicting table names, so opening or restoring a foreign database silently
   modifies it and reports an empty memory as a successful restore;
2. the workspace root is stored as text rather than as a canonical directory,
   so one workspace can be recorded under several rows;
3. the shared Cargo target directory can satisfy the test gate with a test
   binary compiled in another worktree, so `cargo test --workspace` is not
   reproducible from the documented command without a clean step.

The remaining findings are a workspace label overwritten by the last run name,
an import path that accepts records the store rejects, an update path that can
move `updated_at` before `created_at`, a zero-duration run record written by the
desktop UI, workspace-wide notes shown under a selected run, and post-release
documentation/status drift.

The audit is read-only. No code, test, registry, config, issue, or pull request
was changed. The behaviour probes are in a scratch crate outside the repository;
the Planner must turn each confirmed finding into a repository regression test.

## Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| AUD-2026-11-11-02 | Major | store/integrity | The memory adopts a SQLite database with `user_version = 0` and no conflicting table names; `restore` reports success with an empty memory when handed a foreign database | `crates/quantrule-app/src/memory/schema.rs:135-149`; `crates/quantrule-app/src/memory/store.rs:38-54`; `crates/quantrule-app/src/memory/transfer.rs:376-416`; scratch probe `opening_a_foreign_sqlite_database_adopts_it`, `restore_accepts_a_foreign_database_as_an_empty_memory` | A foreign SQLite file is modified (memory tables added, `user_version` set to 1); a wrong restore file is reported as restored but contains no records; the other application's migration state can be corrupted | A. Require an empty database or the expected `meta` schema before migrating, and validate the backup before copying; B. require a sidecar manifest or known extension; C. require explicit adoption and document it | A | A foreign database is refused and left unchanged; `restore` does not create the target; a valid backup still restores | Open | — |
| AUD-2026-11-11-03 | Major | app/identity | The workspace root is stored as the supplied path string, not a canonical directory; the same workspace can become several rows | `crates/quantrule-app/src/workspace.rs:42-53`; `crates/quantrule-app/src/service.rs:171-176`; `crates/quantrule-app/src/memory/schema.rs:22-27`; `crates/quantrule-app/src/memory/store.rs:105-114`; `crates/quantrule-app/src/config/run.rs:89-90`; scratch probe `the_same_workspace_root_can_be_recorded_twice` | Runs, datasets, configs, and notes split across duplicate workspaces; the UI shows duplicate roots and a run appears missing under the other form | A. Canonicalize the workspace root and run config path at open/record time; B. add a canonical root column with a unique key; C. normalize in every adapter | A | `/repo` and `/repo/.` record one workspace; a relative and absolute config path record one config; a regression test | Open | — |
| AUD-2026-11-11-04 | Major | tests/tooling | The shared Cargo target directory can reuse a test binary compiled in another worktree; the documented gate fails on a correct tree | `docs/execution-plans/2026-11-11/608-local-research-memory.md:90-91`; `crates/quantrule-model/tests/boundaries.rs:11-15`; observed `cargo test --workspace --no-fail-fast` failure reading `/home/zantgo/.local/share/opencode/worktree/bd3b02/readme-refresh/crates/...`; pass after `cargo clean -p quantrule-model` | The gate can fail or pass based on cache provenance rather than the current tree; release evidence is not reproducible from the shared target without a clean step | A. Use a target directory inside each worktree and stop sharing the main target; B. keep sharing but require a clean step before the gate; C. remove compile-time absolute paths from the boundary tests and add a clean step | A (C as defense) | A fresh worktree with its own target passes without a clean; the shared target fails before the fix and passes after the chosen fix | Open | `docs/reports/2026-11-11/implementation-verification.md:32-35` |
| AUD-2026-11-11-05 | Minor | app/store | The workspace name is the last recorded run name and is overwritten on every run | `crates/quantrule-app/src/service.rs:172-176`; `crates/quantrule-app/src/memory/store.rs:109-110`; scratch probe `the_workspace_name_is_the_last_recorded_run_name` | The UI workspace list renames the workspace after each run; runs in one root cannot be grouped under a stable workspace label | A. Derive the name from the root directory or accept an explicit workspace name; B. display the root as the primary label; C. add an editable display name | A | Two runs in one root keep the root-derived name; a regression test | Open | — |
| AUD-2026-11-11-06 | Minor | store/import | Import accepts records the store rejects: non-numeric integer text, `finished_at < started_at`, a `null` metric with a value, and non-positive metric IDs | `crates/quantrule-app/src/memory/transfer.rs:245-301`; `crates/quantrule-app/src/memory/store.rs:215-221,718-733`; scratch probes `import_accepts_an_integer_metric_that_cannot_be_read`, `import_accepts_inverted_timestamps_and_null_metric_values` | A malformed export can create a memory whose metrics cannot be read and whose logical export fails; the target is left in a state the app cannot display | A. Share one record validator between store and import and reject each case; B. validate after import and roll back; C. document import as trusted-only | A | Each invalid document is rejected and the target stays empty; a valid export round-trips | Open | — |
| AUD-2026-11-11-07 | Minor | store | `update_note` can set `updated_at` before `created_at`, although `insert_note` enforces the opposite | `crates/quantrule-app/src/memory/store.rs:342-346,391-398`; scratch probe `update_note_can_put_updated_at_before_created_at` | The memory can hold an impossible note timestamp; an export/import reproduces it; time-ordered notes are misleading | A. Validate in `update_note`; B. clamp; C. document the backward move | A | An update before `created_at` is rejected; equal and later values pass | Open | — |
| AUD-2026-11-11-08 | Minor | desktop/records | The desktop records every run with `started_at == finished_at == now` | `apps/desktop/src/lib/MemoryPanel.svelte:101-108`; `apps/desktop/src/lib/RunPanel.svelte:14-26` | Every recorded run has zero duration; run timestamps cannot compare runs or measure execution time | A. Track the model-run start/finish and pass them; B. pass `finished_at: null` when unknown; C. add a duration input | A (or B) | A UI test shows a non-zero duration or a null finish, never equal timestamps for an unknown duration | Open | — |
| AUD-2026-11-11-09 | Minor | desktop/evidence | Notes from every run in the workspace are shown under the selected run's evidence | `apps/desktop/src/lib/MemoryPanel.svelte:75-94,256-295`; `apps/desktop/src-tauri/src/commands.rs:365-370`; `crates/quantrule-app/src/memory/store.rs:634-642` | A note can be attributed to the wrong run; workspace-only notes appear as run evidence | A. Add a run-scoped notes view/query; B. label and separate workspace notes; C. filter `note.run_id` in the UI | A or B | A UI test with notes on two runs shows only the selected run's notes under its evidence; workspace notes are separate | Open | — |
| AUD-2026-11-11-10 | Minor | docs/records | Post-release records still describe the pre-desktop surface and leave shipped records in earlier statuses | `AGENTS.md:7,28,47`; `docs/architecture/README.md:38,52`; `docs/architecture/command-surface.md:13`; `docs/README.md:219`; `crates/quantrule-app/src/lib.rs:10-11`; `docs/execution-plans/2026-11-11/608-local-research-memory.md:6`; `docs/config/2026-11-11/run-v3-and-portfolio-constraints.md:4` | Agents and readers believe the memory is absent, the desktop UI is unbuilt, and every capability must be typeable; the memory is UI-only and the plan/contract statuses misstate what shipped | A. Recognize the desktop UI as a first-class surface, update the records, and mark shipped plans/contracts `Verified`/`Current`; B. keep the shell-only rule and add a `memory` shell command; C. update the records now and schedule the shell command separately | A (decided 2026-11-11) | No stale phrases; plan 608 and run v3 statuses match the workflow; checks pass | Open | [`v0.9.0`](../../releases/2026-11-11/v0.9.0.md); [`release readiness`](../../reports/2026-11-11/release-readiness.md) |

## Detail

### AUD-2026-11-11-02 — the memory adopts a foreign SQLite database

- **Expected invariant:** opening a memory must either create a new database or
  open one whose `meta.schema` is `quantrule.research-memory/v1`; it must not
  modify an unrelated SQLite file. A restore must verify that the backup is a
  memory before copying or reporting success (`docs/config/2026-11-11/research-memory-v1.md:10-26,41-53`;
  `schema.rs:3-5` says a newer version is refused rather than silently modified).
- **Observed:** `schema::apply` enables foreign keys, reads `PRAGMA user_version`,
  and runs `MIGRATION_V1` whenever the version is 0, without first checking that
  the file is empty or carries the memory schema; a database with no conflicting
  table names is adopted (`schema.rs:135-149`). `ResearchMemory::open` calls it
  (`store.rs:38-54`). `restore` copies the backup to the target and then calls
  `ResearchMemory::open` (`transfer.rs:376-416`). A scratch probe created a
  SQLite database with one unrelated table and `user_version = 0`:
  - `ResearchMemory::open` returned `Ok`, `is_empty()` was true, the database
    then contained `meta` and `workspaces`, and `user_version` was 1; the
    unrelated table and row were still present.
  - `restore` of that file returned `Ok`, created the target, reported an empty
    memory, and stamped the same version.
- **Root cause:** the migration treats `user_version = 0` as proof of a fresh
  database. SQLite uses 0 as the default for every database, so the version is
  not evidence of identity. The `meta` table is the identity, but it is created
  by the migration, so it cannot be checked first.
- **Impact:** pointing the app at another SQLite file, or restoring the wrong
  backup, adds QuantRule tables and changes the file's `user_version`. Other
  applications that use `user_version` can skip their own migrations. The
  restore reports success while restoring nothing, and the UI notice
  (`MemoryPanel.svelte:164-176`) says "Restored to ...".
- **Affected contracts/docs:** `research-memory-v1.md` §Rules and §Backup and
  restore; `store.rs` open docs; `schema.rs` module docs; the desktop settings
  text "The app creates it when you open the memory".
- **Dependencies:** a new error variant or reuse of `UnsupportedSchema`; the
  restore path must read the backup before copying.
- **Effort:** S–M.
- **Options:**
  - A. Before migrating, require the database to be empty of user tables, or
    already carry the expected `meta` schema. For restore, open the backup
    read-only (or with `SQLITE_OPEN_READ_ONLY`) and require `meta` before
    copying. Refuse and leave the file unchanged otherwise.
  - B. Add a sidecar manifest or require the `.quantrule.db` extension; refuse
    unknown files. This is weaker because a rename defeats it.
  - C. Keep the migration but require an explicit `--adopt` confirmation and
    document that it modifies the file. This accepts the corruption risk.
- **Recommendation:** A. It uses the stored identity that already exists, keeps
  fresh database creation working, and makes restore fail loudly.
- **Verification:** a regression test creates a database with an unrelated table
  and `user_version = 0`; `ResearchMemory::open` returns an error and the file's
  bytes and `user_version` are unchanged. `restore` with that file returns an
  error and no target exists. A valid `backup`/`restore` still round-trips.
- **Links:** — (the Planner assigns the fix issue).

### AUD-2026-11-11-03 — the workspace root is not canonical

- **Expected invariant:** `workspaces.root` is unique and identifies one local
  directory; the application service owns path resolution
  (`research-memory-v1.md:24`; `workspace.rs:1-16` "one directory").
- **Observed:** `Workspace::open` stores `root.to_path_buf()` without
  `fs::canonicalize` (`workspace.rs:42-53`). `AppService::record_run` passes
  `self.workspace.root().to_string_lossy()` to `upsert_workspace`
  (`service.rs:171-176`). The unique constraint is on the text `root`
  (`schema.rs:22-27`; `store.rs:105-114`). A scratch probe opened the repository as
  `/home/zantgo/Proyectos/quant-rule` and as
  `/home/zantgo/Proyectos/quant-rule/.`, recorded the same fixture config under
  each, and found two workspace rows. `RunConfig::load` also stores
  `paths.run` as the supplied path (`config/run.rs:89-90`), so the configs
  table has the same spelling sensitivity.
- **Root cause:** the service canonicalizes a dataset in `Dataset::open`
  (`dataset.rs:36`) but not the workspace root or the run config path. The
  memory's uniqueness rule therefore protects the string, not the directory.
- **Impact:** a normal path variation splits a research history. The UI's
  workspace list shows two roots; a run recorded under one is absent under the
  other. Notes are workspace-scoped, so they split too. The data is not lost,
  but the workspace as an identity is unreliable.
- **Affected contracts/docs:** `research-memory-v1.md` Identity/Order;
  `workspace.rs` docs; `service.rs` `record_run` docs; desktop settings.
- **Dependencies:** whether the UI should display the canonical path or the
  user-entered path. A display field can preserve the entered form.
- **Effort:** S.
- **Options:**
  - A. Canonicalize the workspace root and run config path when opening or
    recording, and store the canonical path as the key. Keep a display path if
    the UI must show the entered form.
  - B. Add a `canonical_root` column with a unique constraint and keep `root`
    for display. This preserves the user's spelling but requires a migration.
  - C. Normalize in each adapter (shell, desktop, tests). The application
    service is the owner, so this leaves the bug for the next adapter.
- **Recommendation:** A. It is small, uses the existing filesystem ownership,
  and makes the memory key match the dataset key.
- **Verification:** opening `/repo` and `/repo/.` and recording the same config
  produces one workspace and one config row. A relative config path and its
  absolute form produce one config row. The test fails before the fix.
- **Links:** —.

### AUD-2026-11-11-04 — the shared target directory makes the gate non-reproducible

- **Expected invariant:** `cargo test --workspace` on the merged tree proves the
  current tree and does not depend on a previous checkout
  (`AGENTS.md` §4 "Isolation"; §12 reproducibility).
- **Observed:** the memory plan's verification table says commands run with
  `CARGO_TARGET_DIR=/home/zantgo/Proyectos/quant-rule/target`
  (`docs/execution-plans/2026-11-11/608-local-research-memory.md:90-91`), and
  the release evidence claims 1,476 passed
  (`docs/reports/2026-11-11/implementation-verification.md:32-35`). On the audit
  baseline, `cargo test --workspace --no-fail-fast` failed in
  `crates/quantrule-model/tests/boundaries.rs` because the test read
  `/home/zantgo/.local/share/opencode/worktree/bd3b02/readme-refresh/crates/quantrule-model/src/run.rs`.
  The file does not exist; the path is the compile-time
  `env!("CARGO_MANIFEST_DIR")` from the removed `readme-refresh` worktree
  (`boundaries.rs:11-15`). Cargo had reused that test binary because the source
  content was identical in both worktrees.
- **Observed resolution:** `cargo clean -p quantrule-model` removed the stale
  package artifacts; `cargo test -p quantrule-model --test boundaries` then
  passed, and a full `cargo test --workspace --no-fail-fast` passed with 1,476
  passed, 0 failed, 8 ignored. The source was correct; the gate was not
  reproducible from the shared target.
- **Root cause:** Cargo fingerprints source content, not the absolute manifest
  path. A shared target directory across worktrees can serve a test binary whose
  compile-time paths point at another worktree. The two model boundary tests
  embed those paths.
- **Impact:** a correct tree can fail the documented gate, and a stale binary
  can make a tree look tested when the current checkout was not the one that
  compiled the binary. The release report's number is reproducible, but only
  after a clean step that the plan does not name.
- **Affected contracts/docs:** `AGENTS.md` §4 and §12; the execution-plan
  template's verification commands; the memory plan's `CARGO_TARGET_DIR` line;
  the release-readiness evidence.
- **Dependencies:** build-time cost of per-worktree targets; CI configuration.
- **Effort:** XS–S.
- **Options:**
  - A. Use a target directory inside each worktree (or a per-task
    `CARGO_TARGET_DIR`) and stop sharing the main target. Add the rule to the
    plan template and the gate documentation.
  - B. Keep sharing but require `cargo clean` (or `cargo clean -p` for affected
    packages) before the gate, and record the clean step in the evidence.
  - C. Remove compile-time absolute paths from the boundary tests (for example
    read through a path derived from `file!()` or a runtime workspace root) and
    add a clean step. This makes the tests robust but does not fix the cache
    identity for other tests.
- **Recommendation:** A, with C as a defense for the two source-reading tests.
  The worktree isolation rule exists to prevent exactly this cross-contamination.
- **Verification:** a fresh worktree with its own target passes
  `cargo test --workspace` without a clean. The shared target with a removed
  worktree's binary fails before the fix and passes after the chosen fix.
- **Links:** —.

### AUD-2026-11-11-05 — the workspace name is the last run name

- **Expected invariant:** a workspace name labels the workspace and stays stable
  while the root is unchanged; a run name labels the run.
- **Observed:** `record_run` passes `summary.name` (the run name) as
  `NewWorkspace.name` (`service.rs:172-176`). `upsert_workspace` updates the
  name on conflict (`store.rs:109-110`). A scratch probe created one root with
  two run configs, `run-a` and `run-b`; after recording both, the single
  workspace row was named `run-b`.
- **Root cause:** the service reused the run summary's `name` for the workspace
  record instead of a workspace-level name.
- **Impact:** the desktop workspace list renames the workspace after every
  recorded run. A researcher cannot give a workspace a stable label, and two
  runs in one root cannot be grouped under a meaningful workspace name.
- **Affected contracts/docs:** `research-memory-v1.md` Tables; `service.rs`
  `record_run` docs; desktop `MemoryPanel` workspace list.
- **Dependencies:** whether a workspace name is user-editable. If not, the root
  directory name is the natural default.
- **Effort:** XS–S.
- **Options:**
  - A. Derive the name from the root directory (or accept an explicit
    workspace name in the service) and do not overwrite it on a run record.
  - B. Keep the run name but display the root as the primary label and document
    that the name is the last run.
  - C. Add an editable `display_name` field.
- **Recommendation:** A. The root is the identity; its directory name is the
  least surprising label.
- **Verification:** two runs in one root keep the root-derived name; a
  regression test records `run-a` then `run-b` and asserts the workspace name
  stays the root name.
- **Links:** —.

### AUD-2026-11-11-06 — import bypasses the store's record invariants

- **Expected invariant:** import accepts the document produced by export and
  rejects any record the store would reject; a rejected record leaves the
  target empty (`research-memory-v1.md:41-47`; `transfer.rs:157-167`).
- **Observed:** the import path checks shape and presence but not every value:
  - An integer metric's `text_value` is only checked for presence
    (`transfer.rs:267-301`), so `"not-an-integer"` imports. `metrics()` then
    fails when it parses the text (`store.rs:718-733`).
  - `finished_at < started_at` is accepted by the run insert
    (`transfer.rs:245-265`), although `insert_run` rejects it
    (`store.rs:215-221`).
  - A `null` metric with a `value` is accepted (`transfer.rs:282`); `metrics()`
    returns `Null` and silently discards the stored value (`store.rs:730`).
  - A metric `id` uses `integer`, not `positive_id` (`transfer.rs:293-294`),
    unlike the other records.
  - Note `updated_at < created_at` and dataset
    `first_open_time > last_open_time` are not checked, although the store
    checks them on insert.
- **Root cause:** the store validates in each `insert_*`/`upsert_*` method, and
  the import reimplements a partial validation instead of reusing it.
- **Impact:** a hand-edited or corrupted export can leave the target memory in a
  state the app cannot read or logically export. The UI shows an error when the
  affected run is selected. The file backup path still works, but the logical
  export does not.
- **Affected contracts/docs:** `research-memory-v1.md` §Export and import;
  `transfer.rs` import docs; the memory tests.
- **Dependencies:** none; the validators can be factored without a schema
  change.
- **Effort:** S.
- **Options:**
  - A. Factor one record validator used by both the store and import; parse
    integer text, check timestamps, reject null-with-value and non-positive
    IDs; add a fail-before test for each case.
  - B. Import first, then run `verify_integrity` plus a full read/export in the
    same transaction; roll back on any failure. This is simpler but produces
    poorer error messages and still needs the parsers.
  - C. Document import as trusted-only and require a clean export. This does not
    protect against corruption, which is a reason to keep a backup.
- **Recommendation:** A. The store already encodes the rules; the import should
  call the same code.
- **Verification:** each malformed document is rejected and the target remains
  empty; a clean export/import round-trip still produces byte-identical output.
- **Links:** —.

### AUD-2026-11-11-07 — update_note can move `updated_at` before `created_at`

- **Expected invariant:** a note's `updated_at` is not before `created_at`;
  `insert_note` enforces this (`store.rs:342-346`).
- **Observed:** `update_note` writes the supplied `updated_at` without the
  check (`store.rs:391-398`). A scratch probe inserted a note at `created_at =
  10, updated_at = 10`, called `update_note(id, "second", 5)`, and read back
  `created_at = 10, updated_at = 5`.
- **Impact:** the memory can hold an impossible note timestamp; a logical export
  reproduces it, and time-ordered research notes are misleading. No current UI
  calls `update_note`, so the impact is on the public service API and future
  editors.
- **Affected contracts/docs:** `research-memory-v1.md` Time; `store.rs`
  `update_note` docs.
- **Dependencies:** none.
- **Effort:** XS.
- **Options:**
  - A. Validate `updated_at >= created_at` in `update_note`, reading the
    existing `created_at`.
  - B. Clamp the timestamp to `created_at`.
  - C. Document that an update may move the timestamp backwards.
- **Recommendation:** A.
- **Verification:** an update before `created_at` is rejected; equal and later
  values pass; the existing insert test still passes.
- **Links:** —.

### AUD-2026-11-11-08 — the desktop records a zero-duration run

- **Expected invariant:** a run record's `started_at` and `finished_at` reflect
  the run, or `finished_at` is null when the duration is unknown; the caller
  supplies the timestamps (`research-memory-v1.md:20`).
- **Observed:** `MemoryPanel.svelte:101-108` computes `const now =
  Math.floor(Date.now() / 1000)` and passes `started_at: now, finished_at: now`
  to `record_run`. The run itself is executed in `RunPanel.svelte:14-26`, which
  returns only the model view and no timing. Every recorded run therefore has a
  zero duration.
- **Impact:** the memory's timestamps cannot compare runs or measure execution
  time. The desktop is the only surface that records runs, so the data is
  uniformly affected.
- **Affected contracts/docs:** `research-memory-v1.md` Time; the desktop
  memory panel; the run panel.
- **Dependencies:** whether the run record is meant to measure the model run or
  only to note the record time. If the latter, `finished_at` should be null.
- **Effort:** S.
- **Options:**
  - A. Track `started_at` before `run_model` and `finished_at` after it in
    `RunPanel`, pass them to `MemoryPanel`, and record them.
  - B. Pass `finished_at: null` when the duration is unknown; the schema
    already allows null.
  - C. Add a duration input.
- **Recommendation:** A; B is acceptable if timing is deliberately out of
  scope.
- **Verification:** a UI test records a run with a non-zero duration or a null
  finish; it never asserts two equal timestamps for an unknown duration.
- **Links:** —.

### AUD-2026-11-11-09 — workspace notes appear under a selected run

- **Expected invariant:** an evidence panel for run #N shows records linked to
  run #N, or labels workspace-wide records as workspace-wide.
- **Observed:** `MemoryPanel.svelte:75-94` loads `memory_metrics` and
  `memory_claims` for the run, but loads `memory_notes` with
  `workspace_id: selectedWorkspace`. The command
  (`commands.rs:365-370`) calls `memory.notes(workspace_id)`, which returns
  every note in the workspace (`store.rs:634-642`). The panel heading is
  "Evidence for run #selectedRun" (`MemoryPanel.svelte:256-258`), and the notes
  are rendered next to the run's claims (`291-295`).
- **Impact:** notes from other runs and workspace-only notes are presented as
  evidence for the selected run. A reader can attribute a note to the wrong run,
  which is exactly the kind of evidence error the memory exists to prevent.
- **Affected contracts/docs:** the desktop memory panel; `memory_notes` command
  docs; `notes.run_id` semantics in `research-memory-v1.md`.
- **Dependencies:** a run-scoped query or a UI filter; the store already has
  `run_id` on notes.
- **Effort:** S.
- **Options:**
  - A. Add a `memory_notes_for_run(run_id)` command/store query and use it for
    the evidence panel.
  - B. Keep the workspace query but label the section "Workspace notes" and
    separate it from the run's claims.
  - C. Filter `note.run_id == selectedRun` in the UI; this requires loading the
    workspace notes and is fine for a local dataset.
- **Recommendation:** A or B. The evidence panel should not mix scopes.
- **Verification:** a UI test with notes on two runs shows only the selected
  run's notes under its evidence; workspace-only notes are separately labelled.
- **Links:** —.

### AUD-2026-11-11-10 — post-release records still describe the pre-desktop surface

- **Expected invariant:** the living documentation describes the current
  release, and a shipped plan/contract carries a status consistent with the
  workflow (`AGENTS.md` §5 "No stale documentation"; the execution-plan
  lifecycle).
- **Observed:**
  - `AGENTS.md:7` says "The current release is an interactive command shell"
    and "The desktop UI is being built"; `AGENTS.md:28` makes the shell the
    "current user-facing surface" and says a capability that cannot be typed
    "is not a feature"; `AGENTS.md:47` says the app is called by the desktop UI
    "in Phase 3".
  - `docs/architecture/README.md:38` says "The shell is the current surface";
    line 52 says "the future desktop UI".
  - `docs/architecture/command-surface.md:13` says the desktop UI "is being
    built".
  - `docs/README.md:219` says "There is no database store in the current
    surface."
  - `crates/quantrule-app/src/lib.rs:10-11` says "A future local research
    memory ... is not part of this module yet", while `pub mod memory;` is at
    line 36 and `AppService::record_run` exists.
  - `docs/execution-plans/2026-11-11/608-local-research-memory.md:6` is
    `Status: In Review`, and V1–V10 are `Pass` with "Human review pending",
    although the memory shipped in v0.9.0.
  - `docs/config/2026-11-11/run-v3-and-portfolio-constraints.md:4` is
    `Status: Target contract for Phase 3`, although run v3 shipped in v0.9.0.
  - `CHANGELOG.md`'s 0.9.0 section says "future desktop UI". A released section
    is immutable, so this needs a dated addendum or a next-release correction.
- **Root cause:** the v0.9.0 release closed the code and release records but did
  not update the living architecture/agent documents, the memory module doc, or
  the shipped plan/contract statuses.
- **Impact:** agents following `AGENTS.md` will treat the desktop UI as future
  and the memory as absent. The product now has a capability (the research
  memory) that is only reachable through the desktop UI, while the normative
  rule says a capability that cannot be typed does not exist. A reader cannot
  tell whether the shell-only rule is still intended.
- **Affected contracts/docs:** `AGENTS.md` §1–2; `docs/architecture/README.md`;
  `docs/architecture/command-surface.md`; `docs/README.md`; the app module doc;
  plan 608; the run v3 contract; `CHANGELOG.md`.
- **Dependencies:** a human decision on the surface rule: is the desktop UI a
  first-class surface, or must the memory be reachable from the shell?
- **Effort:** S for the records; M if a shell `memory` command is chosen.
- **Options:**
  - A. Recognize the desktop UI as a first-class surface in `AGENTS.md` and the
    architecture docs, define discovery for both surfaces, update the module
    doc, mark plan 608 `Verified` if the human accepts the release evidence,
    mark run v3 `Current`, and add a changelog addendum for the "future desktop
    UI" phrase.
  - B. Keep the shell-only rule and add a `memory` shell command that exposes
    the memory operations; the docs stay shell-first and the desktop UI is
    documented as an additional surface.
  - C. Update the records now and record the shell command as a separate
    follow-up issue.
- **Human decision (2026-11-11):** Option A, with the stronger requirement that
  the GUI and the shell share one source and have feature parity; the shell is
  primarily for agents. The remaining work is not only a docs update: the
  pipeline use cases that live in the shell and the memory CRUD that lives only
  in the service must move behind the shared application service, and both
  adapters must expose them. The native-window audit records that gap.
- **Recommendation:** A. The v0.9.0 release note and README already present the
  desktop UI as shipped; the normative documents should match.
- **Verification:** a repository search finds no "future desktop UI", "no
  database store", or "not part of this module yet" in current docs; plan 608
  and run v3 statuses match the execution-plan/workflow vocabulary; the link,
  term, workflow, and execution-plan checks pass.
- **Links:** [`v0.9.0`](../../releases/2026-11-11/v0.9.0.md); [`release readiness`](../../reports/2026-11-11/release-readiness.md).

## Verification performed

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy -p quantrule-app --all-targets -- -D warnings` | no warnings |
| `cargo test -p quantrule-app` | 103 passed, 0 failed; 1 doctest passed |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p quantrule-app --no-deps` | pass |
| `cargo test --workspace --no-fail-fast` | first run failed in `quantrule-model` boundary tests with a path from the removed `readme-refresh` worktree; after `cargo clean -p quantrule-model`, 1,476 passed, 0 failed, 8 ignored |
| Scratch probe `opening_a_foreign_sqlite_database_adopts_it` | `ResearchMemory::open` accepted a database with an unrelated table, added `meta`/`workspaces`, and set `user_version = 1` |
| Scratch probe `restore_accepts_a_foreign_database_as_an_empty_memory` | `restore` returned `Ok`, created the target, and reported an empty memory at version 1 |
| Scratch probe `the_same_workspace_root_can_be_recorded_twice` | `/repo` and `/repo/.` produced two workspace rows |
| Scratch probe `the_workspace_name_is_the_last_recorded_run_name` | one workspace row was named `run-b` after recording `run-a` and `run-b` |
| Scratch probe `import_accepts_an_integer_metric_that_cannot_be_read` | import accepted `"not-an-integer"`; `metrics()` then failed |
| Scratch probe `import_accepts_inverted_timestamps_and_null_metric_values` | import accepted `finished_at = 0` with `started_at = 10` and a null metric with a value |
| Scratch probe `update_note_can_put_updated_at_before_created_at` | `created_at = 10, updated_at = 5` was stored |
| `python3 tools/check_links.py` | 839 Markdown files; no broken relative links |
| `python3 tools/check_terms.py` | 540 files; no banned terms |
| `python3 tools/check_structure.py` | pass |
| `python3 tools/check_file_size.py` | pass |
| `python3 tools/check_execution_plans.py` | pass |
| `python3 tools/check_workflows.py` | 17 workflows, 8 templates, 17 index rows |
| `python3 tools/check_changelog.py` | 5 releases, 90 entries |
| `python3 tools/check_releases.py` | 7 release notes, 4 changelog versions |
| `python3 tools/check_command_surface.py` | 15 implemented, 0 planned |
| `python3 tools/check_legacy.py` | 1,532 current files; pass |
| `python3 tools/check_claims.py` | pass |

The scratch probe lives outside the repository at
`/tmp/opencode/qr-audit-probe` and depends on `quantrule-app` by path. It is
evidence for the audit, not a committed test suite; the Planner must add the
regression tests named in each finding to the owning crate.

## Limitations and unexamined areas

- One sequential audit pass by one agent. The audit is not an independent
  review and does not claim to be one.
- The audit is a program audit. No model, strategy, execution, portfolio, or
  analytics result was evaluated, and no Quant Validator record is required;
  the memory is outside the signal path.
- The desktop UI was inspected read-only. The native Tauri window was not
  driven, the browser E2E with the fixture backend was not re-run, and the
  `.deb`/`.AppImage` bundles were not rebuilt.
- The `cargo test --workspace` failure is a cache-provenance failure, not a
  source failure. The audit did not determine whether the v0.9.0 release gate
  actually used a stale binary; it established that the documented shared
  target can produce that outcome and that a clean package rebuild passes.
- The foreign-database probe used a database with one unrelated table and
  `user_version = 0`. A database with a colliding table name or an existing
  `meta` table was not probed; the existing schema tests cover the newer-version
  refusal.
- The workspace-path probe used the repository itself and a trailing `/.`; other
  forms (`..`, symlinks, a relative path from a different working directory)
  were not exhaustively tested. The root cause (no canonicalization) covers
  them.
- The untracked `data/improvement/2026-11-08` through `2026-11-10` directories
  in the main worktree were not inspected; they are outside this audit.
- Native-window follow-up (2026-11-11): the native Tauri window was audited in
  [`native-tauri-window-audit.md`](native-tauri-window-audit.md). It confirms
  AUD-05 and AUD-08 in the native path, confirms the path-spelling issue in
  AUD-03 in the native memory record, and records the GUI/shell parity gap under
  the human decision above.
- No 2024 or 2025 market partition was accessed, and no run outcome, baseline,
  or edge claim is made.

## Recommended order

1. Fix AUD-02 (foreign-database adoption/restore) first: it can modify a file
   outside the workspace and reports a false successful restore.
2. Fix AUD-03 (canonical workspace root) before more runs are recorded.
3. Fix AUD-04 (target isolation) before the next gate or release-evidence claim.
4. Fix AUD-05, AUD-06, and AUD-07 on the memory semantics.
5. Fix AUD-08 and AUD-09 on the desktop evidence panel.
6. Settle AUD-10's surface rule with the human, then update the records and
   statuses.

## Links

- Audit workflow: [`audit-workflow.md`](../../workflows/audit-workflow.md)
- Native-window follow-up: [`native-tauri-window-audit.md`](native-tauri-window-audit.md)
- Memory contract: [`research-memory-v1.md`](../../config/2026-11-11/research-memory-v1.md)
- v0.9.0 release: [`v0.9.0.md`](../../releases/2026-11-11/v0.9.0.md)
- Release readiness: [`release-readiness.md`](../../reports/2026-11-11/release-readiness.md)
- Memory plan: [`608-local-research-memory.md`](../../execution-plans/2026-11-11/608-local-research-memory.md)
- Application-service plan: [`588-tauri-platform.md`](../../execution-plans/2026-11-11/588-tauri-platform.md)
