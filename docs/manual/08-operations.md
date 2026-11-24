# 8. Operations

- Status: Current
- Audience: operator running, exposing, backing up, and checking the deployment

## Goal

Check the stack and its profiles, read retention and cleanup status, verify a
recorded digest, and operate the deployment remotely without losing evidence.

## Before you start

| Need | Value |
|---|---|
| Stack | Started with `make up` or `make remote` |
| Profile | The active profile for record reads |
| Token | Only when the gateway enforces one |
| Host access | The machine that runs the containers for backup and restore |

Operations is operational metadata. It is not a model measurement, a run
artifact, or research evidence.

## Internal TLS

`make up` runs `make pki` first and creates `pki/` (mode 700) with a 10-year
offline root, a 5-year intermediate, 180-day server leaves for `postgres`,
`minio` and `rabbitmq`, and a bundled `ca.crt`. The stack mounts only `ca.crt`
into the application containers; the one-shot `pki-init` copies the server key
and certificate into a service-owned volume (key 0600) before the stock
non-root database, broker and store start.

| Task | Command | Expect |
|---|---|---|
| Inspect the bundle | `openssl x509 -in pki/ca.crt -noout -subject -dates` | Intermediate subject and dates |
| Verify a leaf | `openssl verify -CAfile pki/ca.crt pki/postgres/server.crt` | `OK` |
| Add a deployment name | `PKI_EXTRA_SANS=10.10.0.20 make pki FORCE=1` | New SANs; restart the stack |
| Rotate a leaf | `make pki FORCE=1` | New certificates; `make down && make up` |
| Rotate the CA | Re-issue, overlap the intermediate in `ca.crt`, rolling restart | Old and new leaves verify during the overlap |

A missing CA, wrong CA, hostname mismatch or expired leaf fails closed: the pool
does not open, the migration job fails, or `/ready` reports the dependency down;
there is no plaintext fallback. Internal HTTP stays plaintext on the overlay
([TLS contract](../config/2026-11-23/internal-tls.md)).

## Steps

| # | Shell | Browser | Expect |
|---:|---|---|---|
| 1 | `quantrule health` | **Connection** → health table | Aggregate state, service states, image digest, and the reserved queue/worker counts; the gateway's `/ready` adds the upstream readiness states |
| 2 | `quantrule profile list` | **Profiles** → profile table | Profiles with dataset, run, and simulation counts |
| 3 | `quantrule profile create "Research Lab" --id research-lab` | Profiles → create form | Shell prints the row and keeps the active profile; the browser creates and selects it |
| 4 | `quantrule profile use research-lab` | **Profile switcher** → profile | The local default and the active profile change |
| 5 | `quantrule verify <run_id>` | **Operations** → run id → Verify digest | The recorded canonical digest |
| 6 | Read `GET /operations` through the gateway | **Operations** → retention | Days, store presence, last cleanup, counts, error |
| 7 | `make remote` | Connection → enter the printed token | Only `:8080` published; token required |
| 8 | `make backup` | — | One timestamped archive and a SHA-256 sidecar |
| 9 | `make restore FILE=<archive>` | — | A clean stack restored from the verified archive |
`make remote` creates missing files under `secrets/`, refuses a documented
default password, and checks the 401/401/200 token matrix. The browser keeps the
token in local storage and sends it only as `Authorization: Bearer`.

`tools/observability.sh` starts the v3 observability tier: Prometheus 30d/20GB,
Loki 30d, Alertmanager 120h with one internal `quantrule-null` receiver, and
four read-only Grafana dashboards. node_exporter and promtail run everywhere;
manager quorum is a `docker node ls` textfile, never a container socket.
F01..F17 and the ports are in the [observability contract](../config/2026-11-23/observability.md); `qr-deploy check` runs the 28 observability rows, and the `ha` lab asserts the manager and gateway alerts fire and resolve; no port is public.

## Retention fields

| Field | Meaning |
|---|---|
| `stage_retention_days` | The configured `QR_STAGE_RETENTION_DAYS`; default 30 |
| `stage_store` | Whether the evidence service has a stage store |
| `last_cleanup.completed_utc` | When the last cleanup pass ran |
| `last_cleanup.stages_removed` | Diagnostic stage rows removed |
| `last_cleanup.objects_removed` | Stage object keys removed |
| `last_cleanup.error` | The failure, or `null` on success |

`last_cleanup` is `null` before the first cleanup and when no stage store is
configured. Only diagnostic stage artifacts expire. Runs, research records, and
evidence are permanent.

## Token and remote access

| Step | Rule |
|---|---|
| Start | `make remote` reuses `QR_BEARER_TOKEN` or generates one and writes `secrets/bearer_token` at mode 600 |
| Browser | Enter it on Connection; **Forget token** removes it |
| Shell | `--token <token>`, `QUANTRULE_TOKEN` or `QUANTRULE_TOKEN_FILE`; the token is never printed |
| Exposure | Private network or TLS; plain HTTP exposes the header |
| Rotate | Run `make remote` with a new value, or copy the file into the gateway and send `SIGHUP`; no restart; the old token is rejected after the swap |
| Swarm rotate | New secret name plus a rolling gateway update; see [manual 15](15-ha-deployment.md#token-rotation-under-swarm) |
| Invalid edit | The gateway keeps the last good token and increments `quantrule_auth_reload_failures_total`; it never disables authentication |

The gateway reads only `QR_BEARER_TOKEN_FILE`; a plain `QR_BEARER_TOKEN` in the
server environment is a startup error. Application secrets are files under
`/run/secrets`; the entrypoint stages them for UID 10001 and drops privileges.
`make secrets` creates the local set; see
[file-backed secrets](../config/2026-11-23/secret-files.md).

The simplest rotation is a new operator value and `make remote` again:

```bash
export QR_BEARER_TOKEN="$(tools/token.sh)"
make remote
export QUANTRULE_TOKEN="$QR_BEARER_TOKEN"
```

To rotate from a file without starting the stack again, copy it into the staged
path and signal the gateway:

```bash
tools/token.sh --out secrets/bearer_token
docker compose -f compose.yaml -f compose.remote.yaml \
  cp secrets/bearer_token gateway:/run/quantrule-secrets/bearer_token
docker compose -f compose.yaml -f compose.remote.yaml exec -T -u 0 gateway \
  sh -c 'chown 10001:10001 /run/quantrule-secrets/bearer_token && chmod 400 /run/quantrule-secrets/bearer_token'
docker compose -f compose.yaml -f compose.remote.yaml kill -s HUP gateway
export QUANTRULE_TOKEN="$(cat secrets/bearer_token)"
```

On a second machine that runs no containers, point the shell at the server and
use the same token:

```bash
export QUANTRULE_API_URL=http://<server>:8080/api/v1
export QUANTRULE_TOKEN='<token printed by make remote>'
quantrule health
```

Open `http://<server>:8080` in the browser on that machine and enter the token on
**Connection**. The shell default is `http://127.0.0.1:8080/api/v1`, so the
export is what makes the laptop a client rather than a local stack. A token is
a network gate, not an account. There is one principal.

## Metrics and logs

Each service starts an internal Prometheus listener when `QR_METRICS_BIND` is
set; the default Compose stack sets `0.0.0.0:9090` inside the container network
and publishes no metrics port. The gateway does not proxy `/metrics`.

```bash
docker compose exec -T gateway curl -fsS http://127.0.0.1:9090/metrics
docker compose exec -T data curl -fsS http://127.0.0.1:9090/metrics
```

The common families are build identity, process start, readiness, draining,
dependency health, HTTP counts/duration/in-flight, queue depth, worker count,
run/stage outcomes, retries/reconnects, the evidence leader and outbox, and the
gateway auth families. Labels use the route template (`/runs/{id}`) or a fixed
queue kind; never a raw path, generated queue name or id.

`quantrule_ready` is `0` and `quantrule_draining` is `1` on SIGTERM; `/ready`
answers `503 draining` immediately while `/health` stays `200`. The HTTP server
waits `QR_DRAIN_TIMEOUT_HTTP` (20 s), a worker `QR_DRAIN_TIMEOUT_WORK` (25 s),
and Compose allows 40 s. A deadline still exits `0` and logs
`drain.deadline_exceeded`.

### Drain and retries

A draining worker stops claiming and lets its in-flight run finish; if the
deadline aborts it, the run stays `running` with its lease and no digest, and
lease expiry plus the outbox recover it. Shutdown never writes `failed`.

Internal GET reads and the audited dataset-reference writes retry three times
with a 200 ms base, factor four, a 5 s maximum and a 15 s deadline. A claim,
compute, signal/stage create, decision, cancel and streaming body never retry.
A queue consumer that errors reconnects with 1–30 s backoff, rebuilds the
connection, then restarts with `restart_policy: any`; a drain during backoff is
a clean stop.

The shell retries a GET on 3 attempts, 200 ms base, factor four, 5 s cap and
15 s deadline; the browser uses 3 attempts, 250 ms base, factor four and 4 s
cap. A mutation never retries; the browser banner persists until a read succeeds.

Logs are one JSON object per line on stderr; `QR_LOG_FORMAT=text` is local
debugging only and `QR_LOG` is the filter. `x-request-id` and a valid W3C
`traceparent` are forwarded; neither enters an artifact or the digest.

```bash
docker compose logs --tail 20 gateway
curl -isS http://localhost:8080/health | grep -i x-request-id
```

## Queue operations

The run, evidence, and stage work queues are generated quorum queues
(`runs.backtest.g<n>`, `runs.evidence.g<n>`, `stages.worker.g<n>`). Each has a
three-redelivery limit and dead-letters to `runs.dead-letter.g<n>`; the
dead-letter queue is quorum and has no dead-letter target. Inspect the state:

```bash
docker compose exec -T rabbitmq rabbitmqctl list_queues name type messages consumers arguments --formatter json && docker compose exec -T rabbitmq rabbitmqctl list_bindings source_name destination_name routing_key --formatter json
```

A failing delivery is rejected and appears in the dead-letter queue with its
routing key and an `x-death` header; the run row is the record. Every service in
one deployment must use the same `QR_QUEUE_GENERATION`. A topology change is a
new generation: drain the old one, stop its consumers, then unbind or delete the
empty queues; never purge or delete a live queue.

### Evidence leader

With the HA profile, two evidence replicas serve reads; one holds
`runs.leader_lease` and consumes `run.computed`. The lease is renewed every 10 s
against a 30 s database window. Inspect it:

```bash
docker compose exec -T postgres psql -U qr_evidence_app -d qr_evidence \
  -c 'SELECT holder, term, expires_at > now() AS live FROM runs.leader_lease'
docker compose logs evidence | grep 'evidence replica is the leader'
```

A leader kill lets the standby acquire a higher term after expiry; the outbox
re-arms the unfinished run, so a message is never lost. A stale leader's
finalisation is refused and the run record does not change.

### Definitions backup and restore

PLAT-004 exports the topology from a broker, strips `users`, `permissions`, and
`topic_permissions`, verifies the four HA-005 queues, quorum, delivery limit 3,
the DLX/DLQ, bindings, and no `runs.*`/`stages.*` policy, and stores it mode 600
on every broker. HA-013 owns archive/off-host copy and the empty-broker drill.

```bash
tools/queue-definitions.sh export
tools/queue-definitions.sh verify /srv/quantrule/rabbitmq/definitions/<UTC>.json
tools/queue-definitions.sh verify --live
tools/queue-definitions.sh prune --now <UTC> --days 30
```

`import <file>` accepts an empty broker/cluster only; a live `.g*` generation
needs `--force`, and the tool never purges a queue or deletes a user or vhost.
Definitions restore topology, not messages; an acknowledged message survives one
broker loss, and a lost queue is rebuilt from run rows. The app user is recreated
from `queue_url` by `tools/stateful.sh queue-bootstrap`. The v1/v2 Compose broker
still uses `rabbitmqctl export_definitions`; keep that legacy file mode 600 and
out of Git because it contains users and password hashes.

## Backup and restore

| Command | Effect |
|---|---|
| `make backup` | Stops application services, dumps Postgres, mirrors objects, writes the archive and sidecar, restarts services |
| `make restore FILE=<archive>` | Verifies the sidecar and every payload, then restores into a clean stack |
| `tools/restore.sh <archive> --force --yes` | Explicitly replaces a non-empty stack |

The queue is not archived; an acknowledged message survives one broker loss and
a lost queue is rebuilt from run rows. Restore writes objects before rows, so an
interrupted copy leaves unreferenced blobs, not a present but unverifiable run.

## v3 object store

PLAT-005 runs `tools/stateful.sh object-bootstrap` after `wait`: it creates the
`quantrule` bucket, enables versioning, creates `qr_app`, attaches the policy,
and the check adds config/quorum, unit/alias, HAProxy 9000, manifest, app, and
leaf SAN rows. The two root files are `minio` scope. One node loss keeps the
write quorum; the lab re-reads the probe hash through `minio:9000`, heals to
zero, and records `Simulated HA`; the digest is `Measured` only after `verify.sh`.
HA-013 owns off-host/archive and restore; object lock is not in v1.5.10.

`tools/backup.sh` and `tools/restore.sh` still dump one database, so a backup is
incomplete until all four are covered ([four databases](../config/2026-11-22/four-databases.md)).

## Migrations

`db-bootstrap` creates the four databases and roles. Then `gateway-migrate`,
`data-migrate`, `evidence-migrate`, and `simulation-migrate` each apply one
ledger with the owner role and exit. A service starts only after its job
completes. The ledger is forward-only; a rollback changes the image, not the
schema.

| Situation | Operator action |
|---|---|
| A migration job fails | Keep the old stack serving, read `docker compose logs <service>-migrate`, query the ledger, and correct forward with a new version. Never edit an applied migration. |
| A migration job stops | Re-run the job; it resumes from the ledger. Do not start the service by hand and do not set `QR_MIGRATE=1`. |
| The new digest fails the fixture | Keep the previous image and volumes, restore the previous descriptor, start the previous services without the migration job, and compare the canonical digest before retiring anything. |

The exact commands and the rollback conditions are in
[`migrations.md`](../architecture/migrations.md) section 5.

An existing local stack created before file-backed secrets keeps its old store
passwords. Export `QR_POSTGRES_PASSWORD`, `QR_RABBITMQ_PASSWORD` and
`QR_OBJECT_STORE_PASSWORD` before the first `make secrets`, or reset a
disposable stack with `make reset`; a kept stack needs a coordinated change.

### Queue generation cutover

An upgrade that changes the queue topology does not reuse the old names. Stop
the gateway and old workers, let the old queues drain, then unbind or delete the
empty old queues before the new generation publishes; an old queue bound to
`quantrule.runs` receives copies with no consumer. A lost queued run is rebuilt
from its run row.

## What you see

The Operations screen shows connection, product, API, image digest, health,
active profile, token presence, retention, cleanup, the permanent-evidence rule,
and run verification; Connection owns entering and forgetting the token.

## What it proves

**Verification.** Health, retention, token, backup, and digest reads prove the
deployment is reachable and its evidence is intact; two equivalent runs with the
same digest are the reproducibility proof. None of this is research evidence.

## Failure

| Code or symptom | Cause | Next action |
|---|---|---|
| `auth.unauthorized` | Missing or wrong bearer token | Enter the token on Connection, or pass `--token`; after a rotation, re-export it |
| Bearer token reload failure | The token file is missing, empty, short or malformed | Fix the file; the last good token stays active and the reason is counted |
| `profile.not_found` | The active profile does not exist | Create it, or select an existing one |
| `profile.invalid` | The profile name failed validation | Use lowercase letters, digits, and hyphens |
| `run.not_found` | The digest form names an unknown run | Re-read the run list |
| `service.unavailable` | A service is unreachable, restarting, or draining | Read `health`; wait for `/ready`; a GET retries to its budget, a mutation is not retried |
| `queue.unavailable` | The broker or the service queue is unavailable | Read `health`; run submissions are retried from the outbox, so do not resubmit a run |
| `pki-init` failed | `make pki` was not run or a mount is wrong | Run `make pki`, then `make down && make up` |
| `must use sslmode=verify-full` | A database URL uses `prefer`, `require` or `verify-ca` | Fix the URL; do not weaken the mode |
| TLS handshake/CA error | Wrong CA, SAN, or expired leaf | Run `openssl verify`; re-issue with `make pki FORCE=1` and restart |
| Retention error | The last cleanup failed | Read the error; do not delete evidence by hand |
| Backup archive exists | The timestamped name already exists | Re-run later; never overwrite the archive |
| Restore refused | Sidecar, payload, or stack state failed verification | Stop; do not force without a human decision |

## Related

- [First run](01-first-run.md)
- [Improvement](07-improvement.md)
- [End-to-end verification tutorial](../tutorials/2026-11-17/end-to-end-verification.md)
- [Deployment](../architecture/deployment.md)
