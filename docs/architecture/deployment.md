# Deployment

- Status: Target v1.0.0; approved 2026-11-12
- Part of: [`README.md`](README.md)
- Decision: [`ADR-2026-11-12-01`](../adr/2026-11-12/01-adopt-distributed-service-based-architecture.md)
- Claim type: Behaviour Claim

One command starts the stack. One port is published. That is the whole operator
interface.

## 1. Quick start

```bash
git clone git@github.com:quant-rule/quant-rule.git
cd quant-rule
make up
```

`make up` builds the image, starts the stack, waits for health, and prints:

```text
QuantRule v1.0.0 ready → http://localhost:8080
```

Stop with `make down`. Delete volumes with `make reset`.

## 2. Topology — `quantrule.deploy/v1`

| Container | Image | Published | Volume |
|---|---|---|---|
| `gateway` | `quantrule-server` | `8080` | — |
| `data` | `quantrule-server` | — | `config/` read-only |
| `model` | `quantrule-server` | — | — |
| `backtest` | `quantrule-server` | — | — |
| `evidence` | `quantrule-server` | — | — |
| `simulation` | `quantrule-server` | — | `config/` read-only |
| `postgres` | `postgres:17` | — | `pgdata` |
| `minio` | `minio/minio` | — | `objects` |
| `rabbitmq` | `rabbitmq:4-management` | — | `mqdata` |

One bridge network. Services resolve each other by container name. Only the
gateway is reachable from the host.

Nine containers: six service binaries from one image, plus Postgres, the object
store, and the broker.

The image also carries the static client, built with **Bun** (`bun install`,
`bun run build`). No Bun runtime and no Python runtime are present at run time.

## 3. Make targets

| Target | Effect |
|---|---|
| `make up` | Build, start, wait for health, print URL |
| `make down` | Stop, keep volumes |
| `make reset` | Stop and delete volumes, with confirmation |
| `make logs` | Tail all services |
| `make ps` | Status table |
| `make scale N=4` | Scale `model` and `backtest` to `N` |
| `make verify` | Run the fixture and print the canonical digest |
| `make test` | Unit and integration tier, no network |
| `make gate` | Full gate |
| `make remote` | Start with only `:8080` and a bearer token |
| `make token` | Print a new bearer token |
| `make backup` | Dump Postgres and mirror objects |
| `make restore FILE=…` | Restore into a clean stack |
| `make dev-web` | Vite dev server proxying to `localhost:8080` |

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
| `QR_IMAGE_DIGEST` | image label | Reported by `/health` and written to provenance |
| `QR_DATABASE_URL` | compose value | Postgres |
| `QR_OBJECT_STORE_URL` | compose value | MinIO endpoint |
| `QR_QUEUE_URL` | compose value | RabbitMQ |
| `QR_UPLOAD_LIMIT_BYTES` | `536870912` | Upload cap |
| `QR_BEARER_TOKEN` | empty | Enables the token check when set |
| `QR_BACKTEST_CONCURRENCY` | `1` | Runs per worker |

Secrets come from the environment. No secret is committed.

## 5. Two distribution profiles

| Profile | Contains | Installs with |
|---|---|---|
| Server | The image and nine containers | Docker, `make up` |
| Desktop | A Tauri shell; no compute, no local data | A release asset from GitHub Releases |

The desktop shell points at a gateway URL. It does not replace the server and it
does not run one. The embedded, self-contained desktop is `DIST-200` and is not
part of v1.0.0.

## 6. Split host

The same compose file runs on two machines:

| Host | Containers |
|---|---|
| Client machine | none; a browser |
| Server | all eight |

The client points at `http://<server>:8080`. Scaling and splitting are
configuration; no code changes.

## 7. Upgrade and rollback

1. Pull the new image digest.
2. `make down && make up`.
3. Migrations run forward at startup and are idempotent.
4. Rollback is starting the previous digest. Migrations are forward-only, so a
   rollback never reverts the schema; the previous version must tolerate the
   newer schema.

## 8. What deployment does not provide

No high availability, no multi-host scheduling, no autoscaling, no TLS
termination, no external registry. Each is a later decision with its own record.
