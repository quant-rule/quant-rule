# Deployment

- Status: Current; released as v1.0.0 on 2026-11-15 (prepared as v1.1.0); current in v1.4.2; v3 render/check, the PLAT-001 remote check, the PLAT-002 ingress, the PLAT-003 stateful tier, the PLAT-004 RabbitMQ tier, the PLAT-005 MinIO tier/check rows/node-loss lab, and the PLAT-006 observability tier are current; approved 2026-11-12
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

`make up` creates missing secret files under `secrets/` and the internal PKI
under `pki/`, builds the image, starts the stack, waits for health, and prints:

```text
QuantRule v1.4.2 ready → http://localhost:8080
```

Stop with `make down`. Delete volumes with `make reset`; it asks for
confirmation, and `yes | make reset` is the scripted form. Windows runs the same
Compose file without make:

```powershell
wsl bash tools/secrets.sh; docker compose --env-file secrets/compose.env up -d --build --wait
```

Fedora may use Docker Engine or Podman: `make up COMPOSE="podman compose"`; the
provider must support health conditions, anchors and healthchecks
([`docker/README.md`](../../docker/README.md) §7).

## 2. Topology — `quantrule.deploy/v1`

| Container | Image | Published | Volume |
|---|---|---|---|
| `gateway` | `quantrule-server` | `8080` | — |
| `data` | `quantrule-server` | — | `config/` read-only |
| `model` | `quantrule-server` | — | — |
| `backtest` | `quantrule-server` | — | — |
| `evidence` | `quantrule-server` | — | — |
| `simulation` | `quantrule-server` | — | — |
| `strategy` | `quantrule-server` | — | — |
| `postgres` | `postgres@sha256:846a…` (`QR_POSTGRES_IMAGE` override) | — | `pgdata`, `pgpki` |
| `pki-init` / `db-bootstrap` / `*-migrate` | the Postgres image / the Postgres image / `quantrule-server`, one-shot | — | `pgpki`, `mqpki`, `miniopki` |
| `minio` | `bitnamilegacy/minio` (pinned digest; `QR_OBJECT_STORE_IMAGE` override) | — | `objects`, `miniopki` |
| `rabbitmq` | `rabbitmq@sha256:7530…` (`QR_RABBITMQ_IMAGE` override) | — | `mqdata`, `mqpki` |

One bridge network. Services resolve each other by container name. Only the
gateway is reachable from the host. The image carries the Bun-built static client;
no Bun or Python runtime is present at run time.

## 2.1 Current state

All seven binaries and ten long-running containers start; `backtest` serves
`/metrics`, `/health` and `/ready` on its internal listener only. `pki-init`
stages the TLS materials for the non-root stores, `db-bootstrap` creates the four
databases and roles, then four `*-migrate` jobs apply each ledger;
long-running services use `QR_MIGRATE=0`. Compose and v2 run one data and one
simulation replica; HA requires [data active/active](data-active-active.md) and
[simulation concurrency](simulation-concurrency.md). The image and database/migration
contracts are pinned in [four-databases](../config/2026-11-22/four-databases.md) and [migrations.md](migrations.md).

## 3. Make targets

| Target | Effect |
|---|---|
| `make` / `make help` | List every target |
| `make up` / `make secrets` / `make pki` | Create missing secrets and PKI (`FORCE=1` rotates), build, start, wait, print URL |
| `make down` | Stop, keep volumes |
| `make reset` | Stop and delete volumes, after confirmation |
| `make logs` / `make ps` | Tail all services / status table |
| `make scale N=4` | Scale `model` and `backtest` to `N` |
| `make verify` | Submit the two fixture configs through the gateway and print the canonical digest |
| `make test` / `make test-fast` | Fast unit and integration tier, no network |
| `make test-composed` / `test-parity` / `test-ha` | Composed, port-parity, and opt-in Swarm HA lab tiers |
| `make gate` | Full workspace gate |
| `make remote` | Layer `compose.remote.yaml`, write `secrets/bearer_token`, publish only `:8080`, print the token and usage |
| `make token` | Print a new 256-bit bearer token; `--out` writes a mode-600 file |
| `make compose-check` | Validate both Compose files with disposable secrets |
| `make backup` | Dump Postgres (one database until the four-database archive lands) and mirror objects |
| `make restore FILE=…` | Restore into a clean stack |
| `make dev-web` | Bun dev server for the web client, proxying `localhost:8080` |

## 3.1 Remote profile

`make remote` reads `QR_BEARER_TOKEN` or generates one with `tools/token.sh`,
writes `secrets/bearer_token` at mode 600, layers `compose.remote.yaml`, starts
the stack, and prints the token plus a `curl` example. Only the gateway publishes
`:8080`; every other container stays on the bridge network. The script checks the
401/401/200 matrix and the published ports. Rotate by running `make remote` with a
new `QR_BEARER_TOKEN`; it installs the staged file and signals a reload.

Never expose `:8080` to the public internet: use WireGuard or Tailscale, or
terminate TLS at a reverse proxy. The browser enters the token on the Connection
screen and the shell accepts `--token`, `QUANTRULE_TOKEN` or
`QUANTRULE_TOKEN_FILE`; both send `Authorization: Bearer` on every gateway
request. API clients do the same; details in [`security.md`](security.md) §2.

## 4. Configuration

Two sources, one catalog:

| Source | Purpose |
|---|---|
| Repository `config/`, mounted read-only | Git-tracked run configs; the research loop, unchanged |
| `POST /catalog/configs` | A config that is not in the repository |

The mount is read-only and resolved by `data`; a path outside it is never read.
Every config receives a `config_hash`, so origin does not affect reproducibility.
Service configuration is environment variables only; no config file is required.

| Variable | Default | Purpose |
|---|---|---|
| `QR_IMAGE_DIGEST` | image label; `make up` records the built image id | Required for a run/stage submission; reported by `/health` and written to provenance. A server without it refuses the submission. |
| `QR_INTERNAL_ONLY` | unset | When `1`, an HTTP service refuses a public or unspecified bind; the multi-host profile sets it on the internal services. |
| `QR_DATABASE_URL_FILE` / `QR_DATABASE_OWNER_URL_FILE` | secret file | The app role of a long-running service / the owner role of its one-shot migration job; model and backtest receive neither; long-running services require `QR_MIGRATE=0` |
| `QR_DATA_URL` | compose value | Data service, used by the gateway, worker and strategy service |
| `QR_MODEL_URL` / `QR_SIMULATION_URL` | compose value | Model service / simulation service, used by the gateway |
| `QR_EVIDENCE_URL` | compose value | Evidence service, used by the gateway and the backtest worker |
| `QR_STRATEGY_URL` | compose value | Optional strategy probe; v2 leaves it absent and still starts |
| `QR_OBJECT_STORE_URL` | compose value | MinIO endpoint without userinfo; `https://` in the deployed profile |
| `QR_DATABASE_TLS_CA_PATH` / `QR_QUEUE_TLS_CA_PATH` / `QR_OBJECT_STORE_TLS_CA_PATH` | `/run/pki/ca.crt` | Internal CA bundles; a TLS endpoint without its CA is a startup error |
| `QR_OBJECT_STORE_ACCESS_KEY_FILE` / `QR_OBJECT_STORE_SECRET_KEY_FILE` | secret file | The explicit S3 credentials; no AWS default chain |
| `QR_QUEUE_URL_FILE` / `QR_QUEUE_GENERATION` | secret file / `1` | RabbitMQ endpoint / queue-generation suffix; a changed queue declaration uses a new generation |
| `QR_LEADER_ID` | hostname + process id | Evidence leader holder; a distinct value per replica makes logs clear, and the term, not the holder, is the fence |
| `QR_UPLOAD_LIMIT_BYTES` | `536870912` | Dataset upload cap; a config bundle is capped at 4 MiB separately |
| `QR_BEARER_TOKEN_FILE` | unset | Enables authentication when set; the gateway re-reads it every `QR_TOKEN_RELOAD_SECONDS` and on `SIGHUP` |
| `QR_BACKTEST_CONCURRENCY` | `1` | Runs per worker |
| `QR_STAGE_RETENTION_DAYS` | `30` | Diagnostic stage retention in days |
| `QR_DRAIN_TIMEOUT_HTTP` / `QR_DRAIN_TIMEOUT_WORK` | `20` / `25` | SIGTERM grace for a request and a run; `1`–`300` seconds; Compose waits `40s` |
| `QR_LOG_FORMAT` / `QR_METRICS_BIND` | `json` / unset | JSON logs (`text` opt-in) / internal Prometheus listener; unset disables it, and it is never published |
| Store passwords | random from `make secrets` | `QR_POSTGRES_PASSWORD` seeds every database role; `QR_RABBITMQ_PASSWORD` and `QR_OBJECT_STORE_PASSWORD`; `FORCE=1` rotates deliberately |

A config bundle is capped at 4 MiB by the config route, independently of `QR_UPLOAD_LIMIT_BYTES`; a larger config returns `413 upload.too_large`. Secrets come from files; no secret is committed.
## 5. Distribution profile

QuantRule ships one deployment profile: the Linux image with Postgres, S3, and
RabbitMQ; the embedded desktop profile, SQLite, filesystem CAS, and in-process
queue are retired. Clients are the browser and the `quantrule` CLI on Windows and
Linux; macOS is unsupported next release. A local server uses Docker or Podman
Compose on Linux, or Docker Desktop on Windows.

## 6. Split host

The server runs the ten long-running containers and the bootstrap; the client
runs none and points at `http://<server>:8080`. Scaling is configuration.
## 7. Upgrade and rollback

1. Record the current image and canonical digest; keep the previous image and
   volumes.
2. `make down`; SIGTERM drains in-flight work before the 40 s grace expires.
   Load the new digest and `make up`: bootstrap, each `<service>-migrate` job,
   then its service. A failed job stops its service.
3. Run `make verify`; the fixture must print the recorded digest before the
   previous image is retired.
4. Rollback starts the previous digest without its job; the schema stays
   expanded and the previous image must tolerate it. See [migrations.md](migrations.md)
   §5. A destructive contract migration closes the window; recovery is PITR.

The split does not copy rows from the old `quantrule` database: use `make reset`
for disposable data, or the migration runbook for kept evidence.
## 8. What deployment does not provide

Autoscaling, registry signing, and real HA evidence are out of this profile. PostgreSQL, RabbitMQ and MinIO use internal TLS; the gateway serves plain HTTP behind the ingress ([ingress contract](../config/2026-11-23/ingress.md)). Multi-host placement is in [multi-host.md](multi-host.md); v3 render/check and the `tools/swarm.sh` control plane are in [deploy/v3](../config/2026-11-22/deploy-v3.md) and [swarm-control](../config/2026-11-23/swarm-control.md); Patroni/etcd/HAProxy is in [patroni-cluster](../config/2026-11-23/patroni-cluster.md), RabbitMQ in [rabbitmq-cluster](../config/2026-11-23/rabbitmq-cluster.md), MinIO in [minio-cluster](../config/2026-11-23/minio-cluster.md), and observability in [observability](../config/2026-11-23/observability.md); real HA is HA-014.
The ingress/stateful/queue/MinIO check rows (`--out`/`QR_DEPLOY_OUT`), the `QUANTRULE_CA_CERT` trust path, the measured lab RTO, the MinIO probe hash/write quorum, `--skip-object`, and the `Unverified` lab boundary are documented in those contracts.
