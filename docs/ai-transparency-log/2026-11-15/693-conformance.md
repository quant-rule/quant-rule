# AI transparency log — DIST-201 P1e SQLite conformance and desktop contract

- Date: 2026-11-15 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Execution Plan; issue #693 (DIST-201), claim type Behaviour Claim;
  the v1.1 plan (PR #810) section 7, P1e.
- Intent: close the SQLite phase with one cross-service guard, one inventory,
  and one written desktop-profile contract, so a prefix, a baseline, a ledger,
  or a store cannot drift away from the declared mapping without failing the
  gate.
- Boundary: `tools/check_schema_prefixes.py`,
  `tools/test_check_schema_prefixes.py`, the `tools/gate.sh` checker list, the
  `.github/workflows/ci.yml` checker step, `docs/architecture/desktop.md`, the
  index and deployment link in `docs/architecture/`, `CHANGELOG.md`, and this
  log. No service, runtime, migration, queue, desktop-shell, compose, Dockerfile,
  or Makefile change.
- Environment: worktree `/home/zantgo/Proyectos/quant-rule--693-conformance`,
  branch `feat/693-conformance`, base `dee568a7` (P1a-P1d merged on `main`).
  #694 merged to `main` while the PR was open; the branch merged `main` and the
  inventory was reconciled with the queue adapter before the final gate.

## What was read

- `AGENTS.md` sections 2, 3, 4, 7, 8, 9, 11, 12, 13, and 14.
- Issue #693 and the v1.1 plan's P1e row, its owned paths, and verification rows
  V15–V17.
- Every `crates/*/migrations/sqlite/*.sql` baseline and each service's
  `migrations.rs` list.
- The runtime `Schema`/`Dialect` mapping, the shared `schema_migrations` runner,
  the service SQLite stores, and the four profile branches in the service
  binaries.
- The gateway/data/evidence/simulation prefix tests that P1a-P1d added, so the
  shared guard extends them instead of contradicting them.
- `tools/check_file_size.py`, `tools/check_python.py`, `tools/check_structure.py`,
  their tests, and the checker list in `tools/gate.sh` and `ci.yml`.

## Implemented

- `tools/check_schema_prefixes.py` holds one inventory of six ledger rows —
  `gateway`, `catalog`, `runs`, `research` (both on the `runs` ledger),
  `simulation`, and `queue` — with the baseline, store module, tables, and
  binary each row owns. The `queue` row has no baseline and no binary because
  #694's adapter owns its two idempotent tables, so the row is a declared
  exception rather than an omission. `check(root)` returns violations for:
  - a baseline file that is missing, undeclared, or in a crate outside the
    inventory;
  - a `CREATE TABLE`, index, or trigger whose name is not `prefix__name` for an
    owned prefix, or a created table set that differs from the inventory;
  - `CREATE SCHEMA`, `jsonb`, `timestamptz`, or `FOR UPDATE` in executable SQL
    (comments are stripped, so the baselines may explain the Postgres spelling);
  - a store that names another service's prefix, omits a declared table, or
    names a table the baseline does not create;
  - a `migrations.rs` that does not list its baseline, or a service binary that
    does not branch on `DeploymentProfile::Desktop` with `.data_dir`,
    `QR_DATA_DIR`, `DATABASE_FILE`, `migrations::sqlite()`, and the expected
    ledger row.
- The guard strips `#[cfg(test)]` modules and comments before reading table
  names, so the existing negative tests may keep naming another service's prefix
  and the runtime's shared `Schema` mapping keeps naming all six.
- `tools/test_check_schema_prefixes.py` builds a complete valid tree from the
  inventory and seeds each rule: a wrong baseline prefix, an unprefixed table, a
  store that names another prefix, a store that drifts from the baseline, JSONB,
  timestamptz, `CREATE SCHEMA`, `FOR UPDATE`, a wrong ledger, a missing ledger,
  a missing baseline wiring, an undeclared baseline, a crate outside the
  inventory, a missing data-directory branch, a missing single-file open, a
  missing store, a queue baseline that would contradict the adapter-owned DDL,
  and a store-owned table that drifts. A comment that names a Postgres spelling
  and a test module that names another prefix must not fail.
- `docs/architecture/desktop.md` records the environment contract, the single
  SQLite file and its pragmas, the service/ledger/prefix inventory, the storage
  rules, the guard, start and stop, current state, and the parity gate. The
  architecture index gains the document and one glossary term; the deployment
  page's stale "the desktop shell runs nothing" sentence now points at the
  current contract and keeps its length cap.
- `tools/gate.sh` runs the checker; the CI workflow runs its test and then the
  checker in a dedicated step.

## Inventory result

| Ledger | Crate | Prefix | Tables | Baseline |
|---|---|---|---|---|
| `gateway` | `quantrule-gateway` | `gateway` | 1 | `0001_profiles.sql` |
| `catalog` | `quantrule-data-service` | `catalog` | 3 | `0001_catalog.sql` |
| `runs` | `quantrule-evidence-service` | `runs` | 4 | `0001_runs_research.sql` |
| `runs` | `quantrule-evidence-service` | `research` | 1 | `0001_runs_research.sql` |
| `simulation` | `quantrule-simulation-service` | `simulation` | 2 | `0001_simulation.sql` |
| `queue` | `quantrule-runtime` | `queue` | 2 | adapter-owned DDL |

The guard reports `6 ledger rows, 13 tables, 4 baselines`. The four database
services match their baselines, stores, and binary branches, and the runtime
queue matches its adapter-owned tables; no wrong prefix or foreign table
reference was found in the service sources.

## Verification

| Command | Observed |
|---|---|
| `python3 tools/check_schema_prefixes.py` | `schema prefixes ok: 6 ledger rows, 13 tables, 4 baselines` |
| `python3 tools/test_check_schema_prefixes.py` | `Ran 21 tests` … `OK` |
| `python3 -m unittest discover -s tools -p 'test_check_schema_prefixes.py'` | `Ran 21 tests` … `OK` (the CI form) |
| `tools/gate.sh` | `workspace gate passed with CARGO_TARGET_DIR=…/target/gate`: fmt, clippy, the workspace tests, rustdoc, and all 23 checkers passed |
| `python3 tools/check_links.py` | `checked 964 Markdown files; no broken relative links` |
| `python3 tools/check_doc_length.py` | `document length ok: 35 file(s) within cap` |
| `python3 tools/check_terms.py` | `checked 595 file(s); no banned terms` |
| `python3 tools/check_legacy.py` | `legacy scan ok: 1807 label files, 3232 current files` |
| Seeded violation | `sed` changed `gateway__profiles` to `catalog__profiles` in the gateway baseline: `baseline-prefix: … catalog__profiles is outside gateway` and `baseline-drift: … missing gateway__profiles; undeclared catalog__profiles`, exit 1. The file was restored; the checker then printed the pass line and `git diff` was empty |

## Learning and rejected alternatives

- **One inventory, in the checker.** A separate inventory file would be a second
  source of truth that a migration could forget. The declared rows live next to
  the rule that reads them, so an unlisted baseline, table, or store fails.
- **A Python guard, not a Rust integration test.** The property is cross-crate:
  no single crate can see every service's baseline, binary, and store. A Rust
  test would need a new shared test crate or a dependency in the wrong
  direction; the repository's existing cross-cutting checks are Python.
- **Test modules are excluded on purpose.** The per-service tests already assert
  that another prefix is refused, and those assertions must name it. The guard
  documents that a test may name another prefix while production source may not.
- **The queue row has no baseline on purpose.** #694's adapter creates
  `queue__messages` and `queue__dead_letter` in one immediate transaction and
  takes no service ledger, because the queue is runtime infrastructure rather
  than a service schema. The row records that exception and checks the store and
  both tables; inventing a ledger for it would misdescribe the code.
- **CI caught the merge that this worktree did not.** #694 merged to `main`
  while the PR was open, so the `verify` job ran on the merge ref and failed on
  the new `inprocess.rs` while the local tree still passed. The branch then
  merged `main`, and the inventory was reconciled before the final gate. The
  local gate cannot see a later `main`; the PR merge ref can.
- **No edit to `apps/desktop-shell/README.md`.** Its status line still says the
  embedded backends are unmerged, which is now stale for SQLite. #695 owns that
  file; the drift is reported rather than fixed here, and `desktop.md` records
  the current state.
- **The Postgres inventory is not re-derived here.** V16's composed-tier half
  compares the Postgres catalogs; this change covers the SQLite side and the
  logical mapping in the fast tier.

## Remaining work

- P2c must wire `InProcessQueue` into the service binaries; today it is
  implemented and tested but not selected by a service.
- #695 owns the supervisor, installer, and parity documentation. The parity
  harness `tools/parity_desktop.sh` does not exist yet, so no digest parity is
  claimed.
