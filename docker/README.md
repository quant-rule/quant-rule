# The server image and the Compose stack

- Status: Partial implementation of `DIST-022` (#643); three of the six binaries
  exist and the nine-container target remains
  [`docs/architecture/deployment.md`](../docs/architecture/deployment.md).
- Claim type: Behaviour Claim

One image, one published port, one bridge network. The image is `linux/amd64`;
Windows and macOS run it through Docker Desktop's Linux backend (WSL2 on
Windows). All state lives in named volumes; no host path is used except the
read-only `config/` mount that the target assigns to the `data` service.

## 1. What the image contains

| Path | Contents |
|---|---|
| `/usr/local/bin` | `qr-gateway`, `qr-backtest`, `qr-evidence` |
| `/srv/static` | The client built with Bun (`bun install --frozen-lockfile`, `bun run build`) |
| `/etc/quantrule-version` | The workspace version, baked at build time |
| `/usr/local/bin/quantrule-entry` | Selects the binary and answers `--version` |

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

`make up` is #644 and is not in this tree yet. `docker compose config` validates
the default stack; `docker compose --profile six-binaries config` validates the
target topology.

The default stack is six containers: `gateway`, `backtest`, `evidence`,
`postgres`, `minio`, and `rabbitmq`. Only `gateway` publishes a port.

## 3. The gap to nine containers

This tree carries three of the six service binaries.

| Target service | Binary | State |
|---|---|---|
| `gateway` | `qr-gateway` | present |
| `data` | `qr-data` | **missing** — `DIST-031` |
| `model` | `qr-model` | **missing** — `DIST-032` |
| `backtest` | `qr-backtest` | present |
| `evidence` | `qr-evidence` | present |
| `simulation` | `qr-simulation` | **missing** — `DIST-102` |

The three missing services are defined under the off-by-default `six-binaries`
profile. They cannot start until their binaries exist: enabling the profile
before the split fails at `exec`. The default `docker compose up` never selects
the profile, so a missing binary does not fail the stack.

Two consequences are recorded rather than hidden:

- The `gateway` currently serves dataset and config ingestion, so it carries the
  read-only `config/` mount that the target assigns to `data`. The mount moves
  with the split.
- `backtest` and `evidence` serve no HTTP and publish no liveness probe, so they
  have no healthcheck. `/health` reports their dependencies; the container
  runtime reports the process. A worker probe is part of the split.

A third finding is a defect in the current runtime, not in the stack: the
migration helper takes a session-level advisory lock and releases it through the
pool, so the lock and the unlock can land on different connections. Two services
that migrate at once therefore deadlock. Compose works around it by making the
gateway the only migrator (`QR_MIGRATE=1`) and setting `QR_MIGRATE=0` on every
worker and placeholder. The follow-up is to pin the migration to one connection
or to give the schema owner the migration step; the Compose workaround is not a
fix.

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
| `QR_POSTGRES_PASSWORD` | Postgres and the gateway URL |
| `QR_OBJECT_STORE_PASSWORD` | MinIO root password and the S3 secret |
| `QR_RABBITMQ_PASSWORD` | Broker password and the gateway URL |
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
`depends_on: condition: service_healthy`, profiles, YAML anchors, and
healthchecks; the provider must support all four. On this development host
`podman compose` delegates to Docker Compose v5.7.1 and the Docker daemon, so it
is not an independent Podman check.
