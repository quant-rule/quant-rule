# Operations

- Status: Current; released as v0.9.0 on 2026-11-15 (prepared as v1.0.0); runtime drain, readiness, retries, file secrets, token rotation, the PLAT-003 stateful tier, check rows, and failover lab, the PLAT-004 queue tier, checks, definitions, and broker-loss lab, the PLAT-005 MinIO tier/object bootstrap/check rows/node-loss lab, the PLAT-006 observability tier, the PLAT-007 registry/signing/SBOM/CVE phases 1–2 (running task images and the all-service digest probe), and HA-013 phase 1a pgBackRest/Compose four-database archive/backup age, phase 2 stage lease/outbox and queue rebuild, and phase 3 object archive/DR/lab integration are in v1.6.0; approved 2026-11-12
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

`GET /health` is the local liveness probe; it stays `200` while the process is
alive. `GET /ready` is the routing authority: it returns `503` when a dependency
is down, when a response has a different `service` identity, or immediately when
the process is draining. The gateway probes each upstream `/ready`; the backtest
worker exposes both routes on its internal listener. A service is healthy when its check passes, not when its container is running. The v3 Swarm check is [manual 15](../manual/15-ha-deployment.md); its ingress is the [Traefik/VIP contract](../config/2026-11-23/ingress.md); its database tier is the [Patroni/etcd/HAProxy contract](../config/2026-11-23/patroni-cluster.md); its rolling update is stop-first when no spare slot exists, per [swarm-control](../config/2026-11-23/swarm-control.md); rotate its token by a new secret name and rolling update, not `SIGHUP`. `--remote docker --overlay` inspects the live Swarm and stateful rows; the ingress rows prove the certificate/headers/port-80 closure/mounts; `tools/test_tiers.sh ha` drains a worker and the VIP holder, stops a manager, and pauses the Patroni primary; HA-014 adds the 23-row [failure matrix](../config/2026-11-24/chaos-matrix.md) and the opt-in `--chaos` worker/leader/gateway/etcd cases; the 2026-11-25 lab records 13 `Simulated HA` rows, while HA-22/HA-23 remain `Unverified`.

| Signal | Healthy | Action |
|---|---|---|
| `gateway` reachable | yes | none |
| `postgres` reachable | yes | restart; data survives |
| `data`, `model`, `evidence`, `simulation` reachable | yes | restart the named service |
| object store reachable | yes | restart; objects survive |
| queue reachable | yes | reconnect; quorum messages survive one broker loss |

### Metrics, logs and request ids

Each service serves `/metrics` on `QR_METRICS_BIND`, on the internal network
only; the gateway never proxies it and no host port is published. Logs are one
JSON object per line on stderr unless `QR_LOG_FORMAT=text`. Every gateway
response carries `x-request-id`, and a valid W3C `traceparent` is forwarded.
In v3, Prometheus scrapes the services and stateful endpoints, routes 38 rules over F01..F17 to one internal `quantrule-null` receiver, and ships four read-only dashboards; `tools/observability.sh` starts and verifies the tier, `qr-deploy check` runs the 28 static observability rows, and the `ha` lab asserts the manager/gateway alerts fire and resolve. These fields are observability, never evidence or a run identity.

`quantrule_ready` is `0` while draining and `quantrule_draining` is `1`. A
`SIGTERM` waits up to 20 s for HTTP and 25 s for a worker; `stop_grace_period:
40s` bounds the container. `quantrule_http_retries_total` and
`quantrule_consumer_reconnects_total` count retries and reconnects; a drain
never writes `failed` for an in-flight run. A token rotation re-reads the file on
`SIGHUP` or the next `QR_TOKEN_RELOAD_SECONDS` (default 5); the last good token
survives an invalid edit and `quantrule_auth_reload_failures_total` counts the
reason. Application secrets are staged from mode-600 mounts for UID 10001; see
[file-backed secrets](../config/2026-11-23/secret-files.md).

```bash
docker compose exec -T gateway curl -fsS http://127.0.0.1:9090/metrics
docker compose logs --tail 20 gateway
```

### Queue generation and dead letters

The work queues are generated quorum queues (`runs.backtest.g<n>`,
`runs.evidence.g<n>`, `stages.worker.g<n>`) with a three-redelivery limit and a
dead-letter exchange; `runs.dead-letter.g<n>` is the quorum dead-letter queue.
Inspect with `docker compose exec -T rabbitmq rabbitmqctl list_queues ...`, or
in v3 `docker exec -u rabbitmq quantrule-rabbitmq-<host> rabbitmqctl ...`.

A message that exhausts the limit is in the dead-letter queue with its original
routing key and an `x-death` header; the run row remains the record. A changed
queue declaration moves to a new `QR_QUEUE_GENERATION`: drain the old
generation, stop its consumers, then unbind or delete the empty queues. In v3,
three brokers sit behind the HAProxy `rabbitmq` alias; a consumer rebuilds its
connection from the stored URL/CA and uses `restart_policy: any`.
`tools/queue-definitions.sh` exports stripped topology, verifies the four
queues/bindings/no policy, and retains 30 days. `tools/queue-rebuild.sh --plan`
prints the recovery; `--confirm` imports into an empty broker, recreates the
user/vhost, and restarts evidence so the run and stage outboxes re-arm. The queue
is not archived; inspect DLQ messages, never replay them.
### Evidence leader

`runs.leader_lease` names the one finalizer; the leader renews every 10 s with a
30 s database window. A standby serves reads and acquires after expiry. Inspect
`SELECT holder, term, expires_at > now() AS live FROM runs.leader_lease;`.
A leader kill or a clean drain expires the lease; the standby takes over,
re-arms the run and stage outboxes, and logs `this evidence replica is the leader`.
## 3. Run lifecycle operations

| Situation | Operator action |
|---|---|
| Run `queued` too long | Check worker count and queue depth |
| Run `running` too long | Check the stage in run detail; cancel if needed |
| Run `failed` | Read the failed stage and error code; resubmit after fixing the input |
| Dead-lettered run | Inspect the attempt count; the run row is the record |
| Stuck `running` after a crash or drain | Restart the worker; the run or stage lease expires and its outbox re-arms it; never mark it failed |
## 4. Backup and restore

One archive holds both durable stores:

```bash
tools/backup.sh                  # optional --out DIR
tools/restore.sh ARCHIVE --yes   # clean stack; --force replaces; --check only verifies
```

`make backup` and `make restore FILE=…` call these scripts.

| Piece | Where |
|---|---|
| Default archive directory | `$QR_BACKUP_DIR`, else `$QUANTRULE_DATA_ROOT/backups`, else `$HOME/quant-rule-data/backups` |
| Archive | `quantrule-YYYY-MM-DDTHH-MM-SSZ.tar.gz` |
| Sidecar | The same name plus `.sha256`: the SHA-256 of the archive |
| Manifest | `manifest.json` inside the archive: schema, one line per payload with size and SHA-256, `kind=database` with its name, and the object key of each mirrored object |
| Payload | One `postgres/qr_*.dump` per logical database (custom format) and `objects/<key>` |

The mirror is staged in the object-store container and on the host. Application
services stop during the dump and mirror, so both stores describe one instant;
Postgres, MinIO and the broker stay up; acknowledged queue messages stay in the
quorum queues, and a lost queue is rebuilt from run and stage rows. The broker topology is
exported separately with `tools/queue-definitions.sh export`; users and
permissions are stripped before storage; a definitions file is not a message backup.
The command pair is in the [multi-host runbook](../../deploy/multi-host/README.md) §11.

`restore.sh` verifies the sidecar, entry list and payload hashes before writing.
It refuses a non-empty database or bucket without `--force` and a non-interactive
run without `--yes`. Objects are written before rows, so an interrupted restore
leaves unreferenced blobs rather than missing bytes; after `make reset`, restore
before `make up`.

The mirror uses the `mc` bundled in the pinned MinIO image; v3 pins `images.minio_client`, enables versioning, and the check/lab verify the bucket, `qr_app`, the probe hash, and zero healing. HA-013 phase 3 adds `tools/object-archive.sh` (one-way, read-only source, no delete/watch/replicate) and `deploy/v3/dr.sh` (scratch or in-place lab restore, canonical digest, RPO/RTO); the 2026-11-25 in-place lab drill is `Measured`; the archive remains one-way ([backup contract](../config/2026-11-24/backup-restore.md)).
In v3, `render` writes `pgbackrest-<host>.conf` and `pgbackrest-backup.sh`; the backup host writes the age textfile only after a real full/diff, and `BackupAgeHigh` fires when it is absent or eight days old.

The default stack starts seven services, three stores, the bootstrap, and four migration jobs ([deployment.md](deployment.md) §2.1). The database contract is [four databases](../config/2026-11-22/four-databases.md); the v3 repository, stanza, retention, and check rows are [backup and restore](../config/2026-11-24/backup-restore.md).
The Compose tools dump all four databases; the v3 pgBackRest path is separate and
its 2026-11-25 in-place backup/restore/PITR drill is `Measured`.

Round trip: `tools/test_backup.sh` is opt-in, uses an isolated project, and never
touches the default project. It seeds the four databases and objects, checks the
refusals, wipes, restores, and compares row digests and object hashes.

## 5. Upgrade

1. Record the current image digest and the canonical digest from `make verify`.
   Keep the previous image loaded and the previous volumes.
2. `make down`
3. Load or pull the new image digest.
4. `make up`: `pki-init` stages the certificates, `db-bootstrap` runs, then the
   four one-shot migration jobs, then each service waits for its own job.
5. `make verify`; do not retire the previous image until the fixture and digest pass.
6. A failed migration job leaves its service down. Do not bypass it with
   `QR_MIGRATE=1`; correct forward with a new migration version. Rollback starts
   the previous digest without the job; the schema is forward-only.

The full migration contract and runbook are in [migrations.md](migrations.md).

The release image is pulled by digest from GHCR and verified as a Sigstore bundle plus saved signed image before deploy; `qr-deploy check` compares running task images and `--runtime` proves seven service digests; SBOM/CVE are release assets ([registry contract](../config/2026-11-24/registry.md)). Reference sizing, lab preconditions, and the four remaining image blockers (app, `minio`, `minio_client`, derived Patroni) are in the [reference contract](../config/2026-11-24/reference-topologies.md); `deploy/v3/lab.sh` fails closed at those blockers and runs the remote client phase from outside the Swarm.

## 6. Incident notes

| Incident | First response |
|---|---|
| Disk full | Stop workers, free object storage, restart |
| Database corruption | Restore from the pgBackRest repository and PITR target, or the last Compose archive; run `deploy/v3/dr.sh --target scratch --confirm` to prove the digest; record the data-loss window; the 2026-11-25 in-place drill is `Measured` |
| Migration job failed or stopped | Keep the old stack serving; read the job log and ledger; re-run it (it resumes) or correct forward, never edit an applied migration |
| Object store loss | Restore from backup; any missing artifact makes its runs unverifiable |
| Queue loss | Acknowledged messages survive one broker loss; rebuild a lost queue from `queued` rows |
| Image regression | Roll back to the previous digest; record the failure |

The last verified backup and digest belong in the long-campaign note.

## 7. Housekeeping

- Unreferenced objects are harmless; no garbage collector is required.
- Logs rotate per container; the host keeps no unbounded log.
- Stage retention sweeps at startup and hourly; everything else is request-driven.
