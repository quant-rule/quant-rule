# AI transparency log — DIST-022 six-binary image and nine-container stack

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Execution Plan (DIST-000, DIST-022); issue #643, claim type
  Behaviour Claim.
- Intent: finish the image and Compose work now that P2 merged the three
  skeleton service binaries. The default stack must start all nine containers,
  the image must carry all six binaries, and the off-by-default
  `six-binaries` profile must be retired.
- Boundary: `compose.yaml`, the `Dockerfile` build line needed to package the
  three new binaries, `docker/README.md`, the current-state and operator
  sections of `docs/architecture/deployment.md`, `CHANGELOG.md`, and this log.
  No Rust source, client source, Makefile, CI workflow, `services.md`, ADR, or
  port-parity file changed. P3's server/data edits were left untouched.

## Implementation

- `Dockerfile`: the three new binaries live in `quantrule-data-service`,
  `quantrule-model-service`, and `quantrule-simulation-service`, not in
  `quantrule-server`. Building only `-p quantrule-server` would package three of
  the six binaries, so the build now names all four packages and keeps the
  existing `qr-*` copy filter. No dependency, base image, or runtime package
  changed.
- `compose.yaml`: nine services start by default. One bridge network, three
  named volumes, and only `gateway` publishes `8080`. `gateway`, `data`, `model`,
  and `simulation` probe their own `/health`; Postgres, MinIO, and RabbitMQ keep
  their native probes; `backtest` and `evidence` have no HTTP surface and
  therefore no healthcheck. `depends_on: service_healthy` remains on the gateway
  and the two workers that connect to Postgres, the object store, and the
  broker.
- The skeletons link no database, object-store, or queue client and do not
  migrate; `QR_MIGRATE=0` is explicit on every non-gateway service. The gateway
  remains the sole migrator (`QR_MIGRATE=1`) until the runtime pins the
  advisory lock to one connection.
- The `six-binaries` profile is removed. A partial stack has no remaining use:
  every binary exists, and one Compose contract is less likely to drift than a
  default plus a profile.
- `docker/README.md` and `docs/architecture/deployment.md` now describe six of
  six binaries and nine containers. The MinIO registry stopgap remains visible
  and overridable with `QR_OBJECT_STORE_IMAGE`.
- `CHANGELOG.md`: the Unreleased image/Compose entry was rewritten to the final
  nine-container state instead of leaving a six-container entry beside it.

## Verification — 2026-11-14 (UTC)

| Check | Command | Observed |
|---|---|---|
| Compose parse | `docker compose config -q` | exit 0; `config --services` lists nine |
| Image build | `docker build -t quantrule-server:test .` | exit 0; manifest list `sha256:f17147eca4ab8994672a4366a5a081c626234b2e399a276e006df58597750371` |
| Image binaries | `docker run --rm quantrule-server:test sh -c 'ls -1 /usr/local/bin'` | `qr-backtest`, `qr-data`, `qr-evidence`, `qr-gateway`, `qr-model`, `qr-simulation`, `quantrule-entry` |
| Clean start | `docker compose down -v && docker compose up -d --build --wait` | exit 0; nine containers created; gateway `migrations applied applied=12` |
| Gateway health | `curl -fsS http://localhost:8080/health` | `state=healthy`; postgres, object-store, queue healthy |
| Static client | `curl -fsS http://localhost:8080/` | `200 text/html; charset=utf-8` |
| Skeleton health | `docker compose exec -T {data,model,simulation} curl -fsS http://127.0.0.1:8080/health` | each `state=healthy` with an empty dependency list |
| No Python | `docker run --rm quantrule-server:test sh -c 'command -v python3'` | empty |
| No Bun | `docker run --rm quantrule-server:test sh -c 'command -v bun'` | empty |
| No `.py` | `docker run --rm quantrule-server:test sh -c 'find / -name "*.py" 2>/dev/null \| head'` | empty |
| One-command path | `make up` | exit 0; `QuantRule v0.3.0 ready → http://localhost:8080` |
| Published ports | `docker compose ps --format json` | only `gateway` publishes `8080`; every other container `PublishedPort 0` |
| Workspace gate | `tools/gate.sh` | exit 0; fmt, clippy, tests, rustdoc, and every checker passed |
| Whitespace | `git diff --check` | exit 0 |
| Stack stop | `docker compose down` | exit 0 |

Container health at the end of the run:

| Container | State |
|---|---|
| `gateway` | healthy |
| `data` | healthy |
| `model` | healthy |
| `backtest` | up; no HTTP probe |
| `evidence` | up; no HTTP probe |
| `simulation` | healthy |
| `postgres` | healthy |
| `minio` | healthy |
| `rabbitmq` | healthy |

`docker compose up --wait` exits 0 and reports every container ready. Compose
prints "Healthy" for a running container without a healthcheck, so the table
above uses `docker compose ps`, where `backtest` and `evidence` carry no health
status.

## Learning and rejected alternatives

- The three P2 binaries are separate crates. The image build line, not the
  runtime, was the only build change needed; the workspace already compiled all
  six.
- Rejected: a process-liveness healthcheck for `backtest` and `evidence` (for
  example reading `/proc/1/comm`). It would report healthy from the container
  being alive, which is exactly what `/health` is defined not to do. A real
  worker probe needs a surface the binary does not have and is part of the
  split.
- Rejected: keeping an optional `six-binaries` profile for a partial stack.
  Every binary exists; a second topology would be a second contract to verify.
- Rejected: dropping the database URL from the skeletons' environment. The
  runtime defaults it anyway, and the actual guarantee is that the crates link
  no database client and never call the migration runner. The explicit
  `QR_MIGRATE=0` records that.
- Rejected: moving migrations off the gateway in this change. The lock is still
  unsafe; the workaround stays until P1's pinned connection or the schema split
  lands.
- The root `README.md` still says the tree starts six containers. It is outside
  this issue's write boundary and is reported as a documentation follow-up, not
  edited here.

## Not verified here

The Windows Docker Desktop and standalone Podman smoke checks remain manual
notes. The data, model, and simulation domain routes, schema ownership, the
gateway proxy, and the worker HTTP path belong to P3-P7. The permanent
object-store registry is a human decision, not a claim of this change.

## Follow-up after P3 (#792) — 2026-11-14 (UTC)

- Merge: #792 merged to `main` at `fffb33c3`; `origin/main` merged into
  `feat/643-compose` at `93693b4c`.
- Intent: enable `qr-data` in the default stack now that it owns the catalog,
  and replace the sole-migrator workaround with per-ledger migration.
- Boundary: `compose.yaml`, `docker/README.md`, the current-state and
  configuration sections of `docs/architecture/deployment.md`, `CHANGELOG.md`,
  and this log. No Rust, client, Makefile, CI, `services.md`, ADR, or
  port-parity change.
- Compose: `QR_DATA_URL=http://data:8080` is set for the gateway and the
  backtest worker, which both refuse to start without it. `QR_MIGRATE=1` is set
  on `gateway` and `data`; the workers, `model`, and `simulation` keep `0`
  because the gateway owns the `runs` ledger they consume. The gateway now
  depends on `data` being healthy, and `backtest` does too. The `config/` mount
  moved from the gateway to `data`, its only reader; `data` and `simulation`
  keep the read-only mount.
- The gateway applies the `gateway` and `runs` ledgers (`gateway=1 service=6`)
  and `data` applies the `catalog` ledger (`applied=5`) on a clean database.
  Both ran concurrently with no deadlock; the P1 pinned advisory lock works as
  intended, so the sole-migrator workaround is gone.

### Verification after P3 — 2026-11-14 (UTC)

| Check | Command | Observed |
|---|---|---|
| Compose parse | `docker compose config -q` | exit 0; nine services |
| Image build | `docker build -t quantrule-server:test .` | exit 0; manifest list `sha256:5c7ab940d5fa0fe8338b58249d9ad8a139a1e87c54fc2edcf4454280278a8c89` |
| Clean start | `docker compose down -v && docker compose up -d --build --wait` | exit 0; nine containers; `UP_EXIT=0` |
| Gateway health | `curl -fsS http://localhost:8080/health` | `state=healthy`; postgres, object-store, queue healthy |
| Gateway catalog | `curl -fsS -H 'X-QuantRule-Profile: default' http://localhost:8080/catalog/datasets` | `200`; `[]` |
| Gateway upload | `curl ... --data-binary @mini.csv 'http://localhost:8080/catalog/datasets?name=mini.csv'` | `201`; `data_version sha256:ae392d2b…`, 2 candles |
| Gateway list after upload | same GET | one record, `mini.csv`, 2 candles |
| Gateway migrations | `docker compose logs gateway` | `migrations applied gateway=1 service=6` |
| Data migrations | `docker compose logs data` | `catalog migrations applied applied=5` |
| Static client | `curl -fsS http://localhost:8080/` | `200 text/html` |
| No Python | `docker run --rm quantrule-server:test sh -c 'command -v python3'` | empty |
| No Bun | `docker run --rm quantrule-server:test sh -c 'command -v bun'` | empty |
| No `.py` | `docker run --rm quantrule-server:test sh -c 'find / -name "*.py" 2>/dev/null \| head'` | empty |
| One-command path | `make up` | exit 0; `QuantRule v0.3.0 ready → http://localhost:8080` |
| Published ports | `docker compose ps --format json` | only `gateway` `8080`; every other container none |
| Workspace gate | `tools/gate.sh` | exit 0; `workspace gate passed` |

Container health after P3:

| Container | State |
|---|---|
| `gateway` | healthy |
| `data` | healthy |
| `model` | healthy |
| `backtest` | up; no HTTP probe |
| `evidence` | up; no HTTP probe |
| `simulation` | healthy |
| `postgres` | healthy |
| `minio` | healthy |
| `rabbitmq` | healthy |

### Learning and rejected alternatives after P3

- P3 makes `QR_DATA_URL` mandatory for the gateway and the backtest worker. A
  Compose file that set it only on the gateway would leave `qr-backtest` failing
  at startup, so the shared environment carries it.
- Only `data` reads `config_root` after P3; the gateway proxy has no config
  handler. Keeping the gateway's `config/` mount would be a stale second copy.
- Rejected: keeping the gateway as the sole migrator. It would leave
  `catalog.schema_migrations` empty and the proxy would answer 500 for an
  unmigrated catalog, which P3 records as the dependency on this issue.
- Rejected: an `mc`/SQL init container for the catalog ledger. `qr-data` owns the
  ledger and applies it on start; a second migrator would duplicate the owner.
- The workers keep `QR_MIGRATE=0`: they share the `runs` ledger the gateway
  already applies, and starting a second writer would reintroduce contention for
  no benefit.

### Not verified here

The Windows Docker Desktop and standalone Podman smoke checks remain manual
notes. Model and simulation domain routes, the remaining split phases, and the
permanent object-store registry are outside this change.

## Follow-up after P4 (#793) — 2026-11-14 (UTC)

- Merge: #793 merged to `main` at `64550efa`; `origin/main` merged into
  `feat/643-compose` at `886b5bc9`.
- Intent: enable `qr-model` in the default stack now that it serves
  `/model/compute`, and wire the gateway's required `QR_MODEL_URL`.
- Boundary: `compose.yaml`, `docker/README.md`, the current-state and
  configuration sections of `docs/architecture/deployment.md`,
  `CHANGELOG.md`, and this log. No Rust, client, Makefile, CI, `services.md`,
  ADR, or port-parity change.
- Compose: `QR_MODEL_URL=http://model:8080` is set for the gateway, which
  refuses to start without it. The gateway now depends on `model` being
  healthy. `model` depends on `data` and the object store; it owns no database
  schema and keeps `QR_MIGRATE=0`. `QR_DATA_URL`, the gateway/data migration
  split, the `config/` mount ownership, the three named volumes, the single
  network, and the single published port are unchanged.
- The model fixture path is real: `/catalog/configs` scans the repository
  mount, `examples/verify-mini` is valid (`cfg_312c0eec3438`), and the model
  route reads the catalogued dataset and config through `qr-data` and the
  object store. No model data was stubbed.

### Verification after P4 — 2026-11-14 (UTC)

| Check | Command | Observed |
|---|---|---|
| Compose parse | `docker compose config -q` | exit 0; nine services |
| Image build | `docker build -t quantrule-server:test .` | exit 0; manifest list `sha256:e78dbabe367ccc351cf5b9274c96cdb6700471e0d0db23d1fa78d18aa34e8939` |
| Clean start | `docker compose down -v && docker compose up -d --build --wait` | exit 0; nine containers |
| Gateway health | `curl -fsS http://localhost:8080/health` | `state=healthy`; postgres and queue healthy |
| Data probe | `curl -H 'X-QuantRule-Profile: default' http://localhost:8080/catalog/datasets` | `200`; `[]` before upload |
| Config scan | `curl -H 'X-QuantRule-Profile: default' http://localhost:8080/catalog/configs` | `200`; `examples/verify-mini` valid, `cfg_312c0eec3438` |
| Dataset upload | `POST /catalog/datasets?name=model-fixture.csv` | `201`; 250 candles; `sha256:3c0106f8…` |
| Model compute | `POST /model/compute` with the dataset version and `cfg_312c0eec3438` | `200`; timeframes `1m,5m,15m,1h`; 66,022 bytes; `sha256:e12f8069…` |
| Gateway migrations | `docker compose logs gateway` | `migrations applied gateway=1 service=6` |
| Data migrations | `docker compose logs data` | `catalog migrations applied applied=5` |
| Model startup | `docker compose logs model` | `starting model service bind=0.0.0.0:8080`; no migration line |
| No Python | `docker run --rm quantrule-server:test sh -c 'command -v python3'` | empty |
| No Bun | `docker run --rm quantrule-server:test sh -c 'command -v bun'` | empty |
| No `.py` | `docker run --rm quantrule-server:test sh -c 'find / -name "*.py" 2>/dev/null \| head'` | empty |
| One-command path | `make up` | exit 0; `QuantRule v0.3.0 ready → http://localhost:8080` |
| Workspace gate | `tools/gate.sh` | exit 0; `workspace gate passed` |

Container health after P4:

| Container | State |
|---|---|
| `gateway` | healthy |
| `data` | healthy |
| `model` | healthy |
| `backtest` | up; no HTTP probe |
| `evidence` | up; no HTTP probe |
| `simulation` | healthy |
| `postgres` | healthy |
| `minio` | healthy |
| `rabbitmq` | healthy |

### Learning and rejected alternatives after P4

- The model route is not a stub: a real catalogued config and a 250-candle
  dataset were uploaded through the gateway, and the response is the full
  four-timeframe model object. A 500 would have been the unmigrated or
  unreachable data service, not the model.
- Rejected: giving `model` a database URL and `QR_MIGRATE=1`. It owns no
  schema; `qr-model` applies no migration, and the model reads the catalog
  through `qr-data`.
- Rejected: giving `model` the `config/` mount. It resolves the config bundle
  through the data service and reads only dataset objects from the object
  store.
- The gateway's `/health` reports postgres and queue after P4; object-store
  checks now belong to the service that owns them. The healthcheck still reads
  the aggregate state.

### Not verified here

The Windows Docker Desktop and standalone Podman smoke checks remain manual
notes. The simulation domain routes, the remaining split phases, and the
permanent object-store registry are outside this change.

## Follow-up after P5 (#794) — 2026-11-14 (UTC)

- Merge: #794 merged to `main` at `80b590f2`; `origin/main` merged into
  `feat/643-compose` at `e0317d53`.
- Intent: enable `qr-evidence` as an HTTP service, wire the gateway's required
  `QR_EVIDENCE_URL`, and move the `runs`/`research` migration ledgers from the
  gateway to evidence.
- Boundary: `compose.yaml`, `Dockerfile` (the evidence package was missing from
  the build), `docker/README.md`, the current-state and configuration sections
  of `docs/architecture/deployment.md`, `CHANGELOG.md`, and this log. No Rust,
  client, Makefile, CI, `services.md`, ADR, or port-parity change.
- Compose: `QR_EVIDENCE_URL=http://evidence:8080` is set for the gateway and the
  backtest worker, which both refuse to start without it. The gateway depends on
  `data`, `model`, and `evidence` being healthy. Evidence depends on data,
  Postgres, the object store, and the broker, serves its own `/health`, and
  applies the `runs`/`research` ledgers with `QR_MIGRATE=1`. The gateway applies
  only the `gateway` ledger; data applies `catalog`. Backtest depends on
  evidence and no longer needs a direct Postgres dependency.
- Build fix: P5 moved `qr-evidence` from `quantrule-server` to
  `quantrule-evidence-service`. The first image build still named only the old
  package, so Cargo's cache supplied the previous P4 `qr-evidence` worker from
  `target/release`; the container came up as a queue consumer with no HTTP
  server and failed its `/health` probe. The Dockerfile now builds
  `-p quantrule-evidence-service` explicitly. This is a packaging defect the
  real stack caught; the stale binary was not a runtime behavior change.

### Verification after P5 — 2026-11-14 (UTC)

| Check | Command | Observed |
|---|---|---|
| Compose parse | `docker compose config -q` | exit 0; nine services |
| Image build | `docker build -t quantrule-server:test .` | exit 0; manifest list `sha256:8a14f94d512733a78b7143602f059d424175785f2ad999dfbc8fe65fdbee2f18` |
| Clean start | `docker compose down -v && docker compose up -d --build --wait` | exit 0; nine containers |
| Gateway health | `curl -fsS http://localhost:8080/health` | `state=healthy`; postgres healthy |
| Data probe | `curl -H 'X-QuantRule-Profile: default' http://localhost:8080/catalog/datasets` | `200`; `[]` before upload |
| Dataset upload | `POST /catalog/datasets?name=p5-fixture.csv` | `201`; 250 candles; `sha256:3c0106f8…` |
| Model probe | `POST /model/compute` with the dataset and `cfg_312c0eec3438` | `200`; timeframes `1m,5m,15m,1h`; 66,022 bytes; `sha256:e12f8069…` |
| Run submission | `POST /runs` with the same dataset and config | `202`; `run_id sha256:a201c650…`; `queued`, `created=true` |
| Run read | `GET /runs/{id}` | `200`; `status=succeeded`; all four stages `done`; artifacts present |
| Run list | `GET /runs` | `200`; one run with the same id |
| Gateway ledger | `gateway.schema_migrations` | `7` |
| Catalog ledger | `catalog.schema_migrations` | `1,2,3,4,5` |
| Runs ledger | `runs.schema_migrations` | `2,4,5,8,9,13` |
| Migration ordering | gateway/data/evidence logs | data 19:25:18, evidence 19:25:24, gateway 19:25:30; all applied; no deadlock or panic |
| No Python | `docker run --rm quantrule-server:test sh -c 'command -v python3'` | empty |
| No Bun | `docker run --rm quantrule-server:test sh -c 'command -v bun'` | empty |
| No `.py` | `docker run --rm quantrule-server:test sh -c 'find / -name "*.py" 2>/dev/null \| head'` | empty |
| One-command path | `make up` | exit 0; `QuantRule v0.3.0 ready → http://localhost:8080` |
| Workspace gate | `tools/gate.sh` | exit 0; `workspace gate passed` |

Container health after P5:

| Container | State |
|---|---|
| `gateway` | healthy |
| `data` | healthy |
| `model` | healthy |
| `evidence` | healthy |
| `backtest` | up; no HTTP probe |
| `simulation` | healthy |
| `postgres` | healthy |
| `minio` | healthy |
| `rabbitmq` | healthy |

### Learning and rejected alternatives after P5

- The image must name every service package that owns a moved binary. Building
  only the old package is not a compile error; it silently packages the cached
  previous binary. The explicit `-p quantrule-evidence-service` is the fix, and
  the real `/health` probe is what caught it.
- Rejected: pinning migrations back to the gateway to avoid the concurrent
  start. The ledgers are owned by their services now; the pinned advisory lock
  serialized all three without deadlock, and the run completed.
- Rejected: giving evidence a `config/` mount. It resolves config bundles through
  `qr-data`, like model.
- The gateway `/health` now reports only Postgres; queue and object-store probes
  belong to the services that own them. The healthcheck still reads the
  aggregate state.

### Not verified here

The Windows Docker Desktop and standalone Podman smoke checks remain manual
notes. The simulation domain routes, the P6 backtest-service move, and the
permanent object-store registry are outside this change.
