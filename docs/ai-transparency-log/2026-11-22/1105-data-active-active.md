# AI Transparency Log — #1105 HA-007 Data Active/Active Upload and Catalog Idempotency

- Date: 2026-11-22 (UTC)
- Issue: [#1105](https://github.com/quant-rule/quant-rule/issues/1105)
- Tool: OpenCode implementation subagent
- Boundary: read-only repository review, implementation, tests, and documentation
  for HA-007 only. No commit, push, pull request, merge, tag, board change,
  secret, or deployment.

## Intent and boundary

The work makes two data replicas safe over one `qr_catalog` database and one
object store. The agent read the v1.6.0 plan, ADR-HA, the HA architecture, the
four-database contract, the research-artifact pack registry contract, the data
service catalog/upload/config/pack/artifact code, the Postgres stores, and the
existing conformance seams before changing anything.

The agent did not implement quorum queues, the evidence leader, simulation
concurrency, the v3 renderer, Swarm, MinIO distribution, or backup/DR. The
two-replica claim is limited to the database and object-store contract; the v2
multi-host renderer still places one data replica.

## Implemented

- `quantrule_runtime::adapters::put_if_absent` writes content-addressed bytes
  only when the key is absent and reports whether it wrote. The check is not a
  database compare-and-swap; the data service's object guard supplies the
  cross-replica ordering.
- `DatasetCatalog::insert_or_get` returns the stored row and a `created` flag.
  The Postgres implementation uses `INSERT ... ON CONFLICT DO NOTHING RETURNING`
  and reads the winner, so a duplicate retry is not an error.
- `DatasetCatalog::object_guard` is a transaction-level PostgreSQL advisory lock
  keyed by the object key. Upload takes it before the object write and row
  insert; erase takes it after its row delete and rechecks `object_referenced`
  before deleting bytes. The in-memory catalog mirrors the guard with an owned
  mutex and the same reference check.
- The dataset upload route is object-first: spool, canonicalize, guard,
  `put_if_absent`, `insert_or_get`, release. A failed object write publishes no
  row; a failed row insert leaves an orphan object, which is the safe direction.
- Dataset reference register and release now lock the dataset row `FOR UPDATE`
  in Postgres, as does erase, so two replicas serialise on one row and cannot
  lose or double-count a reference.
- Pack and artifact registration return a `created` flag. Their routes write the
  object first and use the unique key's conflict handling instead of a separate
  existence check, so one duplicate gets `201` and the other `200` over the same
  stored row. A duplicate pack label with different bytes still returns
  `pack.conflict`; a missing object still fails closed.
- Artifact reference register and release lock the artifact row `FOR UPDATE`,
  so a registration cannot interleave with retention collection.
- Catalog migration 9 adds `datasets.object_key` and `artifacts.object_key`
  indexes for the erase guard and sharing checks. It is pinned in
  `deploy/postgres/migration-ledger.json` with checksum `ef04f69c064a8e52`.
- `docs/architecture/data-active-active.md` records the files the data service
  owns, the upload and erase interleavings, the pack/artifact/reference rules,
  and the v2 placement boundary. HA, services, deployment, multi-host,
  history-retention, four-databases, both relevant manual chapters, the
  architecture index, and the changelog link or state the contract.

## Tests

Fast tests:

- `crates/quantrule-data-service/tests/active_active.rs` starts two independent
  `DataState`s over one memory catalog/object store and covers two concurrent
  identical uploads, two concurrent different uploads, an object-write failure
  with no row, an orphan object reused by the next upload, duplicate pack
  registration across two replicas, a different pack for the same label,
  duplicate artifact registration, duplicate dataset references, a concurrent
  erase/upload, and a missing pack object.
- `catalog_conformance`, `pack_conformance`, and `artifact_conformance` add
  concurrent store cases. The same bodies run against memory and, when
  `QR_TEST_DATABASE_URL` is set, Postgres.
- `migrations::tests` and `migration_contract` pin version 9.

Composed tier:

- `tools/postgres_composed.sh` ran 12 targets on the merged tree: the artifact,
  catalog, d1, pack, evidence leader, lineage, outbox, research ledger, stage,
  gateway/profile, migration lock, and simulation targets. All 12 passed,
  including the new concurrent reference and registration cases, migration 9,
  the four-database bootstrap, and the cross-database refusal checks.

Limitations: the concurrent route cases use the memory stores, and the
Postgres cases exercise the stores' concurrency rather than two live container
replicas with S3. The advisory-lock SQL, row locks, conflict handling, and
migration are exercised against a disposable PostgreSQL 17 instance. A real
two-host run remains HA verification, not a claim in this log.

## Verification and evidence

| Check | Observed |
|---|---|
| `cargo fmt --all` | applied; full gate later ran `cargo fmt --all -- --check` and passed |
| `cargo clippy -p quantrule-data-service -p quantrule-runtime --all-targets -- -D warnings` | pass |
| `cargo test -p quantrule-data-service` | pass, including 9 `active_active` cases |
| `cargo test -p quantrule-deploy` | 29 passed, including the data-replica refusal |
| `cargo test -p quantrule-runtime --test migration_contract` | 5 passed |
| `tools/postgres_composed.sh` | 12 targets passed on the merged tree; bootstrap and role boundary passed |
| `cargo run --quiet -p quantrule-governance -- check --all` | `governance check ok` |
| `tools/gate.sh` | `gate passed: full` with `CARGO_TARGET_DIR=…/target/gate`; fmt, workspace clippy, workspace tests, workspace docs, and governance check all passed |

The Postgres cases used the script's disposable container. The real multi-host
HA lab was not run for this log; the two-replica claim is a contract plus a
single-database concurrency result.

## Learning and corrections

- sqlx 0.9 refuses a dynamic SQL string. The first implementation used a
  `format!` to share a column list between `INSERT ... RETURNING` and `SELECT`;
  the compiler's `SqlSafeStr` guard rejected it. The final code uses static
  `const` SQL literals so the query shape stays literal and injection-auditable.
- The memory catalog needed an owned `tokio::sync::MutexGuard` to represent the
  object guard. A borrowed standard mutex guard would have made the boxed future
  non-`Send`; an `OwnedMutexGuard` keeps the trait usable behind `dyn`.
- The first conformance edit asserted that a second `object_referenced` call was
  false before the row was erased. `object_referenced` means "any dataset row",
  not "any reference row"; the assertion moved after the erase.
- The first concurrent reference assertion expected a fixed order of returned
  counts. Row locking serialises the writes but the scheduler decides which
  replica wins, so the test sorts the two counts instead of assuming an order.
- Artifact registration originally did a `get` before the object write. The
  existence check is a race across replicas; moving the status decision into
  the unique-key registration outcome removed it.
- The orphan-object sweep is deliberately not added. An object-store listing
  contract does not exist, and a sweep would be a separate reconciliation
  decision; the safe direction (orphan allowed, missing object refused) is
  documented and tested instead.
