# 14. Multi-Host Deployment

- Status: Current
- Audience: operator placing the six services on more than one Linux host

## Goal

Run the server profile across a gateway host, a core host, a compute host, and
an infrastructure host, with only the gateway reachable from the client side and
the same canonical digest as the single-host stack.

## Before you start

| Need | Value |
|---|---|
| Hosts | Four Linux hosts or VMs on one WireGuard/Tailscale network |
| Container runtime | Docker Engine and Compose on every host |
| Image | The same `quantrule-server` image on every host; build once and distribute |
| Repository | The tree that contains `deploy/multi-host/` and `qr-deploy` |
| Secrets | A `secrets.env` file with non-default database, object-store, queue, and token values |
| Network | Private overlay addresses for every host; no public port except the gateway |

Do not start this profile while the single-host stack owns
`8080`. Stop them first.

## Steps

| # | Host | Action | Expect |
|---:|---|---|---|
| 1 | build host | `cargo build --release --bin qr-deploy` | `target/release/qr-deploy` |
| 2 | build host | `docker save quantrule-server:<tag> -o quantrule-server.tar` | One image tar |
| 3 | every host | `docker load -i quantrule-server.tar` | The same image ID on every host |
| 4 | build host | Copy `deploy.example.json` to `deploy.json`; set the image reference/digest, host addresses, and ports | One descriptor |
| 5 | build host | Create `secrets.env` from `secrets.example.env`; replace every placeholder | Owner-only file |
| 6 | build host | `qr-deploy validate deploy.json --secrets secrets.env` | `ok: …`, no error |
| 7 | build host | `qr-deploy render deploy.json --out deploy/multi-host/env --secrets secrets.env` | Eleven env files: six services, four migrations, and infra; the services carry app URLs only |
| 8 | infra host | Copy the `deploy/` directory, `compose.infra.yaml`, and `env/infra.env`; start the infra role | Postgres, MinIO, RabbitMQ healthy; `db-bootstrap` created the four databases and roles |
| 9 | core host | Copy `compose.core.yaml`, `env/data.env`, `env/data-migrate.env`, `env/evidence.env`, `env/evidence-migrate.env`, `env/simulation.env`, `env/simulation-migrate.env`, and `config/`; start the three services | Migration jobs complete; three healthy endpoints |
| 10 | compute host | Copy `compose.compute.yaml`, `env/model.env`, `env/backtest.env`; start `model` and `--scale backtest=<replicas>` | Model healthy; workers consuming |
| 11 | gateway host | Copy `compose.gateway.yaml`, `env/gateway.env`, `env/gateway-migrate.env`; start the gateway | `gateway-migrate` completes; `/ready` healthy |
| 12 | gateway or overlay host | `qr-deploy check deploy.json --secrets secrets.env` | HTTP services `ok`, one reported digest; worker `SKIP` |
| 13 | compute host | `docker ps --filter name=backtest`; `docker image inspect --format '{{.Id}}' <image>` | Worker present; image ID matches |
| 14 | client | `QUANTRULE_API_URL=http://<gateway>:8080/api/v1`; `quantrule health` | Connected |
| 15 | client | Submit the verification fixture and read the digest | Same digest as single-host |
| 16 | infra host | Stop gateway, compute, and core; run the backup wrapper with `--application-stopped` | Archive and sidecar written; the current tools cover one database, so treat it as incomplete until the four-database archive lands |

Each host needs only the files for the services it runs. The image is already
loaded; the Compose files use `network_mode: host` and the env file's
`QR_BIND_IP`/`QR_BIND_PORT`.

The v2 profile runs one data and one simulation instance. Two data replicas are
an HA capability and require the [active/active contract](../architecture/data-active-active.md);
the v2 validator refuses a second internal replica. Two simulation replicas
require the [concurrency contract](../architecture/simulation-concurrency.md);
do not start a second v2 instance.

The rendered service env files carry `QR_QUEUE_GENERATION=1`; every queue name
ends in `.g1`. The work queues are quorum with a delivery limit and a dead-letter
queue. A release that changes the topology uses a new generation: stop new
submissions, drain the old generation with its consumers, stop them, then unbind
or delete the empty queues before the new generation publishes. Export the broker
definitions before an upgrade; the backup archive does not contain the broker.

## What you see

| Field | Meaning |
|---|---|
| `qr-deploy validate` | Host count, service count, image digest, network gate |
| `qr-deploy check` | Service, URL, state, image digest, or the failure |
| `/ready` | Gateway plus data, model, evidence, and simulation states |
| `QR_INTERNAL_ONLY` | Internal services refuse a public bind |
| Canonical digest | The same value as the single-host verification run |

## What it proves

**Verification.** A completed multi-host run with one image digest and the same
canonical digest as the single-host stack proves the deployment profile, not an
edge. The local multi-host harness proves separation and digest parity; the real
three-host firewall, overlay, and failure rows are separate and may be blocked
until the hosts exist. The profile is not evidence, not a baseline, and not high
availability.

## Failure

| Code or symptom | Cause | Next action |
|---|---|---|
| `qr-deploy: … default credential` | The secrets file still has a documented default | Replace it; do not weaken the validator |
| `qr-deploy: the secrets file does not define …` | A placeholder has no value | Add the key to `secrets.env` |
| `QR_BIND: must be a private or loopback address` | `QR_INTERNAL_ONLY=1` with a public or `0.0.0.0` bind | Set the host's private address |
| `qr-deploy: default credential` or a template secret | The secrets file still has `replace-with-...` | Replace every password and the token; do not weaken the validator |
| `qr-deploy: placeholder image digest` | The descriptor still has the all-`a` digest | Record the real image ID |
| `qr-deploy: must target ... infrastructure host` | An endpoint points outside the infra host | Fix the endpoint or the infra address |
| `qr-deploy check` digest mismatch | A host runs a different image | Load the same image and confirm `docker image inspect` |
| `qr-deploy check` connection refused | The service is down or the overlay is down | Read the host logs and `/health` |
| `SKIP backtest` | The worker has no HTTP surface | Check the container on the compute host |
| Gateway `degraded` | An upstream is down | Start the missing service; `/ready` names it |
| Run stays `queued` | No backtest worker is consuming | Start a worker on the compute host |
| `simulation.locked` on decide | Another replica already recorded the terminal decision | Read the simulation; the first decision is final |
| 401 | The gateway token is missing or wrong | Enter the token in the client |
| Port already allocated | Another stack owns `8080` | Stop the other stack; do not change the descriptor to hide the conflict |

Do not publish an internal service to fix a connection problem. Fix the overlay
or the service, not the trust boundary.

## Related

- [Multi-host architecture](../architecture/multi-host.md)
- [Deployment](../architecture/deployment.md)
- [Security](../architecture/security.md)
- [Agent runbook](agent-runbook.md)
- [Deploy README](../../deploy/multi-host/README.md)
