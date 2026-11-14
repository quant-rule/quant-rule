# AI transparency log — DIST-022 server image and Compose stack

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: build the single multi-binary server image, the Compose topology, and
  the deployment notes for `DIST-022` (#643).
- Boundary: packaging only. The Dockerfile, `.dockerignore`, `compose.yaml`,
  `docker/`, a current-state note in `docs/architecture/deployment.md`, the
  changelog entry, and this log. No Rust source, client source, Makefile, CI
  workflow, or engine behavior changed. The missing binaries are `qr-data`
  (`DIST-031`), `qr-model` (`DIST-032`), and `qr-simulation` (`DIST-102`); none
  is in this tree.
- Verification: see the table below. The image commands and the Compose startup
  were run on this host; the Windows and Fedora/Podman checks are notes, not
  executed.
- Learning: the runtime already defaults its database, object-store, and broker
  URLs to the Compose host names, so the image needs only paths in its
  environment. The gateway currently serves dataset and config ingestion, which
  is why it carries the read-only `config/` mount that the target assigns to
  `data`. The Rust build is not self-contained in `crates/`: `quantrule-app`
  includes `docs/registry/indicators.json` at compile time and
  `quantrule-backtest` generates its strategy table from the same file, so the
  build context must carry that one file.
- Rejected: copying `tools/` into the image to satisfy a build step. The client
  build needs only `apps/web`; the repository tooling stays outside the context.
  Also rejected: faking `qr-data`, `qr-model`, and `qr-simulation` as shell
  wrappers to reach nine healthy containers. A placeholder that reports healthy
  without the service would be a false claim.

## Implementation — 2026-11-14 (UTC)

- Intent: one image, one published port, named volumes, health-gated startup,
  and an off-by-default profile for the three services whose binaries do not
  exist.
- Boundary: `data`, `model`, and `simulation` are defined but cannot start; the
  default `docker compose up` never selects them. `backtest` and `evidence` have
  no healthcheck because they serve no HTTP and publish no liveness probe.
- MinIO withdrew its public images on 2026-09-11. The Compose default pins the
  available Bitnami legacy archive by digest, and `QR_OBJECT_STORE_IMAGE`
  overrides it. A permanent registry is a human decision, recorded as a risk in
  the pull request.
- Verification: recorded below with the observed output.
- Learning: the Bitnami MinIO image creates `MINIO_DEFAULT_BUCKETS` on first
  start, which removes the need for a separate bucket-init container. The S3
  adapter assumes the bucket exists, so that bootstrap is required.
- Finding: the runtime's migration helper takes a session-level advisory lock
  and releases it through the pool, so the lock and the unlock can land on
  different connections. A second service that migrates blocks forever on the
  leaked lock; the first clean start made the gateway unhealthy for exactly this
  reason. The Compose file makes the gateway the only migrator and sets
  `QR_MIGRATE=0` on every worker and placeholder. That is a workaround; pinning
  the migration to one connection is the fix and is outside this issue's write
  boundary.
- Rejected: an `mc` init sidecar. It adds a container and a second image for one
  idempotent bucket creation the object-store image already performs.

## Verification — 2026-11-14 (UTC)

| Check | Command | Observed |
|---|---|---|
| Compose parse | `docker compose config -q` | exit 0 |
| Profile parse | `docker compose --profile six-binaries config -q` | exit 0 |
| Image build | `docker build -t quantrule-server:test .` | exit 0 |
| Image version | `docker run --rm quantrule-server:test --version` | `quantrule-server 0.3.0` |
| No Python | `docker run --rm quantrule-server:test sh -c 'command -v python3'` | empty |
| No Bun | `docker run --rm quantrule-server:test sh -c 'command -v bun'` | empty |
| No `.py` | `docker run --rm quantrule-server:test sh -c 'find / -name "*.py" 2>/dev/null \| head'` | empty |
| Empty database | `docker compose down -v && docker compose up -d --wait` | exit 0; `migrations applied applied=12` |
| Health | `curl -fsS http://localhost:8080/health` | `state=healthy`; postgres, object-store, queue healthy |
| Client | `curl -fsS http://localhost:8080/` | `200`, `text/html` |
| Workspace gate | `tools/gate.sh` | exit 0; 69 test binaries, 1870 tests passed |
| Podman | `podman compose config` | provider delegates to Docker Compose v5.7.1; not an independent Podman parse |
| Windows | manual smoke note | not executed |

The first `docker compose up` before the migration workaround started six
containers but left the gateway unhealthy: `backtest`, `evidence`, and
`gateway` all ran migrations, one process leaked the advisory lock, and the
other two waited on it. The workaround and the finding above are the record.

