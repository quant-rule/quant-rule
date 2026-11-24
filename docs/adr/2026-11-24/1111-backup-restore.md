# ADR-HA-013: pgBackRest, the Derived Patroni Image, and the Compose Archive

- Date: 2026-11-24 (UTC)
- Status: Accepted
- Issue: [#1111](https://github.com/quant-rule/quant-rule/issues/1111)
- Decision owner: Human
- Related: [ADR-HA](../2026-11-22/1082-ha-deployment-profile.md),
  [backup contract](../../config/2026-11-24/backup-restore.md),
  [HA architecture](../../architecture/ha.md)

## Context

One Patroni cluster holds four logical databases. The database tier has strict
synchronous replication and RPO 0 for committed rows, but no archive: the
Patroni config writes WAL to a node-local placeholder, and the backup tools dump
one `quantrule` database that no longer owns the service schemas. The pinned
Patroni image has no pgBackRest. A host loss therefore loses the archive, and a
database corruption has no point-in-time recovery.

The Compose stack is a single-host deployment with no node_exporter. The v3
profile has node_exporter on every declared host. A backup signal must exist on
the production profile without pretending the Compose path has one.

## Decision

1. Use one `quantrule` stanza for all four databases, because they share one
   cluster and one recovery point.
2. Use an operator-provided off-host S3 repository as the checked-in reference
   in the sample and both reference descriptors. Fall back to a dedicated
   encrypted posix path on `backup.host` when no off-host endpoint exists.
   Permit the in-cluster MinIO only for `lab-single-host` and label it
   `not-off-host`.
3. Derive `ghcr.io/quant-rule/patroni-pgbackrest` from the pinned
   `batonogov/patroni-docker` digest and install `pgbackrest` there. Keep the
   `images.patroni` key; do not add a required image key. Until the operator
   builds and pushes it, use a labelled fixture digest that release mode refuses.
4. Set `archive_mode=on`, `archive_timeout=60`, `wal_level=replica`, and the
   stanza `archive-push`/`archive-get` commands on every node. Remove the
   node-local WAL path.
5. Encrypt with AES-256-CBC by default; `none` is accepted only outside release
   mode. Read the passphrase from a mode-600 file into the process environment or
   standard input, never a rendered config, argument, or log.
6. Keep the repository options in `backup.repo`, `backup.cipher`,
   `backup.schedule`, `backup.host`, and `backup.object_archive`; default to
   posix, AES-256, 02:00/14:00 UTC, `swarm.managers[0]`, and archive disabled.
7. Render `pgbackrest.conf` per Patroni node and one backup/restore script for
   `backup.host`. The script runs `pgbackrest` inside the chosen Patroni
   container, writes the age textfile only after a real full/diff success, and
   never writes it on a dry-run or failure.
8. Use a host systemd timer or cron; no scheduler runs in a container.
9. Archive all four Compose databases one dump each. A restore refuses a partial
   set, a changed payload, an extra file, a missing sidecar, and a non-clean
   target.
10. Make `BackupAgeHigh` fire when the textfile is absent or older than eight
    days, and add a Backup & DR row to an existing dashboard.
11. Give the stage the same lease fence and a stage outbox, so a rebuilt queue
    re-arms the exact unfinished stage set. The queue is not archived; the run
    and stage outboxes are the only replay path.
12. Keep the live backup, verify, restore, PITR, object archive, queue rebuild,
    empty-broker drill, and DR drill `Unverified` in this decision. `Real HA`
    remains HA-014; a scratch drill row is `Measured` only for the operations
    run, and the three-VM lab is `Simulated HA` at most.

## Alternatives considered

| Alternative | Consequence | Rejected because |
|---|---|---|
| Keep node-local WAL | Simple | A host or disk loss destroys the archive; it is not a backup |
| One stanza per database | Independent restore | One cluster, one recovery point; four repositories multiply retention and drill work |
| In-cluster MinIO as the production repository | No off-host step | One host loss takes the database and its backup together |
| Four separate PostgreSQL clusters | Isolation | Four times the etcd, Patroni, HAProxy, PITR, and split-brain surface |
| Host pgBackRest package on every node | No derived image | The pinned base does not contain it and an operator package drifts from the image |
| In-container scheduler | One unit | A scheduler is hidden state; a host timer is visible and testable |
| Object archive in phase 1a | One delivery | The archive tool and drill are phase 3; phase 1a only renders and checks the contract |

## Consequences

Positive:

- One stanza, one repository, one schedule, and one age signal for four
  databases.
- The archive survives a database-host loss when the repository is off-host.
- The derived image is reproducible from a pinned base and a reviewed
  Dockerfile.
- The Compose archive is complete and fails closed on a partial set.
- The operator sees a stale or missing backup; a missing textfile no longer
  looks healthy.

Negative:

- The operator must build, push, and pin the derived image before release.
- The backup host needs SSH/sudo to the Patroni nodes when it is not one.
- The S3 repository needs an account and a CA bundle; the lab account is
  operator-provisioned.
- The archive is still a single off-host copy until phase 3 adds the object
  archive and a second-site decision. Phase 3 adds the one-way archive and the
  DR drill; it does not add a second site, object lock, or a replication claim.
- The live drill, RPO/RTO evidence, and `Real HA` remain outstanding.

## Verification

`cargo test -p quantrule-deploy` covers the descriptor defaults, validation,
release blockers, render goldens, check rows, the Compose scratch archive, the
object archive plan/refusals, the DR plan/gates, the alert expression, and the
dashboard. `qr-governance check --all` enforces the doc caps. A live repository,
backup, verify, restore, PITR, archive, or DR drill was not exercised;
`Simulated HA` is the maximum and `Real HA` is HA-014.
