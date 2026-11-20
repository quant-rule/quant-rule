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
| run | evidence | Yes | Protected states only |
| stage | evidence | No | Existing `stage delete`, with confirmation |
| research record | evidence | Yes | Protected states only |
| simulation run | simulation | Yes | Draft/marking only |
| dataset | data | Yes | **No in v1**; references cross services |
| config | data | No | No; repository evidence |

Dataset erase is deferred until a reference registry exists. A dataset can be
archived, which hides it from default lists and keeps every byte.

## 3. States

A runtime record is `active` or `archived`. Archived records are hidden from a
default list and remain readable with `--include-archived`. Erase removes the
record and its bytes after a tombstone is written.

Protected states:

- run: `succeeded` with a canonical digest; `queued` and `running` are never
  erasable. A succeeded run referenced by a research record or baseline is
  protected by the evidence owner.
- research: `decided` is protected.
- stage: erase uses the existing `stage delete`; a stage referenced by a run is
  protected by the evidence owner.
- simulation: `checked` and `decided` are protected.
- dataset: archive only.

A protected operation returns `history.protected` with the reason. A record in
the wrong state returns `history.state`.

## 4. Tombstones

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

## 5. API

The owning service exposes the operation; the gateway only proxies.

| Method | Path | Effect |
|---|---|---|
| `GET` | existing list endpoint + `include_archived` | list active, or all |
| `POST` | `/runs/{id}/archive` | archive a run |
| `DELETE` | `/runs/{id}` | erase an unprotected run |
| `POST` | `/research/runs/{id}/archive` | archive a research record |
| `DELETE` | `/research/runs/{id}` | erase an unprotected record |
| `DELETE` | `/stages/runs/{id}` | existing stage erase |
| `POST` | `/simulation/runs/{id}/archive` | archive a simulation run |
| `DELETE` | `/simulation/runs/{id}` | erase a draft/marking run |
| `POST` | `/catalog/datasets/{version}/archive` | archive a dataset |

The gateway already forwards `/runs/{id}` and the wildcard `/research`,
`/simulation`, `/catalog`, and `/stages` paths. It needs one new route for
`/runs/{id}/archive`.

The clients map each published list summary to one presentation row. No client
aggregates across services; one command is one kind and one gateway call.

## 6. Clients

Shell:

```text
history list --kind <run|stage|research|simulation|dataset|config>
             [--include-archived] [--older-than <date>] [--limit <n>]
history archive --kind <run|research|simulation|dataset> --id <id>
history erase   --kind <run|research|simulation|stage> --id <id> --confirm <id>
```

`history list` is read-only. `--dry-run` for archive/erase performs the list and
prints the intended operation without calling the mutating endpoint. Erase
requires the exact id as confirmation.

Browser: a History screen with one tab per kind, the same filters, a preview, and
a typed confirmation. Protected rows show the reason and disable the action.

## 7. Safety

- Profile scope comes from the request header; no cross-profile operation.
- No client path is accepted; every operation names an id.
- Archive first, erase second; a tombstone is mandatory.
- No wildcard erase and no automatic agent deletion.
- Dataset erase is not offered; dataset archive keeps the bytes.
- A successful erase is irreversible; the tombstone is the surviving evidence.

## 8. Migration

Each service adds `archived_at` to the owned record table and a
`retention_event` table. Both Postgres and SQLite migrations are required. A
list query sorts deterministically by creation time and id. The archive/erase
transaction locks the record, checks its state and references, writes the event,
and then changes or removes the row.

## 9. Verification

- Service tests on SQLite and Postgres for state, protection, tombstone, and
  rollback.
- Shell tests for list, archive, erase, refusal, and dry-run.
- Browser parity and typed-confirmation tests.
- A completed run cannot be erased; a decided research record cannot be erased;
  a checked simulation run cannot be erased; a dataset cannot be erased.
- The full gate and composed tier pass.

## 10. Deferred

- Dataset erase with a cross-service reference registry.
- Stage archive; stages keep their existing delete.
- Repository evidence in the GUI; it stays in the governance CLI.
- Bulk erase by date in v1; a selection file can follow after the single-id path
  is proven.
