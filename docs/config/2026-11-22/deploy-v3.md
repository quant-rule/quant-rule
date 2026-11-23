# quantrule.deploy/v3

- Date: 2026-11-22 (UTC)
- Status: Current for schema, validation, render, and check (HA-012); platform
  execution and unit configuration belong to PLAT-001..007
- Claim type: Behaviour Claim
- Issue: [#1110](https://github.com/quant-rule/quant-rule/issues/1110)
- Related: [ADR-HA](../../adr/2026-11-22/1082-ha-deployment-profile.md),
  [HA architecture](../../architecture/ha.md),
  [Multi-host](../../architecture/multi-host.md),
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
two services to show every block; the sample has the full sets.

```json
{
  "v": "quantrule.deploy/v3",
  "profile": "swarm-ha",
  "name": "quantrule-ha",
  "image": { "reference": "registry.quantrule.dev/quantrule/server", "digest": "sha256:…" },
  "images": {
    "traefik": { "reference": "traefik", "digest": "sha256:…" },
    "postgres": { "reference": "postgres", "digest": "sha256:…" },
    "etcd": { "reference": "quay.io/coreos/etcd", "digest": "sha256:…" },
    "haproxy": { "reference": "haproxy", "digest": "sha256:…" },
    "rabbitmq": { "reference": "rabbitmq", "digest": "sha256:…" },
    "minio": { "reference": "minio/minio", "digest": "sha256:…" },
    "prometheus": { "reference": "prom/prometheus", "digest": "sha256:…" },
    "grafana": { "reference": "grafana/grafana", "digest": "sha256:…" },
    "loki": { "reference": "grafana/loki", "digest": "sha256:…" },
    "alertmanager": { "reference": "prom/alertmanager", "digest": "sha256:…" }
  },
  "network": { "overlay": "quantrule", "encrypted": true },
  "ingress": {
    "dns": "quantrule.dev",
    "port": 443,
    "tls": "acme",
    "files": {},
    "replicas": 2,
    "hosts": ["edge-1", "edge-2"]
  },
  "auth": { "gateway_token_secret": "gateway_token" },
  "hosts": {
    "edge-1": { "address": "10.0.1.11", "failure_domain": "az1", "public_address": "203.0.113.10" },
    "edge-2": { "address": "10.0.1.12", "failure_domain": "az2", "public_address": "203.0.113.11" },
    "core-1": { "address": "10.0.1.21", "failure_domain": "az1" },
    "compute-1": { "address": "10.0.1.31", "failure_domain": "az1" },
    "compute-2": { "address": "10.0.1.32", "failure_domain": "az2" },
    "compute-3": { "address": "10.0.1.33", "failure_domain": "az3" },
    "infra-1": { "address": "10.0.1.41", "failure_domain": "az1" },
    "infra-2": { "address": "10.0.1.42", "failure_domain": "az2" },
    "infra-3": { "address": "10.0.1.43", "failure_domain": "az3" }
  },
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
    "postgres": { "nodes": ["infra-1", "infra-2", "infra-3"], "synchronous_standby": 1, "pitr": true },
    "etcd": { "nodes": ["infra-1", "infra-2", "infra-3"] },
    "haproxy": { "nodes": ["edge-1", "edge-2"] },
    "rabbitmq": { "nodes": ["infra-1", "infra-2", "infra-3"], "quorum": true },
    "minio": { "nodes": ["infra-1", "infra-2", "infra-3"], "drives": [2, 1, 1] }
  },
  "observability": {
    "prometheus": { "host": "infra-1" },
    "grafana": { "host": "infra-1" },
    "loki": { "host": "infra-2" },
    "alertmanager": { "host": "infra-2" }
  },
  "backup": { "pitr": true, "retention_days": 30, "object_replication": true },
  "secrets": {
    "files": {
      "gateway_token": "/etc/quantrule/secrets/gateway_token",
      "gateway_database_url": "/etc/quantrule/secrets/gateway_database_url"
    }
  }
}
```

The sample has ten hosts: two edge (ingress and gateway), two core (data,
evidence, simulation), three compute (model, strategy, backtest), and three
infrastructure hosts (PostgreSQL, etcd, HAProxy on the edge pair, RabbitMQ,
MinIO, observability). Compute hosts are deliberately spread across `az1`,
`az2`, and `az3`; the old two-host draft put both in `az3` and could not satisfy
anti-affinity. The six HTTP services use their internal `8080` port and
`backtest` uses `9090` for its internal listener; no service port is published.

## 3. Fields

| Field | Rule |
|---|---|
| `v` | `quantrule.deploy/v3` only. |
| `profile` | `swarm-ha`, `single-az`, or `lab-single-host`. |
| `name` | Optional lowercase DNS label; the default stack name is `quantrule`. |
| `image` | The server image every service runs; `sha256:` digest required. |
| `images` | Pins for `traefik`, `postgres`, `etcd`, `haproxy`, `rabbitmq`, `minio`, `prometheus`, `grafana`, `loki`, `alertmanager`; no other key. |
| `network.overlay` | Overlay name; `encrypted` must be true. |
| `ingress.dns` | Exactly `quantrule.dev`. |
| `ingress.port` | Exactly `443`; no second public port. |
| `ingress.tls` | `acme` (empty `files`) or `cert` (exactly `certificate` and `key`). |
| `ingress.replicas` / `hosts` | At least one; `swarm-ha` needs two and two failure domains; every host needs `public_address`. |
| `auth.gateway_token_secret` | A key in `secrets.files`; never an inline token. |
| `hosts` | Private `address`, `failure_domain`, and an optional `public_address` only on an ingress host. |
| `endpoints` | Structured PostgreSQL, queue, and object-store endpoints with absolute `tls_ca` paths and no userinfo. |
| `databases` | Exactly `gateway`, `catalog`, `evidence`, `simulation`, each with its fixed `qr_*` database, owner role, and app role. |
| `services` | Exactly `gateway`, `data`, `model`, `strategy`, `backtest`, `evidence`, `simulation`; `replicas`, `hosts`, `port`; no published port. |
| `stateful` | PostgreSQL, etcd, HAProxy, RabbitMQ, and MinIO nodes; no `reschedule` field. |
| `observability` | `prometheus`, `grafana`, `loki`, `alertmanager`, each on a declared host. |
| `backup` | `pitr` true; retention 1–3650 days; object replication true for `swarm-ha`. |
| `secrets.files` | Non-empty raw name-to-absolute-path map; no inline value. |

## 4. Validation

`qr-deploy validate <deploy.json> [--files]` returns a stable `deploy.*` code, a
field dot path, and an actionable message. `--files` additionally checks local
files and credential quality.

| Rule | Code |
|---|---|
| Exactly seven services, four databases, one ingress, 443 only | `deploy.service.*`, `deploy.database.*`, `deploy.ingress.port` |
| No service port 443 and no published service port | `deploy.service.public_port` |
| `swarm-ha` has at least three failure domains | `deploy.profile.failure_domains` |
| Replicas <= distinct hosts and distinct domains in `swarm-ha` and `single-az` | `deploy.service.anti_affinity` |
| `single-az` keeps host and domain anti-affinity without a three-domain minimum | `deploy.service.anti_affinity` |
| PostgreSQL/etcd/RabbitMQ >=3, HAProxy 2, MinIO >=3 with drives totalling >=4 | `deploy.stateful.*` |
| Stateful members use distinct hosts and, in `swarm-ha`, distinct domains | `deploy.stateful.anti_affinity` |
| Postgres `sslmode=verify-full`; queue `amqps`/5671; object store `https` | `deploy.endpoint.*` |
| CA and secret paths are absolute; no userinfo | `deploy.endpoint.host`, `deploy.file.absolute` |
| App and every infra unit carry a `sha256:` digest, never a tag or placeholder | `deploy.image.*`, `deploy.images.*` |
| `secrets.files` is non-empty and names every required secret | `deploy.secrets.*` |
| `--files`: regular, non-symlink, mode 600, <=8 KiB | `deploy.file.*`, `deploy.secret.mode` |
| `--files`: no default or duplicate role credentials | `deploy.secret.default_credential`, `deploy.secret.duplicate_credential` |
| Backup PITR, retention, and replication; four observability units | `deploy.backup.*`, `deploy.host.undeclared` |
| Unknown field, or a v2-only field in v3 | JSON parse error naming the field |

The required secret names are `queue_url`, `object_store_access_key`,
`object_store_secret_key`, `<service>_database_url`, and
`<service>_owner_database_url` for `gateway`, `data`, `evidence`, and
`simulation`, plus the `auth.gateway_token_secret`. The URLs carry
`sslmode=verify-full`; every role has its own password.

## 5. Render

```bash
qr-deploy render deploy.json --out out --target swarm|infra|all
```

`--target` is v3-only. v2 `render` keeps its original `--out`/`--secrets`
surface and is byte-identical; passing `--target` to v2 is a usage error. v3
without `--target` is a usage error.

| Target | Writes |
|---|---|
| `swarm` | `stack.yaml`, `bootstrap.sh`, `jobs/db-bootstrap.sh`, `jobs/migrate.sh`, `secrets/`, `pki/`, `render.json` |
| `infra` | `infra/<host>.yaml`, `secrets/`, `pki/`, `render.json` |
| `all` | Both sets |

`secrets/` is the one copy of the source `secrets.files` values: one mode-600
file per name, a `manifest.json` that lists names and paths but never a value or
fingerprint, and mode 700 on the directory. `pki/` copies the public CA bundles
at mode 644; for `tls: cert` it also copies the ingress certificate at 644 and
the key at 600. The renderer rejects a path component that contains a separator
or whitespace, removes only its own known outputs, and leaves an operator's
unrelated file in `--out` alone. A render is byte-identical across repeated runs.

`stack.yaml` contains the seven services and one ingress. Every app image is
`reference@sha256:`; every service sets `QR_MIGRATE=0`, `QR_INTERNAL_ONLY=1`, a
`QR_BIND` and `QR_METRICS_BIND`, the service URLs, object-store URL and bucket,
queue generation, stage retention, and backtest concurrency. No secret value is
present. The gateway publishes no port. The ingress is the only `ports` entry:
target 443, published 443, `mode: host`. External Swarm secrets and configs are
mounted read-only; CA bundles land under `/run/quantrule/pki`. For `tls: cert`,
the ingress mounts the certificate config at `/etc/quantrule/tls/tls.crt` and
the key secret at `/run/secrets/quantrule_ingress_key`; the PLAT-002 Traefik
config references those paths. Placement uses
the host/domain node labels, `max_replicas_per_node: 1` (except the lab
co-location case), a domain spread preference, `stop_grace_period: 40s`, a
start-first update with rollback, and a bounded restart policy. The overlay is
created by `bootstrap.sh` as attachable and encrypted; the stack references it
as external.

`infra/<host>.yaml` is a pinned skeleton per host for Patroni/PostgreSQL, etcd,
HAProxy, RabbitMQ, MinIO, Prometheus, Grafana, Loki, and Alertmanager. Each
unit has a pinned image, host-local bind volumes, config-file references, no
`ports`, `restart: unless-stopped`, and the attachable overlay. The unit
configuration itself is PLAT-003/004/005/006; the renderer does not invent it.
The HAProxy unit carries the database endpoint alias, RabbitMQ carries the queue
alias, and MinIO carries the object-store alias; the MinIO drive list is
rendered from `stateful.minio.drives`.

`bootstrap.sh` runs on a Swarm manager. It checks the daemon and manager role,
creates the overlay if absent and refuses one that is not overlay/attachable/
encrypted, creates the external secrets and the HA-012 configs, checks the
PLAT-002 Traefik config, labels nodes from the descriptor (host, domain,
profile, per-service, ingress), checks the label and domain counts, deploys the
stack, and runs the two jobs. It never reads a secret value or assumes a Docker
socket path. Execution is PLAT-001; rendering the script is HA-012.

`jobs/db-bootstrap.sh` applies each database's schema and grants through that
service's **owner** URL, in gateway, data, evidence, simulation order. PLAT-003
owns the cluster, the four databases (each owned by its `<service>_owner` role),
and the eight roles; the job converges the schemas and does not create roles. It uses the pinned PostgreSQL client
image because the application image intentionally contains no `psql`.
`jobs/migrate.sh` runs the pinned application image once per owner, in the same
order, with `--migrate-only`, `QR_MIGRATE=0`, `PGSSLMODE=verify-full`,
`PGSSLROOTCERT`, and only the owner URL file. An application URL and a
superuser URL are absent.

`render.json` records the contract, profile, evidence tier, target, the
canonical `sha256:` descriptor hash, the pinned application image, and the
sorted relative file list. It records no timestamp.

## 6. Check

```bash
qr-deploy check deploy.json [--remote docker] [--overlay]
```

The default check validates the descriptor and placement, reads every local
secret and ingress file with the `validate --files` rules, rejects a second public port
or an internal published port in the rendered model, and probes
`https://quantrule.dev/health`, `/ready`, and `/version` through the ingress.
The probe trusts the CA named by `endpoints.database.tls_ca` in `tls: cert`
mode and the bundled system roots in `acme` mode. It compares the reported
image digest with the descriptor. The report prints the profile, evidence tier,
image digest, placement counts, every secret name and path, and each probe
result; it never prints a secret value.

`--remote docker` and `--overlay` are explicit opt-ins. Their real inspector is
the PLAT-001/PLAT-006 seam; until it lands they return
`deploy.remote.unsupported` or `deploy.overlay.unsupported` and the check exits
non-zero. They never require Docker in the fast tier.

The deterministic test seam is `QR_DEPLOY_PROBE_FIXTURE=<json>`: a map from the
three ingress URLs to a recorded `status`/`body` or `error`. Tests use it to
prove a healthy result, a digest mismatch, an unhealthy state, an unreachable
host, and a wrong TLS/SAN without a network. It is not evidence and is never
set by the operator path.

## 7. Token rotation under Swarm

HA-009 rotates the staged file and sends `SIGHUP`, with no restart. A Swarm
secret is immutable, so the HA profile uses a new secret name and a rolling
service update instead: create `<token>_v2`, update the gateway service to
mount it, wait for `service update --detach=false`, and keep the old secret
until every replica has drained. This is the recorded HA-profile exception to
HA-009. The gateway token code is unchanged; both paths still use
`QR_BEARER_TOKEN_FILE`.

## 8. Commands

| Command | Behavior |
|---|---|
| `qr-deploy validate <deploy.json>` | Dispatches on `v`; v2 output is unchanged; v3 validates structurally. |
| `qr-deploy validate <deploy.json> --files` | v3 only; checks secret and ingress files locally. |
| `qr-deploy render <deploy.json> --out <dir>` | v2 unchanged; v3 requires `--target`. |
| `qr-deploy render <deploy.json> --out <dir> --target …` | v3 only; writes the selected set. |
| `qr-deploy check <deploy.json>` | v2 unchanged; v3 checks files and probes the ingress. |
| `qr-deploy check <deploy.json> --remote docker` | v3 opt-in; currently `deploy.remote.unsupported`. |
| `qr-deploy check <deploy.json> --overlay` | v3 opt-in; currently `deploy.overlay.unsupported`. |

## 9. Migration and compatibility

- v2 remains readable for the multi-host profile; v2 output and rules are
  unchanged.
- v3 is additive; a live deployment moves by runbook, not by editing the
  descriptor in place.
- The old stack remains available until the new stack passes the file,
  placement, and digest checks.
- Rollback restores the previous descriptor and image digest; migrations are
  forward-only.

## 10. Evidence boundary

`lab-single-host` renders three VMs on one physical computer. It proves simulated
node failure. It does not prove physical host, AZ, power, or disk-controller
failure. The release note, `render.json`, and `qr-deploy check` label the tier
`Simulated HA`; `swarm-ha` and `single-az` say unverified until HA-014. No output
says `Real HA`.

## 11. Non-goals

No Kubernetes, autoscaling, accounts, tenancy, live trading, desktop packaging,
a second public port, or inline secrets. Patroni/RabbitMQ/MinIO/observability
configuration is PLAT-003/004/005/006; registry signing is PLAT-007; backup is
HA-013; failure evidence is HA-014. Internal TLS and the CA model are in the
[internal TLS contract](../2026-11-23/internal-tls.md); secret files and rotation
are in the [file-backed secrets contract](../2026-11-23/secret-files.md).
