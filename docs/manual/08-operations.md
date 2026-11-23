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

`make remote` requires `QR_POSTGRES_PASSWORD`, `QR_RABBITMQ_PASSWORD`, and
`QR_OBJECT_STORE_PASSWORD` to be non-default; the local stack keeps the
documented loopback defaults. It checks the 401/401/200 token matrix before it
prints the token. The browser stores the token in local storage and sends it
only as `Authorization: Bearer`; it never enters the URL or page text.

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
| Start | `make remote` generates a token when `QR_BEARER_TOKEN` is unset |
| Browser | Enter it on Connection; **Forget token** removes it |
| Shell | `--token <token>` or `QUANTRULE_TOKEN`; the token is never printed |
| Exposure | Private network or TLS; plain HTTP exposes the header |
| Reset | Revoke by restarting with a new token; old tokens stop working |

On a second machine that runs no containers, point the shell at the server and
use the same token:

```bash
export QUANTRULE_API_URL=http://<server>:8080/api/v1
export QUANTRULE_TOKEN='<token printed by make remote>'
quantrule health
```

Open `http://<server>:8080` in the browser on that machine and enter the token on
**Connection**. The shell default is `http://127.0.0.1:8080/api/v1`, so the
export is what makes the laptop a client rather than a local stack.

A token is a network gate, not an account. There is one principal.

## Metrics and logs

Each service starts an internal Prometheus listener when `QR_METRICS_BIND` is
set. The default Compose stack sets it to `0.0.0.0:9090` inside the container
network and publishes no metrics port. The gateway does not proxy `/metrics`.

```bash
docker compose exec -T gateway curl -fsS http://127.0.0.1:9090/metrics
docker compose exec -T data curl -fsS http://127.0.0.1:9090/metrics
```

The common families are build identity, process start, readiness, draining,
dependency health, HTTP request counts, a duration histogram, and in-flight
requests. The workers add queue depth, worker count, run/stage outcome counters,
retry and reconnect counters; evidence adds the leader gauge and outbox depth.
Labels use the route template (`/runs/{id}`) or a fixed queue kind; never a raw
path, generated queue name or id.

`quantrule_ready` is `0` and `quantrule_draining` is `1` as soon as a SIGTERM
arrives. `/ready` answers `503` with a `draining` detail immediately; `/health`
stays `200` while the process lives. The HTTP server waits up to
`QR_DRAIN_TIMEOUT_HTTP` (20 s) and a worker up to `QR_DRAIN_TIMEOUT_WORK` (25 s);
Compose allows 40 s before a forced kill. If a request or run outlives its
deadline the process still exits `0` and logs `drain.deadline_exceeded`.

### Drain and retries

A draining worker stops claiming and lets its in-flight run finish. If the
deadline aborts it, the run stays `running` with its lease and no digest; lease
expiry and the outbox recover it. Shutdown never writes `failed`.

Internal GET reads and the audited dataset-reference writes retry three times
with a 200 ms base delay, factor four, a 5 s maximum delay and a 15 s total
deadline. A claim, a model compute, a signal create, a stage create, a decision,
a cancel and a streaming body never retry. A queue consumer that errors
reconnects with 1, 2, 4, 8, 16 and 30 s backoff; after that the container
restarts. A drain during a backoff is a clean stop, not a reconnect.

Logs are one JSON object per line on stderr. Set `QR_LOG_FORMAT=text` only for
local debugging; `QR_LOG` is the filter. A request returns `x-request-id`, and a
valid W3C `traceparent` is forwarded. Neither enters an artifact or the
canonical digest.

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
docker compose exec -T rabbitmq rabbitmqctl list_queues name type messages consumers arguments --formatter json
docker compose exec -T rabbitmq rabbitmqctl list_bindings source_name destination_name routing_key --formatter json
```

A delivery that keeps failing is rejected and eventually appears in the
dead-letter queue with its original routing key and an `x-death` header; the run
row is the record. Every service in one deployment must use the same
`QR_QUEUE_GENERATION`. Do not purge or delete a live queue. A topology change is
a new `QR_QUEUE_GENERATION`: stop new submissions, drain the old generation with
its consumers, stop them, then unbind or delete the empty queues before the new
generation publishes.

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

The archive does not contain the broker. Export the topology separately; the
file contains the broker users and password hashes, so keep it mode 600 and out
of Git.

```bash
definitions="qr-definitions-$(date -u +%Y%m%dT%H%M%SZ).json"
docker compose exec -T rabbitmq rabbitmqctl export_definitions /tmp/qr-definitions.json
docker compose cp rabbitmq:/tmp/qr-definitions.json "$definitions"
chmod 600 "$definitions"
docker compose exec -T rabbitmq rm -f /tmp/qr-definitions.json
```

Restore into an empty broker with:

```bash
docker compose cp qr-definitions.json rabbitmq:/tmp/qr-definitions.json
docker compose exec -T rabbitmq rabbitmqctl import_definitions /tmp/qr-definitions.json
```

Definitions restore queues and bindings, not messages. Rebuild queued runs from
run rows. A definitions file names a queue generation; importing it over a live
generation is not the upgrade path.

## Backup and restore

| Command | Effect |
|---|---|
| `make backup` | Stops application services, dumps Postgres, mirrors objects, writes the archive and sidecar, restarts services |
| `make restore FILE=<archive>` | Verifies the sidecar and every payload, then restores into a clean stack |
| `tools/restore.sh <archive> --force --yes` | Explicitly replaces a non-empty stack |

The queue is not archived; messages are rebuilt from run rows. Restore writes
objects before rows, so an interrupted copy leaves unreferenced blobs rather
than a run that looks present but cannot verify. Export the broker topology
separately; definitions restore queues and bindings, not messages.

The stack now stores evidence in four logical databases. `tools/backup.sh` and
`tools/restore.sh` still dump and restore one database, so they do not yet
cover all four; treat a backup as incomplete until the four-database archive
change lands. The database and role contract is
[`docs/config/2026-11-22/four-databases.md`](../config/2026-11-22/four-databases.md).

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

### Queue generation cutover

An upgrade that changes the queue topology does not reuse the old names. Stop
the gateway and old workers, let the old queues drain, then unbind or delete the
empty old queues before the new generation starts publishing. Leaving an old
queue bound to `quantrule.runs` while the new generation runs makes it receive
copies with no consumer. A queued run that is lost with the old queue is rebuilt
from its run row; the run row is authoritative.

## What you see

The Operations screen shows connection, product, API, image digest, health,
active profile, token presence, retention, cleanup counts and error, the
permanent-evidence rule, and the run verification form. The Connection screen
owns entering and forgetting the token.

## What it proves

**Verification.** Health, retention, token, backup, and digest reads prove the
deployment is reachable and its evidence is intact. A recorded digest is not a
reproducibility proof by itself; two equivalent runs with the same digest are.
None of this is research evidence or an edge claim.

## Failure

| Code or symptom | Cause | Next action |
|---|---|---|
| `auth.unauthorized` | Missing or wrong bearer token | Enter the token on Connection, or pass `--token` |
| `profile.not_found` | The active profile does not exist | Create it, or select an existing one |
| `profile.invalid` | The profile name failed validation | Use lowercase letters, digits, and hyphens |
| `run.not_found` | The digest form names an unknown run | Re-read the run list |
| `service.unavailable` | A service is unreachable, restarting, or draining | Read `health`; wait for `/ready`; stop only if it stays down |
| `queue.unavailable` | The broker or the service queue is unavailable | Read `health`; run submissions are retried from the outbox, so do not resubmit a run |
| Retention error | The last cleanup failed | Read the error; do not delete evidence by hand |
| Backup archive exists | The timestamped name already exists | Re-run later; never overwrite the archive |
| Restore refused | Sidecar, payload, or stack state failed verification | Stop; do not force without a human decision |

## Related

- [First run](01-first-run.md)
- [Improvement](07-improvement.md)
- [End-to-end verification tutorial](../tutorials/2026-11-17/end-to-end-verification.md)
- [Deployment](../architecture/deployment.md)
