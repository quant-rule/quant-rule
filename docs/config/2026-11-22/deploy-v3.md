# quantrule.deploy/v3

- Date: 2026-11-22 (UTC)
- Status: Current for schema, validation, render, check, the remote/overlay
  inspector, the PLAT-001 bootstrap/rolling-update control plane, the PLAT-002
  cert-mode ingress, the PLAT-003 Patroni/etcd/HAProxy tier, the PLAT-004
  RabbitMQ tier (broker, front door, reconnect, check rows, definitions, lab),
  the PLAT-005 MinIO tier (erasure, TLS, HAProxy front door, bucket,
  versioning, least-privilege account, object bootstrap, check rows, node-loss
  lab), and the PLAT-006 observability tier (16 pins, scrape, 38 rules over 17
  rows, Loki/promtail, four dashboards, operator tool, static check, lab)
- Claim type: Behaviour Claim
- Issue: [#1110](https://github.com/quant-rule/quant-rule/issues/1110)
- Related: [ADR-HA](../../adr/2026-11-22/1082-ha-deployment-profile.md),
  [HA architecture](../../architecture/ha.md),
  [Multi-host](../../architecture/multi-host.md),
  [Swarm control plane](../2026-11-23/swarm-control.md),
  [ingress](../2026-11-23/ingress.md),
  [Patroni cluster](../2026-11-23/patroni-cluster.md),
  [RabbitMQ cluster](../2026-11-23/rabbitmq-cluster.md),
  [MinIO cluster](../2026-11-23/minio-cluster.md),
  [secret files](../2026-11-23/secret-files.md),
  [internal TLS](../2026-11-23/internal-tls.md),
  [HA deployment chapter](../../manual/15-ha-deployment.md)

`quantrule.deploy/v3` is the single descriptor for the HA profile. It places
the seven stateless services and the one ingress, pins every rendered
infrastructure unit by digest, names secret files, and rejects unsafe placement.
v2 remains readable for the older multi-host profile. The checked-in sample is
[`deploy/v3/deploy.example.json`](../../../deploy/v3/deploy.example.json); the
fixture secret directory is
[`deploy/v3/secrets/`](../../../deploy/v3/secrets/README.md).

The document is JSON and every struct denies unknown fields. A v3-only field in
a v2 document, or a v2-only field in a v3 document, is a parse error rather than
a silent upgrade.

## 1. Profiles

| Profile | Failure domains | Anti-affinity | Evidence |
|---|---|---|---|
| `swarm-ha` | At least three distinct domains | Replicas <= distinct hosts and distinct domains; stateful members one per domain | `HA profile (unverified)` until HA-014 |
| `single-az` | One availability zone, asserted by the operator | Replicas <= distinct hosts and distinct domains; no three-domain minimum | `Single-zone HA (unverified)` |
| `lab-single-host` | One physical host | Co-location allowed for services; the ingress still needs distinct hosts for 443 | `Simulated HA`, never `Real HA` |

The `lab-single-host` profile is the only co-location exception. It runs three
VMs on one physical computer and proves simulated node failure only. A real
host/AZ failure claim is HA-014 evidence, never a profile label. The renderer
raises `max_replicas_per_node` for the lab profile so co-location can schedule;
the other profiles keep `1`.

## 2. Shape

The sample is authoritative. This excerpt repeats one host, one database, and
two services and omits the sixteen pinned infrastructure images to show the
other blocks; the sample lists every pin.

```json
{
  "v": "quantrule.deploy/v3",
  "profile": "swarm-ha",
  "name": "quantrule-ha",
  "image": { "reference": "registry.quantrule.dev/quantrule/server", "digest": "sha256:…" },
  "network": { "overlay": "quantrule", "encrypted": true },
  "ingress": {
    "dns": "quantrule.dev",
    "port": 443,
    "vip": "203.0.113.20",
    "interface": "eth0",
    "tls": "cert",
    "files": { "certificate": "/etc/quantrule/pki/ingress.crt", "key": "/etc/quantrule/pki/ingress-key.pem" },
    "replicas": 2,
    "hosts": ["edge-1", "edge-2"]
  },
  "auth": { "gateway_token_secret": "gateway_token" },
  "hosts": {
    "edge-1": { "address": "10.0.1.11", "failure_domain": "az1", "public_address": "203.0.113.10" }, "edge-2": { "address": "10.0.1.12", "failure_domain": "az2", "public_address": "203.0.113.11" },
    "core-1": { "address": "10.0.1.21", "failure_domain": "az1" },
    "compute-1": { "address": "10.0.1.31", "failure_domain": "az1" }, "compute-2": { "address": "10.0.1.32", "failure_domain": "az2" }, "compute-3": { "address": "10.0.1.33", "failure_domain": "az3" },
    "infra-1": { "address": "10.0.1.41", "failure_domain": "az1" }, "infra-2": { "address": "10.0.1.42", "failure_domain": "az2" }, "infra-3": { "address": "10.0.1.43", "failure_domain": "az3" }, "infra-4": { "address": "10.0.1.44", "failure_domain": "az4" }
  },  "swarm": { "managers": ["edge-1", "core-2", "compute-3"] },
  "endpoints": {
    "database": { "host": "postgres", "port": 5432, "sslmode": "verify-full", "tls_ca": "/etc/quantrule/pki/ca.crt" },
    "queue": { "host": "rabbitmq", "port": 5671, "scheme": "amqps", "tls_ca": "/etc/quantrule/pki/ca.crt" },
    "object_store": { "host": "minio", "port": 9000, "scheme": "https", "tls_ca": "/etc/quantrule/pki/ca.crt" }
  },
  "databases": {
    "gateway": { "database": "qr_gateway", "owner_role": "qr_gateway_owner", "app_role": "qr_gateway_app" }
  },
  "services": {
    "gateway": { "replicas": 2, "hosts": ["edge-1", "edge-2"], "port": 8080 },
    "model": { "replicas": 3, "hosts": ["compute-1", "compute-2", "compute-3"], "port": 8080 }
  },
  "stateful": {
    "postgres": { "nodes": ["infra-1", "infra-2", "infra-3"], "synchronous_standby": 1, "watchdog": "automatic", "pitr": true },
    "etcd": { "nodes": ["infra-1", "infra-2", "infra-3"] },
    "haproxy": { "nodes": ["edge-1", "edge-2"] },
    "rabbitmq": { "nodes": ["infra-1", "infra-2", "infra-3"], "quorum": true, "disk_free_limit_mib": 5120, "memory_high_watermark_percent": 60 },
    "minio": { "nodes": ["infra-1", "infra-2", "infra-3", "infra-4"], "drives": [1, 1, 1, 1], "parity": "EC:2" }
  },
  "observability": { "prometheus": { "host": "infra-1" }, "grafana": { "host": "infra-1" }, "loki": { "host": "infra-2" }, "alertmanager": { "host": "infra-2" } },
  "backup": { "pitr": true, "retention_days": 30, "object_replication": false },
  "secrets": {
    "files": {
      "gateway_token": "/etc/quantrule/secrets/gateway_token",
      "gateway_database_url": "/etc/quantrule/secrets/gateway_database_url",
      "postgres_superuser_password": "/etc/quantrule/secrets/postgres_superuser_password",
      "patroni_rest_password": "/etc/quantrule/secrets/patroni_rest_password",
      "rabbitmq_erlang_cookie": "/etc/quantrule/secrets/rabbitmq_erlang_cookie"
    }
  }
}
```

The sample has eleven hosts: two edge (ingress, gateway, keepalived), two core (data, evidence, simulation), three compute (model, strategy, backtest), and four infrastructure hosts (PostgreSQL, etcd, HAProxy on the edge pair, RabbitMQ, MinIO, observability). Compute hosts span `az1`..`az3`; the four infrastructure hosts span `az1`..`az4`, so four MinIO nodes each contribute one drive in a distinct failure domain with EC:2. The lab uses three VMs, two drives each, EC:3, so one node loss keeps the write quorum. The six HTTP services use `8080`; backtest uses `9090`; no service port is published.

## 3. Fields

| Field | Rule |
|---|---|
| `v` | `quantrule.deploy/v3` only. |
| `profile` | `swarm-ha`, `single-az`, or `lab-single-host`. |
| `name` | Optional lowercase DNS label; the default stack name is `quantrule`. |
| `image` | The server image every service runs; `sha256:` digest required. |
| `images` | Pins for `traefik`, `postgres`, `patroni`, `etcd`, `haproxy`, `keepalived`, `rabbitmq`, `minio`, `minio_client`, `prometheus`, `grafana`, `loki`, `alertmanager`, `node_exporter`, `promtail`, `blackbox_exporter`; no other key. `postgres` stays for the psql and migration jobs; `minio_client` runs the one-shot `mc`. |
| `network.overlay` | Overlay name; `encrypted` must be true. |
| `ingress.dns` | Exactly `quantrule.dev`. |
| `ingress.port` | Exactly `443`; no second public port. |
| `ingress.vip` | Required IPv4; not loopback, multicast, unspecified, or broadcast; distinct from every ingress host `public_address`. |
| `ingress.interface` | Optional Linux interface, default `eth0`; 1–15 characters, no whitespace or slash. |
| `ingress.tls` | `cert` only in v1.5.10; `acme` is reserved and refused with `deploy.ingress.tls`. |
| `ingress.replicas` / `hosts` | At least one; `swarm-ha` needs two and two failure domains; every host needs `public_address`. |
| `auth.gateway_token_secret` | A key in `secrets.files`; never an inline token. |
| `hosts` | Private `address`, `failure_domain`, and an optional `public_address` only on an ingress host. |
| `swarm.managers` | Odd (at least three), distinct, declared host keys; each host must be a Swarm manager. |
| `endpoints` | Structured PostgreSQL, queue, and object-store endpoints with absolute `tls_ca` paths and no userinfo. |
| `endpoints.queue.host` | The shared `rabbitmq` alias only; the HAProxy pair serves it. |
| `databases` | Exactly `gateway`, `catalog`, `evidence`, `simulation`, each with its fixed `qr_*` database, owner role, and app role. |
| `services` | Exactly `gateway`, `data`, `model`, `strategy`, `backtest`, `evidence`, `simulation`; `replicas`, `hosts`, `port`; no published port. |
| `stateful` | PostgreSQL, etcd, HAProxy, RabbitMQ, and MinIO nodes; `postgres.watchdog` is `off`, `automatic`, or `required` (default `off`); `synchronous_standby` must be exactly one; RabbitMQ is odd and >=3 with `disk_free_limit_mib` (default 5120) and `memory_high_watermark_percent` (default 60); MinIO parity is `auto` or `EC:<n>` and must keep the write quorum after one node loss; no `reschedule` field. |
| `observability` | `prometheus`, `grafana`, `loki`, `alertmanager`, each on a declared host; Prometheus and Alertmanager must be distinct; all four may not share one host. |
| `backup` | `pitr` true (configured, not verified); retention 1–3650 days; `object_replication` must be false in v1.5.10 because HA-013 owns off-host/archive and any second-site copy. |
| `secrets.files` | Non-empty raw name-to-absolute-path map; no inline value. The five stateful secrets, the two MinIO root secrets, and `object_store_access_key`/`object_store_secret_key` are required; the stateful values are URL-safe. |

## 4. Validation

`qr-deploy validate <deploy.json> [--files]` returns a stable `deploy.*` code, a
field dot path, and an actionable message. `--files` additionally checks local
files and credential quality.

| Rule | Code |
|---|---|
| Exactly seven services, four databases, one ingress, 443 only | `deploy.service.*`, `deploy.database.*`, `deploy.ingress.port` |
| VIP is IPv4, routable, and distinct; interface is a simple 1–15 character name | `deploy.ingress.vip`, `deploy.ingress.interface` |
| `ingress.tls` is `cert`; `acme` is reserved and refused until the future path lands | `deploy.ingress.tls` |
| No service port 443 and no published service port | `deploy.service.public_port` |
| `swarm-ha` has at least three failure domains | `deploy.profile.failure_domains` |
| Replicas <= distinct hosts and distinct domains in `swarm-ha` and `single-az` | `deploy.service.anti_affinity` |
| `single-az` keeps host and domain anti-affinity without a three-domain minimum | `deploy.service.anti_affinity` |
| `swarm.managers` is odd, at least three, distinct, and declared; each is a manager | `deploy.swarm.managers` |
| PostgreSQL/etcd/RabbitMQ >=3, HAProxy 2, MinIO >=4 in the HA profiles with one drive per node; the lab may use 2+ drives per node | `deploy.stateful.*` |
| MinIO parity is `auto` or `EC:<n>`; it must exceed the largest per-node drive count and satisfy `2 * parity <= total` | `deploy.stateful.minio.parity` |
| RabbitMQ list is odd, >=3; disk 256–10485760 MiB; memory 10–90% | `deploy.stateful.rabbitmq.nodes`, `.disk_free_limit_mib`, `.memory_high_watermark_percent` |
| `postgres.synchronous_standby` is exactly one; watchdog is a known mode | `deploy.stateful.postgres.synchronous_standby` |
| Stateful members use distinct hosts and, in `swarm-ha`, distinct domains | `deploy.stateful.anti_affinity` |
| Postgres `sslmode=verify-full`; queue `amqps`/5671 and the shared `rabbitmq` host; object store `https` | `deploy.endpoint.*` |
| CA and secret paths are absolute; no userinfo | `deploy.endpoint.host`, `deploy.file.absolute` |
| App and every infra unit carry a `sha256:` digest, never a tag or placeholder | `deploy.image.*`, `deploy.images.*` |
| `secrets.files` is non-empty and names every required secret | `deploy.secrets.*` |
| The five stateful secrets are present, distinct, strong, and URL-safe; the cookie is 32+ characters and the two MinIO root secrets are strong and distinct | `deploy.secrets.missing`, `deploy.secret.url_safe`, `deploy.secret.default_credential`, `deploy.secret.duplicate_credential`, `deploy.secret.weak_cookie` |
| `object_store_access_key` is the dedicated `qr_app` user; the secret is strong and distinct | `deploy.secret.object_store_user`, `deploy.secret.default_credential` |
| `queue_url` names `qr_queue`, the shared `rabbitmq` host, the default `%2f` vhost, and port 5671 | `deploy.secret.queue_user`, `.queue_host`, `.queue_vhost`, `.queue_port` |
| `--files`: regular, non-symlink, mode 600, <=8 KiB | `deploy.file.*`, `deploy.secret.mode` |
| `--files`: no default or duplicate role credentials | `deploy.secret.default_credential`, `deploy.secret.duplicate_credential` |
| Backup PITR, retention, the HA-013 replication boundary; four observability units on distinct alerting hosts | `deploy.backup.*`, `deploy.host.undeclared`, `deploy.observability.placement` |
| Unknown field, or a v2-only field in v3 | JSON parse error naming the field |

The required names are `queue_url`, `object_store_access_key`,
`object_store_secret_key`, `minio_root_user`, `minio_root_password`, the eight
`<service>_database_url`/`<service>_owner_database_url` files, the gateway token,
and the five stateful secrets. The URLs carry `sslmode=verify-full`; every role
has its own password. The stateful values are `stateful` scope, the MinIO root
pair is `minio` scope and never an app Swarm secret, and the Erlang cookie goes
only to brokers while the MinIO root files go only to MinIO hosts.

## 5. Render

```bash
qr-deploy render deploy.json --out out --target swarm|infra|all
```

`--target` is v3-only. v2 `render` keeps its original `--out`/`--secrets`
surface and is byte-identical; `--target` on v2 and a missing target on v3 are
usage errors.

| Target | Writes |
|---|---|
| `swarm` | `stack.yaml`, `quantrule_traefik.yml`, `traefik-dynamic.yml`, `bootstrap.sh`, `jobs/cluster-bootstrap.sh`, `jobs/db-bootstrap.sh`, `jobs/migrate.sh`, `secrets/`, `pki/`, `render.json` |
| `infra` | `infra/<host>.yaml`, `keepalived-<host>.conf`, `patroni-<host>.yml`, `etcd-<host>.conf.yml`, `haproxy-<host>.cfg`, `rabbitmq-<host>.conf`, `rabbitmq-enabled_plugins`, `minio-<host>.conf`, `minio-policy.json`, `prometheus.yml`, `alert-rules.yml`, `alertmanager.yml`, `loki.yml`, `blackbox.yml`, `promtail-<host>.yml`, `grafana/`, `observability-manifest.json`, `stateful-manifest.json`, `secrets/`, `pki/`, `render.json` |
| `all` | Both sets |

`secrets/` is the one copy of the source values: one mode-600 file per name and a
`manifest.json` with names, paths, and `app`/`stateful`/`minio` scope, never a
value or fingerprint. `pki/` copies the public CA bundles at 644, the Postgres,
etcd, RabbitMQ, and MinIO server leaves, and the ingress certificate/key in cert
mode. The `stateful-manifest.json` lists sources, absolute targets, modes,
owners, scopes, and a `hosts` selector; broker material only for broker hosts,
MinIO root/config/PKI only for MinIO hosts. The renderer rejects an unsafe path
component, removes only its own outputs, leaves an operator's file alone, and is
byte-identical across runs.

`stack.yaml` contains the seven services and one ingress. Every app image is
`reference@sha256:`; every service sets `QR_MIGRATE=0`, `QR_INTERNAL_ONLY=1`, its
bind and metrics ports, service URLs, object-store bucket, queue generation,
stage retention, and backtest concurrency. No secret value is present. The
gateway publishes no port; the ingress is the only `ports` entry: 443/443,
`mode: host`. External secrets and configs are mounted read-only; CA bundles
land under `/run/quantrule/pki`. In cert mode the ingress mounts the static and
dynamic Traefik configs, the certificate config, and the key secret at the paths
in [ingress](../2026-11-23/ingress.md). Placement uses the
host/domain/deployment labels, `max_replicas_per_node: 1` (except the lab), a
domain spread preference, `stop_grace_period: 40s`, and a bounded restart
policy. Each HTTP service has a `/health` check (`8080`; backtest `9090`); a
spare slot selects `start-first`, otherwise `stop-first` with `monitor: 180s`
and `max_failure_ratio: 0`. `backtest` and `evidence` use `restart_policy: any`
without a max attempt because they consume the queue; the other services keep
the bounded `on-failure` policy. The overlay is external, attachable, and
encrypted.

`infra/<host>.yaml` is a pinned unit per host. Every declared host carries
`node_exporter` (host root read-only) and `promtail` (Docker JSON logs, 30-day
Loki); the Prometheus host also carries `blackbox`; the four observability units
carry config/CA/Patroni-password mounts, health checks, and 50m×5 JSON logging.
Patroni/etcd/HAProxy/RabbitMQ/MinIO/keepalived keep their PLAT-002..005 units;
every unit has a pinned image, host-local binds, no `ports`, and
`restart: unless-stopped`. HAProxy serves `/primary` and native `8404/metrics`;
MinIO sets `MINIO_PROMETHEUS_AUTH_TYPE=public` on the overlay. Prometheus uses
30d/20GB, Alertmanager 120h with one `quantrule-null` receiver, Loki 30d, and
Grafana anonymous Viewer with no published port. Contracts:
[observability](../2026-11-23/observability.md), [Patroni](../2026-11-23/patroni-cluster.md),
[RabbitMQ](../2026-11-23/rabbitmq-cluster.md), [MinIO](../2026-11-23/minio-cluster.md).

`bootstrap.sh` runs on a manager: it checks the daemon and manager role,
creates or verifies the encrypted attachable overlay, creates the external
secrets/configs (including both Traefik configs) with a
`quantrule.source_sha256` drift label, labels nodes (`quantrule.host`, `.domain`,
`.profile`, `.deploy`, per-service, `.ingress`), refuses ambiguity/drain/foreign
deployment, runs the jobs before the stack, and waits for the pinned digest and
one 443 port. `--network-only` exits before secrets, configs, labels, jobs, or
stack; `tools/stateful.sh network` uses it. The five stateful and two MinIO root
secrets are excluded from app Swarm secrets, and no secret value is printed.
`tools/swarm.sh` wraps validate/render/bootstrap/status/upgrade/rollback/verify
and never removes a stack, secret, config, or volume. Contract:
[swarm-control](../2026-11-23/swarm-control.md).

`jobs/cluster-bootstrap.sh` creates the four databases and eight roles through
the superuser password file and the `postgres` endpoint. `jobs/db-bootstrap.sh`
applies each database's schema and grants through its **owner** URL, in gateway,
data, evidence, simulation order. `jobs/migrate.sh` runs the pinned app image
once per owner with `--migrate-only`, `QR_MIGRATE=0`, `PGSSLMODE=verify-full`,
`PGSSLROOTCERT`, and only the owner URL file. The cluster job uses the pinned
PostgreSQL image because the app image has no `psql`; the order is cluster -> db
-> migrate -> stack.

`render.json` records the contract, profile, evidence tier, target, stack name,
the canonical `sha256:` descriptor hash, the pinned image, and the sorted file list; no timestamp.

## 6. Check

`qr-deploy check deploy.json [--out <render-dir>] [--remote docker] [--overlay]`
validates the descriptor, placement, local files, and stateful rows, rejects a
second public port, and probes `/health`, `/ready`, and `/version` through the
ingress. The stateful rows check Patroni sync/rewind/archive/TLS/pg_hba/aliases/
REST/watchdog, the etcd mTLS cluster, the HAProxy `/primary` check and overlay
stats, the manifest SANs, and the Postgres/etcd certificate SANs through
`QR_DEPLOY_OPENSSL`. The queue rows check the TLS-only broker, three-node classic
cluster, alarms, queue default, Prometheus/JSON logs, HAProxy `rabbitmq`/5671,
the broker unit, the manifest host selector/modes, and the RabbitMQ leaf SANs;
they stay Docker-free. The MinIO rows check the exact config/parity/quorum, the
unique unit and alias, the HAProxy 9000 frontend/backend, the manifest
hosts/modes, the app URL/`_FILE`/`qr_app` path, and the MinIO leaf SANs. The
observability rows check placement, units, jobs/auth, 38 rules/HA matrix,
receiver, Loki/promtail, Grafana, manifest, logging, and internal metrics. `--out`
or `QR_DEPLOY_OUT` reads the rendered tree; without it the configs render in
memory and certificates come from the descriptor CA. A missing `openssl` is a
failed `limited` row.

`--remote docker` and `--overlay` are read-only opt-ins; the default check is
Docker-free, no row prints a secret or inspect body, and a missing object or
command is a failed row.

## 7. Token rotation, commands, and compatibility

A Swarm secret is immutable: rotate the gateway token with a new name and a
rolling update; the mount target and `QR_BEARER_TOKEN_FILE` stay the same.

| Command | Behavior |
|---|---|
| `validate [--files]` | v2 unchanged; v3 structural, then secret/ingress/PKI files. |
| `render --target swarm\|infra\|all` | v2 unchanged; v3 writes the selected set. |
| `check [--out <dir>] [--remote docker] [--overlay]` | v2 unchanged; v3 probes the ingress and checks the stateful rows; `--out` reads a rendered tree. |

v2 stays readable byte-identically. v3 is additive; a live move is a runbook, the
old stack stays until the checks pass, and rollback restores the previous
descriptor and image with forward-only migrations.
## 8. Evidence boundary

`lab-single-host` proves simulated node failure only. The `ha` tier loses one
worker, one manager, the VIP, the Patroni primary, one HAProxy, and one MinIO
node; it records the measured RTO, RPO 0 where the database path supports it,
the rejoin, the object write quorum, and value-free `Simulated HA` evidence.
`--skip-database`/`--skip-queue`/`--skip-object` leave that row `Unverified`;
the end-to-end digest is `Measured` only when `tools/verify.sh` succeeds,
otherwise `Unverified` or `Blocked`. `swarm-ha`/`single-az` stay unverified
until HA-014 and no output says `Real HA`; PLAT-002 ingress, PLAT-003 stateful,
PLAT-004 queue, PLAT-005 MinIO, PLAT-006 observability, HA-013 off-host.
## 9. Non-goals

No Kubernetes, autoscaling, accounts, tenancy, live trading, desktop packaging,
a second public port, or inline secrets. Ingress, keepalived, DNS/TLS,
certificates, headers, and body limits are in the
[ingress contract](../2026-11-23/ingress.md). Patroni/etcd/HAProxy is
[PLAT-003](../2026-11-23/patroni-cluster.md); RabbitMQ is
[PLAT-004](../2026-11-23/rabbitmq-cluster.md); MinIO is
[PLAT-005](../2026-11-23/minio-cluster.md); observability is
[PLAT-006](../2026-11-23/observability.md); signing is
PLAT-007; pgBackRest, off-host/archive copies, and any second-site replication
are HA-013; failure evidence is HA-014. TLS/CA is in
[internal TLS](../2026-11-23/internal-tls.md); secrets and rotation are in
[file-backed secrets](../2026-11-23/secret-files.md).
