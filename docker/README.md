# The server image and the Compose stack

- Status: Implements `DIST-022` (#643); six of six binaries exist and the default
  stack starts nine long-running containers plus the one-shot database
  bootstrap. The permanent object-store registry remains a human decision.
- Claim type: Behaviour Claim

One image, one published port, one bridge network. The image is `linux/amd64`;
Windows and macOS run it through Docker Desktop's Linux backend (WSL2 on
Windows). All state lives in named volumes; no host path is used except the
read-only `config/` mount that the target assigns to the `data` service.

## 1. What the image contains

| Path | Contents |
|---|---|
| `/usr/local/bin` | `qr-gateway`, `qr-data`, `qr-model`, `qr-backtest`, `qr-evidence`, `qr-simulation` |
| `/srv/static` | The client built with Bun (`bun install --frozen-lockfile`, `bun run build`) |
| `/etc/quantrule-version` | The workspace version, baked at build time |
| `/usr/local/bin/quantrule-entry` | Selects the binary and answers `--version` |

The six binaries come from six packages, one per service:
`quantrule-gateway`, `quantrule-backtest-service`,
`quantrule-data-service`, `quantrule-model-service`,
`quantrule-evidence-service`, and `quantrule-simulation-service`.

The runtime is Debian slim with `ca-certificates` and `curl`, and it runs as
UID 10001. It carries no Python interpreter, no Bun runtime, no Rust toolchain,
and no `tools/` directory.

## 2. Build and run

```bash
docker build -t quantrule-server:test .
docker compose up -d --build --wait
curl -fsS http://localhost:8080/health
curl -fsS -H 'X-QuantRule-Profile: default' http://localhost:8080/catalog/datasets
docker compose down
```

`make up` is #644. `docker compose config` validates the default stack. There is
no partial-stack profile: every one of the nine services starts by default.

| Container | Binary | Surface |
|---|---|---|
| `gateway` | `qr-gateway` | `/health`, `/version`, profiles, runs, hypotheses, the model route, the proxied data routes, and the static client |
| `data` | `qr-data` | `/health`, `/version`, `/catalog/*`, `/datasets/*`, `/inspect/*`, and the internal config/dataset references |
| `model` | `qr-model` | `/health`, `/version`, `/model/compute`, and the internal model bundle |
| `backtest` | `qr-backtest` | queue consumer; no HTTP |
| `evidence` | `qr-evidence` | `/health`, `/version`, `/runs`, `/hypotheses`, and the internal worker lifecycle |
| `simulation` | `qr-simulation` | `/health`, `/version`, and the eleven research-plane endpoints |
| `postgres` | `postgres:17` | database cluster (four logical databases) |
| `db-bootstrap` | `postgres:17` | one-shot databases and roles |
| `*-migrate` | `quantrule-server` | four one-shot owner migrations |
| `minio` | `bitnamilegacy/minio` | object store |
| `rabbitmq` | `rabbitmq:4-management` | broker |

Only `gateway` publishes a port. The five HTTP services carry a healthcheck that
reads their own `/health`; the three infrastructure containers carry their own
probes. `backtest` serves no HTTP and publishes no liveness probe, so it has no
healthcheck. Docker reports its process state; `/health` reports the
dependencies. A worker probe is part of the split.

## 3. Current state

`data` is complete for P3. It owns the `catalog` schema, the read-only `config/`
mount, the dataset object path, and the public catalog, dataset, config, and
inspection routes. `model` is complete for P4: it serves `/model/compute` and the
internal model bundle, reads the catalog through `qr-data`, and owns no database
schema. `evidence` is complete for P5: it owns the `runs` and `research` schemas,
serves the run and hypothesis surface, and consumes `run.computed` to finalize.
`simulation` is complete for P7: it owns the `simulation` schema, serves the
eleven research-plane endpoints, and reads a recorded bundle from the object
store. The gateway proxies all four surfaces and keeps no domain handler:

| Service | Route owner | Phase |
|---|---|---|
| `data` | `qr-data` (gateway proxy) | P3, landed |
| `model` | `qr-model` (gateway proxy) | P4, landed |
| `evidence` | `qr-evidence` (gateway proxy) | P5, landed |
| `simulation` | `qr-simulation` (gateway proxy) | P7, landed |

Only `data` mounts `config/` read-only. `model` and `evidence` read the dataset
object through data and the object store, and `simulation` reads its recorded
bundle from the object store, so none needs a config mount. `backtest` loads the
dataset and config from data, the model bundle from model, and records the run
through evidence; it serves no HTTP.

Each database owner migrates its own ledger in its own database. The one-shot
`db-bootstrap` service creates the four databases and the eight owner/app roles;
four one-shot `*-migrate` services run the same binary with `--migrate-only`,
apply one ledger with the owner URL, and exit. The long-running service starts
afterwards with its `<database>_app` URL and `QR_MIGRATE=0`; it never receives
the owner credential. `backtest` and `model` own no database and have no
migration job.

## 4. Object store source

MinIO withdrew its public images on 2026-09-11, and `quay.io/minio/minio` now
requires authentication. The Compose default is therefore a pinned Bitnami
legacy archive,
`bitnamilegacy/minio@sha256:451fe6858cb770cc9d0e77ba811ce287420f781c7c1b806a386f6896471a349c`,
which is `linux/amd64` and creates the `quantrule` bucket on first start.
`QR_OBJECT_STORE_IMAGE` overrides it. A permanent registry is a human decision,
not an implementation detail this stack may settle.

## 5. Image checks

The Dockerfile final stage runs `/usr/local/bin/check-no-python` against the
assembled filesystem, and the release workflow runs the same script against the
built image. `tools/check_python.py` gates the source tree.

```bash
docker run --rm quantrule-server:test --version
docker run --rm quantrule-server:test /usr/local/bin/check-no-python  # pass
docker run --rm quantrule-server:test sh -c 'command -v python3'   # empty
docker run --rm quantrule-server:test sh -c 'command -v bun'       # empty
docker run --rm quantrule-server:test sh -c 'find / -name "*.py" 2>/dev/null | head'  # empty
```

## 6. Configuration

The image sets only paths: `QR_STATIC_DIR=/srv/static` and
`QR_CONFIG_ROOT=/srv/config`. Compose supplies the database, object store, and
broker URLs plus the MinIO credentials through the AWS environment. Every
credential has a local default and can be overridden:

| Variable | Purpose |
|---|---|
| `QR_IMAGE` | Image reference; a digest in a release |
| `QR_IMAGE_DIGEST` | Passed to `/health` and provenance; unset by default |
| `QR_DATA_URL` | Data-service base URL used by the gateway and the backtest worker |
| `QR_MODEL_URL` | Model-service base URL used by the gateway |
| `QR_EVIDENCE_URL` | Evidence-service base URL used by the gateway and the backtest worker |
| `QR_SIMULATION_URL` | Simulation-service base URL used by the gateway |
| `QR_DATABASE_URL` | The long-running service's `<database>_app` role; model and backtest receive none |
| `QR_DATABASE_OWNER_URL` | The one-shot `<service>-migrate` job's `<database>_owner` role; never set on a long-running service |
| `QR_POSTGRES_PASSWORD` | Seeds every database role in the local stack |
| `QR_OBJECT_STORE_PASSWORD` | MinIO root password and the S3 secret |
| `QR_RABBITMQ_PASSWORD` | Broker password and the service URL |
| `QR_OBJECT_STORE_IMAGE` | Object-store image reference |

## 7. Host smoke notes

These are manual checks. They were not executed on a Windows host or against a
standalone Podman daemon.

Windows (Docker Desktop with the WSL2 Linux backend):

```powershell
docker context use desktop-linux
docker compose config
docker compose build
docker compose create gateway
$env:QR_IMAGE_DIGEST = (docker compose images -q gateway | Select-Object -First 1)
docker compose up -d --wait
curl.exe -fsS http://localhost:8080/health
docker compose down
```

The explicit `QR_IMAGE_DIGEST` is the Windows equivalent of `make up`. Without
it the stack still starts and `/health` answers, but every run or stage
submission is refused with `internal` and the message to set the digest; the
same rule applies to the browser client.

Fedora (Podman):

```bash
podman compose config
podman compose up -d --wait
```

`podman compose` delegates to an external Compose provider. The stack uses
`depends_on: condition: service_healthy`, YAML anchors, and healthchecks; the
provider must support all three. On this development host `podman compose`
delegates to Docker Compose v5.7.1 and the Docker daemon, so it is not an
independent Podman check.
