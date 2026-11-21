# Execution Plan — Desktop profile v1.1 (DIST-200, #692)

- Status: Approved
- Approved: 2026-11-15
- Claim type: ADR (Architecture Decision)
- Owner: Human
- Workflow: [Execution Plan](../../workflows/execution-plan.md) + [Requirements & Design](../../workflows/requirements-design.md)
- Records: DIST-200 (#692), DIST-201 (#693), DIST-202 (#694), DIST-203 (#695); [ADR-2026-11-12-12](../../adr/2026-11-12/12-desktop-distribution.md)
- Branch: `arch/692-desktop-v1.1`

## 1. Intent

QuantRule v1.0.0 is a distributed platform that installs with Docker. That is
correct for a developer and a reviewer, and wrong for a casual user who wants a
double-click install. The desktop profile must make the platform self-contained
without Docker while keeping six service processes, the same HTTP contracts,
and the same deterministic output.

The profile is a deployment shape, not a second architecture. The six services
keep their boundaries and their schemas; only the three backend adapters change:
SQLite instead of Postgres, a filesystem content-addressed store instead of S3,
and a durable in-process queue instead of RabbitMQ. A supervisor starts the six
sidecars from one installer, and a parity suite proves that the same fixture,
configuration, and image identity produce the same canonical digest as the
server profile.

## 2. Current state

| Area | Observed | Evidence |
|---|---|---|
| Release | v1.0.0 released 2026-11-15 from `6a714ba3`; v1.1 is the next programme | [release note](../../releases/2026-11-15/v0.9.0.md) |
| Topology | Six services, six binaries, one image; nine containers in Compose | [services](../../architecture/services.md), `compose.yaml` |
| Adapter traits | `Database`, `ObjectStore`, `Queue`, `QueueConsumer`; only Postgres, S3, RabbitMQ are implemented | [adapters](../../../crates/quantrule-runtime/src/adapters/mod.rs) |
| Database trait | `Database` is a liveness probe plus a backend name; it exposes no query | `crates/quantrule-runtime/src/adapters/mod.rs` |
| Repositories | Six repository types in four services take `PgPool` directly and use `sqlx` Postgres | `crates/quantrule-{data,evidence,simulation,gateway}-service/src/postgres.rs` |
| SQL surface | About 55 statement sites, 13 migration files, five schemas, 11 `jsonb` uses, 13 `timestamptz` uses, `to_char`, `now()`, `FOR UPDATE`, `ON CONFLICT`, a PL/pgSQL trigger, and advisory locks | migration directories and the Postgres repositories |
| Schema owners | `gateway`, `data` (catalog), `evidence` (runs and research), `simulation`; `model` and `backtest` own no schema | [services](../../architecture/services.md) section 1 |
| SQLite precedent | The application memory was SQLite through bundled `rusqlite`, with `STRICT` tables, `PRAGMA user_version`, triggers, and deterministic ordering; the module was removed on 2026-11-16 (`AUD-2026-11-16-13`) | `crates/quantrule-app/src/memory/mod.rs` (historical) |
| Backend selection | No profile selector; every service unconditionally connects Postgres, S3, and RabbitMQ | `crates/quantrule-runtime/src/config.rs`, service `main.rs` files |
| Desktop shell | Tauri 2 window over the gateway; no IPC command, no plugin, no local file read, no engine; AppImage and NSIS matrix exists; artifacts unsigned | [retired](../../adr/2026-11-21/992-retire-desktop-distribution.md) |
| Test tiers | Fast gate wired; `tools/test_tiers.sh composed` exits 2 because the composed suite is not wired (#643/#644) | `tools/test_tiers.sh` |
| Interpreter boundary | The image has no Python and no Bun runtime; source and image checks exist | `tools/check_python.py`, `docker/check-no-python.sh` |

The gap is precise. `ObjectStore` and `Queue` already have a second in-memory
fake, so a filesystem store and a durable queue can be added behind the traits.
The database is different: the domain repositories are the real seam, and they
are written against one dialect. Adding a `SqliteDatabase` that implements the
ping-only `Database` trait would change nothing.

## 3. Scope and non-goals

**In scope**

- A desktop profile selected explicitly at startup; `server` remains the default.
- SQLite implementations of the four DB-owning services' repositories, with a
  prefixed schema in one database file and per-service forward-only migrations.
- A filesystem content-addressed object store with atomic publish.
- A durable in-process queue persisted in SQLite, with retry and dead-letter.
- A desktop supervisor that allocates ports, starts six sidecars, restarts them,
  and stops them with the shell; orphan recovery.
- Linux AppImage and Windows NSIS installers built from the same commit and
  carrying the shell, the supervisor, and the six service binaries.
- A release-tier parity suite that compares the desktop profile to the server
  profile on the same fixture, configuration, and image identity.
- Tests, documentation, changelog, AI transparency record, and release note text.

**Non-goals**

- No in-process engine, no single binary, no engine linked into the shell or the
  supervisor, and no reduction from six services.
- No embedded Postgres, MinIO, or RabbitMQ.
- No macOS installer, no code signing, no notarization, no auto-update, no MSI.
- No new public HTTP endpoint or contract change; `quantrule.api/v1` and
  `quantrule.internal/v1` are unchanged.
- No migration of an existing Postgres dataset to SQLite, or the reverse; the
  desktop profile starts from a fresh data directory.
- No change to the canonical digest, run identity, model output, strategy,
  execution, or portfolio behavior.
- No storage garbage collection, backup UI, or performance parity target.
- No change to profiles, authentication, or their organizational meaning.
- No Python and no Bun runtime in either installer.

## 4. Assumptions and uncertainty

| Assumption | If it is wrong |
|---|---|
| The engines are a pure function of candle and configuration bytes; persistence only moves the same bytes | A digest difference would be an engine defect, and the work stops rather than adjusting the digest |
| The workspace's `rusqlite` 0.42 can serve the repositories through a blocking wrapper; a second SQLite library cannot enter the graph because Cargo permits one `links = "sqlite3"` package | P1a measures the blocking wrapper and row mapping; if the service shape cannot absorb it, the plan returns for a dependency decision instead of forcing two SQLite libraries |
| One SQLite file in WAL mode supports the evidence and backtest processes on a local disk | Split the queue into its own file, or serialize queue access through one process; both are plan changes |
| Windows rename cannot replace an existing file | The CAS verifies the existing content and treats an identical object as success; a different object is an error |
| The server profile remains the oracle; the desktop must match it | A change to the oracle is a separate contract change and needs human approval |
| `image_digest` is part of run identity, and the desktop package has no image digest | The parity harness passes the same `QR_IMAGE_DIGEST` to both profiles; a real desktop run records the release bundle digest and therefore a different run id, with the same canonical digest |
| Status timestamps are provenance, not signal | Parity compares the canonical digest, artifact bytes, and a projection that excludes timestamps; it never compares wall-clock values |
| The composed tier is not wired yet | The parity suite is a release-tier harness that starts the server side with Docker and the desktop side without it; it is not claimed as a fast test |
| AppImage needs FUSE on some Linux distributions | The smoke procedure documents `--appimage-extract-and-run`; a native package is out of scope |
| No new dependency is needed | A dependency requires the section 14 approval gate before the phase starts |

## 5. Decisions

| ID | Decision | Recommendation | Alternative rejected | Approval |
|---|---|---|---|---|
| D1 | SQLite dialect strategy | Per-dialect repositories behind the existing domain traits; `Database` stays narrow | Widening `Database` into a query abstraction | Recommended; detail in section 8 |
| D2 | Schema mapping | One SQLite file with `service__table` prefixes | `ATTACH` one file per service to emulate schemas | Recommended |
| D3 | Type mapping | JSON as `TEXT` with `json_valid`, timestamps as RFC 3339 seconds, booleans as 0/1 | SQLite `JSONB` blobs or numeric epochs | Recommended |
| D4 | Migration ownership | Each service owns a SQLite baseline; one shared ledger table with a ledger column | Translating the 13 PostgreSQL migrations mechanically | Recommended |
| D5 | Filesystem CAS | Temp file, fsync, rename; hash on read; reject traversal | A single tar or database BLOB store | Recommended |
| D6 | In-process queue | SQLite tables in the same file, WAL, lease, retry, dead-letter | In-memory queue with no durability | Recommended |
| D7 | Sidecar supervision | A `qr-desktop` supervisor spawns six services; the shell spawns only the supervisor | Tauri `shell` plugin plus one IPC command per service | Recommended |
| D8 | Port allocation | Loopback, gateway on 8080 then 8081–8089, internal ports from a fixed private range | Hard-coded 8080 only | Recommended |
| D9 | Orphan recovery | Supervisor-owned control socket; children exit when it closes; stale temp cleanup | PID hunting with platform-specific process APIs | Recommended |
| D10 | Parity oracle | The server profile on the same fixture, configuration, and image digest; see section 10 | A checked-in golden digest alone | Recommended |
| D11 | No in-process engine | Six sidecars, same routers, same contracts; no engine in the shell or supervisor | Linking `quantrule-app` into one binary | Rejected by ADR-12 |
| D12 | No Python, no Bun | Package scan plus the existing source and image gates | Trusting the build inputs | Required |

### 5.1 SQLite dialect strategy (D1)

The domain repository trait is already the seam: `DatasetCatalog`,
`ConfigCatalog`, `RunStore`, `HypothesisStore`, `SimulationStore`, and
`ProfileStore`. The SQLite work adds one implementation per trait and changes no
route, use case, or contract. Section 8 compares this with the alternatives and
records the blast radius.

Backend selection is explicit: `QR_PROFILE=server` is the default and
`QR_PROFILE=desktop` selects SQLite, the filesystem store, and the in-process
queue under `QR_DATA_DIR`. A service branches once in `main.rs`; no route, use
case, or engine reads the profile.

`Database` stays a liveness and backend-name trait. It is not widened into a
query abstraction. A shared `dialect` helper in `quantrule-runtime` may hold
table-name mapping, JSON binding, and timestamp formatting, but it does not
build SQL.

The desktop database uses the workspace's existing `rusqlite` 0.42, not `sqlx`
SQLite. `sqlx-sqlite` 0.9 requires `libsqlite3-sys >=0.30.1, <0.38`, while
`rusqlite` 0.42 links `libsqlite3-sys` 0.40; Cargo permits only one package to
link the native `sqlite3` library, so the two cannot share this workspace.

`rusqlite` is synchronous, so each SQLite repository call runs on
`tokio::task::spawn_blocking` behind one guarded connection. That serializes
local database work, which is acceptable for a single-user desktop and keeps the
async route shape. The application memory remains the schema and migration
precedent, and the bundled SQLite keeps the Windows build from needing a system
library. No new crate version is added; `rusqlite` is already a workspace
dependency. The P1a spike settles the blocking wrapper, transaction handling, and
row mapping before the first repository is copied; if the service shape cannot
absorb it, stop before P1b.

### 5.2 Table prefixes instead of schemas (D2)

SQLite has no schemas. `ATTACH` could emulate one file per schema, but it makes
a backup five files, prevents a single transaction across the queue and a run
row, and gives every connection five names. One file with a prefix is simpler
and keeps the service boundary logical.

| PostgreSQL | SQLite |
|---|---|
| `catalog.datasets` | `catalog__datasets` |
| `catalog.configs` | `catalog__configs` |
| `catalog.config_bundles` | `catalog__config_bundles` |
| `gateway.profiles` | `gateway__profiles` |
| `runs.runs` | `runs__runs` |
| `runs.stages` | `runs__stages` |
| `runs.artifacts` | `runs__artifacts` |
| `runs.submissions` | `runs__submissions` |
| `research.hypotheses` | `research__hypotheses` |
| `simulation.runs` | `simulation__runs` |
| `simulation.marks` | `simulation__marks` |
| per-schema ledger | one `schema_migrations` table with a `ledger` column |
| none | `queue__messages`, `queue__dead_letter` |

Ownership is enforced in two places: a service may name only its own prefix, and
no query joins across prefixes. A gate checker scans the service sources and
migration files for foreign prefixes; the test fails on a violation. A shared
ledger is infrastructure, not domain data, and is owned by the runtime.

### 5.3 TEXT, JSON, and time mapping (D3)

| PostgreSQL | SQLite | Rule |
|---|---|---|
| `jsonb` | `TEXT` | Write `serde_json::to_string` of the same typed value; read with `serde_json::from_str` into the same type; never echo the raw column. Add `CHECK (json_valid(col))` where non-null. |
| `timestamptz` | `TEXT` | RFC 3339 UTC with seconds, the exact shape `to_char` already produces; lexicographic order is chronological. |
| `boolean` | `INTEGER` | `CHECK (col IN (0, 1))`; map in the repository. |
| `bigint`, `integer` | `INTEGER` | SQLite stores 64-bit integers; convert at the row boundary. |
| `now()` | `strftime('%Y-%m-%dT%H:%M:%SZ', 'now')` | Status and provenance only; parity excludes wall-clock values. |
| `$1::jsonb`, `$1::timestamptz` | `?` | Bind `TEXT`; the cast has no SQLite equivalent. |
| `to_char(... AT TIME ZONE 'UTC', ...)` | no call | Values are already stored in that shape. |
| `FOR UPDATE` | `BEGIN IMMEDIATE` | One writer; re-read the row status inside the transaction. |
| `jsonb_array_length(selection -> 'windows')` | `json_array_length(json_extract(selection, '$.windows'))` | The only JSON-inspecting query in the repositories. |
| PL/pgSQL immutability trigger | SQLite `BEFORE UPDATE OR DELETE` trigger with `RAISE(ABORT, ...)` | Same refusal. |
| `ON CONFLICT`, `EXCLUDED`, `RETURNING` | supported | Keep the statement. |
| advisory lock | `BEGIN IMMEDIATE` plus `busy_timeout` | The migration runner is the only schema writer. |

STRICT tables are used, as in the application memory. A read that cannot parse a
stored JSON value or timestamp returns a stored-value error; it never substitutes
zero or a default.

### 5.4 Migration ownership (D4)

Each schema owner ships a SQLite baseline under its own migration directory:
gateway, data, evidence (runs and research), and simulation. The SQLite baseline
is the final logical schema, not a translation of the PostgreSQL history:
`CREATE SCHEMA`, `ALTER TABLE ... DROP CONSTRAINT`, PL/pgSQL, and the historical
`jsonb` columns cannot be replayed, and the desktop profile has no existing data
to migrate. Versions start at 1 in each service's SQLite ledger.

The runtime gains a SQLite migration runner with the same pure `plan` logic, a
per-ledger checksum, a forward-only rule, and a refusal when the database is
ahead or a checksum changed. A second apply returns 0. A crash rolls back the
whole migration transaction.

The cost is two migration histories. Drift is caught by a checked-in logical
schema inventory per service and two checks: the fast test compares the SQLite
baseline to the inventory through `pragma_table_info`, `pragma_index_list`, and
`pragma_foreign_key_list`; the composed test compares the PostgreSQL catalog to
the same inventory. A schema change PR updates both sets, the inventory, and the
tests. A future change that forgets one dialect fails the gate.

### 5.5 Filesystem CAS (D5)

The filesystem store implements the existing `ObjectStore` trait; no use case
changes. The root is `QR_OBJECT_STORE_PATH` under the desktop data directory.
Keys are the existing contract keys (`candles/...`, `artifacts/{sha256}`,
`runs/{run_id}/manifest.json`, model and simulation bundles).

- `put` validates the key, creates the parent, writes a temporary file in the
  same directory, calls `sync_all`, renames it into place, and fsyncs the parent
  on Unix. A reader sees the old object or the complete new one.
- On Windows a rename cannot replace an existing file. The store verifies the
  existing object and treats identical bytes as success; different bytes under
  an immutable key are an error.
- `get` verifies a `{sha256}` final segment against the bytes and returns an
  error on a mismatch. Every key is rejected if it is absolute, contains `..`,
  a backslash escape, a drive prefix, or resolves outside the root.
- `exists` uses metadata and applies the same key rules.
- The trait has no delete. Orphan temporary files are removed by the supervisor
  at startup only inside the CAS root and only with the temporary prefix.

No new dependency is needed; `std::fs`, `std::io`, and the existing digest type
are enough.

### 5.6 In-process queue durability (D6)

The queue is durable before it is fast. `queue__messages` and
`queue__dead_letter` live in the same SQLite file, in WAL mode with
`busy_timeout`, foreign keys on, and `synchronous=FULL`. A publish inserts one
row. A consumer claims the oldest ready message inside `BEGIN IMMEDIATE`, sets a
lease, and increments the attempt count. A handler success deletes the row; a
handler failure clears the lease and makes the row immediately ready, matching
the RabbitMQ adapter's immediate requeue. The worker keeps its own backoff and
retry budget; the queue does not invent a second policy.

A killed consumer leaves a lease that expires; the next consumer redelivers. A
killed producer can lose a message between the run-row commit and the publish,
exactly as a lost broker publish can today; the submission is idempotent and a
resubmission republishes. The queue cannot make that atomic without changing the
use case, and this plan does not change it. A power loss may lose the last
committed transaction depending on the filesystem; `synchronous=FULL` narrows
that window and the residual is recorded.

The queue needs a clock for leases and backoff. The adapter takes an injected
`now` function; production passes the system clock, tests pass a fixed one, so
the durability tests stay deterministic. The queue is control-plane
infrastructure and never enters the signal path.

Publishing and consuming in two processes on one file is the deliberate risk.
The write transactions are short, the consumer polls, and `busy_timeout` covers
contention. If the composed test shows unacceptable blocking, the queue moves to
its own file; that is a recorded plan change, not an implementation detail.

### 5.7 Sidecar supervision (D7)

The shell stays a window. It does not gain an IPC domain, a plugin, or a data
file read. A new binary, `qr-desktop`, is the supervisor:

1. Resolves the data directory and the resource directory passed by the shell.
2. Binds the loopback control socket and allocates ports.
3. Cleans stale temporary files.
4. Starts the six service binaries as children with the desktop environment.
5. Waits for gateway, data, model, evidence, and simulation health; the backtest
   worker has no HTTP surface and is checked by process liveness and its queue
   consumption.
6. Restarts a child that exits unexpectedly, with a bounded backoff, and reports
   the failure after the budget.
7. Stops the children when it receives a stop, and exits.

The shell spawns the supervisor once from `setup()` with `std::process::Command`,
using Tauri's resource directory, and terminates it on `RunEvent::Exit`. The six
service binaries and the supervisor are bundled as resources, not through
`tauri-plugin-shell`; the shell still has no plugin and no `#[tauri::command]`.
The supervisor passes an explicit `QR_IMAGE_DIGEST` through when one is set, so
the parity harness can hold run identity constant; otherwise it records the
bundle digest.

`apps/desktop-shell/tests/boundary.sh` is extended to allow exactly that one
spawn and to keep failing on an IPC command, a plugin, an engine import, and a
local data file read.

The supervisor is packaging, not a service. It owns no domain state, no schema,
and no HTTP contract. A boundary test asserts its manifest depends on no
`quantrule-app`, engine, or service crate.

Shutdown is a process kill, not a drain. That is safe because the no-partial-
publish invariant already holds: an interrupted run leaves no manifest, no
succeeded row, and is reclaimed on redelivery. A graceful drain would require a
new control contract and is not part of v1.1.

### 5.8 Port allocation (D8)

The supervisor binds loopback only. The gateway prefers 8080 and falls back to
8081–8089; the internal services use a fixed private range (for example
18081–18084) with the same fallback. The chosen bindings are passed as
`QR_BIND` and as the service URLs, and written to `ports.json` for diagnostics.

The shell probes the configured URL first, then the gateway candidates, so an
existing setting still wins and a fallback is reachable without reading a file.
A running Docker stack on 8080 keeps 8080; the desktop stack moves and the shell
finds it. The parity harness pins the ports so a comparison never depends on the
fallback.

### 5.9 Orphan recovery (D9)

The supervisor owns one loopback control socket for its lifetime. Every child
opens and holds a connection to it; a watchdog thread exits the child when the
connection closes. If the supervisor is killed, the children exit within a
bounded interval and release their ports and the SQLite file. The next start
retries the bind for a short window if the previous children are still exiting.
Stale temporary files are removed after the bind succeeds.

This avoids PID hunting, a process-inspection crate, `unsafe`, and a
platform-specific process API. The residual is a child that is wedged and cannot
run its watchdog: the supervisor reports the busy port and a restart is required.
The recovery test kills the supervisor and asserts that all six children exit and
that a second start completes.

### 5.10 Parity oracle (D10)

See section 10. The oracle is the server profile, not a checked-in number. The
harness holds the image digest constant so run identity can also be compared;
the acceptance criterion is the canonical digest and the artifact bytes.

### 5.11 No in-process engine (D11)

The six services remain six processes. The desktop profile changes adapters and
configuration, not routers or engines. The gateway still proxies the same
contracts; the shell still loads the gateway URL. Guard tests prove that neither
the shell nor the supervisor links an engine or `quantrule-app`, and the parity
smoke proves six service processes answer health and complete one run.

### 5.12 No Python, no Bun (D12)

The image gate already proves the server image has no interpreter. The desktop
packages add a second check: extract the AppImage and install the NSIS package to
a temporary directory, then fail on `python`, `python3`, `bun`, `node`, or any
`.py` file. The static client assets are allowed; a runtime is not.

## 6. Acceptance criteria

1. One installer starts the six service processes plus the supervisor, and a
   fixture run completes with no Docker daemon and no Docker socket available.
2. The desktop profile produces the same canonical digest as the server profile
   for the same fixture, configuration, and `QR_IMAGE_DIGEST`.
3. Every canonical artifact is byte-identical and the manifest entry set matches.
4. The four DB-owning services answer every repository method on SQLite, and the
   same conformance suite passes on Postgres in the composed tier.
5. A second migration apply is a no-op; a changed checksum, an unknown applied
   version, and a database ahead of the binary are refused.
6. A process killed mid-write leaves no visible artifact and no succeeded run; a
   redelivered job produces the same bytes and digest.
7. The desktop package contains no Python interpreter, no Bun runtime, and no
   engine.
8. Linux and Windows installers are produced by the release matrix from the same
   commit; the unsigned SmartScreen warning is documented.
9. The server profile, the HTTP contracts, the run identity inputs, and the
   canonical digest algorithm are unchanged; the fast gate passes.
10. Documentation, changelog, AI log, release note text, and task cleanup are
    complete.

## 7. Phases and work breakdown

| Phase | Issue | Work | Depends on | Exit criteria |
|---|---|---|---|---|
| P0 | #692 | This plan, the AI log, the issue-body summary | — | Plan checks pass; PR merged |
| P1 | #693 | SQLite persistence for four services | P0 | Repository conformance on SQLite; migrations idempotent; prefix guard |
| P2 | #694 | Filesystem CAS and durable in-process queue | P0; P1a for the queue | Full fixture run on desktop backends; kill and redelivery tests |
| P3 | #695 | Supervisor, shell lifecycle, installers, parity suite | P1, P2 | AppImage and NSIS start six sidecars and match the server digest |
| P4 | #692 | Documentation, changelog, board, cleanup | P3 | Every verification row Pass or accepted risk |

### P0 — Plan and records (this PR)

- Owned paths: `docs/execution-plans/2026-11-15/692-desktop-v1.1.md`,
  `docs/ai-transparency-log/2026-11-15/692-desktop-v1.1.md`.
- Exit: `check_execution_plans`, `check_doc_length`, `check_links`, and
  `check_terms` pass; the PR uses `Refs #692`.

### P1 — SQLite persistence (#693)

`#693` is one issue and may be up to five PRs. It stays `In Progress` until the
last one merges. No sub-issues are created.

The filed done-when names the canonical digest, which also needs the storage and
queue from #694 and the packaging from #695. P1's gate is therefore repository
and migration parity; the digest criterion closes jointly in P3, and the
proposed #693 note in section 14 records that amendment.

| Step | Work | Owned paths | Exit |
|---|---|---|---|
| P1a | SQLite adapter over the workspace `rusqlite`, migration runner, profile selector, gateway profiles | `crates/quantrule-runtime/src/adapters/sqlite.rs`, `adapters/mod.rs`, `config.rs`, `migrations.rs`; `crates/quantrule-gateway/src/{adapters,profile}/**`, `src/bin/qr_gateway.rs`, `migrations/sqlite/**` | `QR_PROFILE=desktop` starts the gateway on SQLite; health reports `sqlite`; a probe query works |
| P1b | Data catalog and config store | `crates/quantrule-data-service/src/{postgres.rs,sqlite.rs,main.rs}`, `migrations/sqlite/**`, `tests/**` | Every `DatasetCatalog` and `ConfigCatalog` method passes the conformance suite |
| P1c | Evidence runs and research | `crates/quantrule-evidence-service/src/{postgres.rs,sqlite.rs,main.rs}`, `migrations/sqlite/**`, `tests/**` | Every `RunStore` and `HypothesisStore` method passes; sealed rows refuse update and delete |
| P1d | Simulation store | `crates/quantrule-simulation-service/src/{postgres.rs,sqlite.rs,main.rs}`, `migrations/sqlite/**`, `tests/**` | Every `SimulationStore` method passes; mark and decision races hold |
| P1e | Conformance, inventory, guard, docs | service `tests/**`, `tools/check_schema_prefixes.py`, schema inventory fixtures, `docs/architecture/deployment.md` | Fast gate passes; inventory matches SQLite; Postgres inventory matches in the composed tier |

`P1a` is the spike: it settles placeholders, row decoding, and JSON binding
before the other repositories are copied. If it cannot keep the use cases
unchanged, stop and return to the plan.

### P2 — Storage and queue (#694)

| Step | Work | Owned paths | Exit |
|---|---|---|---|
| P2a | Filesystem CAS | `crates/quantrule-runtime/src/adapters/filesystem.rs`, `adapters/mod.rs`, `config.rs`, tests | Atomic publish, hash-on-read, traversal, and kill-mid-write tests pass |
| P2b | Durable in-process queue | `crates/quantrule-runtime/src/adapters/inprocess.rs`, `queue.rs`, `config.rs`, tests | Publish, claim, ack, nack, lease expiry, retry, dead-letter, and cancellation tests pass |
| P2c | Desktop wiring and end-to-end run | service `main.rs` files, `crates/quantrule-backtest-service/src/backtest/**` only if retry wiring needs it, tests | A full fixture run completes on SQLite, filesystem, and the in-process queue without an installer |
| P2d | Parity pre-check | `tools/parity_desktop.sh` (first version), fixture run | Desktop digest equals the server digest before packaging starts |

`P2a` can start after P0. `P2b` needs P1a's SQLite runtime. `P2c` needs both.

### P3 — Packaging and verification (#695)

| Step | Work | Owned paths | Exit |
|---|---|---|---|
| P3a | Supervisor binary | `crates/quantrule-desktop-supervisor/**`, workspace `Cargo.toml` | Six children start, health passes, restart and stop work |
| P3b | Shell lifecycle | `apps/desktop-shell/src/main.rs`, `ui/**`, `tests/**`, `tauri.conf.json`, `README.md` | One spawn, no IPC, no plugin, no engine; boundary test updated |
| P3c | Installer matrix | `.github/workflows/release.yml`, `apps/desktop-shell/**`, `tools/check_desktop_package.sh` | AppImage and NSIS build with the shell plus seven bundled executables and no interpreter |
| P3d | Parity suite and evidence | `tools/parity_desktop.sh`, `tools/verify.sh` if a flag is needed, `docs/releases/**` | Server and desktop digests, artifacts, and manifest agree twice |
| P3e | Documentation and changelog | `docs/architecture/{deployment,services}.md`, `apps/desktop-shell/README.md`, `README.md`, `CHANGELOG.md` | Statements match the shipped profile; no stale claim |

The release workflow is owned by the release work; `P3c` changes it only with the
human's release authorization. Windows evidence is produced on `windows-latest`
and is not claimed from Linux.

### P4 — Closure

- Update the release note text for unsigned artifacts and the desktop profile.
- Add the `CHANGELOG.md` entry under `[Unreleased]` `Added` when the behavior
  lands, or record why no entry is needed for the plan PR.
- Update the plan table with observed results; the reviewer marks each row.
- Move #692–#695 through `In Review` to `Done` after acceptance and merge.
- Remove worktrees and local and remote branches; keep every board item.

## 8. The critical risk: direct sqlx Postgres repositories

The `Database` trait does not abstract a database. It exposes a backend name and
a ping, while six repository types take `PgPool` and send Postgres SQL. The
desktop profile therefore cannot be added by implementing one trait: the
repositories themselves must gain a second implementation or a dialect seam.

| Option | Shape | Consequence | Blast radius | Verdict |
|---|---|---|---|---|
| A. Shared repository plus dialect layer | One repository per domain; SQL assembled from `Dialect` fragments for placeholders, prefixes, JSON, and time | One code path, but the layer must model locks, JSON functions, triggers, and DDL too; the differences leak and the SQL becomes string assembly | Six repositories, runtime, migrations | Reject as the primary; allow tiny helpers |
| B. Per-dialect repositories | `Pg*` and `Sqlite*` implement the same six domain traits; one conformance suite runs every method against both | Most statements exist twice; divergence is contained by the trait, the suite, and the schema inventory; each dialect is idiomatic and reviewable | Six repositories, four services, runtime, 13 migrations, about 55 statements | Recommend |
| C. Narrower `Database` trait | Keep `Database` to backend and liveness; add a separate SQLite handle and migration runner, and put dialect work in repositories | Necessary, but not sufficient alone: a new `Database` impl changes no query because no repository calls it | Runtime plus the six repositories | Adopt together with B |

**Recommendation: B plus C.** The domain trait is the contract; a test body is
written once and run against both implementations. The `Database` trait stays
narrow, and widening it is explicitly rejected because it would turn every typed
query into a dynamic one. The shared helper handles only table names, JSON
binding, and timestamp formatting. This preserves the project rule that a trait
grows when a service needs an operation, not in anticipation.

**Blast radius of B plus C**

| Area | Change |
|---|---|
| Runtime | `sqlite.rs`, `rusqlite` blocking wrapper, migration runner, config selector, health names; `PgDatabase` unchanged |
| Gateway | One profile repository plus a SQLite baseline |
| Data | Two repositories plus a SQLite baseline |
| Evidence | Two repositories plus a SQLite baseline; the sealed-row trigger is rewritten |
| Simulation | One repository plus a SQLite baseline; `FOR UPDATE` becomes `BEGIN IMMEDIATE` |
| Migrations | 13 Postgres files stay; four SQLite baselines are added |
| Tests | A shared conformance suite, a schema inventory per service, a prefix gate, and existing tests |
| Routes and contracts | No change |
| Engines and digest | No change |

**This phase is not one pull request.** P1a through P1e are separate reviewable
changes; #693 stays open until the last one. A partial P1 that leaves one service
on Postgres is not a shippable desktop profile, but it is a valid intermediate
state because the server profile stays the default.

## 9. Six sidecars and the same HTTP contracts

```text
Tauri shell ──spawns──> qr-desktop supervisor
                              ├── qr-gateway      :8080  (or fallback)
                              ├── qr-data         private port
                              ├── qr-model        private port
                              ├── qr-backtest     no HTTP, queue consumer
                              ├── qr-evidence     private port
                              └── qr-simulation   private port

browser client ──> gateway ──> data, model, evidence, simulation
```

- The gateway is the only client entry. The shell loads its URL; the browser
  client is served by it; the command shell reaches it over HTTP at the same
  URL with the same commands. Both clients keep their parity test; the desktop
  profile adds no client-specific endpoint.
- The supervisor starts and stops processes; it does not proxy, route, compute,
  or own a contract.
- Each service keeps its own router, its own schema prefix, and its own health.
- A run is submitted through the gateway, queued, computed by `qr-backtest`, and
  finalized by `qr-evidence`, exactly as in the server profile.
- `qr-backtest` still has no HTTP surface; the supervisor checks it by process
  liveness and the run completing.
- A guard test fails if the shell or supervisor depends on an engine or
  `quantrule-app`; another fails if a route gains a desktop-only branch.

## 10. Parity suite

**Fixture.** `config/examples/verify-mini/run.json`, its replica, and
`candles.csv`; profile `parity`; the same uploaded configuration and dataset
bytes on both profiles.

**Oracle.** The server profile. Both sides receive the same explicit
`QR_IMAGE_DIGEST` (the server image digest) so run identity can be compared too.
A real desktop run uses its own bundle digest; the canonical digest is
independent of it.

**Procedure** (release tier, one command, `tools/parity_desktop.sh`):

1. Start the server profile from the released image with a fresh volume; confirm
   nine containers are healthy.
2. Start the desktop profile from the packaged supervisor in headless mode with a
   fresh data directory, with no Docker socket in its environment.
3. Upload the same candle bytes and the same configuration to each gateway.
4. Submit `verify-mini` on each and wait for `succeeded`.
5. Compare the canonical digest, every artifact's byte length and SHA-256, the
   manifest entry set, the run state and stage states, and the API projection of
   the run detail.
6. Repeat each run a second time, and alternate the start order, to catch a
   profile that depends on first-run state.
7. Tear both down; retain the two run records and the comparison output as the
   review evidence.

**Exclusions.** Timestamps, provenance, log lines, and the image digest field are
excluded from the compared projection; they record where and when the run
happened and are not part of the canonical digest. The digest itself is compared
exactly; there is no tolerance.

**Tier.** The suite is release-tier because it needs the server image for the
oracle. The fast gate keeps the SQLite, CAS, and queue tests; the composed tier
is not wired yet (#643/#644), so this suite is invoked by the release workflow
and by the reviewer, not by `cargo test`.

**Failure rule.** A digest mismatch blocks the phase. It is not resolved by
changing the digest, the fixture, or the oracle. The first step is to diff the
artifact bytes and the engine inputs.

## 11. Installer matrix and package checks

| OS | Runner | Bundle | Contents | Smoke |
|---|---|---|---|---|
| Linux | `ubuntu-latest` | AppImage | Shell, supervisor, six services, static client, example configs | Extract and run without Docker; start headless; run the fixture; stop; assert six processes and no orphans |
| Windows | `windows-latest` | NSIS | Shell, supervisor, six `.exe` services, static client, example configs | Silent install; start headless; run the fixture; uninstall; assert six processes and no interpreter |

- Both artifacts are built from the same commit and the same package version.
- The six service binaries and the supervisor are sidecars from the same
  `cargo build --release --bins` as the release; no service is rebuilt from a
  different source tree.
- The package check extracts the AppImage and the installed NSIS tree and fails
  on `python`, `python3`, `bun`, `node`, or a `.py` file. It also fails when an
  engine crate appears in the shell or supervisor manifest.
- The installers are unsigned. The release note documents the SmartScreen path
  and the Linux executable bit; no signing key is invented.
- The Windows build and smoke run on `windows-latest`; the Linux build does not
  claim Windows evidence.
- `config/examples/**` is bundled read-only as the catalog source; user uploads
  and generated data live under the per-user data directory, not in the install
  directory.

## 12. Rollback and recovery

| Failure | Recovery |
|---|---|
| Installer build fails | The server release is unaffected; the desktop job is retried from the same commit |
| Desktop run fails | The run row records the error; the supervisor reports the failing service; no artifact is visible until the manifest publishes |
| Supervisor killed | Children exit when the control socket closes; the next start rebinds and cleans temporary files |
| A child crashes | The supervisor restarts it with bounded backoff; a run is reclaimed by redelivery |
| SQLite migration fails | The transaction rolls back; the ledger is unchanged; the service refuses to start with the error |
| Database is ahead of the binary | The migration planner refuses it; restore the data directory from backup or reinstall the matching version |
| Queue message is lost after a crash | The run is still `queued`; resubmission republishes it, matching the server profile's lost-publish behavior |
| New version is bad | Reinstall the previous installer; the data directory is preserved. A schema downgrade is not supported and is refused rather than guessed at |
| Whole programme is abandoned | The desktop profile is additive; remove the package and keep the server profile. No server data is migrated or touched |

The desktop data directory is the backup unit: one SQLite database, the CAS
tree, and the supervisor's diagnostic files. A backup is a directory copy while
the supervisor is stopped; no new backup tool is added in v1.1.

## 13. Verification

| Check | Command | Scope |
|---|---|---|
| Plan structure | `python3 tools/check_execution_plans.py` | This PR |
| Document length | `python3 tools/check_doc_length.py` | This PR |
| Links | `python3 tools/check_links.py` | This PR |
| Terms | `python3 tools/check_terms.py` | This PR |
| Fast workspace gate | `tools/gate.sh` | Each code PR |
| Desktop repository tests | `cargo test -p quantrule-runtime -p quantrule-data-service -p quantrule-evidence-service -p quantrule-simulation-service -p quantrule-gateway` | P1, P2 |
| Shell tests | `bun test` and `bash tests/boundary.sh` in `apps/desktop-shell` | P3 |
| Linux smoke | `bash apps/desktop-shell/tests/smoke-linux.sh` | P3 |
| Package checks | `bash tools/check_desktop_package.sh <artifact>` | P3 |
| Parity | `tools/parity_desktop.sh` | Release tier |

**Limitations.** The parity suite needs Docker for the server oracle and runs on
the release runner, not in the fast gate. Windows evidence comes only from
`windows-latest`. The real kill test is process-level in P2 and composed in P3;
a platform-level power-loss test is not attempted. No previous desktop version
exists, so an upgrade from v1.0 desktop is not tested; there is none.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | P0 plan structure | The plan has a status, a claim type, and a complete verification table | `python3 tools/check_execution_plans.py` | pass | execution plans ok | Pending | | | |
| V2 | P0 document length | Current documents stay under their caps | `python3 tools/check_doc_length.py` | pass | 34 files within cap | Pending | | | |
| V3 | P0 links | Every relative link resolves | `python3 tools/check_links.py` | pass | 953 Markdown files, no broken links | Pending | | | |
| V4 | P0 terms | No retired term appears | `python3 tools/check_terms.py` | pass | 594 files, no banned terms | Pending | | | |
| V5 | P0 scope | The PR changes only the plan and the AI log | `git diff --name-only main...HEAD` | two files under `docs/` | two files under `docs/` | Pending | | | |
| V6 | P0 AI log | The transparency log records intent, boundary, verification, and learning | `ls docs/ai-transparency-log/2026-11-15/692-desktop-v1.1.md` | file exists | file exists | Pending | | | |
| V7 | P0 issue summary | The plan carries a paste-ready issue body and phase map | read section 14 | present | present | Pending | | | |
| V8 | P0 changelog | No changelog entry is needed for a plan-only PR, and the reason is recorded | PR body | reason present | plan-only; no entry needed | Pending | | | |
| V9 | P1a SQLite adapter | `QR_PROFILE=desktop` starts the gateway on SQLite and health names the backend | gateway test plus `/health` | `sqlite` | | Pending | | | |
| V10 | P1a migration runner | A second apply is a no-op; a changed checksum, an unknown version, and a database ahead are refused | runtime migration tests | 0 pending; each refusal returns an error | | Pending | | | |
| V11 | P1a gateway parity | Every profile method returns the same record on SQLite and Postgres | shared conformance suite | equal records | | Pending | | | |
| V12 | P1b data parity | Every dataset and config method returns the same record on both dialects | shared conformance suite | equal records | | Pending | | | |
| V13 | P1c evidence parity | Every run and hypothesis method returns the same record on both dialects | shared conformance suite | equal records | | Pending | | | |
| V14 | P1d simulation parity | Every simulation method returns the same record on both dialects | shared conformance suite | equal records | | Pending | | | |
| V15 | P1e JSON and time mapping | A typed value round-trips through TEXT identically, and timestamps keep the RFC 3339 seconds shape | repository tests | equal typed values and bytes | | Pending | | | |
| V16 | P1e schema inventory | The SQLite and Postgres catalogs match the checked-in logical inventory | fast test plus composed test | no drift | | Pending | | | |
| V17 | P1e prefix guard | No service names another service's prefix and no query joins across prefixes | `python3 tools/check_schema_prefixes.py` | pass | | Pending | | | |
| V18 | P1 no use-case change | Routes, contracts, and engines are untouched; the server profile stays the default | diff review plus gate | no route or engine diff | | Pending | | | |
| V19 | P1 Postgres unchanged | Existing Postgres tests still pass | `tools/gate.sh` plus composed tier | pass | | Pending | | | |
| V20 | P2a CAS atomic publish | A reader sees the old object or the complete new one, never a partial body | filesystem store tests | atomic | | Pending | | | |
| V21 | P2a kill mid-write | A stopped write leaves no visible final key and no temporary key visible to a reader | process-kill test | no visible artifact | | Pending | | | |
| V22 | P2a hash and traversal | A digest mismatch and a path escape are refused; identical bytes on Windows are accepted | filesystem store tests | refused or accepted as stated | | Pending | | | |
| V23 | P2b queue lifecycle | Publish, claim, ack, nack, and lease expiry behave as the RabbitMQ adapter does | in-process queue tests | same outcomes | | Pending | | | |
| V24 | P2b queue durability | A committed message survives a process restart; an unacked message is redelivered | restart test | durable and redelivered | | Pending | | | |
| V25 | P2b retry and dead-letter | Three failures mark the run failed and publish `run.dead`; cancellation is consumed | worker and queue tests | same state as the server profile | | Pending | | | |
| V26 | P2b multi-process | Evidence and backtest share the queue file without corruption or lost messages | two-process test | no loss, no deadlock | | Pending | | | |
| V27 | P2c full desktop run | The fixture completes with SQLite, filesystem CAS, and the in-process queue before packaging | end-to-end test | succeeded | six sidecars, 12 canonical artifacts, 3,559,171 bytes; digest `sha256:453c90b4…` | Pending | | | |
| V28 | P2d digest pre-check | The desktop digest equals the server digest before an installer exists | `tools/parity_desktop.sh` | equal | desktop and server both `sha256:453c90b4…`; 24 artifact files byte-identical | Pending | | | |
| V29 | P3a supervisor start | Six children start, five health endpoints answer, and the worker consumes | supervisor integration test | six running, health pass | `qr-desktop --profile desktop` started all six; five `/health` endpoints healthy; two runs computed and finalized | Pending | | | |
| V30 | P3a restart | A killed child is restarted within the budget and the run continues or is reclaimed | supervisor test | recovery | | Pending | | | |
| V31 | P3a stop and orphans | A supervisor stop leaves no child running; a supervisor kill makes children exit and frees the ports | kill and restart test | no orphan | | Pending | | | |
| V32 | P3b port allocation | 8080 is used when free, the fallback works when it is taken, and internal ports do not collide | supervisor test | correct URLs | | Pending | | | |
| V33 | P3b shell boundary | One spawn, no IPC command, no plugin, no engine import, no local data file read | `bash apps/desktop-shell/tests/boundary.sh` | pass | | Pending | | | |
| V34 | P3c Linux installer | The AppImage starts six sidecars and completes a run with no Docker socket | release job smoke | succeeded, no Docker | | Pending | | | |
| V35 | P3c Windows installer | The NSIS installer starts six sidecars and completes a run with no Docker socket | `windows-latest` smoke | succeeded, no Docker | | Pending | | | |
| V36 | P3c package contents | The shell plus seven bundled executables; no `python`, `bun`, `node`, or `.py` file | `tools/check_desktop_package.sh` | pass | | Pending | | | |
| V37 | P3d parity digest | Desktop and server canonical digests are equal for the fixture | `tools/parity_desktop.sh` | equal | `sha256:453c90b4…` on both profiles | Pending | | | |
| V38 | P3d parity artifacts | Every artifact byte length and SHA-256, the manifest set, and the API projection match | `tools/parity_desktop.sh` | equal | 24 canonical artifact files byte-identical; `run status` (25-entry manifest, stages) identical | Pending | | | |
| V39 | P3d parity repetition | Two runs per profile and both start orders agree | `tools/parity_desktop.sh` | equal | two rounds, desktop-first then server-first; both `sha256:453c90b4…` | Pending | | | |
| V40 | P3 no in-process engine | The shell and supervisor link no engine or app crate; six processes answer | boundary test plus process list | pass | | Pending | | | |
| V41 | P3 unsigned artifacts | The release note states the SmartScreen path and the Linux executable bit | release note review | present | | Pending | | | |
| V42 | P4 changelog | The `[Unreleased]` `Added` entry, or the recorded reason none is needed | `python3 tools/check_changelog.py` | pass | | Pending | | | |
| V43 | P4 docs | Deployment, services, README, and desktop README match the shipped profile | doc review | no stale claim | | Pending | | | |
| V44 | P4 issues | #692–#695 are `Done` with close dates after acceptance and merge | board query | no open leaf | | Pending | | | |
| V45 | P4 cleanup | The worktree and local and remote task branches are removed | `git worktree list` plus `git branch -a` | absent | | Pending | | | |
| V46 | P4 gate | The full fast gate passes on the final tree | `tools/gate.sh` | pass | | Pending | | | |

## Risks and recovery

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| A second SQLite library conflicts with the app memory | High if `sqlx` SQLite is chosen | High | Use the workspace's `rusqlite` 0.42; `sqlx-sqlite` 0.9 requires `libsqlite3-sys` below 0.38, which cannot coexist with 0.40 |
| A dialect difference changes a stored value or an API response | Medium | High | Per-dialect repositories, a shared conformance suite, a schema inventory, and byte-level parity |
| The two migration histories drift | Medium | High | Inventory tests on both dialects; a schema change updates both sets in one PR |
| SQLite write contention stalls the queue or a run | Medium | Medium | Short transactions, WAL, `busy_timeout`; split the queue file if the composed test shows blocking |
| Windows rename or path semantics break the CAS | Medium | High | Identical-bytes success path, path validation, and a Windows smoke test |
| A wedged child survives the supervisor | Low | Medium | Control-socket watchdog, bounded restart, busy-port report; restart required |
| The installer is flagged by SmartScreen | High | Medium | Documented unsigned path; signing is a separate decision |
| AppImage FUSE is missing | Medium | Low | Document `--appimage-extract-and-run`; native packaging is out of scope |
| A dependency creeps into the shell or supervisor | Low | High | Boundary and manifest tests |
| The parity suite is flaky and gets ignored | Medium | High | Retained records, fixed ports, fixed fixture, no timestamps in the projection |
| Scope expands into a single-binary app | Low | Critical | ADR-12 and the six-process guard; stop condition |
| The server profile regresses | Low | High | Adapter changes are additive; the fast gate and the server parity side run on every code PR |

**Recovery.** The desktop profile is additive. Disabling it is a configuration
change; the server profile, its data, and its contracts are untouched. A failed
desktop data directory is recoverable by restoring the directory copy; there is
no server-to-desktop data migration to undo.

## Stop conditions

- A query cannot be expressed on SQLite without changing a route, a use case, or
  a contract.
- Parity needs a change to the canonical digest, run identity, or an engine
  output. That is a model or strategy decision, not a desktop decision.
- A new dependency is required beyond the existing `sqlx` SQLite feature.
- Sidecar supervision needs an IPC domain that exposes a domain operation.
- One SQLite file cannot support the two processes reliably, and the queue split
  changes the approved data layout.
- A package would include Python, Bun, or an engine.
- A migration cannot stay forward-only, or a downgrade would guess at a schema.
- A verification row fails and the fix changes the approved scope.
- #693 cannot be delivered without creating sub-issues; that needs a human
  decision first.

## 14. Issue-body summary and phase decomposition

**Proposed #692 body** (paste after the plan merges; no issue is edited by this
PR):

> **Goal:** Make the platform self-contained for a casual user without Docker,
> while keeping six services.
>
> **Scope:**
> - SQLite persistence for the four DB-owning services, behind the existing
>   repository traits.
> - Filesystem CAS and a durable in-process queue behind the adapter traits.
> - A supervisor, Linux and Windows installers, and a parity suite.
>
> **Done when:**
> - The desktop profile produces the same canonical digest and artifact bytes as
>   the server profile for the same fixture and configuration.
> - One installer starts six sidecars and completes a run with no Docker daemon.
>
> **Claim type:** ADR (Architecture Decision)
>
> **Plan:** `docs/execution-plans/2026-11-15/692-desktop-v1.1.md`
>
> **Children:** #693 SQLite backend; #694 filesystem CAS and in-process queue;
> #695 packaging and verification.

**Phase map**

| Phase | Issue | Deliverable | Gate |
|---|---|---|---|
| P0 | #692 | Plan, AI log, issue summary | Plan PR merged |
| P1 | #693 | SQLite repositories and migrations | Conformance, inventory, prefix guard |
| P2 | #694 | Filesystem CAS and durable queue | Full desktop run, kill and redelivery |
| P3 | #695 | Supervisor, installers, parity | Six sidecars, no Docker, digest parity |
| P4 | #692 | Docs, changelog, board, cleanup | Every row Pass or accepted risk |

**Proposed note for #693** (append when P1 starts): implementation may land as up
to five PRs — runtime and gateway, data, evidence, simulation, then conformance
and inventory. The issue stays `In Progress` until the last one merges. No
sub-issues are created; the phase table in the plan is the tracking record. The
filed canonical-digest criterion is satisfied jointly with #694 and #695; P1
closes on repository and migration parity.

**Records.** Plan PR uses `Refs #692`. The behavior changelog entry lands under
`[Unreleased]` `Added` with P3, not with this plan. The AI contribution is
recorded in
[`docs/ai-transparency-log/2026-11-15/692-desktop-v1.1.md`](../../ai-transparency-log/2026-11-15/692-desktop-v1.1.md).
No Quant Validator record is required: this plan changes no model, strategy,
execution, or portfolio behavior.
