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
| 1 | `quantrule health` | **Connection** → health table | Aggregate state, service states, image digest, and the reserved queue/worker counts; the gateway's `/ready` adds the upstream states |
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

## Backup and restore

| Command | Effect |
|---|---|
| `make backup` | Stops application services, dumps Postgres, mirrors objects, writes the archive and sidecar, restarts services |
| `make restore FILE=<archive>` | Verifies the sidecar and every payload, then restores into a clean stack |
| `tools/restore.sh <archive> --force --yes` | Explicitly replaces a non-empty stack |

The queue is not archived; messages are rebuilt from run rows. Restore writes
objects before rows, so an interrupted copy leaves unreferenced blobs rather
than a run that looks present but cannot verify.

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
| `service.unavailable` | A service is unreachable | Read `health`; stop |
| `queue.unavailable` | The broker is unreachable | Read `health`; stop |
| Retention error | The last cleanup failed | Read the error; do not delete evidence by hand |
| Backup archive exists | The timestamped name already exists | Re-run later; never overwrite the archive |
| Restore refused | Sidecar, payload, or stack state failed verification | Stop; do not force without a human decision |

## Related

- [First run](01-first-run.md)
- [Improvement](07-improvement.md)
- [End-to-end verification tutorial](../tutorials/2026-11-17/end-to-end-verification.md)
- [Deployment](../architecture/deployment.md)
