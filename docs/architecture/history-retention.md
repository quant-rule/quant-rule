# Runtime History Retention

- Status: Current; design for #937
- Scope: runtime history for runs, stages, research, simulation, and datasets
- Claim type: Behaviour Claim

## 1. Boundary

Runtime history is service-owned product data. Both product clients reach it.
Repository evidence (`config/research`, `config/sr*`, docs records) is source
control and stays in `qr-governance history`; it is not a product capability.

## 2. Kinds and owners

| Kind | Owner | Archive | Erase |
|---|---|---|---|
| run | evidence | Yes | Unprotected states only; successful runs refuse |
| stage | evidence | No | Existing `stage delete`, with confirmation |
| research record | evidence | Yes | Open records only; decided records refuse |
| simulation run | simulation | Yes | Draft/marking only; checked and decided refuse |
| dataset | data | Yes | Yes, after archive and zero registered references |
| config | data | No | No; repository evidence |

A dataset erase is a catalog operation, not a cross-service read. Every consumer
registers a reference in the data service before it uses the dataset and releases
it before the consumer row is erased.

## 3. States

A runtime record is `active` or `archived`. An archived record is hidden from a
default list and stays readable with `--include-archived`; erase removes it and
its bytes after a tombstone is written.

Protected states:

- run: `succeeded` with a canonical digest; `queued` and `running` are never
  erasable. A succeeded run referenced by a research record or baseline is
  protected by the evidence owner.
- research: `decided` is protected.
- stage: erase uses the existing `stage delete`; a stage referenced by a run is
  protected by the evidence owner.
- simulation: `checked` and `decided` are protected.
- dataset: erase requires `archived_at` and zero rows in `dataset_references`.

A protected operation returns `history.protected` with the reason. A record in
the wrong state returns `history.state`.

## 4. Tombstones, references, and shared bytes

Every service owns an append-only `retention_event` table:

| Field | Meaning |
|---|---|
| `profile_id` | organisational scope |
| `kind` | run, stage, research, simulation, dataset |
| `record_id` | stable record identity |
| `action` | archive or erase |
| `reason` | stable code |
| `bytes` | bytes removed or retained |
| `created_at` | UTC timestamp |

The event is written in the same transaction as the state change or delete. A
failed delete rolls the event back. The event survives the record.

The data service owns `dataset_references`:

| Field | Meaning |
|---|---|
| `profile_id`, `data_version` | the referenced dataset |
| `reference_id` | stable consumer identity, such as `run:<id>` |
| `kind` | run, stage, research, simulation, baseline |
| `created_at` | UTC timestamp |

Registration is idempotent on `(profile_id, data_version, reference_id)`. A
reference is a promise, not a copy: the consumer still points at the dataset. The
owning service registers it in the same use case that writes the consumer row,
and releases it before erasing that row. On upgrade, each owner reconciles its
current rows once and registers any missing reference; the operation is
idempotent, so a restart is safe.

Artifacts are content-addressed and may be shared by several runs. Evidence does
not delete an object on the strength of one run row. It deletes the run row and
its `artifacts` rows in one transaction, then counts the remaining rows with the
same `sha256`; the object is deleted only when the count is zero. The dataset
catalog follows the same rule across profiles: the row and tombstone commit
first, then the object is deleted only when no catalog row for that object key
remains. A leaked object is safe; a missing shared object is not.

## 5. API

The owning service exposes the operation; the gateway only proxies.

| Method | Path | Effect |
|---|---|---|
| `GET` | existing list endpoint + `include_archived` | list active, or all |
| `POST` | `/runs/{id}/archive` | archive a run |
| `DELETE` | `/runs/{id}` | erase an unprotected run |
| `POST` | `/research/runs/{id}/archive` | archive a research record |
| `DELETE` | `/research/runs/{id}` | erase an open record |
| `DELETE` | `/stages/runs/{id}` | existing stage erase |
| `POST` | `/simulation/runs/{id}/archive` | archive a simulation run |
| `DELETE` | `/simulation/runs/{id}` | erase a draft/marking run |
| `POST` | `/catalog/datasets/{version}/archive` | archive a dataset |
| `DELETE` | `/catalog/datasets/{version}?confirm=<id>` | erase an archived dataset |
| `POST` | `/internal/v1/datasets/{version}/references` | register a consumer reference |
| `POST` | `/internal/v1/datasets/{version}/references/release` | release a reference |
| `GET` | `/internal/v1/datasets/{version}/references` | list references |

The gateway already forwards `/runs/{id}` and the wildcard `/research`,
`/simulation`, `/catalog`, and `/stages` paths. It needs one new route for
`/runs/{id}/archive`. Internal reference routes are service-to-service only.

The clients map each published list summary to one presentation row. No client
aggregates across services; one command is one kind and one gateway call.

## 6. Clients

Shell:

```text
history list --kind <run|stage|research|simulation|dataset|config>
             [--include-archived] [--older-than <date>] [--limit <n>]
history archive --kind <run|research|simulation|dataset> --id <id>
history erase   --kind <run|research|simulation|stage|dataset> --id <id> --confirm <id>
```

`history list` is read-only. `--dry-run` for archive/erase performs the list and
prints the intended operation without calling the mutating endpoint. Erase
requires the exact id as confirmation, and the server checks the same value.

Browser: a History screen with one tab per kind, the same filters, a preview, and
a typed confirmation. Protected rows show the reason and disable the action.

## 7. Safety

- Profile scope comes from the request header; no cross-profile operation.
- No client path is accepted; every operation names an id.
- Archive first, erase second; a tombstone is mandatory.
- No wildcard erase and no automatic agent deletion.
- A dataset with any registered reference refuses erase.
- Dataset insert, archive, erase, and object deletion share a service gate, so
  a concurrent upload cannot observe an object an erase is removing.
- A successful erase is irreversible; the tombstone is the surviving evidence.

## 8. Migration

The data service adds `archived_at`, `dataset_references`, and
`retention_events`. Evidence adds `archived_at` to runs and research records;
artifact counting uses the existing `sha256` column and needs no new relation.
Simulation adds `archived_at` and its own event table. Both Postgres and SQLite
migrations are required. A list sorts by creation time and id; the transaction
locks the row, checks state and references, writes the event, then commits.

## 9. Verification

- Service tests on SQLite and Postgres for state, protection, tombstone, and
  rollback.
- Shell tests for list, archive, erase, refusal, and dry-run.
- Browser parity and typed-confirmation tests.
- A succeeded run, a decided research record, a checked simulation run, and a
  referenced dataset refuse erase; a shared object survives one profile's erase.
- The full gate and composed tier pass.

## 10. Deferred

- Stage archive; stages keep their existing delete.
- Repository evidence in the GUI; it stays in the governance CLI.
- Bulk erase by date in v1; a selection file can follow after the single-id path
  is proven.
- Orphan-object sweeping; an erase can leave a leak, never a missing artifact.
