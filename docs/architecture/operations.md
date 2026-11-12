# Operations

- Status: Target v1.0.0; approved 2026-11-12
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

`GET /health` is the only health authority. It reports each service, queue depth,
and worker count. A service is healthy when its check passes, not when its
container is running.

| Signal | Healthy | Action |
|---|---|---|
| `gateway` reachable | yes | none |
| `postgres` reachable | yes | restart; data survives |
| object store reachable | yes | restart; objects survive |
| queue reachable | yes | restart; durable messages replay |
| workers ≥ 1 | yes | `make scale N=1` |
| queue depth growing | no | add workers or investigate a stuck run |

## 3. Run lifecycle operations

| Situation | Operator action |
|---|---|
| Run `queued` too long | Check worker count and queue depth |
| Run `running` too long | Check the stage in run detail; cancel if needed |
| Run `failed` | Read the failed stage and error code; resubmit after fixing the input |
| Dead-lettered run | Inspect the attempt count; the run row is the record |
| Stuck `running` after a crash | Restart the worker; finalization retries |

## 4. Backup

```bash
make backup                 # writes an archive: postgres dump + object mirror
make restore FILE=…         # restores into a clean stack
```

Restore verifies each artifact hash. A restore that fails verification stops and
reports the mismatched object.

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
- No cron, no daemon, no scheduled job. The system is request-driven.
