# Changelog

All notable changes to QuantRule are documented here. The changelog is grouped
by release and records meaningful user-facing or developer-facing changes. It is
the release history, not the execution tracker: issues, sub-issues, pull
requests, and their status live on the GitHub Project.

The changelog starts at the 2026-11-07 workflow documentation reset. Earlier
release notes are in [`docs/releases/`](docs/releases/), and the detailed change
history is in Git and the pull requests.

> Release tags were renumbered on 2026-11-15 so the history reads `v0.1.0`
> through `v1.0.0`. On 2026-11-16 the release-note filenames and the sections
> below were normalized to those current tags, and the next two releases were
> cut as `v1.1.0` and `v1.2.0`. Each release note keeps its historical heading
> and records the version it was prepared as. The current tag is `v1.5.2`. See
> [`docs/releases/README.md`](docs/releases/README.md) for the mapping.

## [Unreleased]

### Added

- The v3 observability tier is executable: `images` requires 16 pins (adding `node_exporter`, `promtail`, and `blackbox_exporter`), Prometheus and Alertmanager must sit on distinct declared hosts, and no four units may share one host (`deploy.observability.placement`). Prometheus scrapes the seven services through `tasks.<service>:9100`, Traefik 8082, Patroni 8008 with `basic_auth.password_file`, etcd 2381, HAProxy `8404/metrics`, RabbitMQ 15692, MinIO HTTPS 9000, node_exporter, blackbox, and itself.
- 38 bounded alert rules cover the 17 HA failure rows `F01`..`F17` with a `failure_row` label; the rules use Patroni 4 `patroni_primary`/`patroni_sync_standby` and received-minus-replayed lag, MinIO v2 drive/erasure-set/cluster-health names, and add `GatewayReplicaDown`, `EtcdNodeDown`, `PostgresNoPrimaryBackend`, `MinioClusterUnhealthy`, and `SwarmManagerQuorumAtRisk`. Alertmanager has one internal `quantrule-null` receiver and no external notification. Loki keeps 30 days of Docker JSON logs from a five-label-whitelisted promtail, Grafana serves four read-only dashboards as anonymous Viewer, and Prometheus keeps 30d/20GB. `tools/observability.sh` copies, starts, waits, verifies, and writes `quantrule_swarm_managers_ready` from `docker node ls`; no container socket is added.
- `qr-deploy check` adds 28 Docker-free observability rows: placement, the four units, per-host agents, 14 scrape jobs and their auth/TLS, 38 rules and the 17-row HA matrix, the null receiver, Loki/promtail/blackbox, Grafana provisioning/dashboards, the manifest, 50m×5 logging, internal Traefik/HAProxy metrics, and the Docker-socket/port boundaries. The `ha` tier starts the tier, proves `GatewaySyntheticDown` with a promtool rule unit, and asserts `SwarmManagerQuorumAtRisk` and `GatewayReplicaDown` fire and resolve during the existing reversible phases; `--skip-observability` leaves the row `Unverified`.
- The runtime publishes `quantrule_leader_term` from the evidence leadership, and evidence publishes `quantrule_runs_missing_digest`; the verify-age textfile is opt-in and its rule stays dormant without it.
- Traefik exposes internal metrics on 8082 (never published), HAProxy serves native metrics on the existing 8404 stats listener, MinIO sets `MINIO_PROMETHEUS_AUTH_TYPE=public` on the overlay, and app/infra units use `json-file` 50m×5 rotation.
- The v3 MinIO tier is executable: four nodes × one drive at EC:2 (lab 3 × 2 at EC:3) behind a health-checked HAProxy `minio:9000` TCP front door; `minio-<host>` nodes carry TLS-only 9000 and no console or published port. `stateful.minio.parity` is `auto` or `EC:<n>` and must exceed the largest per-node drive count so one node loss keeps the write quorum.
- `minio_root_user` and `minio_root_password` are required `minio`-scope stateful files copied only to MinIO hosts; the app `object_store_access_key` is the dedicated `qr_app` user, and `images.minio_client` runs the one-shot `mc`. The object bootstrap creates `quantrule`, enables versioning, recreates the bucket-scoped `quantrule-app` policy, adds `qr_app`, and proves put/get/delete.
- `qr-deploy check` adds the MinIO rows: the exact `MINIO_VOLUMES`/`EC:N`/browser/root `_FILE`/region config, the layout and write-quorum rule after one node loss (including refusal of `[2,1,1]`), the unique unit key/hostname/alias and `/certs` wrapper with no published 9001, the HAProxy `minio`/9000 health-checked frontend and shutdown-sessions, the manifest host selector/modes/SANs, the app `https://minio:9000`/`_FILE`/`qr_app` path with `object_replication: false`, and the MinIO leaf SAN/expiry. The rows stay Docker-free and read `--out`/`QR_DEPLOY_OUT`.
- The `ha` tier adds the MinIO node-loss phase: it writes a `qr_app` probe through `https://minio:9000` and records its SHA-256, stops one node whose loss leaves the EC:3 write quorum with `docker stop`, records the reduced `mc admin info` and re-reads the probe hash, starts the node, runs a bounded `mc admin heal -r qr/quantrule`, waits for zero healing, and re-reads the probe. Evidence is value-free `Simulated HA`; `--skip-object` leaves the row `Unverified`, the end-to-end digest is `Measured` only after `tools/verify.sh` (otherwise `Unverified`/`Blocked`), and `Real HA` remains HA-014.
- `backup.object_replication` must be false in v1.5.10; PLAT-005 owns local erasure/versioning, and HA-013 owns off-host/archive copies, restore drills, and any second-site replication. No object lock is added.
- The v3 queue tier is executable: three RabbitMQ brokers use short names (`rabbit@rabbitmq-<host>`), classic peer discovery, `pause_minority`, TLS-only 5671, `default_queue_type = quorum`, a 5 GiB disk alarm, a 60% memory high-watermark, JSON console logs, and `rabbitmq_prometheus` on 15692. A TCP `rabbitmq:5671` frontend on the HAProxy pair is health-checked; the shared `rabbitmq` alias is no longer on a broker.
- `stateful.rabbitmq.disk_free_limit_mib` (default 5120) and `memory_high_watermark_percent` (default 60) are validated; the broker list must be odd and at least three. `queue_url` must name `qr_queue`, the shared `rabbitmq` host, the `%2f` vhost, and port 5671.
- A required `rabbitmq_erlang_cookie` stateful secret (32+ URL-safe characters, distinct) is copied only to the brokers and installed as `.erlang.cookie` owner `rabbitmq` mode 600. The stateful manifest now carries a per-entry `hosts` selector, so the broker key, cookie, config, and queue PKI never reach a database host.
- The RabbitMQ boot order seeds the first broker, waits for `rabbitmq-diagnostics -q ping` and `rabbitmqctl await_startup`, then starts the other two. `tools/stateful.sh wait` checks three running nodes and no alarms; `queue-bootstrap` creates the app user, vhost, and permissions from `queue_url` with the password on stdin and a hash in the command, never a password argument.
- `quantrule.deploy/v3` renders per-broker `rabbitmq-<host>.conf`, `rabbitmq-enabled_plugins`, the RabbitMQ leaf, and `PKI_EXTRA_SANS` coverage of `rabbitmq` plus every `rabbitmq-<host>`.
- `qr-deploy check` adds the queue rows: TLS-only 5671 with no plaintext/management listener, the three classic short names, `pause_minority`, quorum default, alarms, Prometheus 15692, JSON logs, the HAProxy `rabbitmq`/5671 health-checked frontend, the broker unit (hostname/node name, mounts, cookie/TLS install, no long name/shared alias/published port), the manifest per-entry `hosts`/modes, and the RabbitMQ leaf SAN/expiry through `QR_DEPLOY_OPENSSL`. The rows stay Docker-free and read `--out`/`QR_DEPLOY_OUT`.
- `tools/queue-definitions.sh` exports the four HA-005 quorum queues from a broker, strips users/permissions/topic permissions, verifies the queues, delivery limit 3, DLX/DLQ, bindings, and no `runs.*`/`stages.*` policy, and stores mode-600 `<UTC>.json` plus a value-free manifest. It verifies live or stored definitions, imports only into an empty cluster (a live `.g*` generation needs `--force`), never purges/deletes, and prunes definition files after 30 days; HA-013 owns archive/off-host copy and the empty-broker drill.
- The `ha` tier adds the queue broker-loss phase: it stops the quorum leader with `docker stop`, proves two running nodes, a new leader, all queues quorum, no alarms, and a publisher-confirmed probe consumed through `rabbitmq:5671`, restores with `docker start`, publishes again, checks the app `/health`/`/ready`, compares the digest when available, and writes scoped `Simulated HA` evidence. `--skip-queue` leaves the row `Unverified`; the end-to-end digest is `Measured` only after `tools/verify.sh` succeeds (otherwise `Unverified`) and `Real HA` remains HA-014. The lab-only `queue_probe` example publishes, drains, and consumes the durable `qr.lab.queue-probe` queue without deleting it.

- The v3 stateful tier is executable: `images.patroni` is the twelfth pinned infrastructure image at that step (`images.minio_client` is the thirteenth), `stateful.postgres.watchdog` selects `off`, `automatic`, or `required`, and `synchronous_standby` must be exactly one. The renderer writes per-host Patroni, etcd, and HAProxy configuration with strict synchronous mode, overlay aliases (`patroni-<host>`, `etcd-<host>`), archive/recovery placeholders, etcd mTLS, Patroni REST auth, HAProxy `/primary` TCP passthrough, a `postgres` alias on both edges, and no published port; only `required` maps a watchdog device.
- Four stateful secrets (`postgres_superuser_password`, `patroni_replication_password`, `patroni_rewind_password`, `patroni_rest_password`) are required, distinct, strong, and URL-safe; `secrets/manifest.json` records their `stateful` scope, `bootstrap.sh` creates only app Swarm secrets, and `tools/stateful.sh` copies the stateful files to the hosts.
- `jobs/cluster-bootstrap.sh` creates the four databases and eight roles before `db-bootstrap.sh` and `migrate.sh`; `bootstrap.sh --network-only` creates or validates the overlay and exits before secrets, labels, jobs, or stack; `tools/stateful.sh` renders, copies, starts etcd -> Patroni -> HAProxy, and waits for quorum, leader, sync standby, and a primary psql. `deploy/v3/deploy.lab.json` is the three-VM `lab-single-host` fixture.
- `qr-deploy check` adds the stateful rows: Patroni strict sync/rewind/archive/TLS/pg_hba/aliases/REST/watchdog, three-member etcd mTLS and metrics, HAProxy `/primary`/`servers`/`shutdown-sessions`/overlay stats, the manifest SANs and modes, and the Postgres/etcd certificate SANs through `QR_DEPLOY_OPENSSL`; `--out` or `QR_DEPLOY_OUT` reads the rendered tree, and a missing `openssl` is a failed `limited` row.
- The `ha` tier defaults to `deploy/v3/deploy.lab.json` and adds the database phase: marker and LSN baseline, a `docker pause` of the Patroni primary with the watchdog off, a new leader and sync standby with measured RTO, a second write through `postgres`, digest/LSN comparison, a one-HAProxy pause whose control dials the surviving edge with `PGHOSTADDR` while keeping `verify-full`, the 8404/5432/8080/9000 negative controls, the rejoin (pg_rewind/reinit), and a value-free evidence JSON. `--skip-database` leaves the row `Unverified`; a three-VM lab is `Simulated HA`, never `Real HA`.

- `qr-deploy` reads the closed `quantrule.deploy/v3` schema and validates the Swarm HA placement, 443-only ingress, failure domains, stateful quorum, TLS endpoints, digest pins, and `secrets.files`; v2 parsing, rendering, checking, and printed output are unchanged.
- The v3 ingress adds `ingress.vip` and `ingress.interface` (default `eth0`), a checked-in `deploy/v3/traefik.yml`, a rendered `traefik-dynamic.yml`, per-edge `keepalived-<host>.conf`, and a pinned keepalived unit on each edge host. Traefik has one 443 entrypoint with no port 80, no dashboard or public metrics, TLS 1.2+, 900 s timeouts, a 512 MiB body limit, HSTS one year without preload, nosniff, no-referrer, and `Server` removed. Cert mode mounts the certificate and key at `/etc/traefik/certs/`; there is no writable ACME state. The `acme` variant is reserved for the future path and is refused by validation with `deploy.ingress.tls` until that path lands.
- `qr-deploy check` adds the PLAT-002 ingress rows: static/dynamic/keepalived configuration, certificate file mode plus a `CertificateQuery` SAN/30-day check (`QR_DEPLOY_OPENSSL`), HTTPS response headers, port-80 closure, the four remote ingress mounts, and a `limited` VIP/firewall row. The shell gains `QUANTRULE_CA_CERT` for the private CA, `tools/verify.sh` gains `--cacert`/`QUANTRULE_CA_CERT`/`CURL_CA_BUNDLE`, and `deploy/v3/test.sh` gains a VIP-holder drain, a bounded 1 Hz `/health` probe, negative controls, `QR_SWARM_CACERT`, and `Simulated HA`/`UNVERIFIED` evidence.
- The v3 `bootstrap.sh` creates `quantrule_traefik` and `quantrule_traefik_dynamic` from the rendered static and dynamic files with the `quantrule.source_sha256` drift label before deploying the stack; a changed file stops the run, and cert mode still creates the ingress cert config and key secret.
- `quantrule.deploy/v3` now declares `swarm.managers` (three odd, distinct, declared hosts), places four MinIO nodes with one drive each in four failure domains, and renders service `/health` checks plus a health-gated rolling update. The reference topology uses `stop-first` because it has no spare slot; `start-first` is selected only when one exists.
- The generated `bootstrap.sh` applies the exact `quantrule.host`, `.domain`, `.profile`, `.deploy`, `quantrule.<service>` and `quantrule.ingress` labels, resolves a host by label then hostname then unique address, refuses ambiguity, a drained node, a foreign deployment, or a changed secret/config file (by its `quantrule.source_sha256` label), runs the database bootstrap and migrations before the stack, and waits for the pinned image and the one 443 port. Rotation remains a new object name plus a rolling update.
- The checked-in `tools/swarm.sh` wrapper exposes `render`, `validate`, `bootstrap`, `status`, `upgrade`, `rollback`, and `verify`; `upgrade` snapshots the previous stack before rendering and `rollback` redeploys it without removing the stack, volumes, secrets, or configs. `make swarm-*` targets call the same script; `make up` is unchanged.
- `qr-deploy render --target swarm|infra|all` writes the seven-service stack, the pinned per-host Patroni/etcd/HAProxy/RabbitMQ/MinIO/observability skeletons, ordered owner jobs, mode-600 secret and CA copies, and a deterministic `render.json`; `qr-deploy check` validates the local files and placement and probes the one HTTPS ingress.
- `qr-deploy check --remote docker` reads the live Swarm through a read-only Docker CLI seam and reports the local state and odd manager count, node roles/availability/labels, encrypted attachable overlay, service replicas and pinned image, one 443 host-mode port with no app port, task errors, and a limited health row; `--overlay` adds the overlay facts and a limited DNS/route row. The default check stays Docker-free, `QR_DEPLOY_DOCKER` selects the executable, and no row prints an environment value, secret/config payload, token, or full inspect body. Fake-Docker fixtures cover a healthy swarm, a missing service, a wrong replica or image, a second published port, an unencrypted overlay, a missing label, a manager count below three, and a command error.
- The opt-in `ha` test tier (`tools/test_tiers.sh ha`, `make test-ha`) runs `deploy/v3/test.sh`: `--plan` is Docker-free, a real run deploys the v3 stack, drains one worker, stops one manager on a distinct host, and verifies quorum, gateway/ingress delivery, the one 443 port, the pinned digest, a `tools/verify.sh` run digest, and surviving stateful containers and bind mounts. It requires `--confirm` for the loss phase, never removes a stack/volume/secret/config/node/service, and prints `UNVERIFIED` instead of a pass when the preflight is not met. The test-tier contract is now five tiers; three VMs on one host are `Simulated HA`, and `Real HA` remains HA-014.
- The image entrypoint resolves an unspecified internal bind to the Swarm task address and fails closed when no private address resolves. The deployment contract, manual chapter 15, and the token-rotation exception are added.
- PostgreSQL, RabbitMQ and MinIO now use TLS in the Compose and v2 multi-host profiles. `QR_DATABASE_TLS_CA_PATH` requires `sslmode=verify-full`, `QR_QUEUE_TLS_CA_PATH` requires `amqps://`, and `QR_OBJECT_STORE_TLS_CA_PATH` requires `https://`; a missing CA or a handshake failure fails closed with no plaintext fallback. `tools/pki.sh` and `make pki` create the offline-root/intermediate/180-day-leaf bundle, `pki-init` stages the key for the stock non-root services, and `make compose-check`, CI and `tools/remote.sh` create a disposable PKI. Internal HTTP, mTLS and client certificates remain deferred to HA-012; the residual risk is recorded.
- Every application secret now arrives as a `*_FILE` value naming an absolute, mode-600 file: `QR_DATABASE_URL_FILE`, `QR_DATABASE_OWNER_URL_FILE`, `QR_QUEUE_URL_FILE`, `QR_BEARER_TOKEN_FILE`, `QR_OBJECT_STORE_ACCESS_KEY_FILE` and `QR_OBJECT_STORE_SECRET_KEY_FILE`. A plain secret variable, both sources together, a relative/missing/oversized file, invalid UTF-8, NUL, an empty value or surrounding whitespace fails startup without printing the value; a database- or queue-owning service without its file fails closed.
- `make secrets` creates the local `secrets/` directory (mode 700, files 600) with random infrastructure passwords and the per-service secret files; `make up` depends on it and reuses existing values, `FORCE=1` rotates deliberately. `make compose-check` validates both Compose files with disposable secrets. The image entrypoint stages the mounted files for UID 10001 and drops privileges; no service binary runs as root.
- `qr-deploy render` writes `env/secrets/` at mode 700 with one mode-600 file per secret and renders `_FILE` keys; the multi-host role files mount them at `/run/secrets`.
- The gateway re-reads `QR_BEARER_TOKEN_FILE` every `QR_TOKEN_RELOAD_SECONDS` (default 5) and on `SIGHUP`, swaps one immutable snapshot, keeps the last good token on an invalid file, and counts `quantrule_auth_reload_failures_total{reason}` with `quantrule_auth_token_age_seconds`. A file present but invalid at startup fails closed.
- The shell accepts `QUANTRULE_TOKEN_FILE` in addition to `QUANTRULE_TOKEN` and `--token`.
- The shell and browser retry only GET reads with a bounded, jitter-free budget: 3 attempts, a 200 ms base, factor four, a 5 s cap and a 15 s deadline in the shell, and 3 attempts, a 250 ms base, factor four and a 4 s cap in the browser, on a transport error, 408, 429, 502, 503 or 504. Mutations, uploads, invalid responses and 401/403/404 stay single-attempt. The shell prints `cannot reach the gateway after N attempts` without a URL, token, or body; the browser shows one reconnect banner. Retries are client-local and do not change server idempotency or run identity.
- SIGTERM starts a real drain: `quantrule_ready=0` and `quantrule_draining=1` immediately, `/ready` answers `503 draining` without probing, `/health` stays live, the HTTP server waits `QR_DRAIN_TIMEOUT_HTTP` (20 s) and a worker `QR_DRAIN_TIMEOUT_WORK` (25 s), and the deadline exits `0` with `drain.deadline_exceeded`. Compose keeps a `40s` stop grace period and the image declares `STOPSIGNAL SIGTERM`.
- Every service serves `/ready`; the gateway probes each upstream `/ready` instead of `/health`, and the backtest worker gains an internal `/health`, `/ready` and `/metrics` listener.
- An evidence drain stops the leader consumer, outbox, reconcile, retention and leader election, releases the lease for immediate standby takeover, and never marks an in-flight run failed; the worker leaves an aborted run `running` with its lease for expiry recovery.
- `quantrule_runtime::retry` adds the bounded deterministic policy (3 attempts, 200 ms base, factor 4, 5 s maximum, 15 s deadline) for audited idempotent GETs and the two dataset-reference writes only; claims, computes, signal creates, stage creates, decisions, cancels and streaming bodies stay single-attempt.
- A queue consumer error reconnects in-process with 1–30 s backoff before the container restarts, and a shutdown during the backoff is a clean stop.
- The internal registry adds `quantrule_http_retries_total`, `quantrule_consumer_reconnects_total`, `quantrule_queue_depth`, `quantrule_workers`, `quantrule_runs_total`, `quantrule_stages_total`, `quantrule_leader` and `quantrule_outbox_pending`.
- The runtime publishes a deterministic in-house Prometheus registry on an internal `QR_METRICS_BIND` listener for all seven services: build, process start, readiness, draining, dependency, HTTP counter, duration histogram, and in-flight families. Families and label sets are sorted; route labels come from the axum route template, never a raw path or identifier.
- The gateway, data, model, evidence, simulation, and strategy routers gain request middleware that validates or mints `x-request-id`, accepts a valid W3C `traceparent`, opens one request span, and logs one completion event with method, route, status and duration. The gateway forwards only the two identifiers and returns `x-request-id`.
- JSON logs are the server default through `QR_LOG_FORMAT=json` (with `text` as the local-debugging opt-in); `QR_LOG` sets the filter, the process identity is added to every record, and URL userinfo, `Bearer` values and `password=` values are redacted.
- Compose sets JSON logs and the internal metrics bind on the private network, publishes no metrics port, and adds no Prometheus service (PLAT-006 owns it).
- `docs/architecture/observability.md` records the metric, log, identifier, internal-listener and non-goal contract; the manual and a runtime-observability tutorial show the operator commands.
- The command shell and browser surface the research artifact layer additively: `pack create/list/show/validate/freeze/retire/fetch`, `signal create/list/show/fetch`, and `run lineage`; the 49 earlier command contracts keep their name, arguments, output fields, errors and renderer.
- The gateway exposes `POST /signals` through the optional strategy service (`503 service.unavailable` when absent), `GET /signals`, `GET /signals/{id}`, `GET /signals/{id}/bytes`, and `GET /runs/{id}/lineage`; signal reads go to data, lineage to evidence, and the gateway decodes neither.
- The data service serves public signal aliases with profile scope, a fixed signal kind, a content ETag, and hash verification; the evidence service serves the public lineage read with the existing run-visibility rule.
- The browser adds a Packs screen with Packs and Signals tabs (list, filter, create, lifecycle, verified download, signal metadata and create) and a lineage block on run detail; a missing lineage is an empty state.
- `SignalRequest` and `SignalResponse` move to `quantrule-api`; `quantrule-service-api` re-exports them with identical wire bytes so the shell and browser name the public contract without a service crate.
- The pre-change command contracts and parity requests are pinned in `tests/fixtures/command-surface/help-49.json` and `tests/fixtures/client-parity/view-49.json`.
- `qr-governance gate plan` and `tools/gate.sh --changed` classify a change as docs, governance, web, or full; engine/shared changes, `main`, and releases keep the full gate.
- Historical research programs carry `retired` and are refused by `research batch` and `research cell`.
- ADR-RA accepts the research artifact layer, model artifact input, strategy/portfolio/execution packs, signal artifacts and the seventh strategy service.
- The research artifact pack envelope and strategy-pack schema validate a closed JSON contract, hash canonical bytes, reject duplicate name/version identities, and enforce the draft, validated, frozen and retired lifecycle.
- The portfolio policy pack schema validates sizing, per-trade risk, exposure, maximum positions, drawdown action, a deterministic rebalance rule and ordered declarative constraints; it shares the pack identity and lifecycle and rejects account cash, currency and statistical fields.
- The execution-simulation pack schema validates the fill timeframe, market-next-open order and stop-first tie rule, fee and slippage, commission, funding, zero-volume and end-of-data policy; it shares the pack identity and lifecycle and rejects broker, venue, routing and statistical fields.
- `quantrule.run/v5` references the model artifact, strategy pack, signal artifact, portfolio policy pack and execution-simulation pack by id and hash; a supplied reference resolver verifies every content hash, rejects a missing, mismatched or wrong-kind object, and materializes the strategy, execution and portfolio configurations.
- Every v5 pack and signal hash enters the config hash and therefore the run identity; the `RunId` formula is unchanged, the v4 config hash is unchanged, and a v5 fold or signal run records its references in the backtest manifest and provenance.
- ADR-HA accepts the Swarm HA profile, one Patroni cluster with four logical databases, quorum queues, distributed MinIO and the `quantrule.deploy/v3` descriptor.
- The `v1.6.0` execution plan, issue tree, research-artifacts architecture and HA architecture are recorded as the plan of record.
- The local and multi-host stacks use four logical PostgreSQL databases (`qr_gateway`, `qr_catalog`, `qr_evidence`, `qr_simulation`), each with a per-service owner role that migrates and an application role with DML only.
- `deploy/postgres/bootstrap.sh` and the one-shot `db-bootstrap` service create the four databases and eight roles idempotently; four one-shot `<service>-migrate` jobs run the service binary with `--migrate-only` and hold the owner URL.
- The schema-migration contract is recorded in `docs/architecture/migrations.md`: forward-only ledgers, one owner per database, expand/contract release order, checksum pinning, rollback compatibility, and the four-job order.
- `deploy/postgres/migration-ledger.json` pins every ledger version and SQL checksum; a fast test refuses a reordered ledger, an edited migration body, or a database-owning service without its `--migrate-only` entry point.
- The operator runbook covers a failed migration, a stopped migration job, and a rollback to the previous image digest while the old stack stays available.
- RabbitMQ run, evidence, and stage queues are durable quorum queues with a three-redelivery limit, a dead-letter exchange, a quorum dead-letter queue, and generated names (`QR_QUEUE_GENERATION`, default 1); a failed handler rejects so delivery-limit exhaustion is observable in the DLQ.
- The operator runbook, manual, and deployment docs document queue-generation retirement and the separate definitions backup/restore, which records topology but not messages.
- The evidence database adds `runs.outbox` (migration 18): a run row and its `run.requested` or `run.computed` notification commit in one transaction, and a publisher reconciles and drains the intents so a broker outage cannot lose a queued run.
- The evidence database adds `runs.dataset_reference_releases` (migration 18): an erase records the dataset-reference release in the same transaction as the consumer delete, and a reconciler retries the data-service call until it is confirmed.
- `docs/architecture/cross-database-consistency.md` records the outbox, reference, reconciliation, and tombstone contract, and the failure matrix for a crash after commit, a duplicate publish, a restart, and an erase.
- The evidence run row carries a per-run lease (`lease_holder`, `lease_term`, `lease_expires_at`, migration 19); a claim records a monotonic term, the worker renews the lease every 20 s from a control-plane task, and `computed`, `complete`, and `fail` carry the term so evidence refuses a stale worker.
- The data service owns a persistent research pack registry in the `catalog` schema (migration 7): a profile-scoped metadata and identity row, append-only validation records and lifecycle events, and an object key, hash and size. The object store is the single byte authority under `packs/<hash>`; registration writes the object first and commits the row second.
- The pack registry serves additive `/catalog/packs` routes for create, list, show, validate, freeze, retire and content-addressed byte fetch, plus service-to-service show and byte routes; registration is idempotent for one content hash and refuses a `(name, version)` label that names different bytes. Show, fetch, validate, freeze and retire re-read the object and verify its hash, and a missing object fails closed.
- The pack lifecycle is explicit and guarded: `draft` to `validated`, `validated` to `frozen`, and `frozen` to `retired`; freeze and retire are idempotent, a frozen pack cannot be validated again, a retired pack is never deleted, and each transition commits with its lifecycle event.
- The evidence database records a run's model, strategy, signal, portfolio and execution references with pack provenance (migration 20) through the internal lineage route; a repeated set is idempotent, a different set is refused, and the route is the run-v5 integration seam.
- The data service owns the generic computed-artifact retention seam (migration 8): content-addressed artifact rows, consumer references that protect them, and an internal retention pass that collects only unreferenced signal artifacts older than 30 days, writes a tombstone in the same transaction, and deletes only an unshared object.
- The signal artifact (`quantrule.signal-artifact/v1`) is the content-addressed output of a frozen model artifact and a signal-mode strategy pack: one row per execution bar with bar identity, availability, action, side, intent, reason and event/state evidence. A deterministic reference position handles event/state entries, the time stop, the ATR stop/target and a two-sided bracket whose side execution chooses; fold mode and giveback are refused, and execution and portfolio policy are not inputs.
- The seventh `strategy` service (`qr-strategy`) resolves a frozen model artifact and signal-mode strategy pack, refuses a fold pack before the model call, computes the signal with `quantrule_app::signal`, writes the canonical bytes directly under `artifacts/<sha256 hex>`, and registers the catalog row. It has no database, queue or config root, and makes one pack plus one model resolution per request.
- The data service accepts metadata-only artifact registration at `POST /internal/v1/artifacts/{kind}/{id}`: it re-reads the stored object, verifies its SHA-256 and byte count, and commits an idempotent `catalog.artifacts` row. A missing or mismatched object publishes no row; the existing 64 MiB body PUT route is unchanged.
- The internal contracts add `quantrule.signal-request/v1`, the signal response, metadata-only artifact registration, `ModelBundleManifest::artifact_id`, the `PackClient` and `ArtifactClient` runtime seams, and the `signal.invalid` error code. `ModelBundle::from_files` assembles a bundle from verified object bytes without a temporary directory.
- The single-host Compose stack starts seven services and ten long-running containers. `Service::Strategy` and the optional `QR_STRATEGY_URL` let the gateway probe the strategy service only when it is configured; the v2 multi-host profile remains six services and the v3 Swarm profile places all seven.
- `deploy/postgres/migration-ledger.json` pins catalog migrations 7, 8 and 9 and evidence migrations 20 and 21; the composed Postgres tier runs the pack, artifact, lineage, leader and outbox conformance suites.
- The simulation database adds migration 16: a decision-completeness constraint and a trigger that refuse a changed or cleared decision. The repository locks the run row for mark, check, decision, and erase transactions, so two simulation replicas cannot corrupt the mark set or overwrite a decision.
- The simulation service supports two replicas only through the HA-008 row-lock concurrency contract; the v2 multi-host profile remains single-instance.
- The evidence database adds `runs.leader_lease` (migration 21): one row carries the finalizer holder, a monotonic term and a database-time expiry. Acquisition swaps an expired row for a new term, renewal keeps the term, and a standby takes over after expiry.
- Only the evidence leader consumes `run.computed`; both replicas serve reads. A leader loss drops the consumer, the standby acquires a higher term, and the outbox re-arms the unfinished run.
- `complete` and `fail_finalization` lock `runs.leader_lease` and require the live leader term as well as the run-attempt term, so a stale leader cannot write a terminal state, a duplicate delivery is a lookup, and one run record and one canonical digest remain.
- The data service supports two replicas through the HA-007 active/active contract: content-addressed upload uses a database-backed object guard and `put_if_absent`, the `(profile_id, data_version)` row is inserted with `ON CONFLICT DO NOTHING`, and a retry returns the stored row and the same `data_version`.
- Pack and artifact registration return whether the row was created, so two replicas answer one duplicate with `201` and the other with `200` instead of racing a separate existence check.
- Catalog migration 9 indexes `datasets.object_key` and `artifacts.object_key` for the erase guard and artifact-sharing checks; the migration is pinned in `deploy/postgres/migration-ledger.json`.
- `docs/architecture/data-active-active.md` records the files the data service owns, the upload/erase interleavings, and the v2 placement boundary.

### Changed

- The `ha` lab reports per-phase results and a summary; it no longer prints an end-to-end `PASS` while PLAT-005 MinIO is unconfigured. The RabbitMQ phase is scoped `Simulated HA`; `--skip-queue` leaves it `Unverified`.
- The four stateful secrets are now five; `rabbitmq_erlang_cookie` joins the Patroni passwords and remains excluded from app Swarm secrets.
- `backtest` and `evidence` render `restart_policy: any` without a maximum attempt because they consume the queue; the other services keep the bounded `on-failure` policy. The readiness gap during a reconnect is deliberate and documented.
- `tools/stateful.sh` copies by manifest `hosts`, creates `/srv/quantrule/rabbitmq/{data,definitions,conf.d}` only on brokers, and includes RabbitMQ in `start`, `wait`, and `status`.
- The v3 sample now uses a private-CA `tls: cert` ingress with a VIP and a keepalived edge unit; the bootstrap no longer only checks for `quantrule_traefik`, and the edge infra files include the keepalived unit. The ingress contract is `docs/config/2026-11-23/ingress.md`.
- Runtime logs default to one JSON object per line; `QR_LOG_FORMAT=text` is the explicit local-debugging switch, and the driver crates stay at `warn` unless `QR_LOG` names them. Request and trace ids are observability data only and never enter an artifact or the canonical digest.
- The command surface grows to 61 commands; `help`, the README and manual command tables, the Help screen badge, and the command-surface registry list the new commands while every earlier row stays byte-identical.
- `docs/architecture/http-api.md`, `research-artifacts.md`, the pack and signal contracts, the client and architecture indexes, the manual and the multi-host guide describe the public signal routes, the Packs screen, run lineage, and the v2 signal-create gap.
- A simulation decision is terminal: the first decision wins and every later decision, including an identical retry, returns `simulation.locked`. A duplicate mark identity remains an idempotent upsert that keeps its stored id and creation time.
- Queue names now end in `.g<generation>`; the pre-generation classic queues are drained or retired by the operator instead of being redeclared with new arguments.
- A failed handler uses `basic.reject` rather than `basic.nack`, because RabbitMQ 4.3 and later count only a rejected delivery toward the quorum delivery limit.
- The multi-host renderer writes `QR_QUEUE_GENERATION=1`, and the composed queue check verifies the quorum type, delivery limit, dead-letter bindings, and generated names.
- `run submit` is accepted when the run row and its outbox intent commit; a broker that is down delays the notification instead of returning `503 queue.unavailable`, and a repeated submission republishes the queued run while returning the same run record.
- A run claim is lease-based: a queued run is claimable, a running run only when its lease is absent, expired, or continued by the same holder, and a live lease is never stolen. Cancellation increments the term and clears the lease; a terminal run releases it.
- Evidence finalisation is leader-only: a standby does not consume `run.computed`, and a finalisation that loses the lease is refused instead of acknowledging the delivery.
- A run erase releases its dataset reference from the committed tombstone rather than a best-effort call, so a crash between the databases leaves a retry instead of a reference that blocks the dataset forever.
- Each database-owning service receives only its own `QR_DATABASE_URL_FILE` and `QR_MIGRATE=0`; model and backtest receive no database URL. The long-running process never receives `QR_DATABASE_OWNER_URL_FILE`, so a compromised service cannot reconnect with DDL rights.
- The S3 adapter builds its client from the two file credentials and no longer calls `aws_config::defaults()`; `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` and the shared AWS config are not a second secret reader. `QR_OBJECT_STORE_URL` rejects userinfo.
- `tools/remote.sh` writes the gateway token file instead of exporting it, and `tools/token.sh --out` writes a mode-600 token. The shell reads its own `QUANTRULE_TOKEN_FILE`, never the gateway's mounted file.
- An existing local volume created with the documented `quantrule` store passwords keeps them. Export `QR_POSTGRES_PASSWORD`, `QR_RABBITMQ_PASSWORD` and `QR_OBJECT_STORE_PASSWORD` with the old values before the first `make secrets`, or reset the disposable stack with `make reset`.
- A rollback starts the previous image without its migration job: the runner refuses a database ahead of the binary, and the previous image must tolerate the expanded schema.
- No role is granted connect on another service's database.
- The multi-host `qr-deploy render` derives the four application URLs, the four migration owner URLs, and the eight role names from the v2 Postgres endpoint, so v2 remains readable.

- The pull-request CI gate is scope-aware: docs, governance, and web scopes skip Rust or web work they cannot observe, and a `gate` aggregator fails when a needed job failed or was wrongly skipped.
- The policy is now “No Python” repository-wide; current docs, the manual, the registry, and the verification evidence name `qr-governance` checkers instead of the deleted Python scripts.

### Security

- A plain secret environment variable is refused in the server profile, so `docker inspect` and `/proc/<pid>/environ` never carry a DSN, queue URL, token or S3 key. An invalid token file cannot silently disable gateway authentication.
- The private CA and server keys are Git-ignored and a `private-keys` governance checker refuses a committed key file or PEM private-key block. PostgreSQL and RabbitMQ do not request client certificates; a compromised overlay host can still impersonate a service over plaintext internal HTTP, as recorded in the TLS contract.

### Fixed

- A RabbitMQ consumer no longer retries a dead `Arc<Connection>`: the reconnect loop rebuilds the connection from the stored URL and CA on the next attempt, so one broker loss no longer ends the consumer or exhausts the container restart budget.
- The stale "queues rebuild from run rows" wording is corrected: an acknowledged message survives one broker loss in a quorum queue; rebuilding from rows is the lost-queue/DR path.
- Two concurrent identical dataset uploads now produce one catalog row and one content-addressed object; the loser of the unique-key race returns the winner's row instead of failing. A failed object write still publishes no row.
- Dataset reference register, release, and erase now lock the dataset row in Postgres, so two replicas cannot lose a reference, double-count one, or erase a dataset whose reference committed first.
- A concurrent erase and upload of the same content are serialised by the database object guard and a post-commit reference recheck, so a published row cannot point at deleted bytes.
- Concurrent simulation marks and the terminal decision no longer race: a mark that loses to a decision is refused instead of inserted, an erase takes the same run lock as a decision, and a failed mark transaction leaves no mark and no status change.
- The run outbox now re-arms a `running` run without a digest as `run.requested`; a startup reconciliation names every unfinished run even after its worker or delivery was lost, while a terminal run still deletes its intents and a retry cannot create a second run record.
- A running run whose worker died is reclaimed after its lease expires and a stale term is refused, so a rebuilt queue recovers the run without a partial artifact set or a duplicate run record.

## [1.5.2] - 2026-11-21

### Fixed

- The in-process fixture server makes each accepted socket blocking before the first read. On Windows an accepted socket could inherit the listener's non-blocking flag, the first read returned no data, and the client aborted with `os error 10053`. v1.5.0 and v1.5.1 are withdrawn because that gate failed; v1.5.2 carries the same feature scope.

## [1.5.1] - 2026-11-21

### Fixed

- The release CLI gate keeps its in-process fixture server alive after a peer vanishes and closes each response stream cleanly, so the Windows job no longer aborts with `os error 10053`. v1.5.0 is withdrawn because that gate failed; v1.5.1 carries the same feature scope.

## [1.5.0] - 2026-11-21

### Added

- `qr-governance` is the Rust repository tool for the checker gate, verification matrices, labels, worktrees, improvement records, and research programs.
- `main` publishes a `release-binaries-<sha>` artifact with a SHA-256 manifest; the release assembles the thin image from that exact commit and downloads the same artifact at tag time.

### Removed

- Retire the Tauri desktop shell, supervisor, AppImage, NSIS, and every installer.
- Retire the embedded desktop profile: SQLite, `rusqlite`, the filesystem CAS, and the in-process queue.
- Remove the desktop release job, artifacts, and publish dependency; releases publish the image and note only.
- Remove `apps/desktop-shell`, `tools/check_schema_prefixes.py`, and `tools/parity_desktop.sh`.
- Retire the historical Python research stack (`tools/research/`): the M2.x–M4.x runners, coverage checker, grid builders, edge diagnostics, and tests. Current research commands are `qr-governance research ...`; the committed configs and Git history are the record.
- Retire the historical Python improvement stack (`tools/improvement/`): the shared `common.py`, confidence, claim, count, matrix, and SR2 audit scripts, and their tests. Current improvement commands are `qr-governance improvement ...`; the run records and Git history are the record.

### Changed

- The browser is the only GUI; the headless `quantrule` CLI remains the agent client.
- Windows and Linux are the supported browser and CLI platforms; macOS is unsupported in the next release.
- The gate has 26 checkers after the desktop-only checker is removed.
- Remove every Python checker and checker test; `qr-governance check --all` is the gate and CI authority.
- Retire the Python data tooling (`tools/data/`); data-service ingestion and the committed sources are the record.
- CI no longer compiles or runs the historical research or improvement Python tests.
- The gate, CI, release, composed tier, and multi-host harness no longer invoke Python.

## [1.4.2] - 2026-11-20

### Fixed

- The packaged desktop parity smoke compares against the current canonical digest `sha256:a0372f02…`, so the packaged Linux and Windows profiles match the server profile.
- The v1.4.1 release is withdrawn because its desktop package smoke failed on the stale digest; v1.4.2 supersedes it.

## [1.4.1] - 2026-11-20

### Fixed

- The simulation Postgres list queries return `archived_at` as RFC3339 text and exclude archived runs from the active list, matching the shared row type and the SQLite store.
- The desktop shell `SHELL_VERSION` moves to 1.4.1, and the version-boundary test no longer hardcodes a version literal.
- The release recovery records the v1.4.0 workflow failure and supersedes it with v1.4.1.

## [1.4.0] - 2026-11-20

### Added

- The user manual adds chapter 13 (desktop install) and chapter 14
  (multi-host), plus an agent operator runbook with copyable prompts and
  guardrails.
- The Linux desktop install documents a user-level `.desktop` entry so
  QuantRule appears in GNOME Activities search; the Windows NSIS path is
  documented.
- Multi-host backup and restore wrappers reuse the single-host archive format
  against the infrastructure host, with an explicit application-stopped gate.
- The `multi-host` test tier runs the six services on separate container
  addresses, proves one image digest and gateway-only publication, and compares
  the canonical digest with one and two backtest workers.
- The multi-host validator now rejects public or unspecified hosts, external
  database/queue/object-store endpoints, template secrets, placeholder image
  digests, IPv6, port zero, and placements the role Compose files cannot run.
- The multi-host Compose files require `QR_BIND_IP`, set `pull_policy: never`,
  and require a healthy state in every healthcheck.
- `qr-deploy check` no longer sends the bearer token on open probes and prints
  the backtest worker as `SKIP` instead of a false success.
- The runtime redacts connection strings and the token from `Config`'s debug
  output and rechecks the internal bind at the serve boundary.
- The multi-host deployment profile adds `qr-deploy` and a
  `quantrule.deploy/v2` descriptor that validates placement, renders per-role
  env files, and checks the deployed image digest.
- HTTP services can refuse a public bind with `QR_INTERNAL_ONLY`; the
  multi-host profile sets it on the internal services.
- The browser History screen lists run, stage, research, simulation, and
  dataset records, archives them, and erases them with typed confirmation.
- The shell gains `history list`, `history archive`, and `history erase` for
  run, stage, research, simulation, and dataset records, with
  `--include-archived`, `--dry-run`, and typed confirmation.
- Evidence runs and research records can be archived, listed with
  `include_archived`, and erased after typed confirmation; succeeded runs and
  decided records refuse erase, run blobs are removed only when unreferenced,
  and current rows are reconciled into the dataset registry at startup.
- Simulation runs can be archived, listed with `include_archived`, and erased
  after typed confirmation; checked and decided runs refuse erase.
- Dataset runtime history: archive, a cross-service reference registry, and an
  erase that refuses a referenced dataset and deletes shared bytes only when the
  last catalog row is gone.
- `qr-governance history` lists, archives, and erases historical repository
  evidence with a verified manifest; it is developer tooling and is not in the
  product image.
- The model bundle publishes `simulation-<timeframe>.json` artifacts and stores
  each under `simulation/bundles/<sha256>.json`; the shell `simulation create`
  gains `--inputs`.
- The gateway's `/ready` endpoint aggregates the configured upstream services
  and returns `503` when one is down or answers under another service identity.
- RabbitMQ publishes wait for the broker's confirmation; a nack, a missing
  confirm mode, or a confirmation timeout is a failed publish.
- The composed tier runs the ignored Postgres conformance suites against a
  disposable database.

### Changed

- The shell command pipeline is split by command family into `command/parse/`,
  `command/dispatch/`, and `output/`, so a command change stays local; the
  global documentation index is now a navigation map, and status lives in the
  owning indexes.
- The research surface and improvement run have one glossary definition; the
  command and workflow lists live only in their indexes.
- The gateway's `quantrule-service-api` dependency is test-only.
- The legacy checker scans tracked and untracked non-ignored files, reducing its
  local runtime from about 61 seconds to about 4 seconds.

### Fixed

- Stage recording and stage row/blob deletion share one gate, so a concurrent
  delete waits for the record and keeps the new stage's blob.
- Determinism evidence now covers a reordered ladder and two workers on one
  run, and the checker gate has a registry meta-test plus a file-size unit test.
- The browser clears an open config detail on a profile switch, stage forms
  start empty instead of the fixture `cfg_abc`, and the packaged shell passes
  `--profile desktop` rather than inheriting the environment.
- `help` works with a malformed token, `simulation check` prints a table and
  verdict, and the parity suite exercises every command.
- The simulation tutorial uses a 1m bundle with enough bars for the default
  window and marks a bar inside the candidate zone, so the documented path
  creates windows and a mark.
- A conditional candle or ladder request with a matching `If-None-Match`
  returns `304` instead of the full CSV.
- The gateway gives uploads, artifact downloads, and model compute a 15-minute
  deadline and other proxied calls 30 seconds; a timeout is a retryable `503`.
- A non-numeric simulation chart returns the shared error envelope.
- The HTTP contract documents `created` and the `stage.*` error codes.
- The model bundle key includes the serving image identity, so a rebuilt image
  cannot reuse a bundle whose presentation bytes changed.
- Postgres and RabbitMQ are pinned by digest, and `make remote` refuses the
  local store defaults.
- The browser reports a clear secure-context error when Web Crypto or the
  clipboard is unavailable, instead of failing silently on a plain HTTP origin.
- The compose stack pins its DNS search to the stack network, so a stopped
  service name cannot resolve to a LAN host.
- A late detail response cannot replace a newer run, stage, or research
  selection in the browser.

- A run config reference is confined to the config directory: an absolute path
  or a `..` component in `run.json` is refused before the loader reads it.
- `tools/verify.sh` includes `metrics.json` in its canonical set, so the
  verifier recomputes the same digest as the service and the composed tier
  passes on a clean stack.

### Changed

- The release workflow runs the composed tier on the locally built image before
  pushing and requires a browser/tooling quality job.
- The old audit findings carry dated triage statuses; the release note names
  the audit status and `check_release_audit`/`check_audit_backlog` enforce it.
- The release workflow runs the desktop parity harness before it builds the
  desktop supervisor.
- Server artifact event, source, and stage words now come from the same registry
  values as the browser table; the label gate compares both.
- The backtest worker opens a channel per consumer with its own prefetch slot
  and starts `QR_BACKTEST_CONCURRENCY` run consumers.
- Stage retention sweeps at startup and hourly, so an expired stage no longer
  waits for an evidence restart.
- The documentation-length checker now covers `docs/README.md`.

### Fixed

- The configuration identity is one SHA-256 across the app, catalog, run
  record, and stage provenance. The old 64-bit FNV fingerprint is gone.
- The canonical digest now covers every `metrics.json`, so a metrics-only change
  cannot leave the digest unchanged.
- A model bundle names its model-input identity (data version label, model
  config, and ladder) in provenance. A strategy-only change reuses the frozen
  model artifact; the worker refuses a changed model input.
- `portfolio_holdout` excludes `COPPERCMDUSD` and `TSLAUSUSD` until a 2025
  source is admitted; the committed coverage for both ends on 2024-12-31.
- Restore now counts, drops, and round-trips all six schemas, including
  `gateway` and `stage`; a `--force` restore can no longer partially destroy a
  started stack.
- The evidence service refuses a run or stage submission when the server has no
  `QR_IMAGE_DIGEST`, instead of recording a synthetic image identity; `make up`
  records the built image id.
- Research creation rejects a run config whose data window leaves its declared
  partition, closing the `strategy_dev`/2024 and holdout leakage path.
- The Postgres run cancel is conditional on `queued`/`running`, so a concurrent
  complete cannot be overwritten by a later cancel.
- The portfolio ledger caps a fixed notional at current equity, refuses an entry
  that would make cash negative or a halted account, and marks funding at the
  charge price. The golden short fixture is unchanged; the long-entry tests use
  a funded account.
- Session VWAP no longer substitutes zero for a missing candle volume; it is
  unavailable from the first missing-volume bar for the rest of the session and
  resets at the next session. The normal-volume golden fixture is unchanged.
- A divergence RSI window is clamped to the pivot's availability bar, so it
  cannot read past the bar on which the event claims to be knowable. The model
  v4 config now refuses a window above the pivot-scale lookback. The default
  and golden fixtures are unchanged.
- A one-bar Donchian or Keltner window no longer indexes the previous bar at
  bar zero; the position events require a prior bar. The normal-window golden
  fixtures are unchanged.
- VWAP slope now subtracts the VWAP from one full window back
  (`S[t] = VWAP[t] - VWAP[t - window]`), matching the registered formula. The
  first value appears at bar `2*window - 1`, and the extended readings and
  event goldens were regenerated deliberately.
- `structure.scale` now selects the confirmed pivot scale for the structure
  readings and the lower-high/lower-low sequence events. The default `major`
  scale keeps the existing output; a `minor` scale now changes the structure
  output as the registry states.
- VWAP bands and slope publish when the ATR family is inactive, and range
  expansion no longer requires ATR; only the ATR distance stays null without
  ATR.
- The `session` indicator registers `session_vwap` as a published reading, and
  the session row contract covers it against the golden fixture.
- Event edge evidence now names an event-constructing suite that covers the
  empty, single, short, flat, zero-volume, missing-volume, equal-tie, and gap
  inputs. The checker rejects a readings-only test as event-edge evidence.
- The breakout-retest-hold event has its own golden fixture, and the extended
  indicators' edge evidence names the family tests that check their readings.
- The strategy event snapshot excludes unregistered and `diagnostic` events,
  and no longer exposes the raw model `ContextBar`. The model-bundle reader
  applies the same registry filter.
- The run loader rejects a strategy execution timeframe that differs from the
  fill timeframe, and the coupled `run submit` rejects `quantrule.run/v3`.
- The execution contract states the conservative entry-bar bracket convention:
  OHLC cannot order a pre-trigger low or high against the entry, so the bracket
  is live for the whole entry bar. A fixture with a pre-trigger low now proves
  the rule; no fill behavior changed.
- The model event batch and bounded history now include the extended events as
  well as the core batch, matching the retention contract. The retention test
  asserts both sources are present.
- The data-v3 document no longer requires the retired policy universe; the
  loader's non-empty instrument and ordered-ladder checks are the actual
  admission rules.
- Current documentation now matches the tree: the `AGENTS.md` crate and workflow
  map, the workflow and stage status, the HTTP stage routes, the config-upload
  cap, the service digest ownership, the improvement index table, and the
  `docs/README.md` length cap.

### Changed

- The documentation-length checker now covers `docs/README.md`.
- An execution plan must resolve to a same-name AI transparency log or carry a
  recorded `- AI log:` link; the execution-plan checker enforces it.

## [1.3.0] - 2026-11-17

The stage execution surface release. QuantRule now runs the model, strategy, and
portfolio engines alone for diagnosis, promotes a frozen model artifact into a
coupled run, records governed improvement evidence in the v3 research ledger,
and verifies the deployment from the Operations screen. The coupled backtest
remains the only evidence run; diagnostic stages never become a baseline. The
compiled product version is the only intended model-object delta.

### Added

- The stage resource contract and schema: `quantrule.api/v1` stage kinds,
  states, input references, artifact bundles, and provenance;
  `quantrule.stage-manifest/v1` with a computed bundle digest and
  `evidence: false`; and the evidence service's profile-scoped `stage` schema
  with Postgres and SQLite migrations and a content-addressed stage store.
- The diagnostic stage control plane: `stage.*` queue routes with id-only
  payloads, evidence submit/list/detail/artifacts/cancel/delete routes, the
  backtest worker's executor seam and cancellation boundaries, `QR_STAGE_RETENTION_DAYS`
  cleanup that never touches a run, and the gateway `/stages/*` proxy.
- The model stage vertical slice: a real `StageExecutor` that reuses the
  content-addressed model bundle byte-for-byte, verified stage artifact
  download, the `stage run model`, `stage list`, `stage show`, `stage events`,
  `stage fetch`, `stage cancel`, and `stage delete` shell commands, and the
  browser Workbench Data and Model tabs with parity coverage.
- The strategy stage vertical slice: a profile-scoped artifact lookup that
  derives the dataset from a succeeded model stage, a strategy executor that
  writes only `decisions.ndjson`, `summary.txt`, and `provenance.json`, the
  `stage run strategy` and `stage explain` shell commands, and the browser
  Workbench Strategy tab with parity and divergence coverage.
- The portfolio stage vertical slice: a profile-scoped decisions-artifact lookup,
  an executor that replays execution and portfolio into prefixed `execution/`
  and `portfolio/` bundles, the `stage run portfolio` shell command, and the
  browser Workbench Portfolio tab with parity and invariant coverage.
- Backtest promotion: `run submit --model-artifact` reuses a succeeded model
  stage, records the artifact id on the run, recomputes strategy, execution, and
  portfolio, and keeps the full-run canonical digest unchanged.
- The stage execution and evidence-boundary ADR: diagnostic stage runs are
  server-side and content-addressed, while the coupled backtest remains the only
  evidence path and canonical-digest producer.
- The Engine Execution workflow: four stage runbooks, the diagnostic/evidence
  rule, model artifact reuse, and the strategy and portfolio recompute rule.
- The Backtest workflow: the coupled evidence path from freeze and preflight
  through fetch, verify, validate, record, and reproduce.
- The End-to-End Verification workflow: the clean-stack procedure from health
  and upload through a verified canonical digest, owned by Release & Operations.
- The `docs/manual` user-manual kind and its 12-chapter skeleton; the status
  index now names the accepted model baselines and no strategy or portfolio
  baseline.
- The research-ledger schema and manifest v3: `quantrule.improvement-run/v3`
  gateway-reference pre-registrations in the evidence service, a one-time
  pipeline-run link, a one-time canonical-digest record when the run produces it,
  a one-time human decision with a note, append-only baseline and invalidation
  registries with append methods, Postgres `0016` and SQLite `0004` migrations,
  and the committed model baselines and tombstones seeded for the `default`
  profile.
- The research surface: `research create/submit/list/show/compare/decide`,
  `baseline list/show`, and `invalidation list`; evidence `/research` routes with
  the stable `research.*` error family; the gateway `/research` proxy; the
  browser Research screen with create, submit, compare, decision, baseline, and
  invalidation controls; and parity, command-surface, and client-doc coverage.
  A model-tier research run is a coupled full run, the comparison reads only
  published numbers, and an accepted decision requires a succeeded run and
  digest.
- The operations surface: `GET /operations` reports `QR_STAGE_RETENTION_DAYS`,
  whether a stage store exists, the last startup cleanup's UTC instant, removed
  stage and object counts, and an optional cleanup error; the evidence service
  keeps the shared cleanup status; the gateway proxies `/operations`; the
  browser Operations screen shows health, version, image digest, active profile,
  token presence, retention, and a run-digest verification form. The
  End-to-End Verification tutorial records `make up`, `make verify`, the expected
  two-run digest, and the composed tier.
- The user manual: twelve chapters covering the first run, data, model,
  strategy, portfolio, backtest, improvement, operations, all 46 commands, every
  browser screen and Workbench tab, the glossary, and the stable error families.
  `tools/check_manual.py` keeps the chapters, command rows, screen rows, section
  shape, and evidence labels aligned with the registry. The stage-to-backtest
  tutorial records the diagnostic-to-promoted-run journey.

### Changed

- The Improvement Run Contract now treats `quantrule.improvement-run/v3` as
  current and v2 as historical; the v3 pre-registration lives in the evidence
  service and the committed `docs/improvement/` registry remains human-owned.
- The command surface grows from 37 to 46 commands; the shell, browser, gateway,
  and evidence service expose the same research use cases.

### Fixed

- The shell's `simulation marks` usage error no longer advertises `--origin all`,
  which the parser rejects; the hint now matches `help` and the accepted values.

## [1.2.0] - 2026-11-16

The version alignment release. The product version, the current tag, the
release records, the documentation, and the course deliverables now name
`1.2.0` / `v1.2.0`. The canonical pipeline output is unchanged; the only
model-object delta is the compiled product version.

### Changed

- The workspace and its 16 lockfile entries, the web package, the desktop shell,
  the supervisor, the Tauri config, `SHELL_VERSION`, and the desktop version
  test move to `1.2.0`.
- The release index, current-tag map, README, status docs, architecture status
  index, deployment and user-journey examples, and course sources name
  `v1.2.0`; the course PDFs are rebuilt.

### Fixed

- The model-object regression oracle is regenerated for `1.2.0`. The object is
  66,022 bytes, one byte differs, and a semantic diff reports only
  `.model_version`; no reading, event, report, or canonical artifact value
  changed.

## [1.1.0] - 2026-11-16

The 1.1.0 alignment release. The existing 1.1.0 tree is published under the
corrected tag sequence, and the release history is normalized so the note
filenames, changelog headings, and current tags agree. The server platform and
the desktop package are unchanged; the browser client gains the remaining
inspection and verification surface, the composed test tier and the release
checks land, and the missing-volume and artifact-verification fixes are
included. No model, strategy, execution, or portfolio behavior changed.

### Added

- The browser client reaches the full shell surface: it inspects candles,
  timeframe ladders, and event batches, verifies a run's recorded digest, and
  downloads an artifact only after its SHA-256 matches the run record. The
  shared parity fixture enumerates all 27 registered commands.
- The composed test tier runs a real suite through `tools/composed.sh` and is
  called by `tools/test_tiers.sh composed` and the release workflow.
- `tools/check_client_docs.py` keeps the client screen and store tables, the
  navigation labels, and the 49/150 indicator/event counts aligned with the
  tree and the registry.
- The release workflow validates the current-tag map before it resolves a
  release note, and the desktop supervisor exports a package digest computed
  over the installed sidecars.
- The browser reaches `dataset show` and `config show` through record detail
  views, and a simulation chart exports its rendered SVG. The capability table
  and parity test now verify a control test id, not only a command name.
- The checker gate parses documented shell invocations, scans Current config
  contracts for retired commands, rejects a Current workflow link to a
  Historical workflow, and rejects a living-doc tag outside the current-tag map.

### Changed

- The simulation create request and list summary are the `quantrule-api`
  contract types; the service re-exports them, and the shell's older body keeps
  its wire shape because an empty `inputs` field is omitted.
- A profile switch and a disconnection clear every profile-scoped store, and
  the browser no longer presents a previous profile's runs, simulations, or
  inspection result while a new read is pending.
- The release image is built and checked locally before any push, and the
  release concurrency key uses the tag name for both a tag push and a manual
  dispatch.
- Worktree cleanup verifies the archive manifest, deletes the archived local
  data, and removes the worktree without `--force`.
- The desktop Continue action opens the gateway that answered the probe, not
  the typed candidate, while the typed URL remains the persisted setting.
- Release tags were renumbered on 2026-11-15 to a continuous `v0.1.0`–`v1.0.0`
  history. Commits, images, and published artifacts are unchanged; historical
  release notes keep their original version headings.
- The Current runbooks and config contracts now name the 27-command gateway
  surface and the run v4 path; the retired policy layer is removed from the
  Current workflows, and the living release/status docs use the current tags.
- The storage, HTTP, control-plane, deployment, operations, and characteristics
  docs match `datasets/`, the evidence-side submission path, the pinned MinIO
  image, the nine-container stack, and the six services.

### Fixed

- A price-only or partially missing-volume candle series publishes `null` for
  mean volume, VWAP, volume profile, flow, MFI, CMF, and participation; OBV and
  AD stay null after the first missing volume. A valid zero volume is unchanged.
- The runtime clock returns a typed error instead of panicking, filesystem CAS
  uses a no-replace hard link with a locked fallback, and migration planning
  refuses an unordered version set.
- `verify` resolves a run's recorded digest, and an artifact whose bytes do not
  match the run record is refused rather than saved.
- The current runbooks use the positional shell grammar; the retired local-stage
  command names no longer appear as Current contract instructions. The model
  docs point to M1.3 and the 100-parameter registry, and the `model_version`
  rustdoc distinguishes the run-config identity, registry version, and package
  version.

### Removed

- The retired local SQLite research memory and its `rusqlite` dependency were
  removed from `quantrule-app`; research memory lives in the evidence service.

## [1.0.0] - 2026-11-15

The desktop release. The same six services, HTTP contracts, and deterministic
output now run from one installer with no Docker: SQLite replaces Postgres, a
filesystem content-addressed store replaces S3, and a durable in-process queue
replaces RabbitMQ, while a supervisor starts and stops the six sidecars. The
Linux AppImage and Windows NSIS package carry the shell, the supervisor, and the
six services, and the packaged profile reproduces the v1.0.0 server canonical
digest `sha256:453c90b4…`. The v1.0.0 distributed platform remains the baseline.

### Added

- The evidence service can start on the embedded SQLite profile.
  `QR_PROFILE=desktop` with `QR_DATA_DIR` opens `quantrule.sqlite`, applies the
  runs/research SQLite baseline under the shared `runs` ledger, serves the
  existing `RunStore` and `HypothesisStore` traits through SQLite
  implementations, and reports `sqlite` at `/health`. Run claims, decisions, and
  finalization use `BEGIN IMMEDIATE` and re-read the row inside the transaction,
  preserving the queued/running/succeeded/failed/cancelled state machine, the
  retry budget, the artifact set, and the canonical digest; the sealed-hypothesis
  trigger refuses update and delete. Postgres remains the default.
- The gateway can start on the embedded SQLite profile. `QR_PROFILE=desktop`
  with `QR_DATA_DIR` opens `quantrule.sqlite`, applies the gateway's SQLite
  baseline to one shared `schema_migrations` ledger with a `ledger` column,
  serves `/profiles` through the existing `ProfileStore` trait, and reports
  `sqlite` at `/health`. `quantrule-runtime` maps `schema.table` to
  `schema__table`, stores JSON as `TEXT`, timestamps as RFC 3339 `TEXT`, and
  runs repository calls on the blocking pool through the workspace's existing
  `rusqlite` 0.42.
- The simulation service can start on the embedded SQLite profile
  (DIST-201, #693 P1d). `QR_PROFILE=desktop` with `QR_DATA_DIR` opens
  `quantrule.sqlite`, applies `migrations/sqlite/0001_simulation.sql` to the
  `simulation` ledger, and serves the eleven research endpoints through the
  existing `SimulationStore` trait, so no route, contract, or engine changes.
  The baseline writes `simulation__runs` and `simulation__marks` with `TEXT`
  JSON and RFC 3339 `TEXT` instants, keeps the mark identity unique, and keeps
  a decision and its time together. SQLite has no `FOR UPDATE`, so every mark
  and decision mutation opens an immediate transaction and re-reads the run row;
  triggers refuse a mark after a decision and freeze a decided run. Profile
  scoping, the transactional mark upsert, the decision lock, the server
  timestamps, the idempotent oracle and check reads, and the CSV mirror are
  unchanged. The object-store wiring lands with the desktop backend wiring.
- The desktop profile is wired end to end (DIST-203, #695 P2c/P2d/P3d). One
  `quantrule-runtime::backends` construction path selects the backend set from
  `QR_PROFILE`: the server keeps Postgres, S3, and RabbitMQ, and the desktop
  opens one SQLite file at `<QR_DATA_DIR>/quantrule.sqlite` for state and the
  durable queue, plus a filesystem CAS at `<QR_DATA_DIR>/objects`. The data,
  model, evidence, simulation, and backtest binaries now select the object
  store and the queue by profile, so a published `run.requested` is a row
  `qr-backtest` claims from the shared file and evidence's `run.computed` is
  visible to the finalizer. The full verify-mini fixture completes through the
  gateway with no Docker: 12 canonical artifacts, 3,559,171 bytes, canonical
  digest
  `sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa`,
  byte-identical to the server profile. `tools/parity_desktop.sh` runs the
  desktop fixture and compares the digest, the canonical artifact bytes, and
  the run status with a server-profile gateway or an isolated Compose oracle;
  a seeded difference fails. `tools/verify.sh --evidence-dir` retains the
  compared evidence. The server profile remains the default.
- `FilesystemStore`, the object-store adapter for the desktop profile
  (DIST-202, #694 P2a). A publish is a temporary file in the destination
  directory, an fsync, and an atomic rename; a key whose last segment is a bare
  SHA-256 is verified on write and read; a staged write that never commits is
  invisible, a killed writer process leaves no visible object, and a stale
  temporary file is ignored; traversal, empty, and non-portable key segments are
  refused; an identical republish is a no-op and a different body under an
  existing key is refused.
- `InProcessQueue`, the durable queue for the desktop profile (DIST-202, #694
  P2b). The queue stores `queue__messages` and `queue__dead_letter` in the
  shared SQLite file: a publish commits before it returns, a claim takes a lease
  and counts the delivery in one immediate transaction, a successful handler
  deletes the row, and a failed handler makes it ready again, matching the
  RabbitMQ adapter's immediate requeue. A killed consumer's lease expires and
  the same bytes are redelivered with `redelivered` set; the caller can apply
  the worker's three-delivery 5 s/25 s policy through the explicit claim, nack,
  and dead-letter surface, which files `run.dead` with the original payload;
  cancellation removes a ready message by routing key and payload digest and
  survives a reopen.
- The desktop package now carries and supervises the six service binaries. A
  dependency-free `qr-desktop` supervisor starts them on loopback, waits for
  health, restarts a crashed child within a bounded budget, and stops them when
  the shell exits; a guard around each service also ends the children of a
  crashed supervisor. The Linux AppImage and Windows NSIS jobs build the six
  services from the same commit, stage all seven executables and the example
  configs, and fail when one is missing or an interpreter appears. The packaged
  build selects the desktop profile by default; the parity harness and its
  digest evidence are recorded in the desktop wiring entry above.
- The packaged desktop profile is verified from the artifacts, not the tree
  (DIST-203, #695 P3c/P3d).
  `apps/desktop-shell/tests/smoke-package-parity-linux.sh` extracts the
  AppImage, checks the payload, starts the packaged supervisor with a fresh
  `QR_DATA_DIR`, runs the verify-mini fixture from the staged config, requires
  the server canonical digest `sha256:453c90b4…`, and fails on a seeded wrong
  digest. `apps/desktop-shell/tests/smoke-package-parity.ps1` performs the same
  silent-install, six-sidecar, fixture, and digest check for the Windows NSIS
  artifact; the release desktop job runs both after the existing package
  checks.
- The data service can start on the embedded SQLite profile (DIST-201, #693
  P1b). `QR_PROFILE=desktop` with `QR_DATA_DIR` opens `quantrule.sqlite`, applies
  the catalog's SQLite baseline to the shared `schema_migrations` ledger under
  the `catalog` name, and serves `/catalog/datasets` and `/catalog/configs`
  through the existing `DatasetCatalog` and `ConfigCatalog` traits; `/health`
  reports `sqlite`. The baseline is `catalog__datasets`, `catalog__configs`, and
  `catalog__config_bundles`, with JSON as `TEXT` checked by `json_valid`,
  booleans as `INTEGER`, and RFC 3339 UTC timestamps. A config upsert writes its
  shared bundle and profile-scoped row in one transaction, and a dataset keeps
  its object key shared across profiles. The server profile remains on Postgres.
- The desktop SQLite surface has one conformance guard and one written contract
  (DIST-201, #693 P1e). `tools/check_schema_prefixes.py` reads every
  `crates/*/migrations/sqlite/*.sql` baseline and every service store against a
  declared ledger inventory: service-owned `service__table` prefixes, no
  `CREATE SCHEMA`, `jsonb`, `timestamptz`, or `FOR UPDATE`, exactly one baseline
  per ledger, one store module per service, the ledger row in the service's
  profile branch, and the `QR_PROFILE=desktop` with `QR_DATA_DIR` branch that
  opens the single `quantrule.sqlite` file. Each service source outside its
  `#[cfg(test)]` modules may name only its own prefixes. The runtime queue is
  covered through its adapter-owned DDL and its two tables, because it takes no
  service ledger. `docs/architecture/desktop.md` records the environment
  contract, the mapping, the ledgers, the guard, start and stop, and the
  server-profile default.

### Changed

- The desktop shell probes the supervisor's `8080`–`8089` loopback range after
  the configured gateway, and does not persist a fallback address. A development
  shell with no supervisor beside it still opens the configured gateway.

### Fixed

- The desktop filesystem content-addressed store maps each logical key segment
  to a portable filename. `:` and `%` are percent-encoded, a trailing dot or
  space is escaped, and a Windows device name has its first character encoded,
  so `runs/sha256:<digest>/manifest.json` no longer fails with a Windows
  filename error. The logical key, the byte-address check, immutable publish,
  and idempotent republish are unchanged.

## [0.9.0] - 2026-11-15

The first distributed release. One image digest carries six service binaries and
the built browser client; the default Compose stack runs nine containers behind
one published port; the browser, the command shell, and the desktop shell reach
the same `quantrule.api/v1` contract; and the simulation research plane runs as a
separate, hindsight-by-design service. The image contains no Python and
no Bun runtime, and the release publishes the image digest, a Linux AppImage, and
a Windows NSIS installer.

### Changed

- Reviewer verification (`tools/verify.sh`, `make verify`) now runs on the
  distributed path. It uploads the checked-in mini fixture and two
  `verify-mini` configs through the gateway, submits two distinct runs with the
  same effective parameters, waits for both, downloads every canonical
  artifact, computes the digest from the downloaded bytes, cross-checks the run
  record and `/runs/{id}/digest`, and prints one line. A wrong `--expected`
  exits nonzero. The script holds no engine and calls no local `backtest run`;
  the two configs differ only in the run name, so their config hashes and run
  ids differ while their data artifacts must not.

### Fixed

- Run artifact downloads accept nested artifact names. The evidence route
  matches `/runs/{id}/artifacts/{*name}`, and the gateway registers that route
  and refuses an absolute, empty, parent, backslash, or encoded separator or
  parent segment before proxying the full relative name. `run fetch` and the
  reviewer verification download `execution/fills.csv`, `model/readings.csv`,
  and the rest of the canonical set by name.

### Security

- The gateway enforces `QR_BEARER_TOKEN` when it is set. Every API route except
  `/health` and `/version` requires `Authorization: Bearer`, a refusal is `401
  auth.unauthorized` with `WWW-Authenticate: Bearer`, and the static client
  stays reachable so the browser can load. An unset token keeps the documented
  private-network behavior; `/internal/*` never falls back to the SPA.
- The native command boundary now rejects filesystem paths outside the opened
  workspace and memory directory. The desktop Settings, Memory, and Run panels
  use the native dialog picker, and every selected path is scope-validated
  before I/O; typed paths remain validated inline.

### Added

- The product is gated against Python. `tools/check_python.py` fails on a
  `.py` file outside the governance `tools/` and historical `docs/` trees, and
  on a `python` or `python3` reference under `crates/`, `apps/`, `docker/`, or
  `Dockerfile`; it runs in `tools/gate.sh` and CI. The image asserts the same
  invariant directly: `docker/check-no-python.sh` fails on an interpreter or a
  `.py` file, runs in the Dockerfile final stage, and runs again in the release
  image job, so a base-image change cannot quietly reintroduce Python.
- The shell and the browser send the remote profile's bearer token. The shell
  accepts `--token <token>` or `QUANTRULE_TOKEN`, attaches `Authorization:
  Bearer` to every gateway request when set, and keeps the token out of printed
  output, error messages, `Debug` rendering, and local state. The browser's
  Connection screen takes the token, stores it in `localStorage`, sends the same
  header, and turns a `401` into a token-required state with no stale connected
  identity; `Forget token` clears it. The token never enters the URL, a cookie,
  a query string, or the build. An unset token keeps the unauthenticated local
  behavior.
- `make remote` and `make token` implement the remote exposure profile. The
  remote script reads `QR_BEARER_TOKEN` or generates a 256-bit token with
  `tools/token.sh`, exports it to the gateway, starts the stack with only `:8080`
  published, verifies the 401/401/200 token matrix and the published-port set,
  and prints the token with a `curl` example plus the browser and shell token
  steps. The script writes it to no file. Compose passes `QR_BEARER_TOKEN` to
  the gateway with an empty default, so `make up` stays unauthenticated.
- Event inspection is served by `qr-model`. The gateway forwards
  `GET /inspect/events?data_version=…&timeframe=…` to a model-owned internal
  event-batch route with the validated profile, so the data service never
  computes a model. The batch is the full availability-ordered record set the
  `events` command publishes, and inspection writes no object.
- `/profiles` reports the dataset count from the data service, the run count
  from evidence, and the simulation count from simulation, per profile. A run or
  simulation count is the service list page and is bounded at 200 records.
- `quantrule-server` is retired. The gateway is now `quantrule-gateway`
  (`qr-gateway`): profiles in the `gateway` schema, the static client, the proxy
  table, and health, with no `quantrule-app`, engine, or domain service
  dependency. The backtest worker and `qr-backtest` moved to
  `quantrule-backtest-service`. The six binary names, the worker's HTTP path,
  its retry and cancellation boundaries, and the canonical digest are unchanged.
- The #651 service boundary guard reads the workspace manifests, sources, and
  migrations, and fails on a gateway-to-app/engine edge, a second service
  binary, a cross-schema read, or a simulation strategy read, run write, queue
  publish, or model compute. Every rule has a seeded-violation test; the shell's
  27-command contract is untouched.
- `qr-data` serves the catalog, dataset, config, and inspection surface, plus
  the internal config-bundle, dataset-object, and dataset-count references the
  gateway and the worker consume. The gateway proxies `/catalog/*`,
  `/datasets/*`, and `/inspect/*` unchanged and has no data handler.
- `qr-model` serves the public model compute and the internal model-bundle
  reference. The gateway proxies `/model/*` unchanged and has no model handler;
  the public model object is byte-identical to the coupled route, and the
  internal manifest lists every model-bundle artifact with its SHA-256.
- `qr-evidence` serves the run lifecycle and research memory: submission,
  listing, detail, cancel, artifact download, digest, and hypotheses, plus the
  internal claim/read/computed/failed calls the worker uses. Evidence owns the
  `runs` and `research` schemas, validates a submission through the data
  service, and publishes `run.requested`; the gateway proxies `/runs*` and
  `/hypotheses*`, writes no run row, and holds no research store. The parked
  attempt-count migration is registered in the evidence ledger, and the worker
  reaches evidence through the runtime client seam.
- The backtest worker runs on the HTTP data plane. It reads the config and
  dataset through the data client, obtains the model bundle from the model
  client, runs strategy, execution, and portfolio from that bundle through
  `AppService::run_backtest_from_model`, and finalizes through evidence. The
  in-process `AppService::run_backtest` fast path is gone; the model is not
  recomputed. The coupled baseline's artifact names, bytes, and canonical
  digest are unchanged, and the worker checks cancellation after the claim,
  before the model call, before the backtest, and before the first CAS write.
- `qr-simulation` serves the eleven documented simulation endpoints from its own
  `simulation` schema. Runs and marks are scoped by the injected profile, mark
  writes are transactional against the decision lock, timestamps are
  server-assigned, and the human and oracle marks mirror to annotator-schema
  CSV objects. Window selection, the oracle, and the check use the faithful
  `quantrule-simulation` domain crate unchanged; oracle and check are
  idempotent for the same inputs digest. The gateway proxies `/simulation/*`
  with the P3/P4/P5 boundary and keeps no simulation store. The service has no
  queue, no run artifact, no model compute, and no strategy input.
- `quantrule-runtime` gains the concrete HTTP service transport and a
  `DataClient` with an in-memory fake. The runtime migration runner takes a
  ledger schema, so the catalog, gateway, and run migration lists record under
  separate ledgers.
- The six-binary service inventory is complete at milestone M1:
  `quantrule-data-service`, `quantrule-model-service`, and
  `quantrule-simulation-service` add `qr-data`, `qr-model`, and `qr-simulation`
  health/version skeletons, and a workspace guard fails when a service binary is
  removed or an extra one appears. At M1 the gateway still served every route;
  P3 moves the data routes to `qr-data`.
- The service runtime and the internal contract are their own crates:
  `quantrule-runtime` owns configuration, health, logging, clock, shutdown, the
  HTTP serve helper, the Postgres/S3/RabbitMQ adapters, the migration runner, and
  the bytes-only `ServiceClient` seam; `quantrule-service-api` owns the
  `quantrule.internal/v1` run record, claim/complete/fail payloads, run manifest
  and canonical digest, config bundle, dataset object, and model bundle
  manifest. `quantrule-server` keeps its three binaries, routes, and behavior.
- The release pipeline (`.github/workflows/release.yml`) builds the server
  image, records its digest, builds the Linux AppImage and the Windows NSIS
  installer, and attaches the digest and both artifacts to the GitHub release.
  Publishing to a registry is opt-in through documented repository values; a
  `workflow_dispatch` run is a build-only dry run.
- Backup and restore over both durable stores: `tools/backup.sh` writes one
  timestamped archive with a custom-format Postgres dump, an object mirror of
  the bucket, and a manifest carrying the SHA-256 of every payload file plus a
  detached archive digest; `tools/restore.sh` verifies every hash before it
  writes, refuses a corrupt or incomplete archive, refuses a non-empty stack
  without `--force`, and writes objects before rows. The opt-in
  `tools/test_backup.sh` proves a byte-identical round trip on an isolated
  Compose project.
- The single server image and the Compose stack start the full nine-container
  topology by default: one `quantrule-server` image carries `qr-gateway`,
  `qr-data`, `qr-model`, `qr-backtest`, `qr-evidence`, `qr-simulation`, and the
  built client; Postgres, MinIO, and RabbitMQ complete the stack behind
  health-gated startup. `data` serves the catalog, dataset, config, and
  inspection routes through `qr-data`, `model` serves `/model/compute` through
  `qr-model`, `evidence` serves runs and research through `qr-evidence`, and
  `simulation` serves the research plane through `qr-simulation`, while the
  gateway proxies all four; the off-by-default `six-binaries` profile is
  retired. The gateway, `data`, `evidence`, and `simulation` each apply their own
  migration ledger. One published port (`8080`), one bridge
  network, three named volumes. The pinned `bitnamilegacy/minio` archive remains
  a stopgap overridable with `QR_OBJECT_STORE_IMAGE` until the permanent
  registry decision is recorded.
- The operator interface is one command: `make up` builds the default stack,
  waits for health, and prints the URL, and `make help` lists every target.
  `make down`, `make reset` (confirmed), `make logs`, `make ps`, `make scale`,
  `make gate`, `make test`, and the preserved `make verify` and test-tier
  targets complete the set. Windows runs `docker compose up -d` without make,
  and Fedora may use Podman through `make COMPOSE="podman compose"`. The
  `remote` and `token` targets are wired and name their owning distribution
  issue until those scripts land; `backup` and `restore` call
  `tools/backup.sh` and `tools/restore.sh`.
- The shell is a `quantrule.api/v1` client: `health`, `dataset`, `config`,
  `run`, `inspect`, `verify`, `profile`, `simulation`, and `help` commands, a
  global `--profile`, and a `--json` machine view. Every command makes one
  gateway operation; printed output is stable and covered by fixture-server
  tests.
- Client parity is enforced from one shared fixture: the shell's `--json`
  surface and the browser's API layer read the same
  `tests/fixtures/client-parity/api.json` for health, datasets, configs, runs,
  and profiles under one fixed profile, and both must produce the same
  normalized `view.json`. A seeded divergence fails each suite's comparator.
- The simulation research plane gains its meta-label stage: a human entry is
  labelled by its forward path, grouped by EMA stack and ATR bucket, and the
  best cell with enough marks is checked on an unmarked window. `PORT-001`
  registers the simulation domain port from the Python harness at `0814a129`:
  45 real-data cases cover SIM-01..SIM-10 and the named edge cases, every number
  is compared to `1e-9`, and the bulky shared inputs live in
  `docs/port-parity/PORT-001.datasets.json`.
- The desktop shell (`apps/desktop-shell`) is a Tauri 2 window over the gateway
  URL. It remembers the gateway, leaves the active profile to the web client,
  checks GitHub Releases for a newer shell, and shows the bundled client only as
  the disconnected fallback. It owns no IPC command, reads no data file, and
  embeds no engine. The Linux AppImage and Windows NSIS artifacts are unsigned;
  publishing them waits on the release workflow.
- Testing is recorded as three tiers: the fast unit/integration gate, the
  opt-in Compose tier required before a release, and port parity backed by
  golden fixtures from the retired implementation. `tools/check_port_parity.py`
  runs in the gate and fails on a missing registry entry, fixture, marker, or
  comparison test.
- Reviewer verification is four commands and one line: `tools/verify.sh` runs
  the mini fixture twice, compares the canonical digest, and `--expected`
  compares a second host's line. `make verify` is the shortcut. The full
  example remains the release run.
- Crash tests prove that a backtest stopped before the manifest leaves no run
  prefix and no artifact, and that an evidence process stopped before `complete`
  leaves the run `running`; each restarts from the durable object set and
  finishes. A real `SIGKILL` remains composed-tier evidence (#643).
- A scale-invariance harness proves one pass and four model shards share one
  object digest in any completion order, and a reversed requested ladder
  produces the same bytes; an unsorted merge changes the digest.
- A source guard over the signal path fails on transcendental, power, and
  fused-multiply-add calls; `sqrt`, rounding, and the four arithmetic operations
  remain allowed. The benchmark and test fixtures are outside the guard.
- The client label table is generated from `docs/registry/indicators.json` into
  `apps/web/src/lib/labels.generated.ts`; `check_labels` fails the gate when the
  checked-in table drifts, so every numeric code has one word mapping.
- The simulation chart is a client-side SVG renderer for candles, volume,
  EMA 20/50/200, one indicator panel, event markers, confirmed levels, and a
  mark layer. Pointer position maps to a bar through the rendered scale, keys
  1–8 capture the eight entry/exit × good/bad × long/short mark classes, `Z`
  undoes the last session mark, and a note is capped at 500 characters. Marks
  post through one seam against `/simulation/runs/{id}/marks`; the browser test
  mocks that contract until the `simulation` service ships.
- The simulation screens: the browser lists simulation runs, creates one from a
  premise, bundle ref, inputs, seed, and criteria, and opens a run with its
  window navigator, chart, mark toolbar, human and oracle marks, review counts,
  check table, and decision. A decision shows its note and locks marking in the
  toolbar and the mark endpoint; a disconnected gateway clears the list. The
  screens read the documented `simulation` contract through one seam; the real
  service (#680) is still pending, so the browser tests mock that contract.
- The browser client distinguishes a gateway API mismatch from a healthy
  connection and shows the version it expected; a shared Playwright fixture
  router drives the connected, mismatch, degraded, and unreachable tests.
- The control-plane queue topology: `runs.backtest`, `runs.evidence`, and
  `runs.dead-letter` are named durable queues. A run records its attempt count;
  a retryable failure holds the delivery for 5 s, then 25 s; the third failure
  marks the run `failed` and publishes `run.dead`. A broker redelivery reclaims
  an uncomputed running run. Cancelling publishes `run.cancelled`, and every
  message carries the run id only.
- The backtest worker core: `POST`-submitted runs are claimed, their six-file
  config and catalogued dataset are materialized into a private workspace, the
  coupled backtest runs, artifacts are written to CAS, and the canonical digest
  is computed over `results.json`, `*.csv`, and `*.ndjson`. The run stays
  `running` for evidence. `qr-backtest` consumes `run.requested`; duplicate
  deliveries skip a terminal or running run, and a cancel before finalization
  leaves it cancelled.
- The evidence service: `qr-evidence` consumes `run.computed`, verifies every
  artifact against its recorded size and SHA-256, recomputes the canonical
  digest, rejects a missing, changed, or mismatched artifact, and marks the run
  `succeeded` with the evidence stage `Done`. A finalized run is never finalized
  again. The gateway and worker now publish the run id only.
- A `QueueConsumer` trait with a RabbitMQ implementation and an in-memory fake;
  a successful handler is acked, a failed one is nacked.
- The model compute route: `POST /model/compute` loads a catalogued dataset and
  run config by reference, materializes the config bundle, computes the declared
  ladder per timeframe, merges the reports sorted, and stores one JSON object in
  CAS. A single timeframe and the full ladder produce identical bytes.
- Config rows are profile-scoped while the file bundle is shared by content id:
  a repository scan writes a row for the requesting profile, an upload writes
  one for its profile, and a run submission scans the mount on a miss. Switching
  the client profile reloads the config list.
- A catalogued config stores its whole bundle, not just `run.json`. A repository
  scan reads the five referenced files; an uploaded config keeps the validated
  path-to-body map. A run can reproduce the configuration after the staging
  directory is gone.
- The Datasets screen exposes tolerant upload: a "Skip bad rows" control uses
  the tolerant route and renders the quality report, including the first skipped
  line and its reason.
- Tolerant ingest: `POST /catalog/datasets/tolerant` skips a bad row instead of
  failing the upload and records the accepted candles plus a quality report
  (rows read, accepted, rejected, duplicate, out of order, invalid, and each
  skipped line with its reason). The accepted rows keep the identity a strict
  upload of the same clean rows would produce, and the report is stored on the
  dataset record.
- Config uploads: `POST /catalog/configs` accepts a run-config bundle as
  relative paths and bodies, stages it privately, validates it with the same
  loader as the mount, and catalogues it as `upload`. A path that is absolute
  or contains `..` is refused; an invalid bundle is still catalogued with its
  parser message.
- Run detail: the browser opens a run from the inbox and shows the stage
  timeline, provenance including the image digest, the artifact list with
  hashes, and the canonical digest with a copy action. A failed run shows its
  error and hides the artifact section.
- The configs screen: the browser lists the config catalog with origin and
  validity, renders the parser message verbatim, disables submit on an invalid
  config or with no dataset, and submits a run from a valid config without a
  path. A queued run can be opened in the runs inbox.
- The runs inbox: the browser lists runs from the server with status, config,
  data version, submission time, and duration; it polls every 2 s while a run is
  active and every 30 s otherwise, cancels only an active run, drops the rows on
  a failed read, and shows a disconnected state instead of stale evidence.
- Run submission: `POST /runs` validates the dataset and the run config, derives
  `run_id` from `data_version`, `config_hash`, `model_version`, and
  `image_digest`, stores a queued run, and publishes `run.requested`. A duplicate
  returns `200` with the same run. `GET /runs`, `GET /runs/{id}`, and
  `POST /runs/{id}/cancel` complete the control-plane surface. Runs are shared
  across profiles through submission rows, so identical inputs produce one run.
- The config catalog: `GET /catalog/configs` scans the read-only run-config
  mount, validates each `run.json` with its referenced files, and lists a
  content-derived id, validity, the parser message, and a SHA-256 over the five
  files. A second read refreshes the rows without duplicating them. Config
  uploads remain planned.
- The gateway serves the built client from `QR_STATIC_DIR`: assets are cached
  immutably, an unknown page falls back to `index.html`, a write method is not
  served the SPA, and an `/api/*` path never falls back.
- A Playwright harness for the client: create a profile, switch it, survive a
  reload, and surface an unknown stored profile. The API is mocked at the
  network boundary, so the tests need no backend.
- The profile switcher and Profiles screen: the client lists profiles, creates
  one, remembers the active id locally, and sends it on every call. Switching
  reloads the dataset list for that profile.
- The browser client's Datasets screen: it lists `data_version`, candles, and
  UTC coverage, uploads a CSV with the file as the request body (no path field),
  shows an indeterminate upload bar, and renders the stable error code. A
  duplicate upload returns the existing record and does not add a row.
- The gateway now serves the full API under `/api/v1`, the base the client
  already uses, while the same routes remain at the root for probes.
- The stored ladder: `GET /datasets/{version}/ladder/{timeframe}` folds the
  canonical M1 bytes into one timeframe's CSV and writes it under
  `ladder/{data_version}/full/{timeframe}`. A repeat request reuses the object.
  `LadderWriter` streams the same buckets as `TimeframeSet`, so an emitted bar
  and a published bar cannot disagree.
- Hypotheses are profile-scoped: migration 0008 adds `profile_id`, the store keys
  by `(profile_id, fingerprint)`, and the hypothesis routes require a known
  profile. The same hypothesis can be sealed in two profiles without one
  becoming visible to the other.
- The candle stream: `GET /datasets/{version}/candles` returns the stored
  canonical CSV after re-hashing it against `data_version` and sets an ETag, so
  a run consumes exactly the bytes the catalog identifies.
- Read-only inspection over stored bytes: `GET /inspect/candles` returns counts
  and coverage, and `GET /inspect/timeframes` returns complete bars and partial
  buckets per timeframe. The object is re-hashed before it is parsed, so a
  catalog row and its bytes cannot disagree silently.
- The ladder's streaming count (`LadderSummary`) folds the same buckets as
  `TimeframeSet` without retaining the series, so a large dataset can be
  inspected in bounded memory.
- Ingest now refuses an out-of-order, duplicate, misaligned, or coarser-than-M1
  source instead of cataloguing a series the ladder would reject.
- Profiles are real records: `GET /profiles` lists them with dataset counts,
  `POST /profiles` creates one idempotently, and migration 0007 seeds the
  `default` profile. The dataset routes now refuse an unknown profile instead of
  accepting any syntactically valid id.
- The dataset catalog: `POST /catalog/datasets` turns CSV bytes into a
  content-addressed, profile-scoped record, `GET /catalog/datasets` lists the
  profile's summaries, and `GET /catalog/datasets/{version}` returns one record.
  A repeat upload is idempotent and returns the existing record.
- The code module map at `docs/architecture/module-map.md`: crate and module
  ownership, reading order, and the where-to-add guide for a reading, event,
  command, configuration field, use case, or desktop command.
- Native file and folder pickers in the desktop Settings, Memory, and Run
  panels. A selected path still passes the backend workspace/memory scope check
  before any I/O, and typed paths remain available.
- The shell `memory` command family and the desktop memory write forms:
  status, record with computed model counts, workspace/run lists, metric,
  claim, and note CRUD, logical export/import, and SQLite backup/restore. The
  shared application service owns the validation; the command surface now has
  16 implemented commands.
- The board permanence checker (`tools/check_board_permanence.py`) and the
  project-sync status test. CI rejects a delete or archive mutation in the board
  automation and proves that `Done` stays terminal.

### Changed

- The simulation domain is a faithful Rust port of the Python harness. Window
  selection reproduces the reference's seeded MT19937 year-stratified event
  sample, marks validation and the CSV round-trip match the annotator schema,
  and the oracle, review, check, exit lab, and meta stages reproduce the
  reference numbers. The earlier equal-span selector, forward-return oracle,
  two-set review, and three-policy exit lab are gone. `PORT-001` compares the
  full domain across SIM-01..SIM-10 to `1e-9`.
- The shell depends only on `quantrule-api`, `ureq`, and `serde_json`. Local
  source fetch and local stage execution are gone; a run is submitted to the
  gateway and read back through the run and inspection routes.
- Run artifacts publish atomically: bytes live at `artifacts/{sha256}` and one
  `runs/{run_id}/manifest.json` is the publish point. A worker stopped mid-write
  leaves blobs but no manifest, no recorded artifact, and no succeeded run.
- The v4 strategy example uses a 1m execution fill to match the strategy. The
  worker exposed that the app requires the strategy execution timeframe and the
  execution fill timeframe to agree; the v4 contract now states it.
- The model engine's extended signal detector now sits under
  `events/extended/`, the extended submodules use explicit imports, and source
  codes are defined once in `events/source_codes.rs`.
- The stage-provenance reader is shared by the model, strategy, execution, and
  portfolio bundles; the duplicate reader error names are gone.
- The desktop command boundary is split into `views`, `scope`, `workspace`, and
  `memory` modules.
- The model report and data-inspection use cases now live behind
  `quantrule-app`; the model bundle, report renderer, labels, and model-spec
  parser moved out of the shell. The shell keeps parsing and terminal output.
- The strategy, execution, portfolio, backtest, qualification, and analytics
  use cases now live behind `quantrule-app`; the shell is a parse/render/fetch
  adapter and the desktop has one Tauri command per use case.
- The desktop UI has a Pipeline section for inspecting data and running the
  model, strategy, execution, portfolio, backtest, qualification, and analytics
  stages. Source fetch remains shell-only.
- The desktop UI is a first-class surface of the same program as the shell. Both
  call the shared application service and are required to have feature parity;
  the shell is primarily for agents. The
  [one-program ADR](docs/adr/2026-11-11/one-program-first-class-desktop.md) and
  the [audit remediation plan](docs/execution-plans/2026-11-11/audit-app-service-native-remediation.md)
  record the target and the remaining parity extraction.
- The project board is append-only history. A `Done` item is terminal and is
  never deleted, archived, removed, bulk-cleaned, or moved back to an earlier
  status; worktree and branch cleanup never touches the board.
- `project-sync.sh` now keeps a `Done` leaf or container `Done` instead of
  resetting an open leaf to `Todo` or recomputing a container when a child
  reopens.
- Execution plans now carry a delivery sequence and a verification table with a
  row for every work-breakdown phase, acceptance criterion, and closure step. The
  template names roles, dependencies, and affected paths, and the plan checker
  rejects duplicate IDs, empty core cells, and unresolved rows on a `Verified`
  plan.

### Removed

- The local stage commands (`candles`, `candles-tolerant`, `timeframes`,
  `events`, `report model`, `run check`, `strategy explain`, `strategy run`,
  `execution run`, `portfolio replay`, `backtest run`, `qualify`, and
  `analytics evaluate`), source `fetch`, and the local `memory` command are
  removed from the shell in v1.0.0. Their capability moves into the run and
  read-only inspection.
- The Python simulation harness, chart annotator, Progression replay oracle, and
  trade-context exporter under `tools/simulation`, `tools/charts`,
  `tools/oracle`, and `tools/trades`, with their tests and READMEs. The Rust
  `simulation` service and the browser client own chart-first research, and
  `PORT-001` keeps the golden fixture that compares the port at `1e-9`. The
  golden fixtures needed no move: the Progression fixture is already under
  `crates/quantrule-model/tests/fixtures/` and the simulation fixture is under
  `docs/port-parity/`. The run folders under `data/simulation/` remain local
  data. The retired-path gate now rejects a reintroduced path in a current
  document or source file.

### Fixed

- The migration runner pins the advisory lock to one Postgres session: it
  acquires one `PoolConnection`, applies the ledger and migrations, and releases
  on that same connection. A release through the pool could land on another
  connection and leave the lock on an idle session, which deadlocked concurrent
  migrators in #772. A composed two-migrator test with a 10 s budget covers the
  fix and its postcondition.
- The in-memory run store now rejects a `complete` or `fail` transition from a
  queued or terminal run, matching the Postgres `status = 'running'` predicate;
  a test that accepted a transition production rejects can no longer pass.
- An invalid uploaded config now keeps the `upload` origin; the shared record
  loader had hard-coded `repository` on the invalid branch.
- The client's startup load no longer loops: the two initial requests run from
  `onMount` instead of an effect that read the state it updated.
- Stale module docs that described the shell as the only surface, the backtest
  runner as future work, the app as the only configuration parser, the desktop
  as doing no path resolution, the model as six scalar readings, and the flat
  report as the M1.0 view. The shell glossary now names M1.3 as the current
  model.

### Added

- The v1.0.0 distributed architecture record: eight decision records under
  `docs/adr/2026-11-12/`, the service catalog, and the data-plane,
  control-plane, HTTP API, deployment, client, user-journey, security, and
  operations contracts under `docs/architecture/`.
- The distributed execution plan
  `docs/execution-plans/2026-11-12/dist-000-distributed-platform.md` and its
  issue tree `DIST-000` … `DIST-090`.
- Verification tutorials for the distributed target: run a backtest, verify the
  canonical digest, use remote compute, and scale workers.

- The `simulation` service and the research plane: chart-first, hindsight-labeled
  research, isolated from the signal path, with an inputs digest and no output
  digest (`ADR-2026-11-12-09`).
- The Python boundary: the product image carries no Python interpreter; Python is
  confined to repository governance (`ADR-2026-11-12-10`).

- Profiles: organisational scoping over every record, with shared content-addressed
  bytes and no security promise (`ADR-2026-11-12-11`).
- A Tauri desktop shell over the gateway URL, and backend adapter traits so the
  embedded desktop profile is additive rather than a fork (`ADR-2026-11-12-12`).

### Changed

- The board is aligned to the kata: six issues carry the six requirements
  (hypothesis record, exploration, costs, statistical significance,
  out-of-sample validation, comparison). Profiles and the desktop shell stay in
  scope because they serve the same single user; multi-user is not built.
- The platform grows from five services to six; the deployment from eight
  containers to nine.
- The browser client renders charts as SVG and captures marks in the page; the
  server no longer renders PNGs.
- The client toolchain is Bun; the image carries static assets, not a Bun runtime.
- `AGENTS.md` now describes a distributed single-operator platform: a browser
  client and a command shell on one HTTP contract, two planes, two test tiers,
  and a bounded determinism claim.
- `README.md` now states the v1.0.0 target and keeps the implemented command
  table labelled as the v0.3.0 tree.
- `docs/architecture/` now separates current decisions from target contracts;
  every target document carries `Status: Target v1.0.0`.

## [0.8.0] - 2026-11-11

### Added

- The four-stage development workflow: `Todo`, `In Progress`, `In Review`, and
  `Done`, with a required claim type, start and close dates, a human review gate,
  and an execution-plan verification table that the reviewer checks row by row.
  The new `check_claims.py` and `check_execution_plans.py` gates cover the
  templates and current plans.
- The Tauri Platform definition: local-first ADR, course folders, kata,
  architecture characteristics, architecture-description outline, methodology
  checkpoints, and a Dockerized PDF build. The current product is solo and local;
  the team cloud is future work.
- The `quantrule-app` application service: workspace discovery, dataset summaries,
  run validation, model-run summaries, and deterministic run-summary export. The
  command shell and the future desktop UI call the same service; `run check` now
  goes through it.
- The local research memory: `quantrule.research-memory/v1` over bundled SQLite,
  with workspace, dataset, config, run, metric, claim, and note records, ordered
  queries, deterministic JSON export/import, and consistent backup/restore.
  `AppService::record_run` writes a validated run and its workspace, dataset, and
  configuration in one call.
- The Tauri 2 + Svelte 5 desktop UI in `apps/desktop`: workspace, data, configs,
  model run, research memory, export/backup, and settings over the application
  service. It has no account, network path, or domain computation. Frontend unit
  tests, Playwright browser E2E, and a native Xvfb launch smoke test cover it.
- The Linux distribution path: desktop version `0.9.0`, Tauri `.deb` and
  `.AppImage` bundles, a static download page at `site/`, SHA-256 checksums, and
  `site/check.py` verification. The page is distribution only; no account,
  backend, or cloud runtime.

### Changed

- The current run schema is `quantrule.run/v3`; the policy layer is removed and
  `run check` replaces `policy check`. Portfolio capital, sizing, costs,
  drawdown, leverage, and position limits remain first-class constraints. The
  qualification dimension `prop_firm_limits` is removed.
- The course deliverables are final: the architecture description now cites the
  implemented desktop, application service, memory, and distribution code, the
  characteristics and methodology checkpoints match the Phase 3 evidence, and
  all nine PDFs are rebuilt. The 0.9.0 release-readiness record is added; the tag
  remains a human gate.

### Removed

- The current `policy.json`, policy loader, `policy check` command, and policy
  cross-config checks. Historical `run/v2` evidence remains read-only.

## [0.7.0] - 2026-11-11

### Added

- The Binance BTCUSDT public-archive importer and coverage audit, with 96
  checksum-verified monthly files covering 2018–2025. The source is admitted for
  cross-venue robustness outside the formal partitions; 2023 remains excluded.
- `analytics evaluate` now publishes buy-and-hold and matched random-timing
  benchmarks in `analytics/benchmarks.csv` and `analytics/metrics.json`, with an
  explicit seed, path count, cost assumptions, and null reasons.
- A deterministic 15m trade-chart tool that renders one 1,000-candle chart per
  trade, with OHLC, volume, model EMA 20/50/200, entry/exit markers, and a manifest.
- A local data-cleanup record: large historical evidence is archived outside the
  working tree and reachable through the original `data/` symlinks.
- The `tools/trades/` context exporter, annotation mapper, and exit-quality
  table. Each trade gets a 250-bar entry/exit window with all readings, events,
  decisions, orders, and fills; the post-fill window is separate.
- A local click-to-annotate chart page. It maps a click to the candle from the
  chart manifest and writes `annotations.csv` directly, with no hand-edited CSV
  and no pixel detection.
- The `M1.2` model structure extensions: confirmed pivot-sequence counts and
  lower-high/lower-low events, causal Fibonacci retracements and level tests,
  and multi-touch horizontal support/resistance bands and level tests. The
  readings CSV grows to 121 columns and every annotation context carries them.
  The registry is `2026-11-10.1` with 49 indicators and 150 events.
- Strategy position excursions: entry price/ATR, unrealized ATR, MFE, MAE, and a
  configurable giveback exit with an arming threshold. The strategy still sees
  no cash, equity, P&L, fill, order, or quantity.
- The `tools/trades/annotation_zones.py` zone extractor and the 2018–2019
  annotation zone report. It publishes EMA-tap, Fibonacci, band, pivot-sequence,
  and forward-return features around each human mark.
- A deterministic raw-edge diagnostic (`tools/research/raw_edge.py`) that measures
  forward returns, MFE/MAE, and barrier outcomes for a published entry signal,
  with event-only, state-only, and unconditional controls. It writes only an
  evaluation report and never a strategy artifact.
- A configurable event-edge screen (`tools/research/event_edge.py`) that applies
  the same forward and barrier measurements to a declared event identity and
  window, with an all-bars control. It rejects event subtypes the strategy
  engine cannot read.
- A deterministic SBE-01 scale screen (`tools/research/scale_screen.py`) that
  tests a range-fraction stop/target geometry for cost feasibility against a
  published compression range and ATR. It writes only an evaluation report and
  no strategy or portfolio artifact.
- A fresh-onset 5m breakout screen (`tools/research/breakout_edge.py`) with nine
  fixture tests and the SR13 development record. The 6-bar pooled result is
  negative; no candidate and 2024 remains closed.
- Undo for the click annotator: `Ctrl+Z`, `Cmd+Z`, or the **Undo** button
  reverses the last save or delete in the current server session.
- The simulation-run harness (`tools/simulation/`) and the
  `simulation-research` workflow. A run selects deterministic M1.3 windows,
  renders clean charts, writes hindsight good/bad marks in the annotator schema,
  reviews the candle-shape context, and checks the extracted rule on 2020–2022
  with an unfiltered control and a 6 bps proxy. It is exploration only; a
  survivor goes to Strategy Research. The same harness includes the SIM-01
  human-mark meta-label and the SIM-03 five-exit comparison.
- A persistent local data root outside the repository and every worktree
  (`$QUANTRULE_DATA_ROOT` or `$HOME/quant-rule-data`), and the
  `tools/worktree_cleanup.py` guard. The guard classifies local data, archives
  it with a SHA-256 manifest, and refuses to remove an unmerged or unclassified
  worktree. The merged-task cleanup now checks and archives before removal.

### Changed

- Merged-task cleanup is mandatory: `AGENTS.md` now requires removing the task
  worktree and deleting its local and remote branches before a task is reported
  complete. The completion report says `Clean up` while cleanup remains;
  `No action` is reserved for after cleanup.
- The BTC 15m break-only long control is not promoted. On the dense Binance
  BTCUSDT series, 2018–2022 is 65 trades, −$389.64, PF 0.871, and 2023 is 8
  trades, −$202.50, PF 0.229; the Dukascopy lead is not venue-robust.
- The model identity is `M1.3` with registry `2026-11-10.2`. It corrects the
  M1.2 sequence-event occurrence bar, makes `level.fib_test` a bar-range
  intersection with one event per tested level, and fixes the Fibonacci/band
  tolerances as model constants; M1.2 remains historical.
- The annotation-zone close-based features are renamed `fib_close_proximity`
  and `band_close_inside` and are documented as distinct from the model's
  `level.fib_test` and `level.band_test` events.

### Fixed

- Pivot-sequence events now record the confirmed pivot's occurrence bar
  separately from its confirmation availability bar.
- `level.fib_test` now detects a level the bar trades through, not only a level
  near the candle high or low, and emits one event per tested level.
- Spec v2 now rejects the fixed `fib_test_atr` and `band_atr` overrides, and the
  model validates both tolerances for programmatic callers.
- The strategy contract now describes the registered-subtype rule the
  event-composite validator actually enforces.
- The `band` reading no longer uses `expect` in library code; an empty cluster
  or band publishes null instead of panicking.
- Trade charts now render the model's `ema_fast`/`ema_mid`/`ema_slow`
  (20/50/200) instead of a chart-side EMA 10/50/200 calculation.
- Trade charts now draw the exit marker and label the exit action (`SELL` for a
  long, `BUY` for a short); previously only the entry marker was visible.

## [0.6.0] - 2026-11-09

### Added

- An optional `session` calendar in `data.json` and session-aware coverage
  fields in the model report. Weekends and declared holidays no longer count as
  missing bars for session-bound instruments; the existing span coverage is
  retained.
- The `signal.breakout_retest_hold` event: a compression breakout followed by a
  4-bar retest within 0.2 ATR that closes back on the breakout side. It is a
  signal event with code 146, subtype 24, source 5; the break-only event is the
  control.
- The `vol_regime` and `structure_state` context states. `vol_regime` is the
  persistent 0/1/2 volatility regime; `structure_state` is the confirmed major
  +1/0/-1 swing structure. Both are strategy-readable and leave existing event
  values unchanged.
- `help workflows` in the shell: an intention-to-workflow selection guide with
  the first command or tool for each workflow.
- The workflow lab now shows the command-level selection table and the model
  research/swarm quick path.
- The model-prediction condition gate accepts the published `level_touches`
  reading and integer `min`/`max` bounds, enabling a location-conditioned model
  evaluation without a Rust model change.
- The `MR`/`SR`/`PR` research-family prefix and the `-Cnn` configuration suffix;
  historical `M2.x`–`M4.x` aliases remain in `docs/registry/history.md`.
- Per-cell model configuration in the research runner, with the model config
  hash recorded in provenance.
- Sequence, confluence, freshness, and descriptive path evaluators for model
  research; the MR5 inherited-coverage checker, grid builder, and reference
  grid.
- Event-family sequence conditions (`first_events` / `second_events`), a strict
  ordering gap, and a confirmed-level filter on the confirming bar.
- Per-hypothesis timeframes, a configurable family/run prefix, and synthesized
  run configs for an extra instrument in the research grid builder.
- The MR6 ordered-confirmation grid (15 cells, five primary, reverse-order
  controls) and its `no_edge` development result.
- The `M1.1` opening-range sweep and no-reclaim control events, with the
  `session.sweep_atr` spec field; the registry moves to 145 events and 191
  validation claims.
- `COPPERCMDUSD` and `TSLAUSUSD` as price-only instruments, with aliases
  `COPPER` and `TSLA`; volume, participation, and flow events exclude them.
- The MR7 opening-range sweep grid (15 cells) and its `no_edge` result.
- The research report applies a declared mean-net-R path gate before naming a
  candidate.
- An optional `contrarian` boolean on an event-composite event entry: the
  entry takes the opposite side of the matched event while the event identity
  and evidence stay unchanged. The default is `false`; the parser rejects the
  field on a state condition and on the compression-bracket engine.
- The SR1 compression-breakout fade strategy config, 15-cell development
  matrix, 9 bps sensitivity configs, and per-cell qualification criteria.
- A deterministic strategy feasibility preflight
  (`tools/improvement/feasibility.py`) and its Strategy Research gate: a
  hypothesis whose target is below its round-trip cost, whose breakeven win
  rate exceeds 50% after costs, or whose timeframe exceeds 1 hour is rejected
  before any backtest cell. The SR1 development run stopped early as
  infeasible after 10 of 15 cells; the completed cells all halted, and the 4:1
  payoff failed the preflight.
- The SR2-A BTCUSD robustness audit and its read-only analysis tool: the
  positive BTC lead survives 9 bps and the 5% drawdown run in sign, but is
  concentrated in the top five trades, is not statistically significant, and is
  about 0.1%/month at the 5% ceiling. The audit generates condition hypotheses
  but no candidate.
- The SR2 dual-design fade configs and its 30-cell development run: 1h with a
  1 ATR stop / 3 ATR target and 15m with a 1 ATR stop / 4 ATR target, both on
  BTCUSD/EURUSD/XAUUSD. Both designs returned `no_edge`; the 9 bps sensitivity
  and the 2024 validation were not opened.
- The `rvol_regime` model context state: `-1` below 1.4, `0` from 1.4 to 2.0,
  and `+1` at or above 2.0. It is published in `readings.csv` and `report.json`
  and is readable as a strategy state; no event value changes.
- The SR2-BTC-FEATURES top-trade attribution tool and record: 139 decision-bar
  feature rows, five 1,000-candle charts, and the stack-aligned low-RVOL fade
  hypothesis. The 2019 loss persists in every filter subset.
- The SR3-FEATURES feature-filtered BTCUSD fade programme: eight 2018–2022
  variants, 40 base cells, and 30 9 bps sensitivity cells. The 1h sides are
  positive at 6 and 9 bps but have 9–21 trades per side; the 15m entry is
  negative; H4 long reaches 5.04%. Verdict: no candidate; 2024 not opened.
- The SR4-STACK-FADE multi-instrument programme: three 1h rules × BTCUSD,
  EURUSD, and XAUUSD × two sides × 2018–2022, 90 base cells. The pooled S1 sides
  are positive but BTC-only; EURUSD and XAUUSD are negative on every variant and
  side, and the BTC cells breach 5% drawdown. Verdict: no candidate; 9 bps and
  2024 not opened.
- The SR5 absorption-rejection programme and its count gate: 15m
  `signal.absorption_rejection` with a 1h EMA-stack context on BTCUSD. A1 has 5
  long and 8 short events over 2018–2022, below the 30-per-side gate; the
  event-only control is 15/17. Verdict: underpowered; no backtest, 9 bps, or
  2024 access.
- The SR6-VWAP programme: 15m `signal.vwap_rejection` with a 1h EMA-stack
  context on BTCUSD, 30 base cells. V1 long and short are negative with PF
  0.966/0.853 and every cell breaches 5% drawdown. Verdict: no candidate; 9 bps
  and 2024 not opened.
- The SR7-H3-CRYPTO programme: BTCUSD+ETHUSD H3 1h fade, 20 base cells. H3
  long is +$2,289 with PF 2.098 and 5/5 positive years but 27 trades and 5.62%
  drawdown; H3 short is PF 1.067 with 1/5 years and a negative ETH side.
  Verdict: no candidate; 9 bps and 2024 not opened.
- The SR8-H3-BUFFER programme: one-shot 2023 BTC buffer for the frozen H3 rule.
  2023 1h coverage is 61.3%, below the 95% gate; H3 long has 0 trades and H3
  short 4. Verdict: buffer ineligible; no no-edge conclusion; 2024 not opened.
- The SR8-A-H3-ARCHIVE check: the Dukascopy 2023 archive has 525,600 rows but
  38,948 flat zero-volume placeholders; only 61.4% of 1h hours are genuine. The
  archive is rejected, no H3 run is made, and 2024 remains closed.
- The SR9-15M-STRUCTURE campaign: 15m structure-transition on BTCUSD, XAUUSD,
  and EURUSD. All six primary cells are below 30 trades and five have PF below
  1.0; the break-only BTC long control is positive at 6 and 9 bps (30 trades,
  PF 1.746, max drawdown 2.87%) but is not promoted. Verdict: no candidate,
  underpowered; 2024 not opened.

### Changed

- `run.json` accepts an optional `model_timeframes` ladder. The model computes
  only the selected timeframes while the data engine still builds the full data
  ladder; the selected timeframe's model output is unchanged. The SR2 cells
  dropped from minutes to seconds.

### Fixed

- The SR9 data configs resolve the source path from `config/sr9/data` with three
  parent levels; the previous path skipped one too many and could not rerun.
- The event-composite strategy accepts registered signal subtypes above 7; it
  previously rejected `signal.vwap_rejection` (subtype 14) even though the
  registry marks it a signal. Diagnostics remain rejected.
- The SR5 absorption count gate now filters on the unique event code 52; the
  first gate also matched `signal.absorption_break` (code 51). The corrected A1
  count is 5 long / 8 short; the underpowered verdict is unchanged.
- The model condition evaluator accepts float `min`/`max` bounds for continuous
  readings such as `rvol_ratio` and `atr`; integer `values` is unchanged.
- The strategy event snapshot selects its event window by binary search over
  the availability-sorted event batch instead of rescanning the full batch on
  every execution bar. The selected events are unchanged; a one-year 1-minute
  cell no longer spends its runtime in a quadratic scan.

## [0.5.0] - 2026-11-07

### Added

- Workflow index, tier runbooks for model research, model change, strategy
  research, and portfolio research, and engineering workflows for requirements,
  release/operations, deprecation, and recordkeeping.
- Workflow and record templates, including the workflow skeleton, policy change,
  data change, release, deprecation, and requirements records.
- Workflow, changelog, and partition checkers, with CI coverage.
- Current improvement run adapter (`tools/improvement/run_current.py`) and the
  `quantrule.improvement-run/v2` manifest.
- Issue templates for task, audit, refactor, research programme, data, policy,
  release, deprecation, and remediation work.
- Workflow lab tutorial.

### Changed

- `AGENTS.md`, `README.md`, and `docs/README.md` now describe the current S3
  command surface and point to the workflow index.
- The improvement run contract is `quantrule.improvement-run/v2`; v1 is
  historical.
- `data/partitions.json` uses canonical instrument IDs with aliases for the
  historical research configs.
- The improvement evidence extractor reads the current analytics, portfolio,
  qualification, and model bundles.

### Fixed

- Removed stale pre-S3 status claims from the workflow, architecture, policy, and
  model documentation.
- The improvement run adapter invokes `backtest run`, `qualify`, and
  `analytics evaluate` instead of the retired `backtest` command.

### Removed

- No runtime surface was removed in this change.

## Release rules

- `## [Unreleased]` accumulates entries until a release.
- A released section is `## [<version>] - YYYY-MM-DD`.
- Entries are grouped under `Added`, `Changed`, `Deprecated`, `Removed`,
  `Fixed`, or `Security`.
- One entry per meaningful user-facing or developer-facing change, not one per
  pull request. Internal refactors, test-only changes, CI-only changes, and typo
  fixes do not need an entry.
- A pull request that does not change user-facing or developer-facing behavior
  records "no changelog entry needed" and the reason in its description.
- Released sections are immutable. A correction is a new release or a dated
  addendum in the next release.
- A release tag `vX.Y.Z` has a matching `## [X.Y.Z] - YYYY-MM-DD` section and a
  release note at `docs/releases/<YYYY-MM-DD>/vX.Y.Z.md`.
- Published tags are immutable. A bad release gets a new tag; a published tag is
  never moved, deleted, or reused.
- The changelog is not a duplicate of the GitHub Project, the AI transparency
  log, or the audit/refactor/improvement records. Each layer answers a different
  question.
