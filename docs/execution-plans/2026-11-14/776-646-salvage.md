# Parked #646 gateway salvage map

- Status: In Progress
- Date: 2026-11-14 (UTC)
- Claim type: ADR (Architecture Decision)
- Owner: Human
- Plan: [`776-split-service-binaries.md`](776-split-service-binaries.md) (P1, §5.8)
- Parked worktree: `/home/zantgo/Proyectos/quant-rule--646-gateway`
- Parked branch: `feat/646-gateway` at `b63b9015`
- State: **not landed.** The branch has no commit for this work; it is a reuse
  candidate only and is not evidence for any acceptance criterion.

The parked tree carries 16 modified files and 6 untracked files: `568`
insertions and `618` deletions in the tracked diff. The untracked files are not
in that stat. The plan's §5.8 snapshot said 5 untracked; the observed count is 6.
It was read read-only for this map. Nothing was copied, rebased, or committed
from it.

- AI log: [646-gateway.md](../../ai-transparency-log/2026-11-15/646-gateway.md)

## Inventory and destination

| Parked path | What it carries | Destination | Action |
|---|---|---|---|
| `crates/quantrule-api/src/error.rs` | `auth.unauthorized` 401 code | P8 | Reuse the additive code and its `ALL` test with the gateway bearer layer |
| `crates/quantrule-api/src/lib.rs` | Re-exports for the new event and digest types | P3, P5 | Split the re-export by owner; the public contract keeps one listing |
| `crates/quantrule-api/src/model.rs` | `EventRecord`, `EventBatch` | P3 | Reuse the public event batch contract in the data service |
| `crates/quantrule-api/src/run.rs` | `RunDigest` response | P5 | Reuse the digest response with evidence |
| `crates/quantrule-server/src/bin/qr_gateway.rs` | Bearer layer and route-module wiring | P8 | Reimplement against the final proxy table; do not keep the direct handlers |
| `crates/quantrule-server/src/data/inspect.rs` | Event inspection in the dataset response | P3 | Move the logic and its test to the data service |
| `crates/quantrule-server/src/data/mod.rs` | `Timeframe` re-export for the gateway boundary | P3 | Replace with a direct data-service import; the gateway stops needing it |
| `crates/quantrule-server/src/profile/routes.rs` | Profile route module | P8 | Move to the gateway crate with the `gateway` schema |
| `crates/quantrule-server/src/runs/store.rs` | `RunStore::count` | P5 | Reuse the profile run count in the evidence store |
| `crates/quantrule-server/src/runtime/adapters/postgres/runs.rs` | Postgres run count query | P5 | Port with the run store; re-test against the `runs` schema |
| `crates/quantrule-server/src/runtime/migrations.rs` | Migration 13 registration | P5 | Add to the evidence migration list; see D2 |
| `crates/quantrule-server/src/runtime/mod.rs` | Module re-exports for the split | P8 | Discard; the crate split replaces the module |
| `crates/quantrule-server/src/runtime/server.rs` | Slim route table, bearer layer, route modules | P3, P5, P8 | Reuse the route-table shape per owner; replace handlers with the proxy phase by phase |
| `crates/quantrule-server/src/runtime/auth.rs` (untracked) | Bearer middleware, constant-time compare, open paths | P8 | Reuse the logic and security tests after the gateway crate exists |
| `crates/quantrule-server/src/runtime/routes/data.rs` (untracked) | Data and config handlers, event inspection, ladder, CSV | P3 | Move handlers, adapt state and imports, keep the route tests |
| `crates/quantrule-server/src/runtime/routes/runs.rs` (untracked) | Submission, list, detail, cancel, artifact, digest | P5 | Move handlers and store calls; evidence owns the row and artifacts |
| `crates/quantrule-server/src/runtime/routes/mod.rs` (untracked) | Route module declarations | P3, P5 | Split by owner; discard the mixed declaration |
| `crates/quantrule-server/tests/boundary.rs` | Gateway names no engine; server carries no simulation domain | P8 | Replace with the compile-time guard and per-service schema guards |
| `crates/quantrule-server/tests/evidence.rs` | Digest/run-count coverage | P5 | Re-run in the evidence crate |
| `crates/quantrule-server/tests/runs.rs` | Run route and digest coverage | P5 | Re-run in the evidence crate against the gateway proxy |
| `crates/quantrule-server/tests/fixtures/events_m1.csv` (untracked) | Event inspection fixture | P3 | Reuse with the data-service event test |
| `crates/quantrule-server/tests/surface.rs` (untracked) | Every documented endpoint, profile, token, and event route | P3, P5, P8 | Split by owner; run against the proxy fakes, then the proxy |

Rules for reuse: a hunk is not accepted until its test passes in the owning
service crate; adapt `AppState`, imports, and the profile boundary rather than
copying the mixed state; never implement a route twice. If the parked tree
disappears before P3, the public contract in
[`docs/architecture/http-api.md`](../../architecture/http-api.md) is the source
and this map is unchanged.

## Pre-existing defects confirmed by the #671 backup work

These are recorded here so they are not lost. **P1 does not fix either one**;
both are owner-phase defects, and P1 keeps the current behavior unchanged.

| ID | Defect | Evidence | Owning phase |
|---|---|---|---|
| D1 | `PgDatasetCatalog` decodes `quality` and `created_at` in the wrong order | `crates/quantrule-server/src/adapters/postgres/mod.rs` selects `quality::text, to_char(created_at ...)` into `DatasetRow` positions 8 and 9, then builds `quality: row.9` and `created_at: row.8`. A stored quality JSON is parsed from the timestamp text, so the catalog read fails and the gateway cannot reliably submit or read a run through this path. Reported by #671 (DIST-080) | P3 — data service owns the `catalog` schema |
| D2 | `migrations/0013_run_attempts.sql` exists but is not registered | `crates/quantrule-server/src/migrations.rs` lists versions 1-12; `0013_run_attempts.sql` adds `runs.runs.attempts`, which the run store selects. A fresh database therefore lacks the column. The parked work registers it, but that is not landed | P5 — evidence owns the `runs` schema |

D1 and D2 are not acceptance criteria for P1 and are excluded from its diff. A
fix needs its own issue under the owning phase and a fail-before regression
test.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| S1 | Parked worktree | The worktree exists read-only at the named branch and base | `git -C /home/zantgo/Proyectos/quant-rule--646-gateway status --short --branch` | `feat/646-gateway` at `b63b9015`, 16 modified + 6 untracked | as expected | Pass | P1 agent | 2026-11-14 | Not landed; no commit |
| S2 | Diff size | The map records the diff stat | `git -C ... diff --stat` | 16 files, 568 insertions, 618 deletions | as expected | Pass | P1 agent | 2026-11-14 | Excludes the 5 untracked files |
| S3 | Coverage | Every parked file is mapped to a phase and an action | Manual read of the inventory table against `git status --porcelain -uall` | 22 paths, one row each | 22 rows | Pass | P1 agent | 2026-11-14 | |
| S4 | Defect D1 | The wrong-order decode is recorded with its location | `sed -n '230,320p' crates/quantrule-server/src/adapters/postgres/mod.rs` | select order and field assignment disagree | confirmed | Pass | P1 agent | 2026-11-14 | P3 owns; not fixed in P1 |
| S5 | Defect D2 | The unregistered migration is recorded | `grep -n "version: 13" crates/quantrule-server/src/migrations.rs` | no match; files 0001-0013 exist | confirmed | Pass | P1 agent | 2026-11-14 | P5 owns; not fixed in P1 |
| S6 | Not landed | No parked hunk is cited as landed evidence | `git -C ... log --oneline -1` | `b63b9015` with no gateway-split commit | as expected | Pass | P1 agent | 2026-11-14 | Reuse only after the owning phase re-tests it |

## P8 disposition (2026-11-15)

- The parked gateway auth and route-module hunks were not copied. The final
  gateway already carries the P3-P7 proxy table, the profile store in `gateway`,
  the static client, and health; P8 renamed the crate and deleted the worker.
- `crates/quantrule-server/tests/boundary.rs` was replaced by
  `crates/quantrule-gateway/tests/service_boundary.rs`, which reads the
  workspace manifests, service sources, and SQL migrations. It covers the
  gateway dependency closure, one binary per service, per-service schema
  ownership, simulation isolation, and a seeded violation per rule.
- The parked `RunStore::count` and migration 13 registration were consumed by
  evidence in P5, and the parked `Timeframe` re-export was retired in P3. The
  final gateway names no data, run, research, or simulation type.
