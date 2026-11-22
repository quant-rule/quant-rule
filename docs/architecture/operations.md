# Operations

- Status: Current; released as v0.9.0 on 2026-11-15 (prepared as v1.0.0); unchanged in the v1.0.0 desktop release; approved 2026-11-12
- Part of: [`README.md`](README.md)
- Claim type: Behaviour Claim

## 1. Daily use

```bash
make up      # start
make ps      # status
make logs    # follow
make down    # stop, keep data
```

## 2. Health

`GET /health` is the local liveness probe. `GET /ready` is the dependency
authority: it aggregates the configured upstream services and returns `503`
when one is down or answers with a different `service` identity. A service is
healthy when its check passes, not when its container is running.

| Signal | Healthy | Action |
|---|---|---|
| `gateway` reachable | yes | none |
| `postgres` reachable | yes | restart; data survives |
| `data`, `model`, `evidence`, `simulation` reachable | yes | restart the named service |
| object store reachable | yes | restart; objects survive |
| queue reachable | yes | restart; durable messages replay |

Queue depth and worker count remain in the report for the control plane; they
are currently published as zero and are not an operational signal.

## 3. Run lifecycle operations

| Situation | Operator action |
|---|---|
| Run `queued` too long | Check worker count and queue depth |
| Run `running` too long | Check the stage in run detail; cancel if needed |
| Run `failed` | Read the failed stage and error code; resubmit after fixing the input |
| Dead-lettered run | Inspect the attempt count; the run row is the record |
| Stuck `running` after a crash | Restart the worker; finalization retries |

## 4. Backup and restore

One archive holds both durable stores:

```bash
tools/backup.sh                  # optional --out DIR
tools/restore.sh ARCHIVE --yes   # clean stack; --force replaces
```

`make backup` and `make restore FILE=…` are the #644 shortcuts and call these
scripts.

| Piece | Where |
|---|---|
| Default archive directory | `$QR_BACKUP_DIR`, else `$QUANTRULE_DATA_ROOT/backups`, else `$HOME/quant-rule-data/backups` |
| Archive | `quantrule-YYYY-MM-DDTHH-MM-SSZ.tar.gz` |
| Sidecar | The same name plus `.sha256`: the SHA-256 of the archive |
| Manifest | `manifest.json` inside the archive: schema, one line per payload file with size and SHA-256, and the object key of each mirrored object |
| Payload | `postgres/quantrule.dump` (custom format) and `objects/<key>` |

The mirror is staged inside the object-store container and again on the host,
so the backup host needs free space for the archive plus one mirror.

The application services are stopped while the dump and mirror run, so the two
stores describe one instant; Postgres, the object store, and the broker stay
up. The queue is not archived: a message is derived from a run row and is
rebuilt. Backup briefly takes `/health` down and restarts the services that were
running on exit, including after a failure.

`restore.sh` verifies the sidecar, the archive entry list, and every payload
hash before it starts a write. It then refuses a non-empty database or bucket
unless `--force` is given, and refuses a non-interactive run without `--yes`.
Objects are written before rows, so an interrupted restore leaves unreferenced
blobs rather than rows that point at missing bytes. A corrupt, incomplete, or
sidecar-less archive is refused with the mismatched path. After a clean
`make reset`, restore before `make up`; a stack that has already migrated needs
`--force` or a reset.

The object mirror uses the `mc` bundled in the pinned MinIO image. A replacement
`QR_OBJECT_STORE_IMAGE` must provide `mc`, or backup stops with a clear error.
The archive contains candle data and is handled as sensitive; see
[`data-lifecycle.md`](../workflows/data-lifecycle.md).

The default stack starts all six services, three stores, one-shot database
bootstrap, and four one-shot migration jobs (see
[`deployment.md`](deployment.md) section 2.1). The four logical databases are a
current deployment contract
([`docs/config/2026-11-22/four-databases.md`](../config/2026-11-22/four-databases.md)).
Backup covers the durable stores; it cannot contain state that a missing
`data`, `model`, or `simulation` binary never produced. The scripts stop every
non-store service in the project.

The current backup tools dump one database. Until the four-database archive
lands, a backup after the split is incomplete and a restore drill must be
re-run; do not claim evidence preservation from a single dump.

Round trip: `tools/test_backup.sh` is opt-in and uses an isolated
`quantrule-backup-test` project, so it needs port 8080 free and never touches
the default project. It creates a dataset, config, and hypothesis row through the
gateway, seeds a run with artifact blobs, checks the refusals, wipes the
volumes, restores, and compares every row digest and object hash. Record the
archive digest in the run note for a long campaign.

## 5. Upgrade

1. Record the current image digest.
2. `make down`
3. `docker compose pull` or rebuild.
4. `make up`
5. `make verify` and compare with the recorded digest.

## 6. Incident notes

| Incident | First response |
|---|---|
| Disk full | Stop workers, free object storage, restart |
| Database corruption | Restore from the last backup; record the data loss window |
| Object store loss | Restore from backup; any missing artifact makes its runs unverifiable |
| Queue loss | Rebuild from run rows in `queued`; resubmit those runs |
| Image regression | Roll back to the previous digest; record the failure |

The last verified backup and its digest belong in the run note for any long
campaign.

## 7. Housekeeping

- Unreferenced objects are harmless; no garbage collector is required.
- Logs rotate per container; the host keeps no unbounded log.
- Stage retention sweeps at startup and hourly; every other action is
  request-driven.
