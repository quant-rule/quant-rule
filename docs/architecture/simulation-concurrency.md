# Simulation Concurrency

- Status: Current for HA-008 (2026-11-22); two simulation replicas are supported only with this contract
- Part of: [`README.md`](README.md)
- Decision: [ADR-HA](../adr/2026-11-22/1082-ha-deployment-profile.md)
- Claim type: Behaviour Claim

Simulation is the interactive research plane. In the HA profile two replicas
serve the same `qr_simulation` database, and the gateway may send two requests
for one run to different replicas. The database is the coordination point. No
replica keeps a write cache or assumes it is the only writer.

## 1. Transaction boundary

Every mutation begins a transaction, locks one `simulation.runs` row
`FOR UPDATE`, checks the status, writes, and commits. Mutations on different runs
do not block each other, and a read is never blocked by a writer.

| Operation | Writes in the same transaction |
|---|---|
| put or delete a mark | mark row; `draft -> marking` |
| replace the oracle | oracle delete and insert; `reviewed` |
| save windows | selection; `draft -> marking` |
| record a check | check result; `checked` |
| decide | decision, note, time; `decided` |
| erase | run row and marks (cascade); retention event |

A mark that arrives after a decision is refused with `409 simulation.locked`;
the mark insert never runs. A decision is terminal: the transaction checks
`decision` and `status` under the lock, and the `UPDATE` repeats
`decision IS NULL AND status <> 'decided'`, so a second writer cannot overwrite
it even if it bypasses the read. Migration 16 adds a check constraint and a
trigger that refuse a changed or cleared decision. Erase takes the same row lock,
so a decided run cannot be erased by a concurrent erase.

## 2. Marks

A mark identity is `(sim_id, origin, chart, bar, action, side)` and the schema
has a unique constraint on it. A duplicate submission is an upsert: the identity
and the server-assigned creation time survive, and quality and note may be
corrected. An identical resubmission leaves the stored row unchanged. An oracle
replacement deletes and inserts in one transaction, so a reader sees the old set
or the new set, never a mix.

Marks list in a fixed order: `chart, bar, action, side, origin`. That order is
part of the API output, not an accident of a query plan.

## 3. Reads

A run read is one statement; its mark counts are subqueries in that statement.
A mark read is one statement. Each sees only committed rows, so a reader sees
the state before or after a mark/decision transaction, never a half-applied one.
The CSV mirror is derived and refreshed after the commit; the database remains
authoritative, and a failed mirror write is logged rather than rolled back into
the mark.

## 4. Two replicas

Two simulation replicas are supported only with this contract (HA-008). The
gateway may send two requests for the same run to different replicas; per-run row
locks make the cases safe:

| Case | Result |
|---|---|
| two decisions | one `201`; the other `409 simulation.locked`; one decision stored |
| mark and decision | the winner commits first; the loser sees `simulation.locked` or `not_found` after erase |
| two marks, different identity | both commit; the full set is preserved |
| duplicate mark | one row; the same `mark_id` is returned |
| erase and decision | the run lock decides; a decided run cannot be erased |
| replica restart | no in-memory state; the database is the only authority |

The v2 multi-host profile still runs one simulation instance. The HA profile's
two replicas use the same application role and the same `qr_simulation`
database.

## 5. Verification

- `cargo test -p quantrule-simulation-service --test concurrency` covers the
  memory repository in the fast gate.
- `tools/postgres_composed.sh` runs the ignored Postgres cases against a
  disposable database: one winner for concurrent decisions, the full mark set
  for concurrent marks, one consistent outcome for a mixed mark/decision race, a
  duplicate-decision conflict, a schema-level decision overwrite refusal,
  rollback of a failed mark transaction, and read isolation from an uncommitted
  mark and decision.
- `cargo test -p quantrule-runtime --test migration_contract` checks the pinned
  simulation ledger and checksum.
- `qr-governance check --all` checks the document links and length caps.

## 6. Non-goals

This contract does not add an evidence leader, active/active data, quorum queues,
the v3 renderer, Swarm, backup/DR, or a simulation queue. It does not make human
marks evidence and it does not add an output digest.
