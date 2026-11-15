# AI transparency log — DIST-200 desktop profile v1.1 plan

- Date: 2026-11-15 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: write the approved execution plan for the v1.1 desktop profile (#692,
  DIST-200), decompose it into the #693/#694/#695 phases, and surface the
  architectural decisions and the repository-dialect risk before implementation.
- Boundary: planning only. No Rust, Cargo, application, Dockerfile, Compose,
  workflow, issue, tag, or existing architecture document was edited. The only
  files written are this log and
  `docs/execution-plans/2026-11-15/692-desktop-v1.1.md`. No code was built or
  run beyond the documentation checkers.
- Environment: worktree `/home/zantgo/Proyectos/quant-rule--692-plan`, branch
  `arch/692-desktop-v1.1`, base `6a714ba3` (v1.0.0 released).

## What was read

- `AGENTS.md` sections 2, 3, 4, 7, 8, 9, 11, 12, 13, and 14.
- Issues #692–#695 as filed, plus ADR-12
  `docs/adr/2026-11-12/12-desktop-distribution.md`.
- `docs/architecture/services.md` (sections 1, 2, 7), `deployment.md`,
  `control-plane.md`, `http-api.md`, and the v1.0.0 release note.
- The adapter traits in `crates/quantrule-runtime/src/adapters/mod.rs`, the
  Postgres and RabbitMQ and S3 adapters, the migration planner and runner, and
  `Config`.
- The six Postgres repositories in the four DB-owning services and their 13
  migration files; the simulation `FOR UPDATE` path; the evidence sealed-row
  trigger; the SQLite research memory in `crates/quantrule-app/src/memory`.
- The Tauri shell (`apps/desktop-shell`), its boundary and smoke tests, the
  release workflow desktop job, and `tools/verify.sh`.

## Findings that shaped the plan

- The `Database` trait is only `backend()` and `ping()`. The six repositories
  take `PgPool` directly, so implementing `Database` for SQLite would change no
  query. The domain repository trait — not the database trait — is the seam.
- Four services own a schema; `model` and `backtest` own no schema. The SQLite
  phase is therefore smaller than "all six services" but still four services,
  six repositories, about 55 statements, 13 migrations, 11 JSON columns, and
  five logical schemas.
- The SQL is not portable by syntax alone: `jsonb`, `timestamptz`, `to_char`,
  `now()`, `FOR UPDATE`, `jsonb_array_length`, a PL/pgSQL trigger, advisory
  locks, and schema-qualified DDL all differ. A dialect abstraction would have to
  model the database, so the plan recommends per-dialect repositories behind the
  existing traits with a shared conformance suite.
- `sqlx` SQLite cannot share this workspace with the existing `rusqlite` 0.42.
  `sqlx-sqlite` 0.9 requires `libsqlite3-sys >=0.30.1, <0.38`, while `rusqlite`
  0.42 links 0.40, and Cargo permits only one package for the native `sqlite3`
  link. The plan therefore uses `rusqlite` through `spawn_blocking`, not a second
  SQLite library. This correction was found while checking the dependency graph,
  after the first draft recommended `sqlx` SQLite.
- `image_digest` is an input to run identity. A desktop package has no image, so
  the parity harness must pass the same explicit digest to both profiles; a real
  desktop run records its own bundle digest and may have a different run id with
  the same canonical digest.
- The composed tier (`tools/test_tiers.sh composed`) is not wired (#643/#644), so
  the parity suite is release-tier, not a fast gate test.
- The shell currently has no IPC command, no plugin, and no local data read. The
  plan keeps that boundary by spawning one supervisor from the Rust entry and
  bundling the binaries as resources, rather than adding `tauri-plugin-shell` and
  an IPC surface.
- A supervisor can avoid PID hunting and platform process APIs by owning a
  control socket that every child watches; a dead supervisor closes it and the
  children exit. The residual is a wedged child, which is recorded.

## Rejected options

- A widened `Database` trait or an `AnyPool`: it turns every typed query into a
  dynamic one and reimplements a driver. The trait stays narrow.
- A shared query-builder or dialect layer as the primary implementation: the
  forced differences are structural, not lexical, and the layer would leak into
  every statement. Only table-name and JSON/time helpers are shared.
- `sqlx` SQLite for the services: its `libsqlite3-sys` range (below 0.38) cannot
  coexist with `rusqlite` 0.42's 0.40 under Cargo's one-package-per-`links`
  rule. The plan uses `rusqlite` behind `spawn_blocking` instead; the blocking
  wrapper is the P1a spike, not an assumed success.
- Downgrading `rusqlite` to fit `sqlx`: it would change the application memory's
  dependency and tests for no product reason, and it still leaves two database
  APIs in the services.
- Mechanically translating the 13 PostgreSQL migrations: historical DDL uses
  `DROP CONSTRAINT`, PL/pgSQL, and `jsonb`, and the desktop database starts
  empty. The plan uses one logical baseline per service plus an inventory test.
- A `tauri-plugin-shell` sidecar API with IPC commands: it changes the shell's
  boundary and exposes a command surface. One Rust spawn of a supervisor keeps
  the shell presentation-only.
- PID-file hunting for orphan recovery: it needs platform process APIs or a new
  dependency. The control-socket watchdog uses the standard library only.
- Deleting or rewriting failing tests to fit the desktop profile: not proposed.
- Creating sub-issues for #693's five PRs: the plan records the split, and
  creating sub-issues needs a human decision.

## Verification

| Command | Observed |
|---|---|
| `python3 tools/check_execution_plans.py` | `execution plans ok: current plans carry a claim type and verification table` |
| `python3 tools/check_doc_length.py` | `document length ok: 34 file(s) within cap` |
| `python3 tools/check_links.py` | `checked 953 Markdown files; no broken relative links` |
| `python3 tools/check_terms.py` | `checked 594 file(s); no banned terms` |

The implementation rows in the plan are `Pending` by design. This PR changes no
behavior, and no test result is claimed for code that does not exist yet. The
plan is approved for the v1.1 programme; its technical recommendations are
reviewable decisions, not implemented work.

## Learning

The desktop profile is mostly a persistence-and-packaging programme, not an
engine programme. The risk sits in one place: the repositories are written
against Postgres, and the `Database` trait gives a false impression that the
adapter boundary is already complete. Writing that down before implementation is
the main value of this plan, together with the multi-PR disclosure for #693 and
the parity oracle's explicit exclusion of timestamps and provenance. The SQLite
library check is the concrete example of why the plan is worth reviewing before
code: the obvious async choice (`sqlx`) does not link beside the existing
`rusqlite`, so the implementation must use the dependency the workspace already
has and measure the blocking wrapper first.
