# The server image and the Compose stack

- Status: Implements `DIST-022` (#643); six of six binaries exist and the default
  stack starts all nine containers. The permanent object-store registry remains
  a human decision.
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

The six binaries come from four packages: `quantrule-server` carries `gateway`,
`backtest`, and `evidence`; `quantrule-data-service`, `quantrule-model-service`,
and `quantrule-simulation-service` carry their own `qr-*` binary.

The runtime is Debian slim with `ca-certificates` and `curl`, and it runs as
UID 10001. It carries no Python interpreter, no Bun runtime, no Rust toolchain,
and no `tools/` directory.

## 2. Build and run

```bash
docker build -t quantrule-server:test .
docker compose up -d --build --wait
curl -fsS http://localhost:8080/health
docker compose down
```

`make up` is #644. `docker compose config` validates the default stack. There is
no partial-stack profile: every one of the nine services starts by default.

| Container | Binary | Surface |
|---|---|---|
| `gateway` | `qr-gateway` | `/health`, `/version`, the domain routes, and the static client |
| `data` | `qr-data` | `/health`, `/version` only until P3 |
| `model` | `qr-model` | `/health`, `/version` only until P4 |
| `backtest` | `qr-backtest` | queue consumer; no HTTP |
| `evidence` | `qr-evidence` | queue consumer; no HTTP |
| `simulation` | `qr-simulation` | `/health`, `/version` only until P7 |
| `postgres` | `postgres:17` | database |
| `minio` | `bitnamilegacy/minio` | object store |
| `rabbitmq` | `rabbitmq:4-management` | broker |

Only `gateway` publishes a port. The four HTTP services carry a healthcheck that
reads their own `/health`; the three infrastructure containers carry their own
probes. `backtest` and `evidence` serve no HTTP and publish no liveness probe, so
they have no healthcheck. Docker reports their process state; `/health` reports
the dependencies. A worker probe is part of the split.

## 3. Current-state notes

The three skeleton services are real binaries, not placeholders: each builds
from its own crate, answers `/health` and `/version`, and claims no domain
behavior. Their routes stay in the gateway until their split phases land:

| Service | Route owner until | Phase |
|---|---|---|
| `data` | gateway | P3 |
| `model` | gateway | P4 |
| `simulation` | gateway | P7 |

Two consequences are recorded rather than hidden:

- The `gateway` still serves dataset and config ingestion, so it carries the
  read-only `config/` mount that the target assigns to `data`. The mount moves
  with the split. `data` and `simulation` also mount `config/` read-only so the
  target topology is already in place.
- The new skeletons link no database, object-store, or queue client. They do not
  connect to Postgres and do not migrate; `QR_MIGRATE=0` is explicit.

A third finding is a defect in the current runtime, not in the stack: the
migration helper takes a session-level advisory lock and releases it through the
pool, so the lock and the unlock can land on different connections. Two services
that migrate at once therefore deadlock. Compose works around it by making the
gateway the only migrator (`QR_MIGRATE=1`) and setting `QR_MIGRATE=0` on every
other service. The follow-up is to pin the migration to one connection or to give
the schema owner the migration step; the Compose workaround is not a fix.

## 4. Object store source

MinIO withdrew its public images on 2026-09-11, and `quay.io/minio/minio` now
requires authentication. The Compose default is therefore a pinned Bitnami
legacy archive,
`bitnamilegacy/minio@sha256:451fe6858cb770cc9d0e77ba811ce287420f781c7c1b806a386f6896471a349c`,
which is `linux/amd64` and creates the `quantrule` bucket on first start.
`QR_OBJECT_STORE_IMAGE` overrides it. A permanent registry is a human decision,
not an implementation detail this stack may settle.

## 5. Image checks

```bash
docker run --rm quantrule-server:test --version
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
| `QR_POSTGRES_PASSWORD` | Postgres and the service URL |
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
docker compose up -d --build --wait
curl.exe -fsS http://localhost:8080/health
docker compose down
```

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
