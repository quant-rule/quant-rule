# Deployment

- Status: Current; released as v1.0.0 on 2026-11-15 (prepared as v1.1.0); current in v1.3.0; approved 2026-11-12
- Part of: [`README.md`](README.md)
- Decision: [`ADR-2026-11-12-01`](../adr/2026-11-12/01-adopt-distributed-service-based-architecture.md)
- Claim type: Behaviour Claim

One command starts the stack; one port is published; that is the whole operator interface.

## 1. Quick start

```bash
git clone git@github.com:quant-rule/quant-rule.git
cd quant-rule
make up
```

`make up` builds the image, starts the stack, waits for health, and prints:

```text
QuantRule v1.3.0 ready → http://localhost:8080
```

Stop with `make down`. Delete volumes with `make reset`; it asks for
confirmation, and `yes | make reset` is the scripted form. Windows runs the same
Compose file without make:

```powershell
docker compose up -d --build --wait
```

Fedora may use Docker Engine or Podman: `make up COMPOSE="podman compose"`. The
provider must support health conditions, YAML anchors, and healthchecks;
see [`docker/README.md`](../../docker/README.md) §7.

## 2. Topology — `quantrule.deploy/v1`

| Container | Image | Published | Volume |
|---|---|---|---|
| `gateway` | `quantrule-server` | `8080` | — |
| `data` | `quantrule-server` | — | `config/` read-only |
| `model` | `quantrule-server` | — | — |
| `backtest` | `quantrule-server` | — | — |
| `evidence` | `quantrule-server` | — | — |
| `simulation` | `quantrule-server` | — | — |
| `postgres` | `postgres@sha256:846a…` (`QR_POSTGRES_IMAGE` override) | — | `pgdata` |
| `minio` | `bitnamilegacy/minio` (pinned digest; `QR_OBJECT_STORE_IMAGE` override) | — | `objects` |
| `rabbitmq` | `rabbitmq@sha256:7530…` (`QR_RABBITMQ_IMAGE` override) | — | `mqdata` |

One bridge network. Services resolve each other by container name. Only the
gateway is reachable from the host.

The image also carries the static client, built with **Bun** (`bun install`,
`bun run build`). No Bun runtime and no Python runtime are present at run time.

## 2.1 Current state

All six binaries and nine containers start; only `backtest` is HTTP-less and has
no healthcheck. Gateway, data, evidence, and simulation migrate their own ledgers
under P1's pinned lock; `backtest` and `model` keep `0`. The MinIO image is a
pinned legacy archive, overridable by `QR_OBJECT_STORE_IMAGE`, pending a registry
decision.

## 3. Make targets

| Target | Effect |
|---|---|
| `make` / `make help` | List every target |
| `make up` | Build, start, wait for health, print URL |
| `make down` | Stop, keep volumes |
| `make reset` | Stop and delete volumes, after confirmation |
| `make logs` | Tail all services |
| `make ps` | Status table |
| `make scale N=4` | Scale `model` and `backtest` to `N` |
| `make verify` | Submit the two fixture configs through the gateway and print the canonical digest |
| `make test` / `make test-fast` | Fast unit and integration tier, no network |
| `make test-composed` | Composed tier against a running stack |
| `make test-parity` | Port-parity tier |
| `make gate` | Full workspace gate |
| `make remote` | Start with only `:8080` published and a bearer token; print the token and usage |
| `make token` | Print a new 256-bit bearer token; writes no file |
| `make backup` | Dump Postgres and mirror objects |
| `make restore FILE=…` | Restore into a clean stack |
| `make dev-web` | Bun dev server for the web client, proxying `localhost:8080` |

`make scale` scales `model` and `backtest`. `make backup` and `make restore`
call `tools/backup.sh` and `tools/restore.sh`. `make test-composed` runs the
composed tier against a stack started by `make up`.

## 3.1 Remote profile

`make remote` reads `QR_BEARER_TOKEN` or generates one with `tools/token.sh`,
exports it to the gateway, starts the stack, and prints the token plus a `curl`
example. Only the gateway publishes `:8080`; every other container stays on the
bridge network. The script checks the 401/401/200 matrix and the published port
set on every start.

Never expose `:8080` to the public internet: use WireGuard or Tailscale, or
terminate TLS at a reverse proxy. The browser enters the token on the Connection
screen and the shell accepts `--token` or `QUANTRULE_TOKEN`; both send
`Authorization: Bearer` on every gateway request. API clients do the same; details
in [`security.md`](security.md) §2.

## 4. Configuration

Two sources, one catalog:

| Source | Purpose |
|---|---|
| Repository `config/`, mounted read-only | Git-tracked run configs; the research loop, unchanged |
| `POST /catalog/configs` | A config that is not in the repository |

The mount is read-only and resolved by the `data` service. A config outside the
mount is never read from an arbitrary path. Every config receives a
`config_hash`, so the origin does not affect reproducibility.

Environment variables only for service configuration. No config file is required
to start.

| Variable | Default | Purpose |
|---|---|---|
| `QR_IMAGE_DIGEST` | image label; `make up` records the built image id | Required for a run/stage submission; reported by `/health` and written to provenance. A server without it refuses the submission. |
| `QR_DATABASE_URL` | compose value | Postgres |
| `QR_DATA_URL` | compose value | Data service, used by the gateway and the backtest worker |
| `QR_MODEL_URL` | compose value | Model service, used by the gateway |
| `QR_EVIDENCE_URL` | compose value | Evidence service, used by the gateway and the backtest worker |
| `QR_SIMULATION_URL` | compose value | Simulation service, used by the gateway |
| `QR_OBJECT_STORE_URL` | compose value | MinIO endpoint |
| `QR_QUEUE_URL` | compose value | RabbitMQ |
| `QR_UPLOAD_LIMIT_BYTES` | `536870912` | Dataset upload cap; a config bundle is capped at 4 MiB separately |
| `QR_BEARER_TOKEN` | empty | Enables the token check when set |
| `QR_BACKTEST_CONCURRENCY` | `1` | Runs per worker |
| `QR_STAGE_RETENTION_DAYS` | `30` | Diagnostic stage retention in days |
| Store passwords | `quantrule` on the loopback stack | `QR_POSTGRES_PASSWORD`, `QR_RABBITMQ_PASSWORD`, and `QR_OBJECT_STORE_PASSWORD`; required non-default for `make remote` |

A config bundle is capped at 4 MiB by the config route, independently of `QR_UPLOAD_LIMIT_BYTES`; a larger config returns `413 upload.too_large`. Secrets come from the environment; no secret is committed.

## 5. Two distribution profiles

| Profile | Backends | Contains | Installs with |
|---|---|---|---|
| Server | Postgres, S3, RabbitMQ | The image and nine containers | Docker, `make up` |
| Desktop | SQLite, filesystem CAS, in-process queue | Six bundled sidecars, one supervisor, one SQLite file | A release asset from GitHub Releases |

The server profile is the default; `QR_PROFILE=desktop` with `QR_DATA_DIR`
selects the embedded backends in [`desktop.md`](desktop.md) from a fresh data
directory. v0.9.0 shipped the server image; v1.0.0 adds the desktop installer.
`tools/parity_desktop.sh` proves both profiles match on one fixture.

## 6. Split host

The same compose file runs on two machines: the server runs all nine containers,
the client runs none. The client points at `http://<server>:8080`. Scaling and
splitting are configuration; no code changes.

## 7. Upgrade and rollback

1. Pull the new image digest.
2. `make down && make up`.
3. Migrations run forward at startup and are idempotent.
4. Rollback is starting the previous digest. Migrations are forward-only, so a
   rollback never reverts the schema; the previous version must tolerate the
   newer schema.

## 8. What deployment does not provide

No high availability, no autoscaling, no TLS termination, no external registry.
Multi-host placement is in [multi-host.md](multi-host.md); it is not HA.
