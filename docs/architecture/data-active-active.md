# Data Active/Active Upload and Catalog Idempotency

- Status: Current for HA-007 (2026-11-22); two data replicas are supported only with this contract
- Part of: [`README.md`](README.md)
- Issue: [#1105](https://github.com/quant-rule/quant-rule/issues/1105)
- Decision: [ADR-HA](../adr/2026-11-22/1082-ha-deployment-profile.md)
- Claim type: Behaviour Claim

The data service owns ingestion, the `catalog` schema, and the content-addressed
dataset, ladder, pack, and artifact objects. In the HA profile two replicas
serve the same `qr_catalog` database and object store. The database is the
coordination point; no replica keeps a write cache or assumes it is the only
writer.

## 1. Files the service owns

| Path | Responsibility |
|---|---|
| `crates/quantrule-data-service/src/catalog.rs` | `DatasetCatalog`, `DatasetInsert`, `ObjectGuard`, the memory catalog |
| `crates/quantrule-data-service/src/postgres.rs` | Postgres dataset and config stores, object guard, row locks |
| `crates/quantrule-data-service/src/routes.rs` | upload, inspect, archive, erase, and dataset reference routes |
| `crates/quantrule-data-service/src/upload.rs` | bounded upload spool |
| `crates/quantrule-data-service/src/ingest.rs` | CSV to canonical candle bytes and `data_version` |
| `crates/quantrule-data-service/src/packs/` | pack parse, routes, memory registry, Postgres registry |
| `crates/quantrule-data-service/src/artifacts/` | artifact registration, references, retention |
| `crates/quantrule-data-service/migrations/` | the forward-only `catalog` ledger |
| `qr_catalog` | `catalog` schema only |
| object store | `datasets/`, `ladder/`, `packs/`, `artifacts/` |

The gateway, model, strategy, backtest, evidence, and simulation services never
write these objects or tables.

## 2. Dataset upload

A dataset is identified by the SHA-256 of its canonical candle bytes. The upload
path is:

1. spool the body and parse it into canonical bytes;
2. acquire the database-backed object guard for `datasets/<hash>`;
3. `put_if_absent` the object;
4. `insert_or_get` the `(profile_id, data_version)` row;
5. release the guard;
6. answer `201` when the row was created and `200` with the stored row when it
   already existed.

`insert_or_get` uses the primary key `(profile_id, data_version)` and
`ON CONFLICT DO NOTHING`. The conflict branch waits for the winning transaction
and reads its row, so a retry returns the same `data_version`, the same
`object_key`, and the same metadata. The object is written before the row, so a
failed put publishes no row. A failed row insert leaves an orphan object, which
is the safe direction and is reused by the next identical upload.

The object guard is a transaction-level PostgreSQL advisory lock keyed by a
hash of the object key. It is the cross-replica serialiser; the in-process
`DataState::dataset_gate` only serialises archive and erase inside one process.
Two replicas therefore cannot interleave an upload's put-and-insert with an
erase's delete.

## 3. Erase and the object guard

Erase first commits the row delete, its reference check, and its retention
tombstone in one transaction. It then acquires the same object guard, rechecks
whether any dataset row still names the object, and deletes the bytes only when
none does. The two orders are both safe:

| Interleaving | Result |
|---|---|
| Upload holds the guard first | Upload inserts its row; erase rechecks, sees it, keeps the object |
| Erase holds the guard first | Erase deletes the object; upload waits, then writes the object and inserts its row |
| Upload crashes before its row commit | The guard releases; the object is an orphan; no row is visible |
| Erase's delete fails | The row and tombstone are already committed; the object is an orphan, which is safe |

A row is never published without its bytes while both replicas run this
contract. A rolling upgrade from a pre-contract binary is not active/active;
drain the old replica before starting the new pair.

## 4. Packs and artifacts

Pack registration and computed-artifact registration use the same object-first
rule and a unique metadata key:

| Object | Unique key | Same content | Different bytes, same label |
|---|---|---|---|
| Pack | `(profile_id, pack_id)` and `(profile_id, name, version)` | Idempotent: the stored row is returned | `pack.conflict`; one row survives |
| Artifact | `(profile_id, artifact_kind, artifact_id)` | Idempotent: the stored row is returned | Not applicable; identity is the hash |
| Dataset reference | `(profile_id, data_version, reference_id)` | Idempotent: one row | Not applicable; the id names one promise |
| Artifact reference | `(profile_id, artifact_kind, artifact_id, reference_id)` | Idempotent: one row | Not applicable |

Pack show, validate, freeze, retire, and byte fetch re-read the object and verify
its hash; a missing object fails closed. A duplicate label is refused after the
object is written; the losing object is an orphan and no second row is
published. The pack and artifact routes do not delete the bytes they registered,
so an orphan is a storage leak, not a lost artifact.

## 5. References and erase

A dataset reference is registered with `INSERT ... ON CONFLICT DO NOTHING` and
released with an exact-id delete. The Postgres store locks the dataset row
`FOR UPDATE` for register, release, and erase, so concurrent replicas serialise
on one row:

- two registrations of one id return one row;
- two different ids return two rows and an exact final count;
- register versus erase is decided by the lock: erase sees the reference and
  refuses, or registration sees no row and returns `dataset.not_found`;
- release versus erase cannot lose a reference or double-count one.

A dataset with any reference refuses erase with `history.protected`. A shared
object survives one profile's erase because the erase transaction and the
post-commit recheck both count every remaining row.

## 6. Placement

Compose and the `quantrule.deploy/v2` multi-host profile run one data replica;
the v2 validator refuses a replica count other than one. Two data replicas are
an HA-profile placement and require this contract. The v3 renderer, Swarm,
MinIO distribution, quorum queues, the evidence leader, simulation concurrency,
and backup/DR are separate HA leaves.

## 7. Verification

- `cargo test -p quantrule-data-service --test active_active` covers two
  replicas over one memory catalog: identical upload, different uploads,
  object-first failure, duplicate pack and artifact registration, duplicate
  reference registration, and an erase/upload race.
- `cargo test -p quantrule-data-service --test catalog_conformance`,
  `--test pack_conformance`, and `--test artifact_conformance` run the shared
  store bodies against memory and, when `QR_TEST_DATABASE_URL` names a
  disposable database, Postgres.
- `tools/postgres_composed.sh` runs those ignored Postgres cases, the bootstrap,
  and the role boundary.
- `cargo test -p quantrule-deploy a_second_data_replica_is_rejected_by_v2`
  proves the v2 placement boundary.
- `cargo test -p quantrule-runtime --test migration_contract` pins catalog
  migration 9, the object-key indexes, and every checksum.
- `tools/gate.sh` runs the whole workspace: fmt, clippy, tests, docs, and the
  governance checkers.

## 8. Non-goals

- No quorum queue, evidence leader, simulation concurrency, v3 renderer, Swarm,
  MinIO distribution, or backup/DR in this slice.
- No background orphan-object sweep. An orphan is safe and is reused by the next
  identical upload; a sweep requires an object-store listing contract and is a
  later reconciliation decision.
- No change to the client commands or screens.
