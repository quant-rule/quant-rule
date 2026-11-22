# AI Transparency Log — RA-006 Pack Registry, Lineage, Retention and GC

- Date: 2026-11-22 (UTC)
- Work item: [#1094](https://github.com/quant-rule/quant-rule/issues/1094)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017
- Plan: [`docs/execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md`](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md)
- Branch: `feat/1094-pack-registry`

## Intent and boundary

Implement RA-006 as the storage and use-case layer for research artifacts: the
persistent pack registry in the `catalog` schema, the run lineage record in
evidence, and the generic computed-artifact reference and retention seam. The
work covers schema, lifecycle, object-store identity, internal routes and tests.

This entry does not claim the strategy service, signal computation, run-v5
catalog/worker reachability, the signal-artifact writer, a shell command or a
browser screen. Those remain RA-007, RA-008, RA-009 and later integration work.
The execution, portfolio and model engines and the run v4 loader are unchanged.

## Work performed

- Moved the reference vocabulary (`ReferenceKind`, `ContentReference`) and the
  pack lifecycle vocabulary (`PackKind`, `PackState`) into `quantrule-api`, so
  the parser, registry, lineage and wire contract share one definition. The
  application service re-exports them; `ContentReference::parse` became
  `pack::references::parse_content_reference` because the type now lives in the
  contract crate.
- Added `quantrule-api` contract types for packs (`PackSummary`, `PackRecord`,
  validation and lifecycle records), computed artifacts and references, and run
  lineage. Added stable error codes `pack.*`, `artifact.*` and
  `run.lineage_not_found`.
- Added catalog migration 7: `packs`, `pack_validation_records` and
  `pack_lifecycle_events`. Registration canonicalises through the existing
  application-service parser, writes the canonical bytes to the object store
  under `packs/<hash>` first, then commits the metadata row and its creation
  event. The row records the object key, hash and size; the catalog holds no
  byte payload.
- Added catalog migration 8: `artifacts` and `artifact_references`. The
  retention pass uses the existing `catalog.retention_events` as its tombstone,
  collects only unreferenced rows at or before the cutoff, and reports shared
  objects so the route does not delete another profile's bytes.
- Added data-service pack routes (`/catalog/packs`, `/catalog/packs/{id}`,
  `.../bytes`, `.../validate`, `.../freeze`, `.../retire`) and internal show and
  byte routes. Added internal artifact routes for registration, show, list,
  byte fetch, reference add/release/list, and the retention pass.
- Added evidence migration 20: `runs.run_lineage` and
  `runs.run_lineage_references`. Added the lineage store, the internal
  `PUT`/`GET /internal/v1/runs/{id}/lineage` route, and pack provenance as a
  stored JSON document.
- Added memory and Postgres stores for packs, artifacts and lineage, route
  contract tests, conformance suites for both stores, and the composed-tier
  targets.
- Pinned catalog migrations 7 and 8 and evidence migration 20 in
  `deploy/postgres/migration-ledger.json`.
- Updated the config contract, research-artifacts architecture, history
  retention, cross-database consistency, module map, index status, changelog and
  this log.

## Design decisions for review

- The object store is the single pack byte authority. The catalog row stores
  only the object key, content hash and size, so the registry cannot grow a
  second copy and cannot serve a stale one. Registration writes the object
  first; a failed metadata commit leaves an orphan object, which is safe and
  reconcilable, never a row without bytes.
- Show, byte fetch, validate, freeze and retire read the object by its key and
  verify the hash and size before answering. A missing or corrupt object fails
  closed; there is no database fallback.
- A pack starts `draft` even though creation already ran the schema validator.
  The explicit validate route re-reads the stored bytes and proves the stored
  identity before it advances the row, which keeps "validated" a registry fact
  rather than a parse-time assumption.
- Freeze and retire are idempotent by returning the stored row without a second
  event. A repeated freeze therefore does not move `frozen_at` or add history.
- The pack label is unique per profile, not globally. Two profiles may cite the
  same content hash, matching the dataset catalog's profile-scoped row and
  shared object.
- Run lineage is recorded through an internal route rather than from the current
  run submission path. The data-service config scan still rejects a v5 config
  because it has no reference resolver, so wiring submission would claim
  reachability that does not exist. The route is the seam the later run-v5
  integration calls.
- The artifact retention window is a named 30-day constant in this slice.
  Making it operator-tunable is a deployment/config decision for the later
  signal work, and the pass contract does not change with it.
- A malformed pack upload is refused without a row; a validation record is
  written only for a successful validate operation. A future validator with a
  failing outcome can extend the record shape without changing this schema.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo test -p quantrule-api` | 85 pass |
| `cargo test -p quantrule-app` | All targets pass, including the v5 identity and pack suites |
| `cargo test -p quantrule-data-service` | All targets pass, including the pack, artifact and route suites |
| `cargo test -p quantrule-data-service --test packs` | 7 pass; missing and tampered pack objects fail closed on show, fetch, validate, freeze and retire |
| `cargo test -p quantrule-evidence-service` | All targets pass, including the lineage suite |
| `cargo test -p quantrule-runtime --test migration_contract` | 5 pass |
| `cargo test --workspace --no-fail-fast` | Pass; no failed target |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | Pass |
| `cargo run -q -p quantrule-governance -- check --all` | `governance check ok` |
| `tools/postgres_composed.sh` | 10 targets pass against a disposable Postgres, including pack, artifact and lineage conformance |
| `tools/gate.sh` | `gate passed: full` |

## Learning and corrections

- Review blocked the first registry because it stored the canonical pack bytes
  in `catalog.pack_files` and in the object store. That created two authorities
  and a payload the catalog did not need. The byte table is gone; the catalog
  stores metadata only, and every read verifies the object. This is corrected in
  the migration, store, routes, conformance tests and docs.
- The first pack/artifact migration checked a 64-character bare-hex id while
  the store binds `Digest` in its 71-character `sha256:` wire form, and the
  artifact insert omitted the `sha256` column. The review pass aligned the
  migration checks and the binds before the composed run; the schema now rejects
  a row whose id and hash disagree.
- The first artifact list route was mounted on `/{kind}` and shadowed the static
  `/collect` route; the test found a 404 where an invalid filter should be a
  400. The list is now on the bare `/internal/v1/artifacts` path.
- The first lineage test assumed caller order; the store deliberately sorts by
  `ReferenceKind` so two callers that send the same set in different orders get
  the same record and the idempotency comparison is stable.
- Two SQLx dynamic `format!` queries were rewritten as fixed SQL. SQLx 0.9
  refuses a dynamic string through the audited `query_as` path, and the fixed
  queries also keep the retention statement readable.

## Residual risk

The pack routes are reachable through the existing gateway `/catalog` proxy, but
no shell command or browser screen calls them. A failed pack metadata commit can
leave an orphan object under `packs/<hash>`; that is the safe direction and no
reconciler sweeps it in this slice. The run-v5 submission, catalog
scan and backtest worker still do not resolve a v5 config, so lineage has no
production caller yet beyond its internal route. The signal-artifact writer and
its reference registration remain RA-007/RA-008. No strategy, model or edge
claim is made by this entry.
